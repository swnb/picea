//! Builtin deterministic scenarios and reset-time overrides.
//!
//! A scenario is the lab's reproducible input fixture. It constructs a fresh
//! `picea::World` from public core APIs so live sessions can reset after edits
//! without mutating an in-flight simulation.

use std::{
    collections::BTreeMap,
    fmt::{Display, Formatter},
    str::FromStr,
};

use picea::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{LabError, LabResult};

mod fixture;
mod geometry;
mod scene_dispatch;
mod scene_lattice;
mod scene_newton_cradle;
mod scene_params;
mod scene_rect_stack;
mod scene_static;

// Kept to preserve the pre-split `crate::scenario::*` path set; no current call site
// goes through this exact re-export path.
pub use fixture::{
    instantiate_scene_fixture, CompoundProvenance, CompoundProvenancePiece, SceneBodyFixture,
    SceneCompoundPieceFixture, SceneCompoundPieceShapeFixture, SceneDistanceJointFixture,
    SceneFixtureWorld, SceneJointFixture, SceneRecipeFixture, SceneShapeFixture,
    SceneWorldAnchorJointFixture, SCENE_RECIPE_SCHEMA_VERSION,
};
#[allow(unused_imports)]
pub(crate) use fixture::{instantiate_scene_fixture_with_provenance, InstantiatedSceneFixture};
pub(crate) use scene_dispatch::build_scenario;
#[allow(unused_imports)]
pub(crate) use scene_dispatch::BuiltScenario;

use scene_lattice::{
    default_lattice_runtime_parts, lattice_parameter_schema, lattice_runtime_config,
    resolve_lattice_runtime_parts,
};
use scene_newton_cradle::{
    default_newton_cradle_runtime_parts, newton_cradle_parameter_schema,
    newton_cradle_runtime_config, resolve_newton_cradle_runtime_parts,
};
use scene_rect_stack::{
    default_rect_stack_runtime_parts, rect_stack_parameter_schema, rect_stack_runtime_config,
    resolve_rect_stack_runtime_parts,
};

/// Stable identifiers for the builtin CS-simulator scenarios.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioId {
    FallingBoxContact,
    #[serde(rename = "stack_4")]
    Stack4,
    StackStabilityTower,
    MatrixStack,
    MatrixStackAligned,
    NewtonCradle,
    JointAnchor,
    LatticeGrid,
    BroadphaseSparse,
    SatPolygon,
    CompoundProvenance,
    ConcaveDecomposition,
    CcdFastCircleWall,
    CcdFastConvexWalls,
    CcdDynamicConvexPair,
    CcdDynamicCompoundWall,
}

impl ScenarioId {
    pub const ALL: [Self; 16] = [
        Self::FallingBoxContact,
        Self::Stack4,
        Self::StackStabilityTower,
        Self::MatrixStack,
        Self::MatrixStackAligned,
        Self::NewtonCradle,
        Self::JointAnchor,
        Self::LatticeGrid,
        Self::BroadphaseSparse,
        Self::SatPolygon,
        Self::CompoundProvenance,
        Self::ConcaveDecomposition,
        Self::CcdFastCircleWall,
        Self::CcdFastConvexWalls,
        Self::CcdDynamicConvexPair,
        Self::CcdDynamicCompoundWall,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FallingBoxContact => "falling_box_contact",
            Self::Stack4 => "stack_4",
            Self::StackStabilityTower => "stack_stability_tower",
            Self::MatrixStack => "matrix_stack",
            Self::MatrixStackAligned => "matrix_stack_aligned",
            Self::NewtonCradle => "newton_cradle",
            Self::JointAnchor => "joint_anchor",
            Self::LatticeGrid => "lattice_grid",
            Self::BroadphaseSparse => "broadphase_sparse",
            Self::SatPolygon => "sat_polygon",
            Self::CompoundProvenance => "compound_provenance",
            Self::ConcaveDecomposition => "concave_decomposition",
            Self::CcdFastCircleWall => "ccd_fast_circle_wall",
            Self::CcdFastConvexWalls => "ccd_fast_convex_walls",
            Self::CcdDynamicConvexPair => "ccd_dynamic_convex_pair",
            Self::CcdDynamicCompoundWall => "ccd_dynamic_compound_wall",
        }
    }
}

impl Display for ScenarioId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ScenarioId {
    type Err = LabError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "falling_box_contact" => Ok(Self::FallingBoxContact),
            "stack_4" => Ok(Self::Stack4),
            "stack_stability_tower" => Ok(Self::StackStabilityTower),
            "matrix_stack" => Ok(Self::MatrixStack),
            "matrix_stack_aligned" => Ok(Self::MatrixStackAligned),
            "newton_cradle" => Ok(Self::NewtonCradle),
            "joint_anchor" => Ok(Self::JointAnchor),
            "lattice_grid" => Ok(Self::LatticeGrid),
            "broadphase_sparse" => Ok(Self::BroadphaseSparse),
            "sat_polygon" => Ok(Self::SatPolygon),
            "compound_provenance" => Ok(Self::CompoundProvenance),
            "concave_decomposition" => Ok(Self::ConcaveDecomposition),
            "ccd_fast_circle_wall" => Ok(Self::CcdFastCircleWall),
            "ccd_fast_convex_walls" => Ok(Self::CcdFastConvexWalls),
            "ccd_dynamic_convex_pair" => Ok(Self::CcdDynamicConvexPair),
            "ccd_dynamic_compound_wall" => Ok(Self::CcdDynamicCompoundWall),
            other => Err(LabError::UnknownScenario(other.to_owned())),
        }
    }
}

/// Human-facing metadata for one builtin scenario.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ScenarioDescriptor {
    pub id: ScenarioId,
    pub name: &'static str,
    pub description: &'static str,
    pub default_runtime_config: ScenarioRuntimeConfig,
    pub parameter_schema: Vec<ScenarioParameterDescriptor>,
}

/// Effective reset-time configuration owned by one scene run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScenarioRuntimeConfig {
    pub step: StepConfig,
    #[serde(default = "default_substeps_per_frame")]
    pub substeps_per_frame: usize,
    #[serde(default)]
    pub scene_params: BTreeMap<String, Value>,
}

impl Default for ScenarioRuntimeConfig {
    fn default() -> Self {
        Self {
            step: StepConfig::default(),
            substeps_per_frame: default_substeps_per_frame(),
            scene_params: BTreeMap::new(),
        }
    }
}

fn default_substeps_per_frame() -> usize {
    1
}

/// TS-friendly schema for one scene parameter. This is intentionally a small
/// lab-owned shape rather than a full JSON Schema implementation.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ScenarioParameterDescriptor {
    pub key: &'static str,
    pub label: &'static str,
    #[serde(rename = "type")]
    pub value_type: ScenarioParameterValueType,
    pub default: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<f32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<ScenarioParameterOption>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioParameterValueType {
    Integer,
    Number,
    Boolean,
    Select,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ScenarioParameterOption {
    pub value: Value,
    pub label: &'static str,
}

pub fn list_scenarios() -> Vec<ScenarioDescriptor> {
    ScenarioId::ALL
        .into_iter()
        .map(|id| ScenarioDescriptor {
            id,
            name: match id {
                ScenarioId::FallingBoxContact => "Falling box contact",
                ScenarioId::Stack4 => "Four box stack",
                ScenarioId::StackStabilityTower => "Stack stability tower",
                ScenarioId::MatrixStack => "Matrix stack 8x6",
                ScenarioId::MatrixStackAligned => "Aligned matrix stack 4x3",
                ScenarioId::NewtonCradle => "Newton cradle",
                ScenarioId::JointAnchor => "World anchor joint",
                ScenarioId::LatticeGrid => "Rigid-body lattice grid proxy",
                ScenarioId::BroadphaseSparse => "Sparse broadphase",
                ScenarioId::SatPolygon => "SAT polygon manifold",
                ScenarioId::CompoundProvenance => "Compound provenance fixture",
                ScenarioId::ConcaveDecomposition => "Concave decomposition fixture",
                ScenarioId::CcdFastCircleWall => "CCD fast circle wall",
                ScenarioId::CcdFastConvexWalls => "CCD fast convex walls",
                ScenarioId::CcdDynamicConvexPair => "CCD dynamic convex pair",
                ScenarioId::CcdDynamicCompoundWall => "CCD dynamic compound wall",
            },
            description: match id {
                ScenarioId::FallingBoxContact => "A dynamic box falling into static floor contact.",
                ScenarioId::Stack4 => "Four dynamic boxes stacked above a static floor.",
                ScenarioId::StackStabilityTower => {
                    "A taller deterministic tower with narrow support, staged settling, and sleep-ready stack facts."
                }
                ScenarioId::MatrixStack => {
                    "A staggered 8x6 dynamic box matrix on a static floor for edge-ejection stress diagnostics."
                }
                ScenarioId::MatrixStackAligned => {
                    "An aligned 4x3 dynamic box matrix on a static floor for stable matrix-form behavior locks."
                }
                ScenarioId::NewtonCradle => {
                    "Five suspended bouncy balls with a long-window kinetic-energy retention check."
                }
                ScenarioId::JointAnchor => "A body constrained toward a fixed world-space anchor.",
                ScenarioId::LatticeGrid => {
                    "A rigid-body joint lattice / grid proxy built from dynamic nodes and distance/world-anchor joints, not a true soft-body solver."
                }
                ScenarioId::BroadphaseSparse => {
                    "Five static boxes with exactly one broadphase overlap."
                }
                ScenarioId::SatPolygon => {
                    "A rectangle and convex polygon exposing clipped manifold points."
                }
                ScenarioId::CompoundProvenance => {
                    "An authored compound body fixture exposing stable piece order and inherited collider semantics."
                }
                ScenarioId::ConcaveDecomposition => {
                    "A static concave polygon decomposed into deterministic convex pieces."
                }
                ScenarioId::CcdFastCircleWall => {
                    "A fast dynamic circle swept against a static thin rectangle wall."
                }
                ScenarioId::CcdFastConvexWalls => {
                    "A fast dynamic rectangle swept against two static thin walls."
                }
                ScenarioId::CcdDynamicConvexPair => {
                    "Two fast dynamic rectangles swept against each other."
                }
                ScenarioId::CcdDynamicCompoundWall => {
                    "A fast compound body swept by its earliest convex piece against a static wall."
                }
            },
            default_runtime_config: default_runtime_config_for_scenario(id),
            parameter_schema: parameter_schema_for_scenario(id),
        })
        .collect()
}

pub fn default_runtime_config_for_scenario(scenario_id: ScenarioId) -> ScenarioRuntimeConfig {
    match scenario_id {
        ScenarioId::Stack4 | ScenarioId::MatrixStack | ScenarioId::MatrixStackAligned => {
            let (params, step, substeps_per_frame) = default_rect_stack_runtime_parts(scenario_id);
            rect_stack_runtime_config(params, step, substeps_per_frame)
        }
        ScenarioId::NewtonCradle => {
            let (params, step, substeps_per_frame) = default_newton_cradle_runtime_parts();
            newton_cradle_runtime_config(params, step, substeps_per_frame)
        }
        ScenarioId::LatticeGrid => {
            let (params, step, substeps_per_frame) = default_lattice_runtime_parts();
            lattice_runtime_config(params, step, substeps_per_frame)
        }
        _ => ScenarioRuntimeConfig::default(),
    }
}

pub(crate) fn effective_runtime_config_for_scenario(
    scenario_id: ScenarioId,
    overrides: &ScenarioOverrides,
) -> LabResult<ScenarioRuntimeConfig> {
    match scenario_id {
        ScenarioId::Stack4 | ScenarioId::MatrixStack | ScenarioId::MatrixStackAligned => {
            let (params, step, substeps_per_frame) =
                resolve_rect_stack_runtime_parts(scenario_id, &overrides.scene_params)?;
            Ok(rect_stack_runtime_config(params, step, substeps_per_frame))
        }
        ScenarioId::NewtonCradle => {
            let (params, step, substeps_per_frame) =
                resolve_newton_cradle_runtime_parts(&overrides.scene_params)?;
            Ok(newton_cradle_runtime_config(
                params,
                step,
                substeps_per_frame,
            ))
        }
        ScenarioId::LatticeGrid => {
            let (params, step, substeps_per_frame) =
                resolve_lattice_runtime_parts(&overrides.scene_params)?;
            Ok(lattice_runtime_config(params, step, substeps_per_frame))
        }
        _ if overrides.scene_params.is_empty() => {
            Ok(default_runtime_config_for_scenario(scenario_id))
        }
        _ => Err(LabError::World(format!(
            "scenario {scenario_id} does not define scene_params"
        ))),
    }
}

fn parameter_schema_for_scenario(scenario_id: ScenarioId) -> Vec<ScenarioParameterDescriptor> {
    match scenario_id {
        ScenarioId::Stack4 | ScenarioId::MatrixStack | ScenarioId::MatrixStackAligned => {
            rect_stack_parameter_schema(scenario_id)
        }
        ScenarioId::NewtonCradle => newton_cradle_parameter_schema(),
        ScenarioId::LatticeGrid => lattice_parameter_schema(),
        _ => Vec::new(),
    }
}

/// Session/scenario overrides are user-supplied knobs stored outside the core
/// world. They are reapplied when a session is reset.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ScenarioOverrides {
    pub frame_count: Option<usize>,
    pub gravity: Option<[f32; 2]>,
    #[serde(default)]
    pub scene_params: BTreeMap<String, Value>,
}

/// Input for one deterministic scenario run.
#[derive(Clone, Debug, PartialEq)]
pub struct RunConfig {
    pub scenario_id: ScenarioId,
    pub frame_count: usize,
    pub run_id: Option<String>,
    pub overrides: ScenarioOverrides,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            scenario_id: ScenarioId::FallingBoxContact,
            frame_count: 120,
            run_id: None,
            overrides: ScenarioOverrides::default(),
        }
    }
}

impl RunConfig {
    pub(crate) fn effective_frame_count(&self) -> usize {
        self.overrides
            .frame_count
            .unwrap_or(self.frame_count)
            .max(1)
    }
}
