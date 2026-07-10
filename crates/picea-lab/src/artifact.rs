//! Run artifacts and deterministic scenario execution.
//!
//! Artifacts are the stable interchange format between the Rust runner, the
//! local server, and the web viewer. They intentionally contain serializable
//! debug facts instead of private engine state.

use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    str::FromStr,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use picea::{debug::DebugAabb, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
    scenario::{build_scenario, CompoundProvenance, RunConfig, ScenarioId, ScenarioRuntimeConfig},
    LabError, LabResult,
};

/// Known artifact files written for every run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactFile {
    Manifest,
    Frames,
    DebugRender,
    FinalSnapshot,
    Perf,
}

impl ArtifactFile {
    pub const ALL: [Self; 5] = [
        Self::Manifest,
        Self::Frames,
        Self::DebugRender,
        Self::FinalSnapshot,
        Self::Perf,
    ];

    pub const fn file_name(self) -> &'static str {
        match self {
            Self::Manifest => "manifest.json",
            Self::Frames => "frames.jsonl",
            Self::DebugRender => "debug_render.json",
            Self::FinalSnapshot => "final_snapshot.json",
            Self::Perf => "perf.json",
        }
    }

    /// Single source of truth for artifact MIME types. Both the manifest entries
    /// and the HTTP `get_artifact` handler read the content type from here.
    pub(crate) const fn content_type(self) -> &'static str {
        match self {
            Self::Frames => "application/x-ndjson",
            _ => "application/json",
        }
    }
}

impl FromStr for ArtifactFile {
    type Err = LabError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        ArtifactFile::ALL
            .into_iter()
            .find(|file| file.file_name() == value)
            .ok_or_else(|| LabError::InvalidArtifactFile(value.to_owned()))
    }
}

/// Metadata entry for one artifact file. An artifact is a durable byproduct of
/// a run, not an engine-owned data structure.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ArtifactEntry {
    pub file: String,
    pub content_type: String,
}

/// Current `manifest.json` schema version. Read back with `#[serde(default)]`
/// so pre-versioning manifests deserialize as version 0.
const RUN_MANIFEST_SCHEMA_VERSION: u32 = 1;

/// Manifest schema for `manifest.json`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RunManifest {
    pub run_id: String,
    pub scenario_id: ScenarioId,
    pub frame_count: usize,
    #[serde(default)]
    pub effective_runtime_config: ScenarioRuntimeConfig,
    pub final_state_hash: String,
    /// Additive artifact-schema version. Older manifests without this field read
    /// back as 0 via `#[serde(default)]`; freshly written manifests carry the
    /// current `RUN_MANIFEST_SCHEMA_VERSION`.
    #[serde(default)]
    pub schema_version: u32,
    pub artifacts: Vec<ArtifactEntry>,
}

/// Where a diagnostics fact came from.
///
/// The distinction matters because the lab may summarize authoritative Rust
/// facts into a more compact carrier, while the web can still add UI-only
/// aggregates later. Callers should not mistake a lab-side summary for a new
/// solver-owned fact.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSource {
    RustAuthoritative,
    LabDerived,
    WebDerived,
    #[default]
    Missing,
}

/// Triage level for one diagnostics marker.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    #[default]
    Info,
    Warning,
    Severe,
}

/// Stable marker kinds used by timeline and copied debug context.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticMarkerKind {
    #[default]
    PerformanceCounterSpike,
    SolverRowSpike,
    CcdSpike,
    NumericWarning,
    PenetrationSpike,
    ContactChurnSpike,
    WarmStartDropSpike,
    SleepTransition,
    SleepNeverConverged,
    AngularDriftSpike,
    BodyDriftSpike,
}

/// Optional evidence that would have made a derived diagnostics fact more precise.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissingEvidenceKind {
    #[default]
    PreviousFrame,
    ContactFacts,
    ContactIds,
    SleepEvents,
    IslandFacts,
}

/// Explanation for one missing evidence slot.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MissingEvidence {
    pub kind: MissingEvidenceKind,
    #[serde(default)]
    pub detail: String,
}

/// One marker that points consumers at a frame worth inspecting.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticMarker {
    pub kind: DiagnosticMarkerKind,
    pub severity: DiagnosticSeverity,
    pub frame_index: usize,
    pub score: f64,
    pub threshold_name: String,
    #[serde(default)]
    pub source: DiagnosticSource,
    #[serde(default)]
    pub body_handles: Vec<BodyHandle>,
    #[serde(default)]
    pub contact_ids: Vec<ContactId>,
    #[serde(default)]
    pub island_ids: Vec<u32>,
    #[serde(default)]
    pub evidence_fields: Vec<String>,
    #[serde(default)]
    pub missing_evidence: Vec<MissingEvidenceKind>,
}

/// Focused per-frame counter deltas. These are lab summaries built from
/// adjacent frames; the raw counters remain in `StepStats`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PerformanceCounterDelta {
    #[serde(default)]
    pub source: DiagnosticSource,
    pub broadphase_candidate_count: Option<i64>,
    pub broadphase_traversal_count: Option<i64>,
    pub broadphase_pruned_count: Option<i64>,
    pub contact_count: Option<i64>,
    pub island_count: Option<i64>,
    pub solver_row_count: Option<i64>,
    pub ccd_candidate_count: Option<i64>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FramePerformanceDiagnostics {
    #[serde(default)]
    pub counter_delta: PerformanceCounterDelta,
}

/// Penetration is the overlap depth currently visible in exported contacts and
/// manifolds. It is lab-derived because the lab summarizes the contact set into
/// one frame-level view instead of exposing a new core-owned aggregate.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PenetrationDiagnostics {
    #[serde(default)]
    pub source: DiagnosticSource,
    pub max_depth: f64,
    pub total_depth: f64,
    pub penetrating_contact_count: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ContactChurnDiagnostics {
    #[serde(default)]
    pub source: DiagnosticSource,
    #[serde(default)]
    pub missing_evidence: Vec<MissingEvidenceKind>,
    pub entered: usize,
    pub persisted: usize,
    pub exited: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WarmStartDiagnostics {
    #[serde(default)]
    pub source: DiagnosticSource,
    #[serde(default)]
    pub counts_source: DiagnosticSource,
    #[serde(default)]
    pub drop_reasons_source: DiagnosticSource,
    pub hit_count: usize,
    pub miss_count: usize,
    pub drop_count: usize,
    #[serde(default)]
    pub drop_reasons: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ImpulseDiagnostics {
    #[serde(default)]
    pub source: DiagnosticSource,
    pub total_normal_impulse: f64,
    pub total_tangent_impulse: f64,
    pub max_normal_impulse: f64,
    pub max_tangent_impulse: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SleepDiagnostics {
    #[serde(default)]
    pub source: DiagnosticSource,
    #[serde(default)]
    pub body_counts_source: DiagnosticSource,
    #[serde(default)]
    pub transition_count_source: DiagnosticSource,
    #[serde(default)]
    pub reasons_source: DiagnosticSource,
    pub awake_dynamic_body_count: usize,
    pub sleeping_dynamic_body_count: usize,
    pub transition_count: usize,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct IslandDiagnostics {
    #[serde(default)]
    pub source: DiagnosticSource,
    pub island_count: usize,
    pub active_island_count: usize,
    pub sleeping_island_skip_count: usize,
    pub solver_body_slot_count: usize,
    pub contact_row_count: usize,
    pub joint_row_count: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FrameStabilityDiagnostics {
    #[serde(default)]
    pub penetration: PenetrationDiagnostics,
    #[serde(default)]
    pub contact_churn: ContactChurnDiagnostics,
    #[serde(default)]
    pub warm_start: WarmStartDiagnostics,
    #[serde(default)]
    pub impulse: ImpulseDiagnostics,
    #[serde(default)]
    pub sleep: SleepDiagnostics,
    #[serde(default)]
    pub island: IslandDiagnostics,
}

/// Lab-owned additive diagnostics carrier. It keeps missing-evidence semantics
/// explicit so older artifacts and partial evidence do not silently read as
/// zeroed physics facts.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FrameDiagnostics {
    #[serde(default)]
    pub performance: FramePerformanceDiagnostics,
    #[serde(default)]
    pub stability: FrameStabilityDiagnostics,
    #[serde(default)]
    pub markers: Vec<DiagnosticMarker>,
    #[serde(default)]
    pub missing_evidence: Vec<MissingEvidence>,
}

/// One line in `frames.jsonl`. A frame is the stable view of one fixed
/// simulation step, designed for replay and SSE consumers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FrameRecord {
    pub frame_index: usize,
    pub simulated_time: f64,
    pub state_hash: String,
    pub report: StepReport,
    pub stats: StepStats,
    pub events: Vec<WorldEvent>,
    pub snapshot: DebugSnapshot,
    /// Lab-owned authoring provenance repeated per frame so replay consumers do
    /// not need a side channel or web-side fixture reconstruction.
    #[serde(default)]
    pub compound_provenance: Vec<CompoundProvenance>,
    /// Server-side live perturbation provenance. This is additive to artifact
    /// authoring provenance and remains empty for ordinary headless runs.
    #[serde(default)]
    pub perturbation_provenance: Vec<LivePerturbationProvenance>,
    /// Lab-owned diagnostics derived from exported frame facts. Missing in
    /// older artifacts, so deserialization must default cleanly.
    #[serde(default)]
    pub diagnostics: FrameDiagnostics,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LivePerturbationProvenance {
    pub action_id: String,
    pub session_id: String,
    pub world_revision: WorldRevision,
    pub session_epoch: u64,
    pub body_handle: BodyHandle,
    pub frame_index: usize,
    pub before_velocity: Vector,
    pub requested_delta: Vector,
    pub computed_target_velocity: Vector,
    pub wake_intent: bool,
    pub commit_outcome: LivePerturbationCommitOutcome,
    pub query_sync_status: LiveQuerySyncStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LivePerturbationCommitOutcome {
    Accepted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveQuerySyncStatus {
    Synced,
    Stale,
}

/// Viewer-oriented, compact render summary derived from debug snapshots.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DebugRenderArtifact {
    pub run_id: String,
    pub scenario_id: ScenarioId,
    pub final_state_hash: String,
    pub frames: Vec<DebugRenderFrame>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DebugRenderFrame {
    pub frame_index: usize,
    pub body_count: usize,
    pub collider_count: usize,
    pub broadphase_candidate_count: usize,
    pub broadphase_update_count: usize,
    pub broadphase_stale_proxy_drop_count: usize,
    pub broadphase_same_body_drop_count: usize,
    pub broadphase_filter_drop_count: usize,
    pub broadphase_narrowphase_drop_count: usize,
    #[serde(default)]
    pub broadphase_traversal_count: usize,
    #[serde(default)]
    pub broadphase_pruned_count: usize,
    pub broadphase_rebuild_count: usize,
    pub broadphase_tree_depth: usize,
    pub contact_count: usize,
    #[serde(default)]
    pub island_count: usize,
    #[serde(default)]
    pub active_island_count: usize,
    #[serde(default)]
    pub sleeping_island_skip_count: usize,
    #[serde(default)]
    pub solver_body_slot_count: usize,
    #[serde(default)]
    pub contact_row_count: usize,
    #[serde(default)]
    pub joint_row_count: usize,
    #[serde(default)]
    pub warm_start_hit_count: usize,
    #[serde(default)]
    pub warm_start_miss_count: usize,
    #[serde(default)]
    pub warm_start_drop_count: usize,
    #[serde(default)]
    pub ccd_candidate_count: usize,
    #[serde(default)]
    pub ccd_hit_count: usize,
    #[serde(default)]
    pub ccd_miss_count: usize,
    #[serde(default)]
    pub ccd_clamp_count: usize,
    #[serde(default)]
    pub position_correction_input_contact_count: usize,
    #[serde(default)]
    pub position_correction_input_max_depth: f32,
    #[serde(default)]
    pub position_correction_input_total_depth: f32,
    #[serde(default)]
    pub position_correction_body_count: usize,
    #[serde(default)]
    pub position_correction_max_translation: f32,
    #[serde(default)]
    pub position_correction_total_translation: f32,
    pub world_bounds: Option<DebugAabb>,
    pub bodies: Vec<DebugBody>,
    pub colliders: Vec<DebugCollider>,
    pub contacts: Vec<DebugContact>,
    pub manifolds: Vec<DebugManifold>,
    #[serde(default)]
    pub joints: Vec<picea::debug::DebugJoint>,
    #[serde(default)]
    pub broadphase_tree: picea::debug::DebugBroadphaseTree,
    #[serde(default)]
    pub islands: Vec<picea::debug::DebugIsland>,
    #[serde(default)]
    pub compound_provenance: Vec<CompoundProvenance>,
    #[serde(default)]
    pub diagnostics: FrameDiagnostics,
    pub unmeasured: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PerfArtifact {
    pub frame_count: usize,
    pub elapsed_micros: u128,
    pub final_state_hash: String,
    #[serde(default)]
    pub counter_summary: PerfCounterSummary,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PerfCounterSummary {
    pub total_broadphase_candidate_count: usize,
    pub total_broadphase_traversal_count: usize,
    pub total_broadphase_pruned_count: usize,
    pub max_broadphase_tree_depth: usize,
    pub total_contact_count: usize,
    pub total_contact_row_count: usize,
    pub total_joint_row_count: usize,
    pub total_solver_body_slot_count: usize,
    pub total_island_count: usize,
    pub total_active_island_count: usize,
    pub total_sleeping_island_skip_count: usize,
    pub total_ccd_candidate_count: usize,
    pub total_ccd_hit_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RunResult {
    pub path: PathBuf,
    pub manifest: RunManifest,
    pub frames: Vec<FrameRecord>,
}

pub fn frame_record_from_step(
    world: &World,
    report: StepReport,
    frame_index: usize,
    compound_provenance: &[CompoundProvenance],
) -> LabResult<FrameRecord> {
    frame_record_from_step_with_provenance(world, report, frame_index, compound_provenance, &[])
}

pub fn frame_record_from_step_with_provenance(
    world: &World,
    report: StepReport,
    frame_index: usize,
    compound_provenance: &[CompoundProvenance],
    perturbation_provenance: &[LivePerturbationProvenance],
) -> LabResult<FrameRecord> {
    let snapshot = DebugSnapshot::from_world_with_step_report(
        world,
        &report,
        &DebugSnapshotOptions::default(),
    );
    let state_hash = state_hash(&snapshot)?;
    Ok(FrameRecord {
        frame_index,
        simulated_time: report.simulated_time,
        state_hash,
        stats: report.stats,
        events: report.events.clone(),
        report,
        snapshot,
        compound_provenance: compound_provenance.to_vec(),
        perturbation_provenance: perturbation_provenance.to_vec(),
        diagnostics: FrameDiagnostics::default(),
    })
}

pub fn refreshed_frame_record_from_world(
    world: &World,
    base_frame: &FrameRecord,
    compound_provenance: &[CompoundProvenance],
    perturbation_provenance: &[LivePerturbationProvenance],
) -> LabResult<FrameRecord> {
    let report = StepReport {
        revision: world.revision(),
        simulated_time: base_frame.simulated_time,
        step_index: base_frame.report.step_index,
        dt: base_frame.report.dt,
        stats: StepStats::default(),
        events: Vec::new(),
    };
    frame_record_from_step_with_provenance(
        world,
        report,
        base_frame.frame_index,
        compound_provenance,
        perturbation_provenance,
    )
}

/// Filesystem boundary that hides `target/picea-lab/runs/<run_id>` from higher
/// level CLI and server flows.
///
/// The default store is intentionally anchored at the workspace `target/`
/// directory, not at the process current directory. That keeps local evidence
/// artifacts next to other generated build outputs whether `picea-lab serve` is
/// launched from the workspace root or from `crates/picea-lab`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactStore {
    root: PathBuf,
}

impl ArtifactStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn default_in_workspace() -> Self {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest_dir
            .parent()
            .and_then(Path::parent)
            .unwrap_or(manifest_dir);
        Self::new(workspace_root.join("target/picea-lab/runs"))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn run_path(&self, run_id: &str) -> PathBuf {
        self.root.join(run_id)
    }

    pub fn artifact_path(&self, run_id: &str, file: ArtifactFile) -> PathBuf {
        self.run_path(run_id).join(file.file_name())
    }

    pub fn read_artifact(&self, run_id: &str, file_name: &str) -> LabResult<Vec<u8>> {
        let file = ArtifactFile::from_str(file_name)?;
        Ok(fs::read(self.artifact_path(run_id, file))?)
    }
}

pub fn run_scenario(store: &ArtifactStore, config: RunConfig) -> LabResult<RunResult> {
    let started = Instant::now();
    let frame_count = config.effective_frame_count();
    let run_id = config.run_id.clone().unwrap_or_else(make_run_id);
    let run_path = store.run_path(&run_id);
    fs::create_dir_all(&run_path)?;

    let mut scenario = build_scenario(config.scenario_id, &config.overrides)?;
    let effective_runtime_config = scenario.effective_runtime_config.clone();
    let mut pipeline = SimulationPipeline::new(effective_runtime_config.step);
    let mut frames = Vec::with_capacity(frame_count);

    for frame_index in 0..frame_count {
        let mut report = pipeline.step(&mut scenario.world);
        for _ in 1..effective_runtime_config.substeps_per_frame.max(1) {
            report = pipeline.step(&mut scenario.world);
        }
        frames.push(frame_record_from_step(
            &scenario.world,
            report,
            frame_index,
            &scenario.compound_provenance,
        )?);
    }
    populate_frame_diagnostics(&mut frames);

    let final_snapshot = frames
        .last()
        .map(|frame| frame.snapshot.clone())
        .unwrap_or_else(DebugSnapshot::default);
    let final_state_hash = state_hash(&final_snapshot)?;
    let manifest = RunManifest {
        run_id: run_id.clone(),
        scenario_id: config.scenario_id,
        frame_count,
        effective_runtime_config,
        final_state_hash: final_state_hash.clone(),
        schema_version: RUN_MANIFEST_SCHEMA_VERSION,
        artifacts: artifact_entries(),
    };

    write_json(run_path.join(ArtifactFile::Manifest.file_name()), &manifest)?;
    write_frames(run_path.join(ArtifactFile::Frames.file_name()), &frames)?;
    write_json(
        run_path.join(ArtifactFile::DebugRender.file_name()),
        &DebugRenderArtifact {
            run_id,
            scenario_id: config.scenario_id,
            final_state_hash: final_state_hash.clone(),
            frames: frames
                .iter()
                .map(|frame| DebugRenderFrame {
                    frame_index: frame.frame_index,
                    body_count: frame.snapshot.bodies.len(),
                    collider_count: frame.snapshot.colliders.len(),
                    broadphase_candidate_count: frame.snapshot.stats.broadphase_candidate_count,
                    broadphase_update_count: frame.snapshot.stats.broadphase_update_count,
                    broadphase_stale_proxy_drop_count: frame
                        .snapshot
                        .stats
                        .broadphase_stale_proxy_drop_count,
                    broadphase_same_body_drop_count: frame
                        .snapshot
                        .stats
                        .broadphase_same_body_drop_count,
                    broadphase_filter_drop_count: frame.snapshot.stats.broadphase_filter_drop_count,
                    broadphase_narrowphase_drop_count: frame
                        .snapshot
                        .stats
                        .broadphase_narrowphase_drop_count,
                    broadphase_traversal_count: frame.snapshot.stats.broadphase_traversal_count,
                    broadphase_pruned_count: frame.snapshot.stats.broadphase_pruned_count,
                    broadphase_rebuild_count: frame.snapshot.stats.broadphase_rebuild_count,
                    broadphase_tree_depth: frame.snapshot.stats.broadphase_tree_depth,
                    contact_count: frame.snapshot.contacts.len(),
                    island_count: frame.snapshot.stats.island_count,
                    active_island_count: frame.snapshot.stats.active_island_count,
                    sleeping_island_skip_count: frame.snapshot.stats.sleeping_island_skip_count,
                    solver_body_slot_count: frame.snapshot.stats.solver_body_slot_count,
                    contact_row_count: frame.snapshot.stats.contact_row_count,
                    joint_row_count: frame.snapshot.stats.joint_row_count,
                    warm_start_hit_count: frame.snapshot.stats.warm_start_hit_count,
                    warm_start_miss_count: frame.snapshot.stats.warm_start_miss_count,
                    warm_start_drop_count: frame.snapshot.stats.warm_start_drop_count,
                    ccd_candidate_count: frame.snapshot.stats.ccd_candidate_count,
                    ccd_hit_count: frame.snapshot.stats.ccd_hit_count,
                    ccd_miss_count: frame.snapshot.stats.ccd_miss_count,
                    ccd_clamp_count: frame.snapshot.stats.ccd_clamp_count,
                    position_correction_input_contact_count: frame
                        .snapshot
                        .stats
                        .position_correction_input_contact_count,
                    position_correction_input_max_depth: frame
                        .snapshot
                        .stats
                        .position_correction_input_max_depth,
                    position_correction_input_total_depth: frame
                        .snapshot
                        .stats
                        .position_correction_input_total_depth,
                    position_correction_body_count: frame
                        .snapshot
                        .stats
                        .position_correction_body_count,
                    position_correction_max_translation: frame
                        .snapshot
                        .stats
                        .position_correction_max_translation,
                    position_correction_total_translation: frame
                        .snapshot
                        .stats
                        .position_correction_total_translation,
                    world_bounds: frame.snapshot.world_bounds(),
                    bodies: frame.snapshot.bodies.clone(),
                    colliders: frame.snapshot.colliders.clone(),
                    contacts: frame.snapshot.contacts.clone(),
                    manifolds: frame.snapshot.manifolds.clone(),
                    joints: frame.snapshot.joints.clone(),
                    broadphase_tree: frame.snapshot.broadphase_tree.clone(),
                    islands: frame.snapshot.islands.clone(),
                    compound_provenance: frame.compound_provenance.clone(),
                    diagnostics: frame.diagnostics.clone(),
                    // M5 exposes contact solver impulses; force/torque accumulation is still
                    // outside the lab artifact contract.
                    unmeasured: ["forces", "torques"]
                        .into_iter()
                        .map(str::to_owned)
                        .collect(),
                })
                .collect(),
        },
    )?;
    write_json(
        run_path.join(ArtifactFile::FinalSnapshot.file_name()),
        &final_snapshot,
    )?;
    write_json(
        run_path.join(ArtifactFile::Perf.file_name()),
        &PerfArtifact {
            frame_count,
            elapsed_micros: started.elapsed().as_micros(),
            final_state_hash: final_state_hash.clone(),
            counter_summary: PerfCounterSummary::from_frames(&frames),
        },
    )?;

    Ok(RunResult {
        path: run_path,
        manifest,
        frames,
    })
}

impl PerfCounterSummary {
    fn from_frames(frames: &[FrameRecord]) -> Self {
        let mut summary = Self::default();
        for frame in frames {
            let stats = &frame.stats;
            summary.total_broadphase_candidate_count += stats.broadphase_candidate_count;
            summary.total_broadphase_traversal_count += stats.broadphase_traversal_count;
            summary.total_broadphase_pruned_count += stats.broadphase_pruned_count;
            summary.max_broadphase_tree_depth = summary
                .max_broadphase_tree_depth
                .max(stats.broadphase_tree_depth);
            summary.total_contact_count += stats.contact_count;
            summary.total_contact_row_count += stats.contact_row_count;
            summary.total_joint_row_count += stats.joint_row_count;
            summary.total_solver_body_slot_count += stats.solver_body_slot_count;
            summary.total_island_count += stats.island_count;
            summary.total_active_island_count += stats.active_island_count;
            summary.total_sleeping_island_skip_count += stats.sleeping_island_skip_count;
            summary.total_ccd_candidate_count += stats.ccd_candidate_count;
            summary.total_ccd_hit_count += stats.ccd_hit_count;
        }
        summary
    }
}

fn populate_frame_diagnostics(frames: &mut [FrameRecord]) {
    let solver_rows: Vec<usize> = frames
        .iter()
        .map(|frame| frame.stats.contact_row_count + frame.stats.joint_row_count)
        .collect();

    for frame_index in 0..frames.len() {
        let previous = if frame_index > 0 {
            Some(&frames[frame_index - 1])
        } else {
            None
        };
        let frame = &frames[frame_index];
        let mut diagnostics = FrameDiagnostics::default();
        if previous.is_none() {
            diagnostics.missing_evidence.push(MissingEvidence {
                kind: MissingEvidenceKind::PreviousFrame,
                detail: "counter deltas and adjacent-frame churn need the previous frame"
                    .to_owned(),
            });
        }
        diagnostics.performance.counter_delta = performance_counter_delta(frame, previous);
        diagnostics.stability.penetration = penetration_diagnostics(frame);
        diagnostics.stability.contact_churn = contact_churn_diagnostics(frame, previous);
        diagnostics.stability.warm_start = warm_start_diagnostics(frame);
        diagnostics.stability.impulse = impulse_diagnostics(frame);
        diagnostics.stability.sleep = sleep_diagnostics(frame);
        diagnostics.stability.island = island_diagnostics(frame);
        diagnostics.markers = diagnostic_markers(
            frames,
            frame_index,
            &solver_rows,
            &diagnostics.performance.counter_delta,
            &diagnostics.stability,
        );
        frames[frame_index].diagnostics = diagnostics;
    }
}

fn performance_counter_delta(
    frame: &FrameRecord,
    previous: Option<&FrameRecord>,
) -> PerformanceCounterDelta {
    let Some(previous) = previous else {
        return PerformanceCounterDelta {
            source: DiagnosticSource::Missing,
            ..PerformanceCounterDelta::default()
        };
    };
    PerformanceCounterDelta {
        source: DiagnosticSource::LabDerived,
        broadphase_candidate_count: Some(
            frame.stats.broadphase_candidate_count as i64
                - previous.stats.broadphase_candidate_count as i64,
        ),
        broadphase_traversal_count: Some(
            frame.stats.broadphase_traversal_count as i64
                - previous.stats.broadphase_traversal_count as i64,
        ),
        broadphase_pruned_count: Some(
            frame.stats.broadphase_pruned_count as i64
                - previous.stats.broadphase_pruned_count as i64,
        ),
        contact_count: Some(frame.stats.contact_count as i64 - previous.stats.contact_count as i64),
        island_count: Some(frame.stats.island_count as i64 - previous.stats.island_count as i64),
        solver_row_count: Some(
            (frame.stats.contact_row_count + frame.stats.joint_row_count) as i64
                - (previous.stats.contact_row_count + previous.stats.joint_row_count) as i64,
        ),
        ccd_candidate_count: Some(
            frame.stats.ccd_candidate_count as i64 - previous.stats.ccd_candidate_count as i64,
        ),
    }
}

fn penetration_diagnostics(frame: &FrameRecord) -> PenetrationDiagnostics {
    let mut max_depth: f64 = 0.0;
    let mut total_depth: f64 = 0.0;
    let mut penetrating_contact_count = 0;
    for contact in &frame.snapshot.contacts {
        let depth = f64::from(contact.depth.max(0.0));
        if depth > 0.0 {
            penetrating_contact_count += 1;
            total_depth += depth;
            max_depth = max_depth.max(depth);
        }
    }
    PenetrationDiagnostics {
        source: DiagnosticSource::LabDerived,
        max_depth,
        total_depth,
        penetrating_contact_count,
    }
}

fn contact_churn_diagnostics(
    frame: &FrameRecord,
    previous: Option<&FrameRecord>,
) -> ContactChurnDiagnostics {
    if previous.is_none() {
        return ContactChurnDiagnostics {
            source: DiagnosticSource::Missing,
            missing_evidence: vec![MissingEvidenceKind::PreviousFrame],
            ..ContactChurnDiagnostics::default()
        };
    }
    let mut entered = 0;
    let mut persisted = 0;
    let mut exited = 0;
    for event in &frame.events {
        match event {
            WorldEvent::ContactStarted(_) => entered += 1,
            WorldEvent::ContactPersisted(_) => persisted += 1,
            WorldEvent::ContactEnded(_) => exited += 1,
            _ => {}
        }
    }
    ContactChurnDiagnostics {
        source: DiagnosticSource::LabDerived,
        missing_evidence: Vec::new(),
        entered,
        persisted,
        exited,
    }
}

fn warm_start_diagnostics(frame: &FrameRecord) -> WarmStartDiagnostics {
    let mut drop_reasons = Vec::new();
    for event in &frame.events {
        let contact = match event {
            WorldEvent::ContactStarted(contact) | WorldEvent::ContactPersisted(contact) => {
                Some(contact)
            }
            _ => None,
        };
        let Some(contact) = contact else {
            continue;
        };
        if contact.warm_start_reason.is_drop() {
            let reason = serde_json::to_string(&contact.warm_start_reason)
                .unwrap_or_else(|_| "\"unknown\"".to_owned())
                .trim_matches('"')
                .to_owned();
            if !drop_reasons.iter().any(|existing| existing == &reason) {
                drop_reasons.push(reason);
            }
        }
    }
    WarmStartDiagnostics {
        source: DiagnosticSource::LabDerived,
        counts_source: DiagnosticSource::RustAuthoritative,
        drop_reasons_source: DiagnosticSource::LabDerived,
        hit_count: frame.stats.warm_start_hit_count,
        miss_count: frame.stats.warm_start_miss_count,
        drop_count: frame.stats.warm_start_drop_count,
        drop_reasons,
    }
}

fn impulse_diagnostics(frame: &FrameRecord) -> ImpulseDiagnostics {
    let mut total_normal_impulse: f64 = 0.0;
    let mut total_tangent_impulse: f64 = 0.0;
    let mut max_normal_impulse: f64 = 0.0;
    let mut max_tangent_impulse: f64 = 0.0;
    for contact in &frame.snapshot.contacts {
        total_normal_impulse += f64::from(contact.solver_normal_impulse);
        total_tangent_impulse += f64::from(contact.solver_tangent_impulse.abs());
        max_normal_impulse = max_normal_impulse.max(f64::from(contact.solver_normal_impulse));
        max_tangent_impulse =
            max_tangent_impulse.max(f64::from(contact.solver_tangent_impulse.abs()));
    }
    ImpulseDiagnostics {
        source: DiagnosticSource::LabDerived,
        total_normal_impulse,
        total_tangent_impulse,
        max_normal_impulse,
        max_tangent_impulse,
    }
}

fn sleep_diagnostics(frame: &FrameRecord) -> SleepDiagnostics {
    let mut awake_dynamic_body_count = 0;
    let mut sleeping_dynamic_body_count = 0;
    for body in &frame.snapshot.bodies {
        if body.body_type.is_dynamic() {
            if body.sleeping {
                sleeping_dynamic_body_count += 1;
            } else {
                awake_dynamic_body_count += 1;
            }
        }
    }
    let reasons = frame
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::SleepChanged(sleep) => Some(format!("{:?}", sleep.reason)),
            _ => None,
        })
        .collect();
    SleepDiagnostics {
        source: DiagnosticSource::LabDerived,
        body_counts_source: DiagnosticSource::LabDerived,
        transition_count_source: DiagnosticSource::RustAuthoritative,
        reasons_source: DiagnosticSource::LabDerived,
        awake_dynamic_body_count,
        sleeping_dynamic_body_count,
        transition_count: frame.stats.sleep_transition_count,
        reasons,
    }
}

fn island_diagnostics(frame: &FrameRecord) -> IslandDiagnostics {
    IslandDiagnostics {
        source: DiagnosticSource::RustAuthoritative,
        island_count: frame.stats.island_count,
        active_island_count: frame.stats.active_island_count,
        sleeping_island_skip_count: frame.stats.sleeping_island_skip_count,
        solver_body_slot_count: frame.stats.solver_body_slot_count,
        contact_row_count: frame.stats.contact_row_count,
        joint_row_count: frame.stats.joint_row_count,
    }
}

fn diagnostic_markers(
    frames: &[FrameRecord],
    frame_index: usize,
    solver_rows: &[usize],
    counter_delta: &PerformanceCounterDelta,
    stability: &FrameStabilityDiagnostics,
) -> Vec<DiagnosticMarker> {
    let frame = &frames[frame_index];
    let mut markers = Vec::new();

    if frame.stats.numeric_warnings > 0 {
        markers.push(DiagnosticMarker {
            kind: DiagnosticMarkerKind::NumericWarning,
            severity: if frame.stats.numeric_warnings > 1 {
                DiagnosticSeverity::Severe
            } else {
                DiagnosticSeverity::Warning
            },
            frame_index,
            score: frame.stats.numeric_warnings as f64,
            threshold_name: "numeric_warning".to_owned(),
            source: DiagnosticSource::LabDerived,
            evidence_fields: vec!["stats.numeric_warnings".to_owned()],
            ..DiagnosticMarker::default()
        });
    }

    let row_count = solver_rows[frame_index];
    let row_baseline = rolling_median(solver_rows, frame_index, 24);
    if (row_baseline == 0 && row_count >= 4)
        || (row_baseline > 0 && row_count >= row_baseline + 4 && row_count * 2 >= row_baseline * 3)
    {
        let repeated = repeated_solver_row_spike(solver_rows, frame_index, row_baseline);
        markers.push(DiagnosticMarker {
            kind: DiagnosticMarkerKind::SolverRowSpike,
            severity: if repeated {
                DiagnosticSeverity::Severe
            } else {
                DiagnosticSeverity::Warning
            },
            frame_index,
            score: row_count as f64,
            threshold_name: "solver_rows_vs_prev_rolling_median".to_owned(),
            source: DiagnosticSource::LabDerived,
            island_ids: frame
                .snapshot
                .islands
                .iter()
                .map(|island| island.id)
                .collect(),
            evidence_fields: vec![
                "stats.contact_row_count".to_owned(),
                "stats.joint_row_count".to_owned(),
                "diagnostics.performance.counter_delta.solver_row_count".to_owned(),
            ],
            missing_evidence: if frame_index == 0 {
                vec![MissingEvidenceKind::PreviousFrame]
            } else {
                Vec::new()
            },
            ..DiagnosticMarker::default()
        });
    }

    if stability.penetration.max_depth > 0.02 || stability.penetration.total_depth > 0.08 {
        markers.push(DiagnosticMarker {
            kind: DiagnosticMarkerKind::PenetrationSpike,
            severity: if stability.penetration.max_depth > 0.08
                || stability.penetration.total_depth > 0.25
            {
                DiagnosticSeverity::Severe
            } else {
                DiagnosticSeverity::Warning
            },
            frame_index,
            score: stability
                .penetration
                .max_depth
                .max(stability.penetration.total_depth),
            threshold_name: "penetration_depth".to_owned(),
            source: DiagnosticSource::LabDerived,
            body_handles: frame
                .snapshot
                .contacts
                .iter()
                .filter(|contact| contact.depth > 0.0)
                .flat_map(|contact| contact.bodies)
                .collect(),
            contact_ids: frame
                .snapshot
                .contacts
                .iter()
                .filter(|contact| contact.depth > 0.0)
                .map(|contact| contact.id)
                .collect(),
            evidence_fields: vec![
                "snapshot.contacts[].depth".to_owned(),
                "diagnostics.stability.penetration.max_depth".to_owned(),
                "diagnostics.stability.penetration.total_depth".to_owned(),
            ],
            ..DiagnosticMarker::default()
        });
    }

    if stability.contact_churn.entered + stability.contact_churn.exited >= 3 {
        markers.push(DiagnosticMarker {
            kind: DiagnosticMarkerKind::ContactChurnSpike,
            severity: if stability.contact_churn.entered + stability.contact_churn.exited >= 6 {
                DiagnosticSeverity::Severe
            } else {
                DiagnosticSeverity::Warning
            },
            frame_index,
            score: (stability.contact_churn.entered + stability.contact_churn.exited) as f64,
            threshold_name: "contact_enter_exit_count".to_owned(),
            source: DiagnosticSource::LabDerived,
            contact_ids: frame
                .events
                .iter()
                .filter_map(|event| match event {
                    WorldEvent::ContactStarted(contact)
                    | WorldEvent::ContactEnded(contact)
                    | WorldEvent::ContactPersisted(contact) => Some(contact.contact_id),
                    _ => None,
                })
                .collect(),
            evidence_fields: vec![
                "events.contact_started".to_owned(),
                "events.contact_ended".to_owned(),
            ],
            ..DiagnosticMarker::default()
        });
    }

    if frame.stats.warm_start_drop_count > 0 {
        markers.push(DiagnosticMarker {
            kind: DiagnosticMarkerKind::WarmStartDropSpike,
            severity: if frame.stats.warm_start_drop_count >= 2 {
                DiagnosticSeverity::Severe
            } else {
                DiagnosticSeverity::Warning
            },
            frame_index,
            score: frame.stats.warm_start_drop_count as f64,
            threshold_name: "warm_start_drop_count".to_owned(),
            source: DiagnosticSource::LabDerived,
            contact_ids: frame
                .events
                .iter()
                .filter_map(|event| match event {
                    WorldEvent::ContactStarted(contact) | WorldEvent::ContactPersisted(contact)
                        if contact.warm_start_reason.is_drop() =>
                    {
                        Some(contact.contact_id)
                    }
                    _ => None,
                })
                .collect(),
            evidence_fields: vec![
                "stats.warm_start_drop_count".to_owned(),
                "events[].warm_start_reason".to_owned(),
            ],
            ..DiagnosticMarker::default()
        });
    }

    if frame.stats.sleep_transition_count > 0 {
        markers.push(DiagnosticMarker {
            kind: DiagnosticMarkerKind::SleepTransition,
            severity: DiagnosticSeverity::Info,
            frame_index,
            score: frame.stats.sleep_transition_count as f64,
            threshold_name: "sleep_transition_count".to_owned(),
            source: DiagnosticSource::LabDerived,
            body_handles: frame
                .events
                .iter()
                .filter_map(|event| match event {
                    WorldEvent::SleepChanged(sleep) => Some(sleep.body),
                    _ => None,
                })
                .collect(),
            island_ids: frame
                .events
                .iter()
                .filter_map(|event| match event {
                    WorldEvent::SleepChanged(sleep) => Some(sleep.island_id),
                    _ => None,
                })
                .collect(),
            evidence_fields: vec![
                "stats.sleep_transition_count".to_owned(),
                "events.sleep_changed".to_owned(),
            ],
            ..DiagnosticMarker::default()
        });
    }

    let perf_spike = counter_delta
        .broadphase_candidate_count
        .unwrap_or_default()
        .abs()
        .max(
            counter_delta
                .broadphase_traversal_count
                .unwrap_or_default()
                .abs(),
        )
        .max(counter_delta.contact_count.unwrap_or_default().abs())
        .max(counter_delta.solver_row_count.unwrap_or_default().abs())
        .max(counter_delta.ccd_candidate_count.unwrap_or_default().abs());
    if perf_spike >= 4 {
        markers.push(DiagnosticMarker {
            kind: DiagnosticMarkerKind::PerformanceCounterSpike,
            severity: if perf_spike >= 8 {
                DiagnosticSeverity::Severe
            } else {
                DiagnosticSeverity::Warning
            },
            frame_index,
            score: perf_spike as f64,
            threshold_name: "counter_delta_abs_ge_4".to_owned(),
            source: DiagnosticSource::LabDerived,
            evidence_fields: vec![
                "diagnostics.performance.counter_delta.broadphase_candidate_count".to_owned(),
                "diagnostics.performance.counter_delta.contact_count".to_owned(),
                "diagnostics.performance.counter_delta.solver_row_count".to_owned(),
            ],
            missing_evidence: if frame_index == 0 {
                vec![MissingEvidenceKind::PreviousFrame]
            } else {
                Vec::new()
            },
            ..DiagnosticMarker::default()
        });
    }
    markers
}

fn repeated_solver_row_spike(solver_rows: &[usize], frame_index: usize, baseline: usize) -> bool {
    if frame_index < 2 {
        return false;
    }
    (frame_index - 2..=frame_index).all(|index| {
        let rows = solver_rows[index];
        (baseline == 0 && rows >= 4)
            || (baseline > 0 && rows >= baseline + 4 && rows * 2 >= baseline * 3)
    })
}

fn rolling_median(values: &[usize], frame_index: usize, window: usize) -> usize {
    if frame_index == 0 {
        return 0;
    }
    let start = frame_index.saturating_sub(window);
    let mut history = values[start..frame_index].to_vec();
    history.sort_unstable();
    history[history.len() / 2]
}

fn artifact_entries() -> Vec<ArtifactEntry> {
    ArtifactFile::ALL
        .into_iter()
        .map(|file| ArtifactEntry {
            file: file.file_name().to_owned(),
            content_type: file.content_type().to_owned(),
        })
        .collect()
}

fn write_json(path: impl AsRef<Path>, value: &impl Serialize) -> LabResult<()> {
    let file = File::create(path)?;
    serde_json::to_writer_pretty(BufWriter::new(file), value)?;
    Ok(())
}

fn write_frames(path: impl AsRef<Path>, frames: &[FrameRecord]) -> LabResult<()> {
    let mut writer = BufWriter::new(File::create(path)?);
    for frame in frames {
        serde_json::to_writer(&mut writer, frame)?;
        writer.write_all(b"\n")?;
    }
    Ok(())
}

fn make_run_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("run-{nanos}")
}

fn state_hash(value: &impl Serialize) -> LabResult<String> {
    let bytes = serde_json::to_vec(value)?;
    state_hash_bytes(&bytes)
}

fn state_hash_bytes(bytes: &[u8]) -> LabResult<String> {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes.iter().copied() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    Ok(format!("{hash:016x}"))
}

#[cfg(test)]
mod tests {
    use super::{run_scenario, ArtifactStore, RunManifest};
    use crate::scenario::{RunConfig, ScenarioId, ScenarioOverrides};
    use serde_json::json;

    #[test]
    fn run_manifest_carries_current_schema_version() {
        let temp = tempfile::tempdir().expect("temp dir should be created");
        let store = ArtifactStore::new(temp.path().join("runs"));
        let result = run_scenario(
            &store,
            RunConfig {
                scenario_id: ScenarioId::FallingBoxContact,
                frame_count: 1,
                run_id: None,
                overrides: ScenarioOverrides::default(),
            },
        )
        .expect("scenario run should succeed");
        let value = serde_json::to_value(&result.manifest).expect("manifest should serialize");
        assert_eq!(
            value["schema_version"], 1,
            "freshly generated manifests should carry schema_version 1"
        );
    }

    #[test]
    fn run_manifest_defaults_schema_version_for_legacy_json() {
        let legacy = json!({
            "run_id": "run-legacy",
            "scenario_id": "falling_box_contact",
            "frame_count": 2,
            "final_state_hash": "legacy-hash",
            "artifacts": []
        });
        let manifest: RunManifest = serde_json::from_value(legacy)
            .expect("legacy manifest without schema_version should still deserialize");
        let reserialized = serde_json::to_value(&manifest).expect("manifest should reserialize");
        assert_eq!(
            reserialized["schema_version"], 0,
            "manifests missing schema_version should default to 0"
        );
    }
}
