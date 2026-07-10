use std::collections::BTreeMap;

use picea::pipeline::ContactPositionCorrectionPolicy;
use picea::prelude::*;
use serde_json::{json, Value};

use crate::LabError;
use crate::LabResult;

use super::scene_params::{
    integer_param, number_param, parse_f32_param, parse_u16_param, parse_usize_param, select_param,
};
use super::{
    ScenarioParameterDescriptor, ScenarioParameterOption, ScenarioParameterValueType,
    ScenarioRuntimeConfig,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct NewtonCradleParams {
    ball_count: usize,
    radius: f32,
    string_length: f32,
    release_offset: f32,
    restitution: f32,
    friction: f32,
}

pub(super) fn default_newton_cradle_runtime_parts() -> (NewtonCradleParams, StepConfig, usize) {
    let params = NewtonCradleParams {
        ball_count: 5,
        radius: 0.18,
        string_length: 1.28,
        release_offset: 0.72,
        restitution: 1.0,
        friction: 0.0,
    };
    let mut step = StepConfig::default();
    step.dt = 1.0 / 960.0;
    step.restitution_velocity_threshold = 0.0;
    step.contact_position_correction = ContactPositionCorrectionPolicy::Conservative;
    (params, step, 16)
}

pub(super) fn newton_cradle_runtime_config(
    params: NewtonCradleParams,
    step: StepConfig,
    substeps_per_frame: usize,
) -> ScenarioRuntimeConfig {
    let mut scene_params = BTreeMap::new();
    scene_params.insert("ball_count".to_owned(), json!(params.ball_count));
    scene_params.insert("radius".to_owned(), json!(params.radius));
    scene_params.insert("string_length".to_owned(), json!(params.string_length));
    scene_params.insert("release_offset".to_owned(), json!(params.release_offset));
    scene_params.insert("restitution".to_owned(), json!(params.restitution));
    scene_params.insert("friction".to_owned(), json!(params.friction));
    scene_params.insert(
        "velocity_iterations".to_owned(),
        json!(step.velocity_iterations),
    );
    scene_params.insert(
        "position_iterations".to_owned(),
        json!(step.position_iterations),
    );
    scene_params.insert(
        "contact_position_correction".to_owned(),
        json!(step.contact_position_correction),
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

pub(super) fn resolve_newton_cradle_runtime_parts(
    overrides: &BTreeMap<String, Value>,
) -> LabResult<(NewtonCradleParams, StepConfig, usize)> {
    let (mut params, mut step, mut substeps_per_frame) = default_newton_cradle_runtime_parts();
    for (key, value) in overrides {
        match key.as_str() {
            "ball_count" => params.ball_count = parse_usize_param(key, value, 2, 12)?,
            "radius" => params.radius = parse_f32_param(key, value, 0.05, 0.4)?,
            "string_length" => params.string_length = parse_f32_param(key, value, 0.4, 3.0)?,
            "release_offset" => params.release_offset = parse_f32_param(key, value, 0.0, 2.8)?,
            "restitution" => params.restitution = parse_f32_param(key, value, 0.0, 1.0)?,
            "friction" => params.friction = parse_f32_param(key, value, 0.0, 1.0)?,
            "velocity_iterations" => step.velocity_iterations = parse_u16_param(key, value, 0, 80)?,
            "position_iterations" => {
                step.position_iterations = parse_u16_param(key, value, 0, 120)?
            }
            "contact_position_correction" => {
                step.contact_position_correction =
                    serde_json::from_value(value.clone()).map_err(|_| {
                        LabError::World(
                            "scene_params.contact_position_correction must be enabled, conservative, or disabled"
                                .to_owned(),
                        )
                    })?;
            }
            "joint_velocity_projection" => {
                step.joint_velocity_projection = value.as_bool().ok_or_else(|| {
                    LabError::World(
                        "scene_params.joint_velocity_projection must be a boolean".to_owned(),
                    )
                })?;
            }
            "substeps_per_frame" => {
                substeps_per_frame = parse_usize_param(key, value, 1, 16)?;
            }
            other => {
                return Err(LabError::World(format!(
                    "scene_params.{other}: unknown newton_cradle parameter"
                )));
            }
        }
    }
    validate_newton_cradle_params(params)?;
    step.dt = 1.0 / (60.0 * substeps_per_frame as f32);
    Ok((params, step, substeps_per_frame))
}

fn validate_newton_cradle_params(params: NewtonCradleParams) -> LabResult<()> {
    if params.release_offset >= params.string_length {
        return Err(LabError::World(
            "scene_params.release_offset must be smaller than string_length".to_owned(),
        ));
    }
    if params.string_length <= params.radius * 2.0 {
        return Err(LabError::World(
            "scene_params.string_length must be longer than the ball diameter".to_owned(),
        ));
    }
    Ok(())
}

pub(super) fn newton_cradle_parameter_schema() -> Vec<ScenarioParameterDescriptor> {
    vec![
        integer_param("ball_count", "Ball count", 5, 2.0, 12.0),
        number_param("radius", "Ball radius", 0.18, 0.05, 0.4, 0.01),
        number_param("string_length", "String length", 1.28, 0.4, 3.0, 0.01),
        number_param("release_offset", "Release offset", 0.72, 0.0, 2.8, 0.01),
        number_param("restitution", "Restitution", 1.0, 0.0, 1.0, 0.01),
        number_param("friction", "Friction", 0.0, 0.0, 1.0, 0.01),
        integer_param("velocity_iterations", "Velocity iterations", 10, 0.0, 80.0),
        integer_param("position_iterations", "Position iterations", 20, 0.0, 120.0),
        select_param(
            "contact_position_correction",
            "Contact position correction",
            json!(ContactPositionCorrectionPolicy::Conservative),
            vec![
                ScenarioParameterOption {
                    value: json!(ContactPositionCorrectionPolicy::Enabled),
                    label: "Enabled",
                },
                ScenarioParameterOption {
                    value: json!(ContactPositionCorrectionPolicy::Conservative),
                    label: "Conservative",
                },
                ScenarioParameterOption {
                    value: json!(ContactPositionCorrectionPolicy::Disabled),
                    label: "Disabled",
                },
            ],
        ),
        ScenarioParameterDescriptor {
            key: "joint_velocity_projection",
            label: "Joint velocity projection",
            value_type: ScenarioParameterValueType::Boolean,
            default: json!(true),
            min: None,
            max: None,
            step: None,
            options: Vec::new(),
        },
        integer_param("substeps_per_frame", "Substeps per frame", 16, 1.0, 16.0),
    ]
}

pub(super) fn newton_cradle_world(gravity: Vector, params: NewtonCradleParams) -> LabResult<World> {
    const ANCHOR_Y: f32 = -2.05;
    let spacing = params.radius * 2.02;
    let rest_y = ANCHOR_Y + params.string_length;

    let mut world = World::new(WorldDesc {
        gravity,
        enable_sleep: false,
    });
    let material = Material {
        friction: params.friction,
        restitution: params.restitution,
    };

    let mut anchors = Vec::with_capacity(params.ball_count);
    let mut balls = Vec::with_capacity(params.ball_count);
    let left = -0.5 * (params.ball_count as f32 - 1.0) * spacing;
    for index in 0..params.ball_count {
        let rest_x = left + index as f32 * spacing;
        let anchor = world
            .create_body(BodyDesc {
                body_type: BodyType::Static,
                pose: Pose::from_xy_angle(rest_x, ANCHOR_Y, 0.0),
                can_sleep: false,
                ..BodyDesc::default()
            })
            .map_err(|error| LabError::World(error.to_string()))?;
        anchors.push(anchor);

        let (x, y) = if index == 0 {
            let raised_y = ANCHOR_Y
                + (params.string_length * params.string_length
                    - params.release_offset * params.release_offset)
                    .sqrt();
            (rest_x - params.release_offset, raised_y)
        } else {
            (rest_x, rest_y)
        };
        let ball = world
            .create_body(BodyDesc {
                body_type: BodyType::Dynamic,
                pose: Pose::from_xy_angle(x, y, 0.0),
                linear_velocity: Vector::default(),
                can_sleep: false,
                ..BodyDesc::default()
            })
            .map_err(|error| LabError::World(error.to_string()))?;
        world
            .create_collider(
                ball,
                ColliderDesc {
                    shape: SharedShape::circle(params.radius),
                    material,
                    density: 1.4,
                    filter: CollisionFilter::default(),
                    ..ColliderDesc::default()
                },
            )
            .map_err(|error| LabError::World(error.to_string()))?;
        balls.push(ball);
    }

    for (anchor, ball) in anchors.into_iter().zip(balls) {
        world
            .create_joint(JointDesc::Distance(DistanceJointDesc {
                body_a: anchor,
                body_b: ball,
                rest_length: params.string_length,
                stiffness: 30.0,
                damping: 0.0,
                ..DistanceJointDesc::default()
            }))
            .map_err(|error| LabError::World(error.to_string()))?;
    }

    Ok(world)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::scene_dispatch::build_scenario;
    use crate::scenario::{ScenarioId, ScenarioOverrides};

    #[test]
    fn newton_cradle_builtin_exports_suspended_bouncy_ball_chain() {
        let builtin = build_scenario(ScenarioId::NewtonCradle, &ScenarioOverrides::default())
            .expect("newton cradle scenario should build");
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
        assert_eq!(dynamic_bodies, 5);

        let distance_joint_count = builtin
            .world
            .joints()
            .filter(|handle| {
                matches!(
                    builtin
                        .world
                        .joint(*handle)
                        .expect("joint should resolve")
                        .desc(),
                    JointDesc::Distance(_)
                )
            })
            .count();
        assert_eq!(distance_joint_count, 5);

        let restitutions = builtin
            .world
            .bodies()
            .flat_map(|body| {
                builtin
                    .world
                    .colliders_for_body(body)
                    .expect("body colliders should resolve")
            })
            .filter_map(|collider| builtin.world.collider(collider).ok())
            .map(|collider| collider.material().restitution)
            .collect::<Vec<_>>();
        assert_eq!(restitutions.len(), 5);
        assert!(
            restitutions.iter().all(|value| *value >= 0.99),
            "newton cradle balls should use near-perfect restitution for long kinetic retention"
        );
    }
}
