use picea::prelude::*;

use crate::LabError;
use crate::LabResult;

use super::fixture::{
    instantiate_scene_fixture, instantiate_scene_fixture_with_provenance, CompoundProvenance,
};
use super::scene_lattice::{lattice_grid_fixture, resolve_lattice_runtime_parts};
use super::scene_newton_cradle::{newton_cradle_world, resolve_newton_cradle_runtime_parts};
use super::scene_rect_stack::{rect_stack_fixture, resolve_rect_stack_runtime_parts};
use super::scene_revolute::revolute_pendulum_fixture;
use super::scene_static::{
    compound_provenance_fixture, concave_decomposition_fixture, falling_box_contact_fixture,
    stack_stability_tower_fixture,
};
use super::{
    effective_runtime_config_for_scenario, ScenarioId, ScenarioOverrides, ScenarioRuntimeConfig,
};

pub(crate) struct BuiltScenario {
    pub(crate) world: World,
    pub(crate) compound_provenance: Vec<CompoundProvenance>,
    pub(crate) effective_runtime_config: ScenarioRuntimeConfig,
}

pub(crate) fn build_scenario(
    id: ScenarioId,
    overrides: &ScenarioOverrides,
) -> LabResult<BuiltScenario> {
    let effective_runtime_config = effective_runtime_config_for_scenario(id, overrides)?;
    let gravity = overrides
        .gravity
        .map(|[x, y]| Vector::new(x, y))
        .unwrap_or_else(|| Vector::new(0.0, 9.8));
    let mut world: World;

    match id {
        ScenarioId::FallingBoxContact => {
            world = instantiate_scene_fixture(&falling_box_contact_fixture([
                gravity.x(),
                gravity.y(),
            ]))?;
        }
        ScenarioId::Stack4 => {
            let (params, _, _) = resolve_rect_stack_runtime_parts(id, &overrides.scene_params)?;
            world =
                instantiate_scene_fixture(&rect_stack_fixture([gravity.x(), gravity.y()], params))?;
        }
        ScenarioId::StackStabilityTower => {
            world = instantiate_scene_fixture(&stack_stability_tower_fixture([
                gravity.x(),
                gravity.y(),
            ]))?;
        }
        ScenarioId::MatrixStack => {
            let (params, _, _) = resolve_rect_stack_runtime_parts(id, &overrides.scene_params)?;
            world =
                instantiate_scene_fixture(&rect_stack_fixture([gravity.x(), gravity.y()], params))?;
        }
        ScenarioId::MatrixStackAligned => {
            let (params, _, _) = resolve_rect_stack_runtime_parts(id, &overrides.scene_params)?;
            world =
                instantiate_scene_fixture(&rect_stack_fixture([gravity.x(), gravity.y()], params))?;
        }
        ScenarioId::NewtonCradle => {
            let (params, _, _) = resolve_newton_cradle_runtime_parts(&overrides.scene_params)?;
            world = newton_cradle_world(gravity, params)?;
        }
        ScenarioId::RevolutePendulum => {
            world = instantiate_scene_fixture(&revolute_pendulum_fixture())?;
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
        ScenarioId::LatticeGrid => {
            let (params, _, _) = resolve_lattice_runtime_parts(&overrides.scene_params)?;
            world = instantiate_scene_fixture(&lattice_grid_fixture(
                [gravity.x(), gravity.y()],
                params,
            ))?;
        }
        ScenarioId::BroadphaseSparse => {
            world = World::new(WorldDesc {
                gravity: Vector::default(),
                enable_sleep: false,
            });
            add_box(&mut world, BodyType::Static, 0.0, 0.0, 1.0, 1.0)?;
            add_box(&mut world, BodyType::Static, 0.75, 0.0, 1.0, 1.0)?;
            add_box(&mut world, BodyType::Static, 5.0, 0.0, 1.0, 1.0)?;
            add_box(&mut world, BodyType::Static, 10.0, 0.0, 1.0, 1.0)?;
            add_box(&mut world, BodyType::Static, 15.0, 0.0, 1.0, 1.0)?;
        }
        ScenarioId::SatPolygon => {
            world = World::new(WorldDesc {
                gravity: Vector::default(),
                enable_sleep: false,
            });
            add_box(&mut world, BodyType::Static, 0.0, 0.0, 2.0, 2.0)?;
            let body = world
                .create_body(BodyDesc {
                    body_type: BodyType::Static,
                    pose: Pose::from_xy_angle(1.5, 0.0, 0.0),
                    can_sleep: false,
                    ..BodyDesc::default()
                })
                .map_err(|error| LabError::World(error.to_string()))?;
            world
                .create_collider(
                    body,
                    ColliderDesc {
                        shape: SharedShape::convex_polygon(vec![
                            Point::new(-1.0, -1.0),
                            Point::new(1.0, -1.0),
                            Point::new(1.0, 1.0),
                            Point::new(-1.0, 1.0),
                        ]),
                        ..ColliderDesc::default()
                    },
                )
                .map_err(|error| LabError::World(error.to_string()))?;
        }
        ScenarioId::CompoundProvenance => {
            let instantiated =
                instantiate_scene_fixture_with_provenance(&compound_provenance_fixture())?;
            return Ok(BuiltScenario {
                world: instantiated.world,
                compound_provenance: instantiated.compound_provenance,
                effective_runtime_config,
            });
        }
        ScenarioId::ConcaveDecomposition => {
            let instantiated =
                instantiate_scene_fixture_with_provenance(&concave_decomposition_fixture())?;
            return Ok(BuiltScenario {
                world: instantiated.world,
                compound_provenance: instantiated.compound_provenance,
                effective_runtime_config,
            });
        }
        ScenarioId::CcdFastCircleWall => {
            world = World::new(WorldDesc {
                gravity: Vector::default(),
                enable_sleep: false,
            });
            add_box(&mut world, BodyType::Static, 0.0, 0.0, 0.1, 10.0)?;
            add_circle(
                &mut world,
                BodyType::Dynamic,
                -1.0,
                0.0,
                0.05,
                Vector::new(200.0, 0.0),
            )?;
        }
        ScenarioId::CcdFastConvexWalls => {
            world = World::new(WorldDesc {
                gravity: Vector::default(),
                enable_sleep: false,
            });
            add_box(&mut world, BodyType::Static, 0.0, 0.0, 0.1, 10.0)?;
            add_box(&mut world, BodyType::Static, 0.8, 0.0, 0.1, 10.0)?;
            add_box_with_velocity(
                &mut world,
                BodyType::Dynamic,
                -1.0,
                0.0,
                0.1,
                0.1,
                Vector::new(200.0, 0.0),
            )?;
        }
        ScenarioId::CcdDynamicConvexPair => {
            world = World::new(WorldDesc {
                gravity: Vector::default(),
                enable_sleep: false,
            });
            add_box_with_velocity(
                &mut world,
                BodyType::Dynamic,
                -1.0,
                0.0,
                0.1,
                0.1,
                Vector::new(200.0, 0.0),
            )?;
            add_box_with_velocity(
                &mut world,
                BodyType::Dynamic,
                1.0,
                0.0,
                0.1,
                0.1,
                Vector::new(-200.0, 0.0),
            )?;
        }
        ScenarioId::CcdDynamicCompoundWall => {
            world = World::new(WorldDesc {
                gravity: Vector::default(),
                enable_sleep: false,
            });
            add_box(&mut world, BodyType::Static, 0.0, 0.25, 0.1, 0.12)?;
            add_compound_box_pair_with_velocity(
                &mut world,
                -1.0,
                0.0,
                0.1,
                0.1,
                Vector::new(200.0, 0.0),
            )?;
        }
    }

    Ok(BuiltScenario {
        world,
        compound_provenance: Vec::new(),
        effective_runtime_config,
    })
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

fn add_box_with_velocity(
    world: &mut World,
    body_type: BodyType,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    linear_velocity: Vector,
) -> LabResult<BodyHandle> {
    let body = world
        .create_body(BodyDesc {
            body_type,
            pose: Pose::from_xy_angle(x, y, 0.0),
            linear_velocity,
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

fn add_circle(
    world: &mut World,
    body_type: BodyType,
    x: f32,
    y: f32,
    radius: f32,
    linear_velocity: Vector,
) -> LabResult<BodyHandle> {
    let body = world
        .create_body(BodyDesc {
            body_type,
            pose: Pose::from_xy_angle(x, y, 0.0),
            linear_velocity,
            can_sleep: false,
            ..BodyDesc::default()
        })
        .map_err(|error| LabError::World(error.to_string()))?;
    world
        .create_collider(
            body,
            ColliderDesc {
                shape: SharedShape::circle(radius),
                ..ColliderDesc::default()
            },
        )
        .map_err(|error| LabError::World(error.to_string()))?;
    Ok(body)
}

fn add_compound_box_pair_with_velocity(
    world: &mut World,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    linear_velocity: Vector,
) -> LabResult<BodyHandle> {
    let body = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(x, y, 0.0),
            linear_velocity,
            can_sleep: false,
            ..BodyDesc::default()
        })
        .map_err(|error| LabError::World(error.to_string()))?;

    for local_y in [0.25, -0.25] {
        world
            .create_collider(
                body,
                ColliderDesc {
                    shape: SharedShape::rect(width, height),
                    local_pose: Pose::from_xy_angle(0.0, local_y, 0.0),
                    ..ColliderDesc::default()
                },
            )
            .map_err(|error| LabError::World(error.to_string()))?;
    }

    Ok(body)
}
