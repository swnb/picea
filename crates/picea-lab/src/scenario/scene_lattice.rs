use std::collections::BTreeMap;

use picea::prelude::*;
use serde_json::{json, Value};

use crate::{LabError, LabResult};

use super::fixture::{
    SceneBodyFixture, SceneDistanceJointFixture, SceneFixtureWorld, SceneJointFixture,
    SceneRecipeFixture, SceneShapeFixture, SceneWorldAnchorJointFixture,
};
use super::scene_params::{
    integer_param, number_param, parse_f32_param, parse_usize_param, select_param,
};
use super::{
    ScenarioParameterDescriptor, ScenarioParameterOption, ScenarioParameterValueType,
    ScenarioRuntimeConfig, SCENE_RECIPE_SCHEMA_VERSION,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LatticeConstraintProfile {
    Soft,
    Balanced,
    Hard,
}

impl LatticeConstraintProfile {
    const fn as_param_value(self) -> &'static str {
        match self {
            Self::Soft => "soft",
            Self::Balanced => "balanced",
            Self::Hard => "hard",
        }
    }

    const fn default_joint_velocity_projection(self) -> bool {
        match self {
            Self::Soft => false,
            Self::Balanced | Self::Hard => true,
        }
    }

    const fn default_substeps_per_frame(self) -> usize {
        match self {
            Self::Soft => 1,
            Self::Balanced => 8,
            Self::Hard => 8,
        }
    }

    const fn values(self) -> LatticeConstraintProfileValues {
        match self {
            Self::Soft => LatticeConstraintProfileValues {
                anchor_stiffness: 6.5,
                anchor_damping: 0.42,
                distance_stiffness: 5.0,
                diagonal_stiffness: 4.4,
                distance_damping: 0.36,
            },
            Self::Balanced => LatticeConstraintProfileValues {
                anchor_stiffness: 260.0,
                anchor_damping: 1.2,
                distance_stiffness: 220.0,
                diagonal_stiffness: 200.0,
                distance_damping: 1.0,
            },
            Self::Hard => LatticeConstraintProfileValues {
                anchor_stiffness: 420.0,
                anchor_damping: 1.8,
                distance_stiffness: 360.0,
                diagonal_stiffness: 320.0,
                distance_damping: 1.4,
            },
        }
    }

    fn from_param_value(key: &str, value: &Value) -> LabResult<Self> {
        match value.as_str() {
            Some("soft") => Ok(Self::Soft),
            Some("balanced") => Ok(Self::Balanced),
            Some("hard") => Ok(Self::Hard),
            _ => Err(LabError::World(format!(
                "scene_params.{key} must be soft, balanced, or hard"
            ))),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct LatticeConstraintProfileValues {
    anchor_stiffness: f32,
    anchor_damping: f32,
    distance_stiffness: f32,
    diagonal_stiffness: f32,
    distance_damping: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct LatticeGridParams {
    columns: usize,
    rows: usize,
    spacing_x: f32,
    spacing_y: f32,
    node_radius: f32,
    constraint_profile: LatticeConstraintProfile,
    joint_velocity_projection: bool,
}

/// M38 intentionally stops at a rigid-body proxy. Each "node" is still one
/// ordinary rigid body, so any visible deformation comes from existing joint
/// constraints and debug facts rather than a new soft-body solver.
pub(super) fn lattice_grid_fixture(
    gravity: [f32; 2],
    params: LatticeGridParams,
) -> SceneRecipeFixture {
    const ORIGIN_Y: f32 = -1.22;

    let columns = params.columns.max(2);
    let rows = params.rows.max(2);
    let origin_x = -0.5 * (columns.saturating_sub(1) as f32 * params.spacing_x);
    let profile = params.constraint_profile.values();

    let mut bodies = Vec::with_capacity(columns * rows);
    for row in 0..rows {
        for col in 0..columns {
            bodies.push(SceneBodyFixture {
                body_type: if row == 0 {
                    BodyType::Kinematic
                } else {
                    BodyType::Dynamic
                },
                pose: [
                    origin_x + col as f32 * params.spacing_x,
                    ORIGIN_Y + row as f32 * params.spacing_y,
                    0.0,
                ],
                linear_velocity: [0.0, 0.0],
                can_sleep: row != 0,
                shape: SceneShapeFixture::Circle {
                    radius: params.node_radius,
                },
                material: if row == 0 {
                    MaterialPreset::Rough
                } else {
                    MaterialPreset::Default
                },
                filter: CollisionLayerPreset::DynamicBody,
                density: 0.9,
                is_sensor: false,
            });
        }
    }

    let body_index = |row: usize, col: usize| row * columns + col;
    let mut joints = Vec::new();

    for col in 0..columns {
        joints.push(SceneJointFixture::WorldAnchor(
            SceneWorldAnchorJointFixture {
                body: body_index(0, col),
                world_anchor: Some([
                    origin_x + col as f32 * params.spacing_x,
                    ORIGIN_Y + if col % 2 == 0 { -0.18 } else { -0.22 },
                ]),
                local_anchor: None,
                stiffness: Some(profile.anchor_stiffness),
                damping: Some(profile.anchor_damping),
            },
        ));
    }

    for row in 0..rows {
        for col in 0..columns {
            if col + 1 < columns {
                joints.push(SceneJointFixture::Distance(SceneDistanceJointFixture {
                    body_a: body_index(row, col),
                    body_b: body_index(row, col + 1),
                    rest_length: Some(params.spacing_x),
                    stiffness: Some(profile.distance_stiffness),
                    damping: Some(profile.distance_damping),
                    local_anchor_a: None,
                    local_anchor_b: None,
                }));
            }
            if row + 1 < rows {
                joints.push(SceneJointFixture::Distance(SceneDistanceJointFixture {
                    body_a: body_index(row, col),
                    body_b: body_index(row + 1, col),
                    rest_length: Some(params.spacing_y),
                    stiffness: Some(profile.distance_stiffness),
                    damping: Some(profile.distance_damping),
                    local_anchor_a: None,
                    local_anchor_b: None,
                }));
            }
            if row + 1 < rows && col + 1 < columns {
                let (body_a, body_b) = if (row + col) % 2 == 0 {
                    (body_index(row, col), body_index(row + 1, col + 1))
                } else {
                    (body_index(row, col + 1), body_index(row + 1, col))
                };
                joints.push(SceneJointFixture::Distance(SceneDistanceJointFixture {
                    body_a,
                    body_b,
                    rest_length: Some(
                        (params.spacing_x * params.spacing_x + params.spacing_y * params.spacing_y)
                            .sqrt(),
                    ),
                    stiffness: Some(profile.diagonal_stiffness),
                    damping: Some(profile.distance_damping),
                    local_anchor_a: None,
                    local_anchor_b: None,
                }));
            }
        }
    }

    SceneRecipeFixture {
        schema_version: SCENE_RECIPE_SCHEMA_VERSION,
        world: SceneFixtureWorld {
            gravity,
            enable_sleep: true,
        },
        bodies,
        joints,
    }
}

pub(super) fn default_lattice_runtime_parts() -> (LatticeGridParams, StepConfig, usize) {
    let profile = LatticeConstraintProfile::Balanced;
    let params = LatticeGridParams {
        columns: 4,
        rows: 3,
        spacing_x: 0.62,
        spacing_y: 0.58,
        node_radius: 0.12,
        constraint_profile: profile,
        joint_velocity_projection: profile.default_joint_velocity_projection(),
    };
    let substeps_per_frame = profile.default_substeps_per_frame();
    let mut step = StepConfig::default();
    step.dt = 1.0 / (60.0 * substeps_per_frame as f32);
    step.joint_velocity_projection = params.joint_velocity_projection;
    (params, step, substeps_per_frame)
}

pub(super) fn lattice_runtime_config(
    params: LatticeGridParams,
    step: StepConfig,
    substeps_per_frame: usize,
) -> ScenarioRuntimeConfig {
    let mut scene_params = BTreeMap::new();
    scene_params.insert("columns".to_owned(), json!(params.columns));
    scene_params.insert("rows".to_owned(), json!(params.rows));
    scene_params.insert("spacing_x".to_owned(), json!(params.spacing_x));
    scene_params.insert("spacing_y".to_owned(), json!(params.spacing_y));
    scene_params.insert("node_radius".to_owned(), json!(params.node_radius));
    scene_params.insert(
        "constraint_profile".to_owned(),
        json!(params.constraint_profile.as_param_value()),
    );
    scene_params.insert(
        "joint_velocity_projection".to_owned(),
        json!(step.joint_velocity_projection),
    );
    scene_params.insert("substeps_per_frame".to_owned(), json!(substeps_per_frame));

    ScenarioRuntimeConfig {
        step,
        substeps_per_frame,
        scene_params,
    }
}

pub(super) fn resolve_lattice_runtime_parts(
    overrides: &BTreeMap<String, Value>,
) -> LabResult<(LatticeGridParams, StepConfig, usize)> {
    let requested_profile = overrides
        .get("constraint_profile")
        .map(|value| LatticeConstraintProfile::from_param_value("constraint_profile", value))
        .transpose()?;
    let (mut params, mut step, mut substeps_per_frame) = default_lattice_runtime_parts();
    if let Some(profile) = requested_profile {
        params.constraint_profile = profile;
        params.joint_velocity_projection = profile.default_joint_velocity_projection();
        substeps_per_frame = profile.default_substeps_per_frame();
    }

    for (key, value) in overrides {
        match key.as_str() {
            "columns" => params.columns = parse_usize_param(key, value, 2, 8)?,
            "rows" => params.rows = parse_usize_param(key, value, 2, 6)?,
            "spacing_x" => params.spacing_x = parse_f32_param(key, value, 0.3, 1.4)?,
            "spacing_y" => params.spacing_y = parse_f32_param(key, value, 0.3, 1.4)?,
            "node_radius" => params.node_radius = parse_f32_param(key, value, 0.05, 0.28)?,
            "constraint_profile" => {}
            "joint_velocity_projection" => {
                params.joint_velocity_projection = value.as_bool().ok_or_else(|| {
                    LabError::World(
                        "scene_params.joint_velocity_projection must be a boolean".to_owned(),
                    )
                })?;
            }
            "substeps_per_frame" => substeps_per_frame = parse_usize_param(key, value, 1, 8)?,
            other => {
                return Err(LabError::World(format!(
                    "scene_params.{other}: unknown lattice_grid parameter"
                )));
            }
        }
    }

    validate_lattice_params(params)?;
    step.dt = 1.0 / (60.0 * substeps_per_frame as f32);
    step.joint_velocity_projection = params.joint_velocity_projection;
    Ok((params, step, substeps_per_frame))
}

fn validate_lattice_params(params: LatticeGridParams) -> LabResult<()> {
    if params.columns.saturating_mul(params.rows) > 48 {
        return Err(LabError::World(
            "scene_params.columns * scene_params.rows must be <= 48 for live lattice playback"
                .to_owned(),
        ));
    }
    let min_spacing = params.spacing_x.min(params.spacing_y);
    if params.node_radius * 2.0 >= min_spacing {
        return Err(LabError::World(
            "scene_params.node_radius must be smaller than half of the lattice spacing".to_owned(),
        ));
    }
    Ok(())
}

pub(super) fn lattice_parameter_schema() -> Vec<ScenarioParameterDescriptor> {
    let (params, _, substeps_per_frame) = default_lattice_runtime_parts();
    vec![
        integer_param("columns", "Columns", params.columns, 2.0, 8.0),
        integer_param("rows", "Rows", params.rows, 2.0, 6.0),
        number_param(
            "spacing_x",
            "Horizontal spacing",
            params.spacing_x,
            0.3,
            1.4,
            0.01,
        ),
        number_param(
            "spacing_y",
            "Vertical spacing",
            params.spacing_y,
            0.3,
            1.4,
            0.01,
        ),
        number_param(
            "node_radius",
            "Node radius",
            params.node_radius,
            0.05,
            0.28,
            0.01,
        ),
        select_param(
            "constraint_profile",
            "Constraint profile",
            json!(params.constraint_profile.as_param_value()),
            vec![
                ScenarioParameterOption {
                    value: json!("soft"),
                    label: "Soft",
                },
                ScenarioParameterOption {
                    value: json!("balanced"),
                    label: "Balanced",
                },
                ScenarioParameterOption {
                    value: json!("hard"),
                    label: "Hard",
                },
            ],
        ),
        ScenarioParameterDescriptor {
            key: "joint_velocity_projection",
            label: "Joint velocity projection",
            value_type: ScenarioParameterValueType::Boolean,
            default: json!(params.joint_velocity_projection),
            min: None,
            max: None,
            step: None,
            options: Vec::new(),
        },
        integer_param(
            "substeps_per_frame",
            "Substeps per frame",
            substeps_per_frame,
            1.0,
            8.0,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::scene_dispatch::build_scenario;
    use crate::scenario::{ScenarioId, ScenarioOverrides};

    #[test]
    fn lattice_grid_builtin_exports_distance_and_world_anchor_joint_grid() {
        let builtin = build_scenario(ScenarioId::LatticeGrid, &ScenarioOverrides::default())
            .expect("lattice grid scenario should build");
        let dynamic_bodies = builtin
            .world
            .bodies()
            .filter(|handle| {
                builtin
                    .world
                    .body(*handle)
                    .expect("body should resolve")
                    .body_type()
                    == BodyType::Dynamic
            })
            .count();
        assert!(
            dynamic_bodies >= 6,
            "lattice grid should keep a multi-body dynamic node lattice below its fixed top edge"
        );
        let kinematic_bodies = builtin
            .world
            .bodies()
            .filter(|handle| {
                builtin
                    .world
                    .body(*handle)
                    .expect("body should resolve")
                    .body_type()
                    == BodyType::Kinematic
            })
            .count();
        assert!(
            kinematic_bodies >= 2,
            "lattice grid should expose a fixed top edge so the proxy is readable in live playback"
        );

        let joints: Vec<_> = builtin.world.joints().collect();
        assert!(
            joints.len() >= 12,
            "lattice grid should include enough joints for a visible grid"
        );

        let mut distance_joint_count = 0usize;
        let mut world_anchor_joint_count = 0usize;
        for handle in joints {
            match builtin
                .world
                .joint(handle)
                .expect("joint should resolve")
                .desc()
            {
                JointDesc::Distance(_) => distance_joint_count += 1,
                JointDesc::WorldAnchor(_) => world_anchor_joint_count += 1,
            }
        }

        assert!(distance_joint_count >= 8);
        assert!(world_anchor_joint_count >= 2);
    }

    #[test]
    fn lattice_grid_default_profile_keeps_proxy_stretch_bounded() {
        let builtin = build_scenario(ScenarioId::LatticeGrid, &ScenarioOverrides::default())
            .expect("lattice grid scenario should build");
        let mut world = builtin.world;
        let runtime = builtin.effective_runtime_config;
        let mut pipeline = SimulationPipeline::new(runtime.step);
        let initial = picea::debug::DebugSnapshot::from_world(
            &world,
            &picea::debug::DebugSnapshotOptions::default(),
        );
        let mut observed_bounds = StretchRatioBounds { min: 1.0, max: 1.0 };

        for _ in 0..180 {
            for _ in 0..runtime.substeps_per_frame.max(1) {
                pipeline.step(&mut world);
            }
            let snapshot = picea::debug::DebugSnapshot::from_world(
                &world,
                &picea::debug::DebugSnapshotOptions::default(),
            );
            let frame_bounds = joint_stretch_ratio_bounds(&initial, &snapshot);
            observed_bounds.min = observed_bounds.min.min(frame_bounds.min);
            observed_bounds.max = observed_bounds.max.max(frame_bounds.max);
        }
        assert!(
                observed_bounds.max <= 2.0 && observed_bounds.min >= 0.5,
                "default lattice profile should stay readable in live-style playback; stretch_bounds={observed_bounds:?}"
            );
    }

    #[test]
    fn lattice_grid_scene_params_make_soft_and_hard_profiles_observable() {
        let mut soft_params = BTreeMap::new();
        soft_params.insert("constraint_profile".to_owned(), json!("soft"));
        let soft = build_scenario(
            ScenarioId::LatticeGrid,
            &ScenarioOverrides {
                scene_params: soft_params,
                ..ScenarioOverrides::default()
            },
        )
        .expect("soft lattice profile should build");

        let mut hard_params = BTreeMap::new();
        hard_params.insert("constraint_profile".to_owned(), json!("hard"));
        let hard = build_scenario(
            ScenarioId::LatticeGrid,
            &ScenarioOverrides {
                scene_params: hard_params,
                ..ScenarioOverrides::default()
            },
        )
        .expect("hard lattice profile should build");

        assert!(
            soft.effective_runtime_config.substeps_per_frame
                < hard.effective_runtime_config.substeps_per_frame,
            "hard profile should buy rigidity with extra substeps"
        );
        assert!(
            !soft.effective_runtime_config.step.joint_velocity_projection
                && hard.effective_runtime_config.step.joint_velocity_projection,
            "soft/hard profiles should expose the velocity-projection difference"
        );
        assert!(
            first_distance_joint_stiffness(&hard.world)
                > first_distance_joint_stiffness(&soft.world),
            "hard profile should author stronger distance joints"
        );
    }

    fn first_distance_joint_stiffness(world: &World) -> f32 {
        world
            .joints()
            .find_map(
                |handle| match world.joint(handle).expect("joint should resolve").desc() {
                    JointDesc::Distance(desc) => Some(desc.stiffness),
                    JointDesc::WorldAnchor(_) => None,
                },
            )
            .expect("lattice should include at least one distance joint")
    }

    #[derive(Debug)]
    struct StretchRatioBounds {
        min: f32,
        max: f32,
    }

    fn joint_stretch_ratio_bounds(
        reference: &picea::debug::DebugSnapshot,
        current: &picea::debug::DebugSnapshot,
    ) -> StretchRatioBounds {
        let reference_by_handle = reference
            .joints
            .iter()
            .map(|joint| (joint.handle, joint))
            .collect::<BTreeMap<_, _>>();
        let mut min_ratio = f32::INFINITY;
        let mut max_ratio: f32 = 1.0;
        for ratio in current.joints.iter().filter_map(|joint| {
            let reference_joint = reference_by_handle.get(&joint.handle)?;
            let reference_length = joint_anchor_distance(reference_joint)?;
            let current_length = joint_anchor_distance(joint)?;
            (reference_length > 1.0e-6).then_some(current_length / reference_length)
        }) {
            min_ratio = min_ratio.min(ratio);
            max_ratio = max_ratio.max(ratio);
        }
        StretchRatioBounds {
            min: if min_ratio.is_finite() {
                min_ratio
            } else {
                1.0
            },
            max: max_ratio,
        }
    }

    fn joint_anchor_distance(joint: &picea::debug::DebugJoint) -> Option<f32> {
        let first = joint.anchors.first()?;
        let second = joint.anchors.get(1)?;
        Some((*second - *first).length())
    }
}
