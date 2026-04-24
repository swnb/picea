//! Builtin deterministic scenarios and reset-time overrides.
//!
//! A scenario is the lab's reproducible input fixture. It constructs a fresh
//! `picea::World` from public core APIs so live sessions can reset after edits
//! without mutating an in-flight simulation.

use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

use picea::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{LabError, LabResult};

/// Stable identifiers for the builtin CS-simulator scenarios.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioId {
    FallingBoxContact,
    #[serde(rename = "stack_4")]
    Stack4,
    JointAnchor,
}

impl ScenarioId {
    pub const ALL: [Self; 3] = [Self::FallingBoxContact, Self::Stack4, Self::JointAnchor];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FallingBoxContact => "falling_box_contact",
            Self::Stack4 => "stack_4",
            Self::JointAnchor => "joint_anchor",
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
            "joint_anchor" => Ok(Self::JointAnchor),
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
}

pub fn list_scenarios() -> Vec<ScenarioDescriptor> {
    ScenarioId::ALL
        .into_iter()
        .map(|id| ScenarioDescriptor {
            id,
            name: match id {
                ScenarioId::FallingBoxContact => "Falling box contact",
                ScenarioId::Stack4 => "Four box stack",
                ScenarioId::JointAnchor => "World anchor joint",
            },
            description: match id {
                ScenarioId::FallingBoxContact => "A dynamic box falling into static floor contact.",
                ScenarioId::Stack4 => "Four dynamic boxes stacked above a static floor.",
                ScenarioId::JointAnchor => "A body constrained toward a fixed world-space anchor.",
            },
        })
        .collect()
}

/// Session/scenario overrides are user-supplied knobs stored outside the core
/// world. They are reapplied when a session is reset.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ScenarioOverrides {
    pub frame_count: Option<usize>,
    pub gravity: Option<[f32; 2]>,
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

pub(crate) struct BuiltScenario {
    pub(crate) world: World,
}

pub(crate) fn build_scenario(
    id: ScenarioId,
    overrides: &ScenarioOverrides,
) -> LabResult<BuiltScenario> {
    let gravity = overrides
        .gravity
        .map(|[x, y]| Vector::new(x, y))
        .unwrap_or_else(|| Vector::new(0.0, 9.8));
    let mut world = World::new(WorldDesc {
        gravity,
        enable_sleep: true,
    });

    match id {
        ScenarioId::FallingBoxContact => {
            add_box(&mut world, BodyType::Static, 0.0, 2.0, 8.0, 0.5)?;
            add_box(&mut world, BodyType::Dynamic, 0.0, -2.0, 1.0, 1.0)?;
        }
        ScenarioId::Stack4 => {
            add_box(&mut world, BodyType::Static, 0.0, 2.5, 10.0, 0.5)?;
            for index in 0..4 {
                add_box(
                    &mut world,
                    BodyType::Dynamic,
                    0.0,
                    1.7 - index as f32,
                    0.9,
                    0.9,
                )?;
            }
        }
        ScenarioId::JointAnchor => {
            world = World::new(WorldDesc {
                gravity: Vector::default(),
                enable_sleep: false,
            });
            let body = add_box(&mut world, BodyType::Dynamic, 2.0, 0.0, 0.8, 0.8)?;
            world
                .create_joint(JointDesc::WorldAnchor(WorldAnchorJointDesc {
                    body,
                    world_anchor: Point::new(0.0, 0.0),
                    stiffness: 4.0,
                    damping: 0.2,
                    ..WorldAnchorJointDesc::default()
                }))
                .map_err(|error| LabError::World(error.to_string()))?;
        }
    }

    Ok(BuiltScenario { world })
}

fn add_box(
    world: &mut World,
    body_type: BodyType,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> LabResult<BodyHandle> {
    let body = world
        .create_body(BodyDesc {
            body_type,
            pose: Pose::from_xy_angle(x, y, 0.0),
            can_sleep: false,
            ..BodyDesc::default()
        })
        .map_err(|error| LabError::World(error.to_string()))?;
    world
        .create_collider(
            body,
            ColliderDesc {
                shape: SharedShape::rect(width, height),
                ..ColliderDesc::default()
            },
        )
        .map_err(|error| LabError::World(error.to_string()))?;
    Ok(body)
}
