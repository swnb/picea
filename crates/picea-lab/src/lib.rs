//! Local C/S tooling around the `picea` core engine.
//!
//! `picea-lab` owns scenario fixtures, artifact persistence, and the local HTTP
//! server used by the React workbench. The physics core crate stays dependency
//! light: this wrapper only talks to `picea` through public APIs and serializable
//! debug facts.

pub mod artifact;
pub mod cli;
mod error;
pub mod scenario;
pub mod server;

pub use artifact::{
    run_scenario, ArtifactEntry, ArtifactFile, ArtifactStore, DebugRenderArtifact,
    DebugRenderFrame, FrameRecord, PerfArtifact, RunManifest, RunResult,
};
pub use error::{LabError, LabResult};
pub use scenario::{list_scenarios, RunConfig, ScenarioDescriptor, ScenarioId, ScenarioOverrides};
pub use server::SessionStatus;
