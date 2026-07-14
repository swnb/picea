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
    DebugRenderFrame, DiagnosticMarker, DiagnosticMarkerKind, DiagnosticSeverity, DiagnosticSource,
    FrameDiagnostics, FrameRecord, MissingEvidence, MissingEvidenceKind, PerfArtifact, RunManifest,
    RunResult,
};
pub use error::{LabError, LabResult};
pub use scenario::{
    default_runtime_config_for_scenario, instantiate_scene_fixture, list_scenarios,
    CompoundProvenance, CompoundProvenancePiece, RunConfig, ScenarioDescriptor, ScenarioId,
    ScenarioOverrides, ScenarioParameterDescriptor, ScenarioParameterOption,
    ScenarioParameterValueType, ScenarioRuntimeConfig, SceneBodyFixture, SceneDistanceJointFixture,
    SceneFixtureWorld, SceneJointFixture, SceneRecipeFixture, SceneRevoluteJointFixture,
    SceneShapeFixture, SceneWorldAnchorJointFixture, SCENE_RECIPE_SCHEMA_VERSION,
};
pub use server::SessionStatus;
