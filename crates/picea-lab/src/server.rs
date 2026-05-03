//! Local HTTP and SSE protocol for the C/S simulator.
//!
//! The server owns both artifact replay sessions and request-driven live
//! sessions. Live sessions keep the authoritative `World + SimulationPipeline`
//! on the Rust side so the web only consumes exported `FrameRecord` facts.

use std::{
    collections::BTreeMap,
    str::FromStr,
    sync::{Arc, Mutex},
};

use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, patch, post},
    Json, Router,
};
use picea::prelude::{SimulationPipeline, StepConfig, World};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{
    artifact::{frame_record_from_step, run_scenario, ArtifactFile, ArtifactStore, FrameRecord},
    scenario::{
        build_scenario, list_scenarios, CompoundProvenance, RunConfig, ScenarioId,
        ScenarioOverrides,
    },
    LabError,
};

#[derive(Clone)]
pub struct LabServerState {
    inner: Arc<Mutex<LabServerInner>>,
}

impl LabServerState {
    pub fn new(store: ArtifactStore) -> Self {
        Self {
            inner: Arc::new(Mutex::new(LabServerInner {
                store,
                next_session: 1,
                sessions: BTreeMap::new(),
            })),
        }
    }
}

struct LabServerInner {
    store: ArtifactStore,
    next_session: u64,
    sessions: BTreeMap<String, Arc<Mutex<SessionState>>>,
}

struct SessionState {
    record: SessionRecord,
    runtime: SessionRuntime,
}

enum SessionRuntime {
    ArtifactReplay,
    Live(LiveSessionState),
}

struct LiveSessionState {
    world: World,
    pipeline: SimulationPipeline,
    frames: Vec<FrameRecord>,
    compound_provenance: Vec<CompoundProvenance>,
}

/// A session is the server-owned handle for one scenario source and its
/// current override state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SessionRecord {
    pub id: String,
    pub scenario_id: ScenarioId,
    pub mode: SessionMode,
    pub status: SessionStatus,
    pub run_id: Option<String>,
    pub frame_count: usize,
    pub buffered_frame_count: usize,
    pub current_frame_index: usize,
    pub overrides: ScenarioOverrides,
    pub final_state_hash: Option<String>,
    pub manifest_artifact: Option<String>,
    pub final_snapshot_artifact: Option<String>,
    pub latest_frame: Option<FrameRecord>,
    pub last_error: Option<String>,
    #[serde(skip)]
    events: Vec<SessionEvent>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionMode {
    ArtifactReplay,
    LiveSession,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Created,
    Running,
    Paused,
    Completed,
    Failed,
}

#[derive(Clone, Debug, PartialEq)]
enum SessionEvent {
    Frame {
        frame_index: usize,
        state_hash: String,
    },
    Paused,
    Failed {
        message: String,
    },
}

#[derive(Clone, Debug, Deserialize)]
struct CreateSessionRequest {
    scenario_id: ScenarioId,
    #[serde(default = "default_session_frame_count")]
    frame_count: usize,
    #[serde(default)]
    overrides: ScenarioOverrides,
    #[serde(default)]
    mode: SessionMode,
}

#[derive(Clone, Debug, Deserialize)]
struct ControlRequest {
    action: String,
}

impl Default for SessionMode {
    fn default() -> Self {
        Self::ArtifactReplay
    }
}

pub fn app(state: LabServerState) -> Router {
    Router::new()
        .route("/api/scenarios", get(get_scenarios))
        .route("/api/sessions", post(create_session))
        .route("/api/sessions/:id", get(get_session))
        .route("/api/sessions/:id/control", post(control_session))
        .route("/api/sessions/:id/overrides", patch(patch_overrides))
        .route("/api/sessions/:id/events", get(session_events))
        .route("/api/runs/:id/artifacts/:file", get(get_artifact))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

fn default_session_frame_count() -> usize {
    120
}

async fn get_scenarios() -> Json<serde_json::Value> {
    Json(json!({ "scenarios": list_scenarios() }))
}

async fn create_session(
    State(state): State<LabServerState>,
    Json(request): Json<CreateSessionRequest>,
) -> Result<impl IntoResponse, LabHttpError> {
    let (id, store) = {
        let mut inner = state
            .inner
            .lock()
            .expect("lab state mutex should not poison");
        let id = format!("session-{}", inner.next_session);
        inner.next_session += 1;
        (id, inner.store.clone())
    };

    let mut session = SessionState {
        record: SessionRecord {
            id: id.clone(),
            scenario_id: request.scenario_id,
            mode: request.mode,
            status: SessionStatus::Created,
            run_id: None,
            frame_count: request.frame_count.max(1),
            buffered_frame_count: 0,
            current_frame_index: 0,
            overrides: request.overrides,
            final_state_hash: None,
            manifest_artifact: None,
            final_snapshot_artifact: None,
            latest_frame: None,
            last_error: None,
            events: Vec::new(),
        },
        runtime: SessionRuntime::ArtifactReplay,
    };

    match request.mode {
        SessionMode::ArtifactReplay => {
            run_artifact_session(&store, &mut session.record);
        }
        SessionMode::LiveSession => {
            session.runtime = SessionRuntime::Live(build_live_runtime(
                request.scenario_id,
                &session.record.overrides,
            )?);
        }
    }

    let response_session = session.record.clone();
    state
        .inner
        .lock()
        .expect("lab state mutex should not poison")
        .sessions
        .insert(id, Arc::new(Mutex::new(session)));

    Ok((
        StatusCode::CREATED,
        Json(json!({ "session": response_session })),
    ))
}

async fn get_session(
    State(state): State<LabServerState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, LabHttpError> {
    let session = get_session_handle(&state, &id)?;
    let session = session
        .lock()
        .expect("session mutex should not poison")
        .record
        .clone();
    Ok(Json(json!({ "session": session })))
}

async fn patch_overrides(
    State(state): State<LabServerState>,
    Path(id): Path<String>,
    Json(overrides): Json<ScenarioOverrides>,
) -> Result<Json<serde_json::Value>, LabHttpError> {
    let session = get_session_handle(&state, &id)?;
    let mut session = session.lock().expect("session mutex should not poison");
    if matches!(session.record.mode, SessionMode::LiveSession) {
        return Err(LabError::LiveOverridesUnavailable.into());
    }
    if let Some(frame_count) = overrides.frame_count {
        session.record.frame_count = frame_count.max(1);
    }
    if overrides.gravity.is_some() {
        session.record.overrides.gravity = overrides.gravity;
    }
    session.record.overrides.frame_count = overrides.frame_count;
    Ok(Json(json!({ "session": session.record.clone() })))
}

async fn control_session(
    State(state): State<LabServerState>,
    Path(id): Path<String>,
    Json(request): Json<ControlRequest>,
) -> Result<Json<serde_json::Value>, LabHttpError> {
    let session = get_session_handle(&state, &id)?;
    let store = state
        .inner
        .lock()
        .expect("lab state mutex should not poison")
        .store
        .clone();
    let mut session = session.lock().expect("session mutex should not poison");

    match session.record.mode {
        SessionMode::ArtifactReplay => {
            control_artifact_session(&store, &mut session.record, &request.action)?
        }
        SessionMode::LiveSession => control_live_session(&mut session, &request.action)?,
    }

    Ok(Json(json!({ "session": session.record.clone() })))
}

async fn session_events(
    State(state): State<LabServerState>,
    Path(id): Path<String>,
) -> Result<Response, LabHttpError> {
    let session = get_session_handle(&state, &id)?;
    let events = {
        let mut session = session.lock().expect("session mutex should not poison");
        std::mem::take(&mut session.record.events)
    };

    let body = format_session_events(events);

    let mut response = Response::new(Body::from(body));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/event-stream"),
    );
    Ok(response)
}

fn control_artifact_session(
    store: &ArtifactStore,
    session: &mut SessionRecord,
    action: &str,
) -> Result<(), LabHttpError> {
    match action {
        "play" | "run" => {
            session.status = SessionStatus::Running;
            if let Some(run_id) = session.run_id.as_deref() {
                let frame_hash = read_frame_hash(store, run_id, session.current_frame_index)
                    .unwrap_or_else(|| "unknown".to_owned());
                session.events.push(SessionEvent::Frame {
                    frame_index: session.current_frame_index,
                    state_hash: frame_hash,
                });
            }
        }
        "reset" => run_artifact_session(store, session),
        "step" => {
            session.status = SessionStatus::Paused;
            session.current_frame_index =
                (session.current_frame_index + 1).min(session.frame_count.saturating_sub(1));
            if let Some(run_id) = session.run_id.as_deref() {
                let frame_hash = read_frame_hash(store, run_id, session.current_frame_index)
                    .unwrap_or_else(|| "unknown".to_owned());
                session.events.push(SessionEvent::Frame {
                    frame_index: session.current_frame_index,
                    state_hash: frame_hash,
                });
            }
        }
        "pause" => {
            session.status = SessionStatus::Paused;
            session.events.push(SessionEvent::Paused);
        }
        _ => return Err(LabError::InvalidControlAction(action.to_owned()).into()),
    }
    Ok(())
}

fn control_live_session(session: &mut SessionState, action: &str) -> Result<(), LabHttpError> {
    match action {
        "play" | "run" => {
            if session.record.buffered_frame_count >= session.record.frame_count {
                session.record.status = SessionStatus::Completed;
            } else {
                session.record.status = SessionStatus::Running;
            }
        }
        "reset" => {
            session.runtime = SessionRuntime::Live(build_live_runtime(
                session.record.scenario_id,
                &session.record.overrides,
            )?);
            session.record.status = SessionStatus::Created;
            session.record.run_id = None;
            session.record.buffered_frame_count = 0;
            session.record.current_frame_index = 0;
            session.record.final_state_hash = None;
            session.record.manifest_artifact = None;
            session.record.final_snapshot_artifact = None;
            session.record.latest_frame = None;
            session.record.last_error = None;
            session.record.events.clear();
        }
        "step" => step_live_session(session)?,
        "pause" => {
            session.record.status = SessionStatus::Paused;
            session.record.events.push(SessionEvent::Paused);
        }
        _ => return Err(LabError::InvalidControlAction(action.to_owned()).into()),
    }
    Ok(())
}

fn step_live_session(session: &mut SessionState) -> Result<(), LabHttpError> {
    let SessionRuntime::Live(runtime) = &mut session.runtime else {
        return Err(LabError::InvalidControlAction("step".to_owned()).into());
    };
    if runtime.frames.len() >= session.record.frame_count {
        session.record.status = SessionStatus::Completed;
        return Ok(());
    }

    let frame_index = runtime.frames.len();
    let report = runtime.pipeline.step(&mut runtime.world);
    let frame = frame_record_from_step(
        &runtime.world,
        report,
        frame_index,
        &runtime.compound_provenance,
    )?;
    session.record.final_state_hash = Some(frame.state_hash.clone());
    session.record.buffered_frame_count = frame_index + 1;
    session.record.current_frame_index = frame_index;
    session.record.latest_frame = Some(frame.clone());
    session.record.last_error = None;
    runtime.frames.push(frame.clone());
    session.record.events.push(SessionEvent::Frame {
        frame_index,
        state_hash: frame.state_hash,
    });
    session.record.status = if session.record.buffered_frame_count >= session.record.frame_count {
        SessionStatus::Completed
    } else if matches!(session.record.status, SessionStatus::Running) {
        SessionStatus::Running
    } else {
        SessionStatus::Paused
    };
    Ok(())
}

fn build_live_runtime(
    scenario_id: ScenarioId,
    overrides: &ScenarioOverrides,
) -> Result<LiveSessionState, LabHttpError> {
    let scenario = build_scenario(scenario_id, overrides)?;
    Ok(LiveSessionState {
        world: scenario.world,
        pipeline: SimulationPipeline::new(StepConfig::default()),
        frames: Vec::new(),
        compound_provenance: scenario.compound_provenance,
    })
}

fn format_session_events(events: Vec<SessionEvent>) -> String {
    let mut body = String::new();
    for event in events {
        match event {
            SessionEvent::Frame {
                frame_index,
                state_hash,
            } => {
                body.push_str("event: frame\n");
                body.push_str(&format!(
                    "data: {}\n\n",
                    json!({ "frame_index": frame_index, "state_hash": state_hash })
                ));
            }
            SessionEvent::Paused => {
                body.push_str("event: paused\n");
                body.push_str("data: {}\n\n");
            }
            SessionEvent::Failed { message } => {
                body.push_str("event: failed\n");
                body.push_str(&format!("data: {}\n\n", json!({ "message": message })));
            }
        }
    }
    if body.is_empty() {
        body.push_str("event: idle\n");
        body.push_str("data: {\"message\":\"no events available\"}\n\n");
    }
    body
}

async fn get_artifact(
    State(state): State<LabServerState>,
    Path((id, file)): Path<(String, String)>,
) -> Result<Response, LabHttpError> {
    let artifact_file = ArtifactFile::from_str(&file)?;
    let store = state
        .inner
        .lock()
        .expect("lab state mutex should not poison")
        .store
        .clone();
    let bytes = store.read_artifact(&id, &file)?;
    let mut response = Response::new(Body::from(bytes));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(match artifact_file {
            ArtifactFile::Frames => "application/x-ndjson",
            _ => "application/json",
        }),
    );
    Ok(response)
}

fn get_session_handle(
    state: &LabServerState,
    id: &str,
) -> Result<Arc<Mutex<SessionState>>, LabHttpError> {
    state
        .inner
        .lock()
        .expect("lab state mutex should not poison")
        .sessions
        .get(id)
        .cloned()
        .ok_or_else(|| LabError::SessionNotFound(id.to_owned()).into())
}

fn run_artifact_session(store: &ArtifactStore, session: &mut SessionRecord) {
    session.status = SessionStatus::Running;
    session.events.clear();
    session.buffered_frame_count = 0;
    session.latest_frame = None;
    let mut overrides = session.overrides.clone();
    overrides.frame_count = Some(session.frame_count);
    match run_scenario(
        store,
        RunConfig {
            scenario_id: session.scenario_id,
            frame_count: session.frame_count,
            run_id: None,
            overrides,
        },
    ) {
        Ok(result) => {
            session.status = SessionStatus::Completed;
            session.run_id = Some(result.manifest.run_id);
            session.frame_count = result.manifest.frame_count;
            session.buffered_frame_count = result.manifest.frame_count;
            session.current_frame_index = 0;
            session.final_state_hash = Some(result.manifest.final_state_hash);
            session.manifest_artifact = Some(ArtifactFile::Manifest.file_name().to_owned());
            session.final_snapshot_artifact =
                Some(ArtifactFile::FinalSnapshot.file_name().to_owned());
            session.last_error = None;
            session.events = result
                .frames
                .iter()
                .map(|frame| SessionEvent::Frame {
                    frame_index: frame.frame_index,
                    state_hash: frame.state_hash.clone(),
                })
                .collect();
        }
        Err(error) => {
            let message = error.to_string();
            session.status = SessionStatus::Failed;
            session.run_id = None;
            session.buffered_frame_count = 0;
            session.current_frame_index = 0;
            session.final_state_hash = None;
            session.manifest_artifact = None;
            session.final_snapshot_artifact = None;
            session.latest_frame = None;
            session.last_error = Some(message.clone());
            session.events = vec![SessionEvent::Failed { message }];
        }
    }
}

fn read_frame_hash(store: &ArtifactStore, run_id: &str, frame_index: usize) -> Option<String> {
    let bytes = store
        .read_artifact(run_id, ArtifactFile::Frames.file_name())
        .ok()?;
    let text = String::from_utf8(bytes).ok()?;
    let line = text.lines().nth(frame_index)?;
    let frame = serde_json::from_str::<FrameRecord>(line).ok()?;
    Some(frame.state_hash)
}

#[derive(Debug)]
struct LabHttpError(LabError);

impl From<LabError> for LabHttpError {
    fn from(value: LabError) -> Self {
        Self(value)
    }
}

impl IntoResponse for LabHttpError {
    fn into_response(self) -> Response {
        let status = match self.0 {
            LabError::SessionNotFound(_) => StatusCode::NOT_FOUND,
            LabError::UnknownScenario(_)
            | LabError::InvalidArtifactFile(_)
            | LabError::InvalidControlAction(_)
            | LabError::LiveOverridesUnavailable => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(json!({ "error": self.0.to_string() }))).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::{format_session_events, SessionEvent};

    #[test]
    fn empty_sse_stream_is_idle_not_failed() {
        let body = format_session_events(Vec::<SessionEvent>::new());

        assert!(body.contains("event: idle"));
        assert!(
            !body.contains("event: failed"),
            "an empty event queue is not a failed simulation"
        );
    }
}
