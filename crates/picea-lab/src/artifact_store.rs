use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use picea::tools::observability::LabArtifacts;

static RUN_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactStore {
    root_dir: PathBuf,
}

impl ArtifactStore {
    pub fn new(root_dir: impl Into<PathBuf>) -> Self {
        Self {
            root_dir: root_dir.into(),
        }
    }

    pub fn default_root_dir() -> PathBuf {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let workspace_dir = manifest_dir
            .parent()
            .and_then(Path::parent)
            .unwrap_or(manifest_dir);
        workspace_dir.join("target").join("picea-lab").join("runs")
    }

    #[cfg(test)]
    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    pub fn run_dir(&self, run_id: impl AsRef<str>) -> PathBuf {
        self.root_dir.join(run_id.as_ref())
    }

    pub fn write_run(
        &self,
        run_id: impl AsRef<str>,
        artifacts: &LabArtifacts,
    ) -> io::Result<PathBuf> {
        let run_dir = self.run_dir(run_id);
        artifacts.write_to_dir(&run_dir)?;
        Ok(run_dir)
    }

    pub fn latest_run_dir(&self) -> io::Result<Option<PathBuf>> {
        let read_dir = match fs::read_dir(&self.root_dir) {
            Ok(read_dir) => read_dir,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(err),
        };

        let mut latest: Option<(Duration, PathBuf)> = None;
        for entry in read_dir {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }

            let modified = entry
                .metadata()?
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .unwrap_or(Duration::ZERO);
            let candidate = (modified, entry.path());

            if latest.as_ref().map_or(true, |current| candidate > *current) {
                latest = Some(candidate);
            }
        }

        Ok(latest.map(|(_, path)| path))
    }
}

impl Default for ArtifactStore {
    fn default() -> Self {
        Self::new(Self::default_root_dir())
    }
}

pub fn make_run_id(prefix: impl AsRef<str>) -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let pid = std::process::id();
    let seq = RUN_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{}-{millis}-{pid}-{seq}", prefix.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use picea::tools::observability::{
        DebugRenderFrame, FinalSnapshot, LabEventKind, LabPhase, PerfSummary, SceneStateSnapshot,
        TraceEvent,
    };

    fn sample_artifacts(run_id: &str) -> LabArtifacts {
        LabArtifacts {
            trace_events: vec![TraceEvent {
                run_id: run_id.to_owned(),
                tick: 1,
                frame: 1,
                timestamp_us: Some(100),
                duration_us: None,
                phase: LabPhase::SceneTick,
                substep: Some(0),
                event_kind: LabEventKind::SnapshotCaptured,
                element_ids: vec![],
                pair_id: None,
                manifold_id: None,
                reason: Some("test".to_owned()),
                values: Default::default(),
            }],
            final_snapshot: FinalSnapshot {
                run_id: run_id.to_owned(),
                tick: 1,
                frame: 1,
                scene_state: SceneStateSnapshot {
                    element_count: 0,
                    total_duration: 0.0,
                },
                elements: vec![],
                active_pairs: vec![],
                contacts: vec![],
                manifolds: vec![],
            },
            debug_render: DebugRenderFrame {
                run_id: run_id.to_owned(),
                tick: 1,
                frame: 1,
                world_bounds: None,
                shapes: vec![],
                contacts: vec![],
                manifold_labels: vec![],
                overlay_text: vec![],
            },
            perf: PerfSummary {
                run_id: run_id.to_owned(),
                tick: 1,
                frame: 1,
                counters: vec![],
            },
        }
    }

    fn unique_test_root() -> PathBuf {
        std::env::temp_dir().join(make_run_id("picea-lab-artifact-store"))
    }

    #[test]
    fn default_root_dir_contains_workspace_runs_path() {
        let store = ArtifactStore::default();
        assert!(
            store
                .root_dir()
                .ends_with(Path::new("target/picea-lab/runs")),
            "default root dir should point at the lab runs tree"
        );
    }

    #[test]
    fn write_run_then_latest_run_dir_can_be_read_back() {
        let root_dir = unique_test_root();
        let store = ArtifactStore::new(&root_dir);
        let run_id = make_run_id("scene-first");
        let artifacts = sample_artifacts(&run_id);

        let written_dir = store
            .write_run(&run_id, &artifacts)
            .expect("artifacts write to run dir");
        let latest_dir = store
            .latest_run_dir()
            .expect("latest run dir lookup succeeds")
            .expect("latest run dir exists after write");

        assert_eq!(written_dir, latest_dir);

        let loaded = LabArtifacts::read_from_dir(&latest_dir).expect("artifacts reload");
        assert_eq!(loaded, artifacts);

        fs::remove_dir_all(&root_dir).expect("cleanup test artifact root");
    }
}
