//! Local HTTP and SSE protocol for the C/S simulator.
//!
//! The server owns sessions and artifact lookup, but each reset delegates to the
//! headless runner. That keeps live protocol state separate from physics state.

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
use serde::{Deserialize, Serialize};
use serde_json::json;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{
    artifact::{run_scenario, ArtifactFile, ArtifactStore, FrameRecord},
    scenario::{list_scenarios, RunConfig, ScenarioId, ScenarioOverrides},
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
    sessions: BTreeMap<String, SessionRecord>,
}

/// A session is the server-owned handle for one scenario run and its
/// current override state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SessionRecord {
    pub id: String,
    pub scenario_id: ScenarioId,
    pub status: SessionStatus,
    pub run_id: Option<String>,
    pub frame_count: usize,
    pub current_frame_index: usize,
    pub overrides: ScenarioOverrides,
    pub final_state_hash: Option<String>,
    pub last_error: Option<String>,
    #[serde(skip)]
    events: Vec<SessionEvent>,
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
}

#[derive(Clone, Debug, Deserialize)]
struct ControlRequest {
    action: String,
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
    let mut session = {
        let mut inner = state
            .inner
            .lock()
            .expect("lab state mutex should not poison");
        let id = format!("session-{}", inner.next_session);
        inner.next_session += 1;
        SessionRecord {
            id,
            scenario_id: request.scenario_id,
            status: SessionStatus::Created,
            run_id: None,
            frame_count: request.frame_count.max(1),
            current_frame_index: 0,
            overrides: request.overrides,
            final_state_hash: None,
            last_error: None,
            events: Vec::new(),
        }
    };

    let store = state
        .inner
        .lock()
        .expect("lab state mutex should not poison")
        .store
        .clone();
    run_session(&store, &mut session);

    let response_session = session.clone();
    state
        .inner
        .lock()
        .expect("lab state mutex should not poison")
        .sessions
        .insert(session.id.clone(), session);

    Ok((
        StatusCode::CREATED,
        Json(json!({ "session": response_session })),
    ))
}

async fn get_session(
    State(state): State<LabServerState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, LabHttpError> {
    let session = state
        .inner
        .lock()
        .expect("lab state mutex should not poison")
        .sessions
        .get(&id)
        .cloned()
        .ok_or_else(|| LabError::SessionNotFound(id))?;
    Ok(Json(json!({ "session": session })))
}

async fn patch_overrides(
    State(state): State<LabServerState>,
    Path(id): Path<String>,
    Json(overrides): Json<ScenarioOverrides>,
) -> Result<Json<serde_json::Value>, LabHttpError> {
    let mut inner = state
        .inner
        .lock()
        .expect("lab state mutex should not poison");
    let session = inner
        .sessions
        .get_mut(&id)
        .ok_or_else(|| LabError::SessionNotFound(id.clone()))?;
    if let Some(frame_count) = overrides.frame_count {
        session.frame_count = frame_count.max(1);
    }
    if overrides.gravity.is_some() {
        session.overrides.gravity = overrides.gravity;
    }
    session.overrides.frame_count = overrides.frame_count;
    Ok(Json(json!({ "session": session.clone() })))
}

async fn control_session(
    State(state): State<LabServerState>,
    Path(id): Path<String>,
    Json(request): Json<ControlRequest>,
) -> Result<Json<serde_json::Value>, LabHttpError> {
    let mut inner = state
        .inner
        .lock()
        .expect("lab state mutex should not poison");
    let store = inner.store.clone();
    let session = inner
        .sessions
        .get_mut(&id)
        .ok_or_else(|| LabError::SessionNotFound(id.clone()))?;

    match request.action.as_str() {
        "play" | "run" => {
            session.status = SessionStatus::Running;
            if let Some(run_id) = session.run_id.as_deref() {
                let frame_hash = read_frame_hash(&store, run_id, session.current_frame_index)
                    .unwrap_or_else(|| "unknown".to_owned());
                session.events.push(SessionEvent::Frame {
                    frame_index: session.current_frame_index,
                    state_hash: frame_hash,
                });
            }
        }
        "reset" => run_session(&store, session),
        "step" => {
            session.status = SessionStatus::Paused;
            session.current_frame_index =
                (session.current_frame_index + 1).min(session.frame_count.saturating_sub(1));
            if let Some(run_id) = session.run_id.as_deref() {
                let frame_hash = read_frame_hash(&store, run_id, session.current_frame_index)
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
        _ => return Err(LabError::InvalidControlAction(request.action).into()),
    }
    Ok(Json(json!({ "session": session.clone() })))
}

async fn session_events(
    State(state): State<LabServerState>,
    Path(id): Path<String>,
) -> Result<Response, LabHttpError> {
    let session = state
        .inner
        .lock()
        .expect("lab state mutex should not poison")
        .sessions
        .get(&id)
        .cloned()
        .ok_or_else(|| LabError::SessionNotFound(id))?;

    let mut body = String::new();
    for event in session.events {
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
        body.push_str("event: failed\n");
        body.push_str("data: {\"message\":\"no events available\"}\n\n");
    }

    let mut response = Response::new(Body::from(body));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/event-stream"),
    );
    Ok(response)
}

async fn get_artifact(
    State(state): State<LabServerState>,
    Path((id, file)): Path<(String, String)>,
) -> Result<Response, LabHttpError> {
    let artifact_file = ArtifactFile::from_str(&file)?;
    let bytes = state
        .inner
        .lock()
        .expect("lab state mutex should not poison")
        .store
        .read_artifact(&id, &file)?;
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

fn run_session(store: &ArtifactStore, session: &mut SessionRecord) {
    session.status = SessionStatus::Running;
    session.events.clear();
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
            session.current_frame_index = 0;
            session.final_state_hash = Some(result.manifest.final_state_hash);
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
            session.current_frame_index = 0;
            session.final_state_hash = None;
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
            | LabError::InvalidControlAction(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(json!({ "error": self.0.to_string() }))).into_response()
    }
}
