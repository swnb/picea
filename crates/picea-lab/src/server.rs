//! Local HTTP and SSE protocol for the C/S simulator.
//!
//! The server owns both artifact replay sessions and request-driven live
//! sessions. Live sessions keep the authoritative `World + SimulationPipeline`
//! on the Rust side so the web only consumes exported `FrameRecord` facts.

use std::{
    collections::{BTreeMap, BTreeSet},
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
use picea::prelude::{
    BodyHandle, BodyPatch, BodyType, QueryPipeline, SimulationPipeline, Vector, World,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tower_http::{catch_panic::CatchPanicLayer, cors::CorsLayer, trace::TraceLayer};

use crate::{
    artifact::{
        frame_record_from_step_with_provenance, refreshed_frame_record_from_world, run_scenario,
        ArtifactFile, ArtifactStore, FrameRecord, LivePerturbationCommitOutcome,
        LivePerturbationProvenance, LiveQuerySyncStatus, RunResult,
    },
    scenario::{
        build_scenario, default_runtime_config_for_scenario, list_scenarios, CompoundProvenance,
        RunConfig, ScenarioId, ScenarioOverrides, ScenarioRuntimeConfig,
    },
    LabError, LabResult,
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
    /// Monotonic counter bumped when an artifact reset starts. A reset only
    /// writes its result back if it is still the latest one, so overlapping
    /// resets resolve to a last-initiated-wins order instead of a torn
    /// interleave while the session mutex is released for the blocking re-run.
    artifact_reset_generation: u64,
}

// The `Live` variant dwarfs the unit `ArtifactReplay` variant, but every session
// stores exactly one `SessionRuntime` behind `Arc<Mutex<SessionState>>`, so
// boxing the live state would only add indirection on the live hot path without
// meaningful memory savings for the handful of local sessions.
#[allow(clippy::large_enum_variant)]
enum SessionRuntime {
    ArtifactReplay,
    Live(LiveSessionState),
}

struct LiveSessionState {
    world: World,
    pipeline: SimulationPipeline,
    substeps_per_frame: usize,
    query: QueryPipeline,
    frames: Vec<FrameRecord>,
    retained_frame_start: usize,
    compound_provenance: Vec<CompoundProvenance>,
    perturbation_provenance: Vec<LivePerturbationProvenance>,
    velocity_preview_cache: BTreeMap<String, CachedVelocityPerturbationPreview>,
    used_velocity_preview_actions: BTreeSet<String>,
}

/// A session is the server-owned handle for one scenario source and its
/// current override state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SessionRecord {
    pub id: String,
    pub scenario_id: ScenarioId,
    pub mode: SessionMode,
    pub status: SessionStatus,
    pub session_epoch: u64,
    pub run_id: Option<String>,
    pub frame_count: usize,
    pub produced_frame_count: usize,
    pub buffered_frame_count: usize,
    pub current_frame_index: usize,
    pub retained_frame_start: usize,
    pub retained_frame_end_exclusive: usize,
    pub live_buffer_capacity: usize,
    pub live_unbounded: bool,
    pub overrides: ScenarioOverrides,
    pub effective_runtime_config: ScenarioRuntimeConfig,
    pub final_state_hash: Option<String>,
    pub manifest_artifact: Option<String>,
    pub final_snapshot_artifact: Option<String>,
    pub latest_frame: Option<FrameRecord>,
    pub last_error: Option<String>,
    #[serde(skip)]
    events: Vec<SessionEvent>,
    /// Per-frame `state_hash` values for the completed artifact run, cached in
    /// memory so play/step index directly instead of re-reading the whole
    /// `frames.jsonl` on every step. Rebuilt on each (re)run and skipped from
    /// the wire contract.
    #[serde(skip)]
    frame_state_hashes: Vec<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionMode {
    #[default]
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
    #[serde(default = "default_live_buffer_capacity")]
    live_buffer_capacity: usize,
    #[serde(default)]
    live_unbounded: bool,
    #[serde(default)]
    overrides: ScenarioOverrides,
    #[serde(default)]
    mode: SessionMode,
}

#[derive(Clone, Debug, Deserialize)]
struct ControlRequest {
    action: String,
    #[serde(default)]
    detail: LiveFrameDetail,
}

#[derive(Clone, Debug, Deserialize)]
struct LiveGravityPatchRequest {
    gravity: serde_json::Value,
    #[serde(default)]
    session_epoch: Option<u64>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum LiveFrameDetail {
    #[default]
    Summary,
    Full,
}

#[derive(Clone, Debug, Deserialize)]
struct VelocityPerturbationPreviewRequest {
    action_id: String,
    world_revision: picea::prelude::WorldRevision,
    session_epoch: u64,
    body_handle: BodyHandle,
    frame_index: usize,
    requested_delta: serde_json::Value,
    #[serde(default)]
    wake_intent: Option<bool>,
}

#[derive(Clone, Debug, Deserialize)]
struct VelocityPerturbationCommitRequest {
    action_id: String,
    world_revision: picea::prelude::WorldRevision,
    session_epoch: u64,
    body_handle: BodyHandle,
    frame_index: usize,
    requested_delta: serde_json::Value,
    #[serde(default)]
    computed_target_velocity: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize)]
struct VelocityPerturbationPreviewResponse {
    action_id: String,
    session_id: String,
    world_revision: picea::prelude::WorldRevision,
    session_epoch: u64,
    body_handle: BodyHandle,
    frame_index: usize,
    before_velocity: Option<Vector>,
    requested_delta: Option<Vector>,
    computed_target_velocity: Option<Vector>,
    wake_intent: bool,
    rejection_reason: Option<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
struct VelocityPerturbationCommitResponse {
    action_id: String,
    session_id: String,
    accepted: bool,
    world_revision: picea::prelude::WorldRevision,
    session_epoch: u64,
    body_handle: BodyHandle,
    frame_index: usize,
    before_velocity: Option<Vector>,
    requested_delta: Option<Vector>,
    computed_target_velocity: Option<Vector>,
    wake_intent: bool,
    query_sync_status: Option<LiveQuerySyncStatus>,
    rejection_reason: Option<&'static str>,
}

#[derive(Clone, Debug)]
struct CachedVelocityPerturbationPreview {
    action_id: String,
    world_revision: picea::prelude::WorldRevision,
    session_epoch: u64,
    body_handle: BodyHandle,
    frame_index: usize,
    before_velocity: Vector,
    requested_delta: Vector,
    computed_target_velocity: Vector,
    wake_intent: bool,
}

pub fn app(state: LabServerState) -> Router {
    Router::new()
        .route("/api/scenarios", get(get_scenarios))
        .route("/api/sessions", post(create_session))
        .route("/api/sessions/:id", get(get_session))
        .route("/api/sessions/:id/control", post(control_session))
        .route("/api/sessions/:id/gravity", post(apply_live_gravity))
        .route("/api/sessions/:id/frames/:index", get(get_live_frame))
        .route(
            "/api/sessions/:id/velocity-perturbations/preview",
            post(preview_velocity_perturbation),
        )
        .route(
            "/api/sessions/:id/velocity-perturbations/commit",
            post(commit_velocity_perturbation),
        )
        .route("/api/sessions/:id/overrides", patch(patch_overrides))
        .route("/api/sessions/:id/events", get(session_events))
        .route("/api/runs/:id/artifacts/:file", get(get_artifact))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        // Outermost layer: a panicking handler becomes a 500 instead of
        // propagating and cascading through poisoned shared mutexes.
        .layer(CatchPanicLayer::new())
        .with_state(state)
}

fn default_session_frame_count() -> usize {
    120
}

fn default_live_buffer_capacity() -> usize {
    600
}

async fn get_scenarios() -> Json<serde_json::Value> {
    Json(json!({ "scenarios": list_scenarios() }))
}

async fn create_session(
    State(state): State<LabServerState>,
    Json(request): Json<CreateSessionRequest>,
) -> Result<impl IntoResponse, LabHttpError> {
    let id = {
        let mut inner = state
            .inner
            .lock()
            .expect("lab state mutex should not poison");
        let id = format!("session-{}", inner.next_session);
        inner.next_session += 1;
        id
    };

    let mut record = SessionRecord {
        id: id.clone(),
        scenario_id: request.scenario_id,
        mode: request.mode,
        status: SessionStatus::Created,
        session_epoch: 0,
        run_id: None,
        frame_count: request.frame_count.max(1),
        produced_frame_count: 0,
        buffered_frame_count: 0,
        current_frame_index: 0,
        retained_frame_start: 0,
        retained_frame_end_exclusive: 0,
        live_buffer_capacity: request.live_buffer_capacity.max(1),
        live_unbounded: request.live_unbounded,
        overrides: request.overrides,
        effective_runtime_config: default_runtime_config_for_scenario(request.scenario_id),
        final_state_hash: None,
        manifest_artifact: None,
        final_snapshot_artifact: None,
        latest_frame: None,
        last_error: None,
        events: Vec::new(),
        frame_state_hashes: Vec::new(),
    };

    let runtime = match request.mode {
        SessionMode::ArtifactReplay => {
            let store = state
                .inner
                .lock()
                .expect("lab state mutex should not poison")
                .store
                .clone();
            let config = artifact_run_config(&record);
            let result = tokio::task::spawn_blocking(move || run_scenario(&store, config))
                .await
                .expect("artifact run worker should not panic");
            apply_artifact_run_result(&mut record, result);
            SessionRuntime::ArtifactReplay
        }
        SessionMode::LiveSession => {
            let (runtime, effective_runtime_config) =
                build_live_runtime(request.scenario_id, &record.overrides)?;
            record.effective_runtime_config = effective_runtime_config;
            SessionRuntime::Live(runtime)
        }
    };

    let response_session = record.clone();
    let session = SessionState {
        record,
        runtime,
        artifact_reset_generation: 0,
    };
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
    session.record.overrides.scene_params = overrides.scene_params;
    session.record.effective_runtime_config =
        crate::scenario::effective_runtime_config_for_scenario(
            session.record.scenario_id,
            &session.record.overrides,
        )?;
    Ok(Json(json!({ "session": session.record.clone() })))
}

async fn control_session(
    State(state): State<LabServerState>,
    Path(id): Path<String>,
    Json(request): Json<ControlRequest>,
) -> Result<Json<serde_json::Value>, LabHttpError> {
    let session = get_session_handle(&state, &id)?;

    let is_artifact_reset = {
        let guard = session.lock().expect("session mutex should not poison");
        matches!(guard.record.mode, SessionMode::ArtifactReplay) && request.action == "reset"
    };

    // The artifact reset re-runs the full simulation and writes five files. Run
    // it on a blocking worker with the session mutex released, then swap the
    // result back atomically under the lock.
    if is_artifact_reset {
        let store = state
            .inner
            .lock()
            .expect("lab state mutex should not poison")
            .store
            .clone();
        let (config, generation) = {
            let mut guard = session.lock().expect("session mutex should not poison");
            guard.artifact_reset_generation = guard.artifact_reset_generation.wrapping_add(1);
            (
                artifact_run_config(&guard.record),
                guard.artifact_reset_generation,
            )
        };
        let result = tokio::task::spawn_blocking(move || run_scenario(&store, config))
            .await
            .expect("artifact reset worker should not panic");
        let mut guard = session.lock().expect("session mutex should not poison");
        // Only the latest reset writes back; an older overlapping reset defers to
        // it so concurrent readers never observe a torn record.
        if guard.artifact_reset_generation == generation {
            apply_artifact_run_result(&mut guard.record, result);
        }
        return Ok(Json(build_control_response(
            &guard,
            &request.action,
            request.detail,
        )));
    }

    let mut guard = session.lock().expect("session mutex should not poison");
    match guard.record.mode {
        SessionMode::ArtifactReplay => {
            control_artifact_session(&mut guard.record, &request.action)?
        }
        SessionMode::LiveSession => control_live_session(&mut guard, &request.action)?,
    }
    Ok(Json(build_control_response(
        &guard,
        &request.action,
        request.detail,
    )))
}

async fn apply_live_gravity(
    State(state): State<LabServerState>,
    Path(id): Path<String>,
    Json(request): Json<LiveGravityPatchRequest>,
) -> Result<Json<serde_json::Value>, LabHttpError> {
    let gravity = parse_gravity_vector(&request.gravity).ok_or_else(|| {
        LabHttpError::bad_request("gravity patch requires a finite [x, y] vector")
    })?;
    let session = get_session_handle(&state, &id)?;
    let mut session = session.lock().expect("session mutex should not poison");
    if !matches!(session.record.mode, SessionMode::LiveSession) {
        return Err(LabHttpError::bad_request(
            "gravity patch is only available for live_session",
        ));
    }
    if let Some(epoch) = request.session_epoch {
        if epoch != session.record.session_epoch {
            return Err(LabHttpError::bad_request(format!(
                "stale session epoch: expected {}, got {}",
                session.record.session_epoch, epoch
            )));
        }
    }
    apply_live_gravity_patch(&mut session, gravity)?;
    Ok(Json(build_control_response(
        &session,
        "gravity",
        LiveFrameDetail::Full,
    )))
}

async fn get_live_frame(
    State(state): State<LabServerState>,
    Path((id, index)): Path<(String, usize)>,
) -> Result<Json<serde_json::Value>, LabHttpError> {
    let session = get_session_handle(&state, &id)?;
    let session = session.lock().expect("session mutex should not poison");
    let SessionRuntime::Live(runtime) = &session.runtime else {
        return Err(LabHttpError::bad_request(
            "live frame lookup is only available for live_session",
        ));
    };
    if index < runtime.retained_frame_start {
        return Err(LabHttpError::gone_json(
            format!("live frame {index} was evicted from session {id}"),
            json!({
                "error_kind": "live_frame_evicted",
                "frame_index": index,
                "retained_frame_start": runtime.retained_frame_start,
                "retained_frame_end_exclusive": live_retained_frame_end(runtime),
            }),
        ));
    }
    let Some(frame) = live_frame_by_absolute_index(runtime, index) else {
        return Err(LabHttpError::not_found(format!(
            "live frame {index} not found for session {id}"
        )));
    };
    Ok(Json(json!({
        "session_id": &session.record.id,
        "session_epoch": session.record.session_epoch,
        "frame_index": frame.frame_index,
        "world_revision": frame.report.revision,
        "frame": frame,
    })))
}

async fn preview_velocity_perturbation(
    State(state): State<LabServerState>,
    Path(id): Path<String>,
    Json(request): Json<VelocityPerturbationPreviewRequest>,
) -> Result<Json<serde_json::Value>, LabHttpError> {
    let session = get_session_handle(&state, &id)?;
    let mut session = session.lock().expect("session mutex should not poison");
    let preview = preview_live_velocity_perturbation(&mut session, request);
    Ok(Json(json!({ "preview": preview })))
}

async fn commit_velocity_perturbation(
    State(state): State<LabServerState>,
    Path(id): Path<String>,
    Json(request): Json<VelocityPerturbationCommitRequest>,
) -> Result<Json<serde_json::Value>, LabHttpError> {
    let session = get_session_handle(&state, &id)?;
    let mut session = session.lock().expect("session mutex should not poison");
    let commit = commit_live_velocity_perturbation(&mut session, request)?;
    Ok(Json(
        json!({ "commit": commit, "session": session.record.clone() }),
    ))
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

fn control_artifact_session(session: &mut SessionRecord, action: &str) -> Result<(), LabHttpError> {
    match action {
        "play" | "run" => {
            session.status = SessionStatus::Running;
            if session.run_id.is_some() {
                let frame_hash = artifact_frame_hash(session, session.current_frame_index);
                session.events.push(SessionEvent::Frame {
                    frame_index: session.current_frame_index,
                    state_hash: frame_hash,
                });
            }
        }
        "step" => {
            session.status = SessionStatus::Paused;
            session.current_frame_index =
                (session.current_frame_index + 1).min(session.frame_count.saturating_sub(1));
            if session.run_id.is_some() {
                let frame_hash = artifact_frame_hash(session, session.current_frame_index);
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
        // `reset` is intercepted in `control_session` so it can run off the async
        // worker; it never reaches this synchronous artifact path.
        _ => return Err(LabError::InvalidControlAction(action.to_owned()).into()),
    }
    Ok(())
}

fn artifact_frame_hash(session: &SessionRecord, frame_index: usize) -> String {
    session
        .frame_state_hashes
        .get(frame_index)
        .cloned()
        .unwrap_or_else(|| "unknown".to_owned())
}

fn control_live_session(session: &mut SessionState, action: &str) -> Result<(), LabHttpError> {
    match action {
        "play" | "run" => {
            if !session.record.live_unbounded
                && session.record.produced_frame_count >= session.record.frame_count
            {
                session.record.status = SessionStatus::Completed;
            } else {
                session.record.status = SessionStatus::Running;
            }
        }
        "reset" => {
            let (runtime, effective_runtime_config) =
                build_live_runtime(session.record.scenario_id, &session.record.overrides)?;
            session.runtime = SessionRuntime::Live(runtime);
            session.record.effective_runtime_config = effective_runtime_config;
            session.record.status = SessionStatus::Created;
            session.record.session_epoch = session.record.session_epoch.saturating_add(1);
            session.record.run_id = None;
            session.record.produced_frame_count = 0;
            session.record.buffered_frame_count = 0;
            session.record.current_frame_index = 0;
            session.record.retained_frame_start = 0;
            session.record.retained_frame_end_exclusive = 0;
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

fn build_control_response(session: &SessionState, _action: &str, detail: LiveFrameDetail) -> Value {
    let mut response_session = session.record.clone();
    let live_frame_summary = match session.record.mode {
        SessionMode::LiveSession => session
            .record
            .latest_frame
            .as_ref()
            .map(|frame| build_live_frame_summary(&session.record, frame)),
        SessionMode::ArtifactReplay => None,
    };
    if matches!(session.record.mode, SessionMode::LiveSession)
        && matches!(detail, LiveFrameDetail::Summary)
    {
        response_session.latest_frame = None;
    }
    json!({
        "session": response_session,
        "live_frame_summary": live_frame_summary,
    })
}

fn build_live_frame_summary(session: &SessionRecord, frame: &FrameRecord) -> Value {
    // The live runtime keeps the full FrameRecord authoritative in memory. This
    // summary is only a transport-layer projection for the hot playback path.
    json!({
        "kind": "summary",
        "frame_index": frame.frame_index,
        "simulated_time": frame.simulated_time,
        "state_hash": &frame.state_hash,
        "session_id": &session.id,
        "session_epoch": session.session_epoch,
        "world_revision": frame.report.revision,
        "status": session.status,
        "buffered_frame_count": session.buffered_frame_count,
        "produced_frame_count": session.produced_frame_count,
        "retained_frame_start": session.retained_frame_start,
        "retained_frame_end_exclusive": session.retained_frame_end_exclusive,
        "live_buffer_capacity": session.live_buffer_capacity,
        "live_unbounded": session.live_unbounded,
        "frame_count": session.frame_count,
        "snapshot": {
            "meta": &frame.snapshot.meta,
            "bodies": &frame.snapshot.bodies,
            "colliders": &frame.snapshot.colliders,
            "joints": &frame.snapshot.joints,
            "primitives": &frame.snapshot.primitives,
            "stats": &frame.snapshot.stats,
            "contacts": Value::Null,
            "manifolds": Value::Null,
            "islands": Value::Null,
            "broadphase_tree": Value::Null,
        },
        "stats": &frame.stats,
        "report": Value::Null,
        "events": Value::Null,
        "diagnostics": Value::Null,
        "compound_provenance": Value::Null,
        "perturbation_provenance": Value::Null,
    })
}

fn preview_live_velocity_perturbation(
    session: &mut SessionState,
    request: VelocityPerturbationPreviewRequest,
) -> VelocityPerturbationPreviewResponse {
    let world_revision = match &session.runtime {
        SessionRuntime::Live(runtime) => runtime.world.revision(),
        SessionRuntime::ArtifactReplay => request.world_revision,
    };
    let mut response = VelocityPerturbationPreviewResponse {
        action_id: request.action_id,
        session_id: session.record.id.clone(),
        world_revision,
        session_epoch: session.record.session_epoch,
        body_handle: request.body_handle,
        frame_index: session.record.current_frame_index,
        before_velocity: None,
        requested_delta: None,
        computed_target_velocity: None,
        wake_intent: request.wake_intent.unwrap_or(true),
        rejection_reason: None,
    };

    let reject = |response: &mut VelocityPerturbationPreviewResponse, reason| {
        response.rejection_reason = Some(reason);
    };

    if !matches!(session.runtime, SessionRuntime::Live(_)) {
        reject(&mut response, "not_live_session");
        return response;
    }
    if let SessionRuntime::Live(runtime) = &session.runtime {
        if runtime
            .used_velocity_preview_actions
            .contains(&response.action_id)
        {
            reject(&mut response, "reused_action");
            return response;
        }
    }
    match session.record.status {
        SessionStatus::Created | SessionStatus::Paused => {}
        SessionStatus::Running => {
            reject(&mut response, "session_running");
            return response;
        }
        SessionStatus::Completed => {
            reject(&mut response, "session_completed");
            return response;
        }
        SessionStatus::Failed => {
            reject(&mut response, "session_failed");
            return response;
        }
    }
    if request.world_revision != world_revision {
        reject(&mut response, "stale_world_revision");
        return response;
    }
    if request.session_epoch != session.record.session_epoch {
        reject(&mut response, "stale_session_epoch");
        return response;
    }
    if request.frame_index != session.record.current_frame_index {
        reject(&mut response, "stale_frame");
        return response;
    }

    let Some(latest_frame) = session.record.latest_frame.as_ref() else {
        reject(&mut response, "missing_frame_snapshot");
        return response;
    };
    if latest_frame.frame_index != session.record.current_frame_index {
        reject(&mut response, "stale_frame");
        return response;
    }
    if latest_frame.snapshot.meta.revision != Some(world_revision) {
        reject(&mut response, "stale_world_revision");
        return response;
    }
    let Some(body) = latest_frame
        .snapshot
        .bodies
        .iter()
        .find(|body| body.handle == request.body_handle)
    else {
        reject(&mut response, "invalid_body_handle");
        return response;
    };

    let requested_delta = match parse_velocity_vector(&request.requested_delta) {
        Some(delta) => delta,
        None => {
            reject(&mut response, "invalid_velocity_delta");
            return response;
        }
    };
    response.requested_delta = Some(requested_delta);

    match body.body_type {
        BodyType::Dynamic => {}
        BodyType::Static => {
            reject(&mut response, "static_body");
            return response;
        }
        BodyType::Kinematic => {
            reject(&mut response, "kinematic_body");
            return response;
        }
    }

    let before_velocity = body.linear_velocity;
    let computed_target_velocity = before_velocity + requested_delta;
    if !computed_target_velocity.x().is_finite() || !computed_target_velocity.y().is_finite() {
        reject(&mut response, "invalid_velocity_delta");
        return response;
    }
    response.before_velocity = Some(before_velocity);
    response.computed_target_velocity = Some(computed_target_velocity);
    if let SessionRuntime::Live(runtime) = &mut session.runtime {
        runtime.velocity_preview_cache.insert(
            response.action_id.clone(),
            CachedVelocityPerturbationPreview {
                action_id: response.action_id.clone(),
                world_revision: response.world_revision,
                session_epoch: response.session_epoch,
                body_handle: response.body_handle,
                frame_index: response.frame_index,
                before_velocity,
                requested_delta,
                computed_target_velocity,
                wake_intent: response.wake_intent,
            },
        );
    }
    response
}

fn commit_live_velocity_perturbation(
    session: &mut SessionState,
    request: VelocityPerturbationCommitRequest,
) -> Result<VelocityPerturbationCommitResponse, LabHttpError> {
    let world_revision = match &session.runtime {
        SessionRuntime::Live(runtime) => runtime.world.revision(),
        SessionRuntime::ArtifactReplay => request.world_revision,
    };
    let mut response = VelocityPerturbationCommitResponse {
        action_id: request.action_id.clone(),
        session_id: session.record.id.clone(),
        accepted: false,
        world_revision,
        session_epoch: session.record.session_epoch,
        body_handle: request.body_handle,
        frame_index: session.record.current_frame_index,
        before_velocity: None,
        requested_delta: None,
        computed_target_velocity: None,
        wake_intent: true,
        query_sync_status: None,
        rejection_reason: None,
    };

    let reject = |response: &mut VelocityPerturbationCommitResponse, reason| {
        response.rejection_reason = Some(reason);
    };

    if !matches!(session.runtime, SessionRuntime::Live(_)) {
        reject(&mut response, "not_live_session");
        return Ok(response);
    }
    match session.record.status {
        SessionStatus::Paused => {}
        SessionStatus::Created => {
            reject(&mut response, "session_created");
            return Ok(response);
        }
        SessionStatus::Running => {
            reject(&mut response, "session_running");
            return Ok(response);
        }
        SessionStatus::Completed => {
            reject(&mut response, "session_completed");
            return Ok(response);
        }
        SessionStatus::Failed => {
            reject(&mut response, "session_failed");
            return Ok(response);
        }
    }
    if let SessionRuntime::Live(runtime) = &session.runtime {
        if runtime
            .used_velocity_preview_actions
            .contains(&request.action_id)
        {
            reject(&mut response, "reused_action");
            return Ok(response);
        }
    }
    if request.world_revision != world_revision {
        reject(&mut response, "stale_world_revision");
        return Ok(response);
    }
    if request.session_epoch != session.record.session_epoch {
        reject(&mut response, "stale_session_epoch");
        return Ok(response);
    }
    if request.frame_index != session.record.current_frame_index {
        reject(&mut response, "stale_frame");
        return Ok(response);
    }

    let Some(latest_frame) = session.record.latest_frame.as_ref() else {
        reject(&mut response, "missing_frame_snapshot");
        return Ok(response);
    };
    if latest_frame.frame_index != session.record.current_frame_index {
        reject(&mut response, "stale_frame");
        return Ok(response);
    }
    if latest_frame.snapshot.meta.revision != Some(world_revision) {
        reject(&mut response, "stale_world_revision");
        return Ok(response);
    }
    let Some(body) = latest_frame
        .snapshot
        .bodies
        .iter()
        .find(|body| body.handle == request.body_handle)
    else {
        reject(&mut response, "invalid_body_handle");
        return Ok(response);
    };

    let requested_delta = match parse_velocity_vector(&request.requested_delta) {
        Some(delta) => delta,
        None => {
            reject(&mut response, "invalid_velocity_delta");
            return Ok(response);
        }
    };
    response.requested_delta = Some(requested_delta);

    match body.body_type {
        BodyType::Dynamic => {}
        BodyType::Static => {
            reject(&mut response, "static_body");
            return Ok(response);
        }
        BodyType::Kinematic => {
            reject(&mut response, "kinematic_body");
            return Ok(response);
        }
    }

    let before_velocity = body.linear_velocity;
    let computed_target_velocity = before_velocity + requested_delta;
    if !computed_target_velocity.x().is_finite() || !computed_target_velocity.y().is_finite() {
        reject(&mut response, "invalid_velocity_delta");
        return Ok(response);
    }
    if let Some(value) = request.computed_target_velocity.as_ref() {
        let Some(client_target) = parse_velocity_vector(value) else {
            reject(&mut response, "stale_preview_action");
            return Ok(response);
        };
        if !vectors_match(client_target, computed_target_velocity) {
            reject(&mut response, "stale_preview_action");
            return Ok(response);
        }
    }
    response.before_velocity = Some(before_velocity);
    response.computed_target_velocity = Some(computed_target_velocity);

    let cache_status = {
        let SessionRuntime::Live(runtime) = &session.runtime else {
            unreachable!("live runtime checked above");
        };
        runtime
            .velocity_preview_cache
            .get(&request.action_id)
            .ok_or("missing_preview_action")
    };
    let cached_preview = match cache_status {
        Ok(cached_preview) => cached_preview.clone(),
        Err(reason) => {
            reject(&mut response, reason);
            return Ok(response);
        }
    };
    response.wake_intent = cached_preview.wake_intent;

    if cached_preview.action_id != request.action_id
        || cached_preview.world_revision != request.world_revision
        || cached_preview.session_epoch != request.session_epoch
        || cached_preview.body_handle != request.body_handle
        || cached_preview.frame_index != request.frame_index
        || !vectors_match(cached_preview.before_velocity, before_velocity)
        || !vectors_match(cached_preview.requested_delta, requested_delta)
        || !vectors_match(
            cached_preview.computed_target_velocity,
            computed_target_velocity,
        )
    {
        reject(&mut response, "stale_preview_action");
        return Ok(response);
    }

    let base_frame = latest_frame.clone();
    {
        let SessionRuntime::Live(runtime) = &mut session.runtime else {
            unreachable!("live runtime checked above");
        };
        if runtime
            .world
            .apply_body_patch(
                request.body_handle,
                BodyPatch {
                    linear_velocity: Some(computed_target_velocity),
                    wake: cached_preview.wake_intent,
                    ..BodyPatch::default()
                },
            )
            .is_err()
        {
            reject(&mut response, "body_patch_failed");
            return Ok(response);
        }
        let new_revision = runtime.world.revision();
        runtime.query.sync(&runtime.world);
        let query_sync_status = if runtime.query.revision() == Some(new_revision) {
            LiveQuerySyncStatus::Synced
        } else {
            LiveQuerySyncStatus::Stale
        };
        session.record.session_epoch = session.record.session_epoch.saturating_add(1);
        let provenance = LivePerturbationProvenance {
            action_id: request.action_id.clone(),
            session_id: session.record.id.clone(),
            world_revision: new_revision,
            session_epoch: session.record.session_epoch,
            body_handle: request.body_handle,
            frame_index: request.frame_index,
            before_velocity,
            requested_delta,
            computed_target_velocity,
            wake_intent: cached_preview.wake_intent,
            commit_outcome: LivePerturbationCommitOutcome::Accepted,
            query_sync_status,
        };
        runtime.perturbation_provenance.push(provenance);
        let frame = refreshed_frame_record_from_world(
            &runtime.world,
            &base_frame,
            &runtime.compound_provenance,
            &runtime.perturbation_provenance,
        )?;
        let Some(retained_index) = live_retained_index(runtime, request.frame_index) else {
            reject(&mut response, "stale_frame");
            return Ok(response);
        };
        runtime.frames.truncate(retained_index + 1);
        runtime.frames[retained_index] = frame.clone();
        runtime.velocity_preview_cache.remove(&request.action_id);
        runtime
            .used_velocity_preview_actions
            .insert(request.action_id.clone());
        refresh_live_record_window(&mut session.record, runtime);
        session.record.current_frame_index = request.frame_index;
        session.record.latest_frame = Some(frame.clone());
        session.record.final_state_hash = Some(frame.state_hash.clone());
        refresh_queued_frame_events_for_edit(
            &mut session.record.events,
            request.frame_index,
            &frame.state_hash,
        );
        session.record.last_error = None;
        response.world_revision = new_revision;
        response.session_epoch = session.record.session_epoch;
        response.frame_index = request.frame_index;
        response.accepted = true;
        response.query_sync_status = Some(query_sync_status);
    }
    Ok(response)
}

fn refresh_queued_frame_events_for_edit(
    events: &mut Vec<SessionEvent>,
    frame_index: usize,
    refreshed_state_hash: &str,
) {
    for event in events.iter_mut() {
        if let SessionEvent::Frame {
            frame_index: queued_frame_index,
            state_hash,
        } = event
        {
            if *queued_frame_index == frame_index {
                *state_hash = refreshed_state_hash.to_owned();
            }
        }
    }
    events.retain(|event| {
        !matches!(
            event,
            SessionEvent::Frame {
                frame_index: queued_frame_index,
                ..
            } if *queued_frame_index > frame_index
        )
    });
}

fn parse_velocity_vector(value: &serde_json::Value) -> Option<Vector> {
    let (x, y) = if let Some(values) = value.as_array() {
        let [x, y] = values.as_slice() else {
            return None;
        };
        (x.as_f64()?, y.as_f64()?)
    } else {
        let object = value.as_object()?;
        (object.get("x")?.as_f64()?, object.get("y")?.as_f64()?)
    };
    if !x.is_finite() || !y.is_finite() {
        return None;
    }
    let delta = Vector::new(x as f32, y as f32);
    if !delta.x().is_finite() || !delta.y().is_finite() {
        return None;
    }
    let magnitude = delta.length();
    if !magnitude.is_finite() || magnitude <= f32::EPSILON {
        return None;
    }
    Some(delta)
}

fn vectors_match(left: Vector, right: Vector) -> bool {
    (left.x() - right.x()).abs() <= 1.0e-5 && (left.y() - right.y()).abs() <= 1.0e-5
}

fn parse_gravity_vector(value: &serde_json::Value) -> Option<Vector> {
    let (x, y) = if let Some(values) = value.as_array() {
        let [x, y] = values.as_slice() else {
            return None;
        };
        (x.as_f64()?, y.as_f64()?)
    } else {
        let object = value.as_object()?;
        (object.get("x")?.as_f64()?, object.get("y")?.as_f64()?)
    };
    if !x.is_finite() || !y.is_finite() {
        return None;
    }
    let gravity = Vector::new(x as f32, y as f32);
    if !gravity.x().is_finite() || !gravity.y().is_finite() {
        return None;
    }
    Some(gravity)
}

fn apply_live_gravity_patch(
    session: &mut SessionState,
    gravity: Vector,
) -> Result<(), LabHttpError> {
    let SessionRuntime::Live(runtime) = &mut session.runtime else {
        return Err(LabHttpError::bad_request(
            "gravity patch is only available for live_session",
        ));
    };
    runtime
        .world
        .set_gravity(gravity)
        .map_err(|error| LabHttpError::bad_request(error.to_string()))?;
    runtime.query.sync(&runtime.world);
    runtime.velocity_preview_cache.clear();

    session.record.session_epoch = session.record.session_epoch.saturating_add(1);
    session.record.overrides.gravity = Some([gravity.x(), gravity.y()]);
    session.record.last_error = None;

    let Some(base_frame) = session.record.latest_frame.clone() else {
        return Ok(());
    };
    let frame = refreshed_frame_record_from_world(
        &runtime.world,
        &base_frame,
        &runtime.compound_provenance,
        &runtime.perturbation_provenance,
    )?;
    let Some(retained_index) = live_retained_index(runtime, base_frame.frame_index) else {
        return Err(LabHttpError::gone_json(
            format!(
                "live frame {} was evicted before gravity patch refresh",
                base_frame.frame_index
            ),
            json!({
                "error_kind": "live_frame_evicted",
                "frame_index": base_frame.frame_index,
                "retained_frame_start": runtime.retained_frame_start,
                "retained_frame_end_exclusive": live_retained_frame_end(runtime),
            }),
        ));
    };
    runtime.frames.truncate(retained_index + 1);
    runtime.frames[retained_index] = frame.clone();
    refresh_live_record_window(&mut session.record, runtime);
    session.record.produced_frame_count = frame.frame_index.saturating_add(1);
    session.record.current_frame_index = frame.frame_index;
    session.record.latest_frame = Some(frame.clone());
    session.record.final_state_hash = Some(frame.state_hash.clone());
    refresh_queued_frame_events_for_edit(
        &mut session.record.events,
        frame.frame_index,
        &frame.state_hash,
    );
    Ok(())
}

fn step_live_session(session: &mut SessionState) -> Result<(), LabHttpError> {
    let SessionRuntime::Live(runtime) = &mut session.runtime else {
        return Err(LabError::InvalidControlAction("step".to_owned()).into());
    };
    if !session.record.live_unbounded
        && session.record.produced_frame_count >= session.record.frame_count
    {
        session.record.status = SessionStatus::Completed;
        return Ok(());
    }

    let frame_index = session.record.produced_frame_count;
    let mut report = runtime.pipeline.step(&mut runtime.world);
    for _ in 1..runtime.substeps_per_frame.max(1) {
        report = runtime.pipeline.step(&mut runtime.world);
    }
    runtime.query.sync(&runtime.world);
    let frame = frame_record_from_step_with_provenance(
        &runtime.world,
        report,
        frame_index,
        &runtime.compound_provenance,
        &runtime.perturbation_provenance,
    )?;
    let state_hash = frame.state_hash.clone();
    session.record.final_state_hash = Some(state_hash.clone());
    session.record.produced_frame_count = frame_index.saturating_add(1);
    session.record.current_frame_index = frame_index;
    session.record.last_error = None;
    runtime.frames.push(frame.clone());
    evict_live_frames_outside_capacity(runtime, session.record.live_buffer_capacity);
    purge_stale_live_previews(runtime, session.record.current_frame_index);
    refresh_live_record_window(&mut session.record, runtime);
    session.record.latest_frame = Some(frame);
    session.record.events.push(SessionEvent::Frame {
        frame_index,
        state_hash,
    });
    session.record.status = if !session.record.live_unbounded
        && session.record.produced_frame_count >= session.record.frame_count
    {
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
) -> Result<(LiveSessionState, ScenarioRuntimeConfig), LabHttpError> {
    let scenario = build_scenario(scenario_id, overrides)?;
    let effective_runtime_config = scenario.effective_runtime_config.clone();
    let mut query = QueryPipeline::new();
    query.sync(&scenario.world);
    Ok((
        LiveSessionState {
            world: scenario.world,
            pipeline: SimulationPipeline::new(effective_runtime_config.step),
            substeps_per_frame: effective_runtime_config.substeps_per_frame.max(1),
            query,
            frames: Vec::new(),
            retained_frame_start: 0,
            compound_provenance: scenario.compound_provenance,
            perturbation_provenance: Vec::new(),
            velocity_preview_cache: BTreeMap::new(),
            used_velocity_preview_actions: BTreeSet::new(),
        },
        effective_runtime_config,
    ))
}

fn live_retained_frame_end(runtime: &LiveSessionState) -> usize {
    runtime
        .retained_frame_start
        .saturating_add(runtime.frames.len())
}

fn live_retained_index(runtime: &LiveSessionState, frame_index: usize) -> Option<usize> {
    if frame_index < runtime.retained_frame_start {
        return None;
    }
    let index = frame_index - runtime.retained_frame_start;
    (index < runtime.frames.len()).then_some(index)
}

fn live_frame_by_absolute_index(
    runtime: &LiveSessionState,
    frame_index: usize,
) -> Option<&FrameRecord> {
    let retained_index = live_retained_index(runtime, frame_index)?;
    runtime.frames.get(retained_index)
}

fn evict_live_frames_outside_capacity(runtime: &mut LiveSessionState, capacity: usize) {
    let capacity = capacity.max(1);
    let overflow = runtime.frames.len().saturating_sub(capacity);
    if overflow > 0 {
        runtime.frames.drain(0..overflow);
        runtime.retained_frame_start = runtime.retained_frame_start.saturating_add(overflow);
    }
}

fn purge_stale_live_previews(runtime: &mut LiveSessionState, current_frame_index: usize) {
    let retained_start = runtime.retained_frame_start;
    let retained_end = live_retained_frame_end(runtime);
    runtime.velocity_preview_cache.retain(|_, preview| {
        preview.frame_index == current_frame_index
            && preview.frame_index >= retained_start
            && preview.frame_index < retained_end
    });
}

fn refresh_live_record_window(record: &mut SessionRecord, runtime: &LiveSessionState) {
    record.buffered_frame_count = runtime.frames.len();
    record.retained_frame_start = runtime.retained_frame_start;
    record.retained_frame_end_exclusive = live_retained_frame_end(runtime);
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
    let bytes = tokio::task::spawn_blocking(move || store.read_artifact(&id, &file))
        .await
        .expect("artifact read worker should not panic")?;
    let mut response = Response::new(Body::from(bytes));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(artifact_file.content_type()),
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

/// Build the deterministic run configuration for an artifact session. The
/// heavy `run_scenario` call is executed off the async worker; this only
/// captures the inputs under the session lock.
fn artifact_run_config(session: &SessionRecord) -> RunConfig {
    let mut overrides = session.overrides.clone();
    overrides.frame_count = Some(session.frame_count);
    RunConfig {
        scenario_id: session.scenario_id,
        frame_count: session.frame_count,
        run_id: None,
        overrides,
    }
}

/// Apply the outcome of a blocking artifact run back onto the session record.
/// This is the single write-back point for both create and reset, so the whole
/// transition happens atomically under the session lock.
fn apply_artifact_run_result(session: &mut SessionRecord, result: LabResult<RunResult>) {
    match result {
        Ok(result) => {
            session.status = SessionStatus::Completed;
            session.run_id = Some(result.manifest.run_id);
            session.frame_count = result.manifest.frame_count;
            session.produced_frame_count = result.manifest.frame_count;
            session.buffered_frame_count = result.manifest.frame_count;
            session.effective_runtime_config = result.manifest.effective_runtime_config.clone();
            session.current_frame_index = 0;
            session.retained_frame_start = 0;
            session.retained_frame_end_exclusive = result.manifest.frame_count;
            session.final_state_hash = Some(result.manifest.final_state_hash);
            session.manifest_artifact = Some(ArtifactFile::Manifest.file_name().to_owned());
            session.final_snapshot_artifact =
                Some(ArtifactFile::FinalSnapshot.file_name().to_owned());
            session.last_error = None;
            session.frame_state_hashes = result
                .frames
                .iter()
                .map(|frame| frame.state_hash.clone())
                .collect();
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
            session.produced_frame_count = 0;
            session.buffered_frame_count = 0;
            session.current_frame_index = 0;
            session.retained_frame_start = 0;
            session.retained_frame_end_exclusive = 0;
            session.final_state_hash = None;
            session.manifest_artifact = None;
            session.final_snapshot_artifact = None;
            session.latest_frame = None;
            session.frame_state_hashes = Vec::new();
            session.last_error = Some(message.clone());
            session.events = vec![SessionEvent::Failed { message }];
        }
    }
}

#[derive(Debug)]
struct LabHttpError {
    status: StatusCode,
    message: String,
    payload: Option<Value>,
}

impl From<LabError> for LabHttpError {
    fn from(value: LabError) -> Self {
        let status = match value {
            LabError::SessionNotFound(_) => StatusCode::NOT_FOUND,
            LabError::UnknownScenario(_)
            | LabError::InvalidArtifactFile(_)
            | LabError::InvalidControlAction(_)
            | LabError::LiveOverridesUnavailable => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        Self {
            status,
            message: value.to_string(),
            payload: None,
        }
    }
}

impl IntoResponse for LabHttpError {
    fn into_response(self) -> Response {
        let mut payload = self
            .payload
            .unwrap_or_else(|| json!({ "error": self.message }));
        if let Some(object) = payload.as_object_mut() {
            object
                .entry("error")
                .or_insert_with(|| Value::String(self.message));
        }
        (self.status, Json(payload)).into_response()
    }
}

impl LabHttpError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
            payload: None,
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
            payload: None,
        }
    }

    fn gone_json(message: impl Into<String>, payload: Value) -> Self {
        Self {
            status: StatusCode::GONE,
            message: message.into(),
            payload: Some(payload),
        }
    }
}

#[cfg(test)]
mod tests {
    use picea::prelude::{
        BodyDesc, BodyType, DebugSnapshot, DebugSnapshotOptions, StepConfig, StepReport, StepStats,
        WorldDesc,
    };
    use serde_json::json;

    use super::{
        format_session_events, preview_live_velocity_perturbation, FrameRecord, LiveSessionState,
        QueryPipeline, ScenarioId, ScenarioOverrides, ScenarioRuntimeConfig, SessionEvent,
        SessionMode, SessionRecord, SessionRuntime, SessionState, SessionStatus,
        SimulationPipeline, World,
    };

    #[test]
    fn empty_sse_stream_is_idle_not_failed() {
        let body = format_session_events(Vec::<SessionEvent>::new());

        assert!(body.contains("event: idle"));
        assert!(
            !body.contains("event: failed"),
            "an empty event queue is not a failed simulation"
        );
    }

    #[test]
    fn live_velocity_preview_rejects_kinematic_body_from_current_snapshot() {
        let mut world = World::new(WorldDesc::default());
        let body_handle = world
            .create_body(BodyDesc {
                body_type: BodyType::Kinematic,
                ..BodyDesc::default()
            })
            .expect("kinematic body should be created");
        let snapshot = DebugSnapshot::from_world(&world, &DebugSnapshotOptions::default());
        let frame = FrameRecord {
            frame_index: 0,
            simulated_time: 0.0,
            state_hash: "kinematic-preview-source".to_owned(),
            report: StepReport {
                revision: world.revision(),
                ..StepReport::default()
            },
            stats: StepStats::default(),
            events: Vec::new(),
            snapshot,
            compound_provenance: Vec::new(),
            perturbation_provenance: Vec::new(),
            diagnostics: crate::artifact::FrameDiagnostics::default(),
        };
        let mut session = SessionState {
            record: SessionRecord {
                id: "session-kinematic".to_owned(),
                scenario_id: ScenarioId::FallingBoxContact,
                mode: SessionMode::LiveSession,
                status: SessionStatus::Paused,
                session_epoch: 0,
                run_id: None,
                frame_count: 1,
                produced_frame_count: 1,
                buffered_frame_count: 1,
                current_frame_index: 0,
                retained_frame_start: 0,
                retained_frame_end_exclusive: 1,
                live_buffer_capacity: super::default_live_buffer_capacity(),
                live_unbounded: false,
                overrides: ScenarioOverrides::default(),
                effective_runtime_config: ScenarioRuntimeConfig::default(),
                final_state_hash: Some("kinematic-preview-source".to_owned()),
                manifest_artifact: None,
                final_snapshot_artifact: None,
                latest_frame: Some(frame),
                last_error: None,
                events: Vec::new(),
                frame_state_hashes: Vec::new(),
            },
            runtime: SessionRuntime::Live(LiveSessionState {
                world,
                pipeline: SimulationPipeline::new(StepConfig::default()),
                substeps_per_frame: 1,
                query: QueryPipeline::new(),
                frames: Vec::new(),
                retained_frame_start: 0,
                compound_provenance: Vec::new(),
                perturbation_provenance: Vec::new(),
                velocity_preview_cache: std::collections::BTreeMap::new(),
                used_velocity_preview_actions: std::collections::BTreeSet::new(),
            }),
            artifact_reset_generation: 0,
        };

        let world_revision = session
            .record
            .latest_frame
            .as_ref()
            .unwrap()
            .snapshot
            .meta
            .revision
            .unwrap();
        let response = preview_live_velocity_perturbation(
            &mut session,
            super::VelocityPerturbationPreviewRequest {
                action_id: "kinematic".to_owned(),
                world_revision,
                session_epoch: 0,
                body_handle,
                frame_index: 0,
                requested_delta: json!([0.25, 0.0]),
                wake_intent: None,
            },
        );

        assert_eq!(response.rejection_reason, Some("kinematic_body"));
        assert_eq!(response.before_velocity, None);
        assert_eq!(response.computed_target_velocity, None);
    }
}
