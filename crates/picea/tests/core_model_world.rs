use picea::prelude::{
    BodyAsset, BodyBundle, BodyDesc, BodyHandle, BodyPatch, BodyType, ColliderBundle, ColliderDesc,
    ColliderPatch, CollisionFilter, DistanceJointDesc, DistanceJointPatch, JointBundle, JointDesc,
    JointPatch, Material, Pose, SharedShape, SimulationPipeline, StepConfig, ValidationError,
    World, WorldAnchorJointDesc, WorldCommand, WorldCommandError, WorldCommandEvent,
    WorldCommandKind, WorldDesc, WorldError, WorldRecipe,
};
use picea::world::HandleError;

const MASS_EPSILON: f32 = 1e-4;

fn assert_near(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= MASS_EPSILON,
        "expected {actual} to be within {MASS_EPSILON} of {expected}"
    );
}

#[test]
fn body_handles_invalidate_after_destroy_and_recreate() {
    let mut world = World::new(WorldDesc::default());

    let body = world
        .create_body(BodyDesc::default())
        .expect("body should be created");
    let original_revision = world.revision();

    world.destroy_body(body).expect("body can be destroyed");
    assert!(matches!(
        world.destroy_body(body),
        Err(WorldError::Handle(HandleError::StaleBody { .. }))
    ));
    assert!(
        matches!(
            world.try_body(body),
            Err(WorldError::Handle(HandleError::StaleBody { .. }))
        ),
        "destroyed body should stop resolve through explicit read path"
    );

    let recreated = world
        .create_body(BodyDesc::default())
        .expect("body should be recreated");
    assert_ne!(body, recreated, "generation must change after slot reuse");
    assert!(world.revision() > original_revision);
}

#[test]
fn world_supports_multiple_colliders_per_body() {
    let mut world = World::new(WorldDesc::default());
    let body = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            ..BodyDesc::default()
        })
        .expect("body should be created");

    let first = world
        .create_collider(
            body,
            ColliderDesc {
                shape: SharedShape::circle(1.0),
                material: Material::default(),
                filter: CollisionFilter::default(),
                ..ColliderDesc::default()
            },
        )
        .expect("first collider can be created");
    let second = world
        .create_collider(
            body,
            ColliderDesc {
                shape: SharedShape::rect(2.0, 1.0),
                local_pose: Pose::from_xy_angle(2.0, 0.0, 0.0),
                material: Material::default(),
                filter: CollisionFilter::default(),
                ..ColliderDesc::default()
            },
        )
        .expect("second collider can be created");

    let attached: Vec<_> = world
        .colliders_for_body(body)
        .expect("body collider list should resolve")
        .collect();
    assert_eq!(attached, vec![first, second]);
}

#[test]
fn world_applies_body_patches_without_exposing_mutable_internals() {
    let mut world = World::new(WorldDesc::default());
    let body = world
        .create_body(BodyDesc::default())
        .expect("body should be created");

    world
        .apply_body_patch(
            body,
            BodyPatch {
                body_type: Some(BodyType::Kinematic),
                pose: Some(Pose::from_xy_angle(4.0, -3.0, 0.5)),
                linear_velocity: Some((1.0, 2.0).into()),
                angular_velocity: Some(3.0),
                wake: true,
                ..BodyPatch::default()
            },
        )
        .expect("body patch should apply");

    let view = world.try_body(body).expect("patched body exists");
    assert_eq!(view.body_type(), BodyType::Kinematic);
    assert_eq!(view.pose(), Pose::from_xy_angle(4.0, -3.0, 0.5));
    assert_eq!(view.linear_velocity(), (1.0, 2.0).into());
    assert_eq!(view.angular_velocity(), 3.0);
}

#[test]
fn world_accepts_distance_and_world_anchor_joint_descriptions() {
    let mut world = World::new(WorldDesc::default());
    let body_a = world
        .create_body(BodyDesc::default())
        .expect("body should be created");
    let body_b = world
        .create_body(BodyDesc::default())
        .expect("body should be created");

    let distance = world
        .create_joint(JointDesc::Distance(DistanceJointDesc {
            body_a,
            body_b,
            ..DistanceJointDesc::default()
        }))
        .expect("distance joint should be created");
    let world_anchor = world
        .create_joint(JointDesc::WorldAnchor(WorldAnchorJointDesc {
            body: body_a,
            ..WorldAnchorJointDesc::default()
        }))
        .expect("world-anchor joint should be created");

    assert_eq!(
        world.joints().collect::<Vec<_>>(),
        vec![distance, world_anchor]
    );
}

#[test]
fn destroying_a_body_cascades_attached_colliders_and_joints() {
    let mut world = World::new(WorldDesc::default());
    let body_a = world
        .create_body(BodyDesc::default())
        .expect("body should be created");
    let body_b = world
        .create_body(BodyDesc::default())
        .expect("body should be created");
    let collider = world
        .create_collider(
            body_a,
            ColliderDesc {
                shape: SharedShape::circle(1.0),
                ..ColliderDesc::default()
            },
        )
        .expect("collider should be created");
    let joint = world
        .create_joint(JointDesc::Distance(DistanceJointDesc {
            body_a,
            body_b,
            ..DistanceJointDesc::default()
        }))
        .expect("joint should be created");

    world.destroy_body(body_a).expect("body can be destroyed");

    assert!(matches!(
        world.try_collider(collider),
        Err(WorldError::Handle(HandleError::StaleCollider { .. }))
    ));
    assert!(matches!(
        world.try_joint(joint),
        Err(WorldError::Handle(HandleError::StaleJoint { .. }))
    ));
}

#[test]
fn mass_properties_are_density_derived_and_body_type_controls_inverses() {
    let mut world = World::new(WorldDesc::default());
    let dynamic = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            ..BodyDesc::default()
        })
        .expect("dynamic body should be created");
    let static_body = world
        .create_body(BodyDesc {
            body_type: BodyType::Static,
            ..BodyDesc::default()
        })
        .expect("static body should be created");
    let kinematic = world
        .create_body(BodyDesc {
            body_type: BodyType::Kinematic,
            ..BodyDesc::default()
        })
        .expect("kinematic body should be created");

    for body in [dynamic, static_body, kinematic] {
        world
            .create_collider(
                body,
                ColliderDesc {
                    shape: SharedShape::circle(1.0),
                    density: 2.0,
                    is_sensor: true,
                    ..ColliderDesc::default()
                },
            )
            .expect("collider should be created");
    }

    let dynamic_mass = world
        .body(dynamic)
        .expect("dynamic body should resolve")
        .mass_properties();
    assert_near(dynamic_mass.mass, 2.0 * std::f32::consts::PI);
    assert_near(dynamic_mass.inverse_mass, 1.0 / dynamic_mass.mass);
    assert_eq!(dynamic_mass.local_center_of_mass, (0.0, 0.0).into());
    assert_near(dynamic_mass.inertia, std::f32::consts::PI);
    assert_near(dynamic_mass.inverse_inertia, 1.0 / dynamic_mass.inertia);

    for body in [static_body, kinematic] {
        let mass = world
            .body(body)
            .expect("body should resolve")
            .mass_properties();
        assert_near(mass.mass, 2.0 * std::f32::consts::PI);
        assert_eq!(mass.inverse_mass, 0.0);
        assert_near(mass.inertia, std::f32::consts::PI);
        assert_eq!(mass.inverse_inertia, 0.0);
    }
}

#[test]
fn mass_properties_recompute_after_collider_and_body_mutations() {
    let mut world = World::new(WorldDesc::default());
    let body = world
        .create_body(BodyDesc::default())
        .expect("body should be created");

    assert_eq!(
        world
            .body(body)
            .expect("body should resolve")
            .mass_properties()
            .mass,
        0.0
    );

    let left = world
        .create_collider(
            body,
            ColliderDesc {
                shape: SharedShape::circle(1.0),
                local_pose: Pose::from_xy_angle(-1.0, 0.0, 0.0),
                density: 1.0,
                ..ColliderDesc::default()
            },
        )
        .expect("left collider should be created");
    let right = world
        .create_collider(
            body,
            ColliderDesc {
                shape: SharedShape::circle(1.0),
                local_pose: Pose::from_xy_angle(1.0, 0.0, 0.0),
                density: 1.0,
                ..ColliderDesc::default()
            },
        )
        .expect("right collider should be created");

    let balanced = world
        .body(body)
        .expect("body should resolve")
        .mass_properties();
    assert_near(balanced.mass, 2.0 * std::f32::consts::PI);
    assert_eq!(balanced.local_center_of_mass, (0.0, 0.0).into());
    assert_near(balanced.inertia, 3.0 * std::f32::consts::PI);

    world
        .apply_collider_patch(
            right,
            picea::prelude::ColliderPatch {
                density: Some(0.0),
                ..picea::prelude::ColliderPatch::default()
            },
        )
        .expect("density patch should apply");
    let after_density_patch = world
        .body(body)
        .expect("body should resolve")
        .mass_properties();
    assert_near(after_density_patch.mass, std::f32::consts::PI);
    assert_eq!(after_density_patch.local_center_of_mass, (-1.0, 0.0).into());
    assert_near(after_density_patch.inertia, 0.5 * std::f32::consts::PI);

    world
        .apply_body_patch(
            body,
            BodyPatch {
                body_type: Some(BodyType::Static),
                ..BodyPatch::default()
            },
        )
        .expect("body type patch should apply");
    let static_mass = world
        .body(body)
        .expect("body should resolve")
        .mass_properties();
    assert_near(static_mass.mass, std::f32::consts::PI);
    assert_eq!(static_mass.inverse_mass, 0.0);

    world
        .destroy_collider(left)
        .expect("collider removal should apply");
    let after_removal = world
        .body(body)
        .expect("body should resolve")
        .mass_properties();
    assert_eq!(after_removal.mass, 0.0);
    assert_eq!(after_removal.inertia, 0.0);
}

#[test]
fn world_commands_create_bundles_with_structured_handles_and_events() {
    let mut world = World::new(WorldDesc::default());
    let report = world
        .commands()
        .create_bodies([
            BodyBundle::static_body()
                .with_collider(ColliderBundle::new(SharedShape::rect(10.0, 1.0))),
            BodyBundle::dynamic().with_collider(ColliderBundle::new(SharedShape::circle(0.5))),
        ])
        .expect("valid batch should create all requested objects");

    assert_eq!(report.body_handles.len(), 2);
    assert_eq!(report.collider_handles.len(), 2);
    assert_eq!(world.bodies().collect::<Vec<_>>(), report.body_handles);
    assert_eq!(
        world
            .colliders_for_body(report.body_handles[0])
            .expect("static body should resolve")
            .collect::<Vec<_>>(),
        vec![report.collider_handles[0]]
    );
    assert!(matches!(
        report.events.as_slice(),
        [
            WorldCommandEvent::BodyCreated { .. },
            WorldCommandEvent::ColliderCreated { .. },
            WorldCommandEvent::BodyCreated { .. },
            WorldCommandEvent::ColliderCreated { .. },
        ]
    ));
}

#[test]
fn world_commands_create_placed_scene_assets() {
    let mut world = World::new(WorldDesc::default());
    let report = world
        .commands()
        .create_scene_bodies([
            BodyAsset::static_rect(10.0, 1.0).at(Pose::from_xy_angle(0.0, 2.0, 0.0)),
            BodyAsset::dynamic_circle(0.5).at(Pose::from_xy_angle(0.0, 0.0, 0.0)),
        ])
        .expect("valid scene body batch should create all requested objects");

    assert_eq!(report.body_handles.len(), 2);
    assert_eq!(report.collider_handles.len(), 2);
    assert_eq!(
        world
            .body(report.body_handles[0])
            .expect("floor body should resolve")
            .pose(),
        Pose::from_xy_angle(0.0, 2.0, 0.0)
    );
}

#[test]
fn world_command_error_struct_literal_shape_stays_compatible() {
    let error = WorldCommandError {
        command_index: 7,
        collider_index: None,
        kind: WorldCommandKind::CreateBody,
        error: WorldError::Handle(HandleError::MissingBody {
            handle: BodyHandle::INVALID,
        }),
    };

    assert_eq!(error.command_index, 7);
    assert_eq!(error.collider_index, None);
    assert_eq!(error.kind, WorldCommandKind::CreateBody);
}

#[test]
fn world_commands_do_not_mutate_when_batch_create_validation_fails() {
    let mut world = World::new(WorldDesc::default());
    let revision = world.revision();

    let error = world
        .commands()
        .create_bodies([
            BodyBundle::dynamic().with_collider(ColliderBundle::new(SharedShape::circle(0.5))),
            BodyBundle::dynamic().with_collider(ColliderBundle::new(SharedShape::circle(-1.0))),
        ])
        .expect_err("invalid collider should reject the whole batch");

    assert_eq!(error.command_index, 1);
    assert_eq!(error.collider_index, Some(0));
    assert_eq!(error.kind, WorldCommandKind::CreateCollider);
    assert!(matches!(
        error.error,
        WorldError::Validation(picea::world::ValidationError::ColliderDesc {
            field: "shape.radius",
        })
    ));
    assert_eq!(
        world.revision(),
        revision,
        "rejected batch must leave revision unchanged"
    );
    assert_eq!(
        world.bodies().count(),
        0,
        "rejected batch must not partially create earlier bodies"
    );
}

#[test]
fn world_commands_create_recipe_joints_atomically() {
    let mut world = World::new(WorldDesc::default());
    let report = world
        .commands()
        .create_recipe(
            [
                BodyBundle::static_body(),
                BodyBundle::dynamic().with_pose(Pose::from_xy_angle(2.0, 0.0, 0.0)),
            ],
            [JointBundle::distance(0, 1).with_rest_length(2.0)],
        )
        .expect("valid recipe command batch should create bodies and joints");

    assert_eq!(report.body_handles.len(), 2);
    assert_eq!(report.joint_handles.len(), 1);
    assert_eq!(world.joints().collect::<Vec<_>>(), report.joint_handles);
    assert!(matches!(
        report.events.as_slice(),
        [
            WorldCommandEvent::BodyCreated { .. },
            WorldCommandEvent::BodyCreated { .. },
            WorldCommandEvent::JointCreated { .. },
        ]
    ));
}

#[test]
fn world_commands_reject_invalid_recipe_joint_without_partial_bodies() {
    let mut world = World::new(WorldDesc::default());
    let revision = world.revision();

    let error = world
        .commands()
        .create_recipe([BodyBundle::dynamic()], [JointBundle::world_anchor(5)])
        .expect_err("invalid recipe joint body index should reject the whole batch");

    assert_eq!(error.command_index, 1);
    assert_eq!(error.kind, WorldCommandKind::CreateJoint);
    assert_eq!(world.revision(), revision);
    assert_eq!(
        world.bodies().count(),
        0,
        "rejected recipe batch must not partially create earlier bodies"
    );
}

#[test]
fn world_recipe_errors_include_nested_body_collider_and_joint_paths() {
    let collider_error = WorldRecipe::new(WorldDesc::default())
        .with_body(BodyBundle::dynamic().with_collider(ColliderBundle::circle(-1.0)))
        .instantiate_with_context()
        .expect_err("invalid nested collider should reject the recipe");

    assert_eq!(
        collider_error.path.as_str(),
        "recipe.bodies[0].colliders[0].desc.shape.radius"
    );
    assert_eq!(collider_error.error.command_index, 0);
    assert_eq!(collider_error.error.collider_index, Some(0));
    assert_eq!(collider_error.error.kind, WorldCommandKind::CreateCollider);
    assert!(matches!(
        collider_error.error.error,
        WorldError::Validation(ValidationError::ColliderDesc {
            field: "shape.radius"
        })
    ));

    let joint_error = WorldRecipe::new(WorldDesc::default())
        .with_body(BodyBundle::dynamic())
        .with_joint(JointBundle::distance(0, 2))
        .instantiate_with_context()
        .expect_err("invalid recipe joint endpoint should reject the recipe");

    assert_eq!(joint_error.path.as_str(), "recipe.joints[0].desc.body_b");
    assert_eq!(joint_error.error.command_index, 1);
    assert_eq!(joint_error.error.collider_index, None);
    assert_eq!(joint_error.error.kind, WorldCommandKind::CreateJoint);
    assert!(matches!(
        joint_error.error.error,
        WorldError::Handle(HandleError::MissingBody { .. })
    ));
}

#[test]
fn world_commands_patch_and_destroy_are_atomic_on_handle_errors() {
    let mut world = World::new(WorldDesc::default());
    let created = world
        .commands()
        .create_bodies([
            BodyBundle::dynamic(),
            BodyBundle::dynamic().with_pose(Pose::from_xy_angle(3.0, 0.0, 0.0)),
        ])
        .expect("setup batch should create bodies");
    let first = created.body_handles[0];
    let second = created.body_handles[1];
    let revision = world.revision();

    let error = world
        .commands()
        .apply([
            WorldCommand::PatchBody {
                body: first,
                patch: BodyPatch {
                    pose: Some(Pose::from_xy_angle(1.0, 2.0, 0.0)),
                    ..BodyPatch::default()
                },
            },
            WorldCommand::DestroyBody {
                body: BodyHandle::INVALID,
            },
        ])
        .expect_err("invalid destroy handle should reject the whole batch");

    assert_eq!(error.command_index, 1);
    assert_eq!(error.kind, WorldCommandKind::DestroyBody);
    assert_eq!(
        world.revision(),
        revision,
        "rejected patch/destroy batch must leave revision unchanged"
    );
    assert_eq!(
        world
            .body(first)
            .expect("first body should still resolve")
            .pose(),
        Pose::default(),
        "earlier patch in rejected batch must not leak into the real world"
    );
    assert!(world.body(second).is_ok());

    let report = world
        .commands()
        .apply([
            WorldCommand::PatchBody {
                body: first,
                patch: BodyPatch {
                    pose: Some(Pose::from_xy_angle(1.0, 2.0, 0.0)),
                    ..BodyPatch::default()
                },
            },
            WorldCommand::DestroyBody { body: second },
        ])
        .expect("valid patch/destroy batch should apply atomically");

    assert_eq!(
        report.events,
        vec![
            WorldCommandEvent::BodyPatched { body: first },
            WorldCommandEvent::BodyDestroyed { body: second },
        ]
    );
    assert_eq!(
        world
            .body(first)
            .expect("first body should still resolve")
            .pose(),
        Pose::from_xy_angle(1.0, 2.0, 0.0)
    );
    assert!(matches!(
        world.body(second),
        Err(WorldError::Handle(HandleError::StaleBody { .. }))
    ));
}

#[test]
fn world_commands_cover_collider_joint_paths_and_step_after_batch() {
    let mut world = World::new(WorldDesc::default());
    let created = world
        .commands()
        .create_bodies([
            BodyBundle::dynamic().with_collider(ColliderBundle::new(SharedShape::circle(0.5))),
            BodyBundle::dynamic().with_pose(Pose::from_xy_angle(2.0, 0.0, 0.0)),
        ])
        .expect("setup batch should create bodies");
    let body_a = created.body_handles[0];
    let body_b = created.body_handles[1];

    let created = world
        .commands()
        .apply([
            WorldCommand::CreateCollider {
                body: body_a,
                collider: ColliderBundle::new(SharedShape::rect(0.5, 0.25)),
            },
            WorldCommand::CreateJoint {
                desc: JointDesc::Distance(DistanceJointDesc {
                    body_a,
                    body_b,
                    rest_length: 2.0,
                    ..DistanceJointDesc::default()
                }),
            },
        ])
        .expect("valid collider/joint batch should apply atomically");
    let extra_collider = created.collider_handles[0];
    let joint = created.joint_handles[0];
    assert_eq!(
        created.events,
        vec![
            WorldCommandEvent::ColliderCreated {
                body: body_a,
                collider: extra_collider,
            },
            WorldCommandEvent::JointCreated { joint },
        ]
    );
    assert_eq!(
        world
            .colliders_for_body(body_a)
            .expect("body should resolve")
            .count(),
        2
    );

    let revision = world.revision();
    let error = world
        .commands()
        .apply([
            WorldCommand::PatchCollider {
                collider: extra_collider,
                patch: ColliderPatch {
                    is_sensor: Some(true),
                    ..ColliderPatch::default()
                },
            },
            WorldCommand::PatchJoint {
                joint,
                patch: JointPatch::Distance(DistanceJointPatch {
                    rest_length: Some(-1.0),
                    ..DistanceJointPatch::default()
                }),
            },
        ])
        .expect_err("invalid joint patch should reject the whole batch");
    assert_eq!(error.command_index, 1);
    assert_eq!(error.kind, WorldCommandKind::PatchJoint);
    assert_eq!(
        world.revision(),
        revision,
        "rejected collider/joint patch batch must leave revision unchanged"
    );
    assert!(
        !world
            .collider(extra_collider)
            .expect("collider should still resolve")
            .is_sensor(),
        "earlier collider patch must not leak from a rejected batch"
    );

    let patched = world
        .commands()
        .apply([
            WorldCommand::PatchCollider {
                collider: extra_collider,
                patch: ColliderPatch {
                    is_sensor: Some(true),
                    ..ColliderPatch::default()
                },
            },
            WorldCommand::PatchJoint {
                joint,
                patch: JointPatch::Distance(DistanceJointPatch {
                    rest_length: Some(3.0),
                    ..DistanceJointPatch::default()
                }),
            },
        ])
        .expect("valid collider/joint patch batch should apply");
    assert_eq!(
        patched.events,
        vec![
            WorldCommandEvent::ColliderPatched {
                collider: extra_collider
            },
            WorldCommandEvent::JointPatched { joint },
        ]
    );
    assert!(world
        .collider(extra_collider)
        .expect("collider should still resolve")
        .is_sensor());
    match world.joint(joint).expect("joint should resolve").desc() {
        JointDesc::Distance(desc) => assert_eq!(desc.rest_length, 3.0),
        JointDesc::WorldAnchor(_) => panic!("expected distance joint"),
    }

    let destroyed = world
        .commands()
        .apply([
            WorldCommand::DestroyCollider {
                collider: extra_collider,
            },
            WorldCommand::DestroyJoint { joint },
        ])
        .expect("valid collider/joint destroy batch should apply");
    assert_eq!(
        destroyed.events,
        vec![
            WorldCommandEvent::ColliderDestroyed {
                collider: extra_collider
            },
            WorldCommandEvent::JointDestroyed { joint },
        ]
    );
    assert!(matches!(
        world.collider(extra_collider),
        Err(WorldError::Handle(HandleError::StaleCollider { .. }))
    ));
    assert!(matches!(
        world.joint(joint),
        Err(WorldError::Handle(HandleError::StaleJoint { .. }))
    ));

    let mut pipeline = SimulationPipeline::new(StepConfig::default());
    let report = pipeline.step(&mut world);
    assert_eq!(report.stats.body_count, 2);
    assert_eq!(report.stats.collider_count, 1);
}
