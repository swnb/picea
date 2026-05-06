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
    BodyHandle, BodyPatch, BodyType, QueryPipeline, SimulationPipeline, StepConfig, Vector, World,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{
    artifact::{
        frame_record_from_step_with_provenance, refreshed_frame_record_from_world, run_scenario,
        ArtifactFile, ArtifactStore, FrameRecord, LivePerturbationCommitOutcome,
        LivePerturbationProvenance, LiveQuerySyncStatus,
    },
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
    query: QueryPipeline,
    frames: Vec<FrameRecord>,
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
            session_epoch: 0,
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
            session.record.session_epoch = session.record.session_epoch.saturating_add(1);
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
        runtime.frames.truncate(request.frame_index);
        runtime.frames.push(frame.clone());
        runtime.velocity_preview_cache.remove(&request.action_id);
        runtime
            .used_velocity_preview_actions
            .insert(request.action_id.clone());
        session.record.buffered_frame_count = runtime.frames.len();
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
    runtime.query.sync(&runtime.world);
    let frame = frame_record_from_step_with_provenance(
        &runtime.world,
        report,
        frame_index,
        &runtime.compound_provenance,
        &runtime.perturbation_provenance,
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
    let mut query = QueryPipeline::new();
    query.sync(&scenario.world);
    Ok(LiveSessionState {
        world: scenario.world,
        pipeline: SimulationPipeline::new(StepConfig::default()),
        query,
        frames: Vec::new(),
        compound_provenance: scenario.compound_provenance,
        perturbation_provenance: Vec::new(),
        velocity_preview_cache: BTreeMap::new(),
        used_velocity_preview_actions: BTreeSet::new(),
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
    use picea::prelude::{
        BodyDesc, BodyType, DebugSnapshot, DebugSnapshotOptions, StepReport, StepStats, WorldDesc,
    };
    use serde_json::json;

    use super::{
        format_session_events, preview_live_velocity_perturbation, FrameRecord, LiveSessionState,
        QueryPipeline, ScenarioId, ScenarioOverrides, SessionEvent, SessionMode, SessionRecord,
        SessionRuntime, SessionState, SessionStatus, SimulationPipeline, StepConfig, World,
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
                buffered_frame_count: 1,
                current_frame_index: 0,
                overrides: ScenarioOverrides::default(),
                final_state_hash: Some("kinematic-preview-source".to_owned()),
                manifest_artifact: None,
                final_snapshot_artifact: None,
                latest_frame: Some(frame),
                last_error: None,
                events: Vec::new(),
            },
            runtime: SessionRuntime::Live(LiveSessionState {
                world,
                pipeline: SimulationPipeline::new(StepConfig::default()),
                query: QueryPipeline::new(),
                frames: Vec::new(),
                compound_provenance: Vec::new(),
                perturbation_provenance: Vec::new(),
                velocity_preview_cache: std::collections::BTreeMap::new(),
                used_velocity_preview_actions: std::collections::BTreeSet::new(),
            }),
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
