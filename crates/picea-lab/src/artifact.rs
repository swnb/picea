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
    scenario::{build_scenario, RunConfig, ScenarioId},
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

/// Manifest schema for `manifest.json`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RunManifest {
    pub run_id: String,
    pub scenario_id: ScenarioId,
    pub frame_count: usize,
    pub final_state_hash: String,
    pub artifacts: Vec<ArtifactEntry>,
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
    pub broadphase_rebuild_count: usize,
    pub broadphase_tree_depth: usize,
    pub contact_count: usize,
    pub world_bounds: Option<DebugAabb>,
    pub bodies: Vec<DebugBody>,
    pub colliders: Vec<DebugCollider>,
    pub contacts: Vec<DebugContact>,
    pub manifolds: Vec<DebugManifold>,
    pub unmeasured: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PerfArtifact {
    pub frame_count: usize,
    pub elapsed_micros: u128,
    pub final_state_hash: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RunResult {
    pub path: PathBuf,
    pub manifest: RunManifest,
    pub frames: Vec<FrameRecord>,
}

/// Filesystem boundary that hides `target/picea-lab/runs/<run_id>` from higher
/// level CLI and server flows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactStore {
    root: PathBuf,
}

impl ArtifactStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn default_in_workspace() -> Self {
        Self::new("target/picea-lab/runs")
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
    let mut pipeline = SimulationPipeline::new(StepConfig::default());
    let mut frames = Vec::with_capacity(frame_count);

    for frame_index in 0..frame_count {
        let report = pipeline.step(&mut scenario.world);
        let snapshot = DebugSnapshot::from_world_with_step_report(
            &scenario.world,
            &report,
            &DebugSnapshotOptions::default(),
        );
        let state_hash = state_hash(&snapshot)?;
        frames.push(FrameRecord {
            frame_index,
            simulated_time: report.simulated_time,
            state_hash,
            stats: report.stats,
            events: report.events.clone(),
            report,
            snapshot,
        });
    }

    let final_snapshot = frames
        .last()
        .map(|frame| frame.snapshot.clone())
        .unwrap_or_else(DebugSnapshot::default);
    let final_state_hash = state_hash(&final_snapshot)?;
    let manifest = RunManifest {
        run_id: run_id.clone(),
        scenario_id: config.scenario_id,
        frame_count,
        final_state_hash: final_state_hash.clone(),
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
                    broadphase_rebuild_count: frame.snapshot.stats.broadphase_rebuild_count,
                    broadphase_tree_depth: frame.snapshot.stats.broadphase_tree_depth,
                    contact_count: frame.snapshot.contacts.len(),
                    world_bounds: frame.snapshot.world_bounds(),
                    bodies: frame.snapshot.bodies.clone(),
                    colliders: frame.snapshot.colliders.clone(),
                    contacts: frame.snapshot.contacts.clone(),
                    manifolds: frame.snapshot.manifolds.clone(),
                    // These names are intentionally explicit so the viewer does
                    // not present first-slice placeholders as measured facts.
                    unmeasured: ["contact_impulses", "forces", "torques"]
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
        },
    )?;

    Ok(RunResult {
        path: run_path,
        manifest,
        frames,
    })
}

fn artifact_entries() -> Vec<ArtifactEntry> {
    ArtifactFile::ALL
        .into_iter()
        .map(|file| ArtifactEntry {
            file: file.file_name().to_owned(),
            content_type: match file {
                ArtifactFile::Frames => "application/x-ndjson",
                _ => "application/json",
            }
            .to_owned(),
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
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    Ok(format!("{hash:016x}"))
}
