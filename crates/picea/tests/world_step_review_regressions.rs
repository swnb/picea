use picea::prelude::{
    BodyDesc, BodyPatch, BodyType, ColliderDesc, ColliderPatch, DistanceJointDesc,
    DistanceJointPatch, JointDesc, JointPatch, Pose, SharedShape, SimulationPipeline, StepConfig,
    World, WorldAnchorJointDesc, WorldAnchorJointPatch, WorldDesc, WorldError, WorldEvent,
};
use picea::world::{HandleError, TopologyError, ValidationError};

#[test]
fn body_inputs_must_be_finite_before_world_state_mutates() {
    let mut world = World::new(WorldDesc::default());
    let original_revision = world.revision();

    let create_error = world
        .create_body(BodyDesc {
            pose: Pose::from_xy_angle(f32::NAN, 0.0, 0.0),
            ..BodyDesc::default()
        })
        .expect_err("non-finite creation inputs must be rejected");
    assert!(matches!(
        create_error,
        WorldError::Validation(ValidationError::BodyDesc {
            field: "pose.translation.x",
        })
    ));
    assert_eq!(
        world.revision(),
        original_revision,
        "rejected descriptors must not mutate authoritative state"
    );

    let body = world
        .create_body(BodyDesc::default())
        .expect("finite body should be created");
    let patch_revision = world.revision();

    let patch_error = world
        .apply_body_patch(
            body,
            BodyPatch {
                gravity_scale: Some(f32::INFINITY),
                ..BodyPatch::default()
            },
        )
        .expect_err("non-finite patch inputs must be rejected");
    assert!(matches!(
        patch_error,
        WorldError::Validation(ValidationError::BodyPatch {
            field: "gravity_scale",
        })
    ));
    assert_eq!(
        world.revision(),
        patch_revision,
        "rejected patches must not bump world revision"
    );
}

#[test]
fn collider_and_joint_inputs_are_validated_before_revision_bump() {
    let mut world = World::new(WorldDesc::default());
    let body_a = world
        .create_body(BodyDesc::default())
        .expect("body should be created");
    let body_b = world
        .create_body(BodyDesc::default())
        .expect("body should be created");

    let create_revision = world.revision();
    let create_collider_error = world
        .create_collider(
            body_a,
            ColliderDesc {
                density: f32::INFINITY,
                ..ColliderDesc::default()
            },
        )
        .expect_err("non-finite collider inputs must be rejected");
    assert!(matches!(
        create_collider_error,
        WorldError::Validation(ValidationError::ColliderDesc { field: "density" })
    ));
    assert_eq!(
        world.revision(),
        create_revision,
        "rejected collider descriptors must not bump the world revision"
    );

    let collider = world
        .create_collider(body_a, ColliderDesc::default())
        .expect("finite collider should be created");
    let collider_patch_revision = world.revision();
    let collider_patch_error = world
        .apply_collider_patch(
            collider,
            ColliderPatch {
                density: Some(-1.0),
                ..ColliderPatch::default()
            },
        )
        .expect_err("negative collider patch density must be rejected");
    assert!(matches!(
        collider_patch_error,
        WorldError::Validation(ValidationError::ColliderPatch { field: "density" })
    ));
    assert_eq!(
        world.revision(),
        collider_patch_revision,
        "rejected collider patches must not bump the world revision"
    );

    let zero_length_segment_revision = world.revision();
    let zero_length_segment_error = world
        .create_collider(
            body_a,
            ColliderDesc {
                shape: SharedShape::segment((1.0, 1.0), (1.0, 1.0)),
                ..ColliderDesc::default()
            },
        )
        .expect_err("zero-length segments are invalid boundary geometry");
    assert!(matches!(
        zero_length_segment_error,
        WorldError::Validation(ValidationError::ColliderDesc {
            field: "shape.segment",
        })
    ));
    assert_eq!(
        world.revision(),
        zero_length_segment_revision,
        "rejected segment geometry must not bump the world revision"
    );

    let same_body_revision = world.revision();
    let same_body_error = world
        .create_joint(JointDesc::Distance(DistanceJointDesc {
            body_a,
            body_b: body_a,
            ..DistanceJointDesc::default()
        }))
        .expect_err("same-body distance joints must be rejected");
    assert!(matches!(
        same_body_error,
        WorldError::Topology(TopologyError::SameBodyJointPair { .. })
    ));
    assert_eq!(
        world.revision(),
        same_body_revision,
        "topology rejection must not bump the world revision"
    );

    let joint = world
        .create_joint(JointDesc::Distance(DistanceJointDesc {
            body_a,
            body_b,
            ..DistanceJointDesc::default()
        }))
        .expect("finite distance joint should be created");
    let joint_patch_revision = world.revision();
    let joint_patch_error = world
        .apply_joint_patch(
            joint,
            JointPatch::Distance(DistanceJointPatch {
                rest_length: Some(f32::NAN),
                ..DistanceJointPatch::default()
            }),
        )
        .expect_err("non-finite joint patches must be rejected");
    assert!(matches!(
        joint_patch_error,
        WorldError::Validation(ValidationError::JointPatch {
            field: "rest_length",
        })
    ));
    assert_eq!(
        world.revision(),
        joint_patch_revision,
        "rejected joint patches must not bump the world revision"
    );

    let world_anchor_joint = world
        .create_joint(JointDesc::WorldAnchor(WorldAnchorJointDesc {
            body: body_a,
            ..WorldAnchorJointDesc::default()
        }))
        .expect("world-anchor joint should be created");
    let world_anchor_patch_revision = world.revision();
    let world_anchor_patch_error = world
        .apply_joint_patch(
            world_anchor_joint,
            JointPatch::WorldAnchor(WorldAnchorJointPatch {
                world_anchor: Some((f32::INFINITY, 0.0).into()),
                ..WorldAnchorJointPatch::default()
            }),
        )
        .expect_err("non-finite world-anchor patches must be rejected");
    assert!(matches!(
        world_anchor_patch_error,
        WorldError::Validation(ValidationError::JointPatch {
            field: "world_anchor.x",
        })
    ));
    assert_eq!(
        world.revision(),
        world_anchor_patch_revision,
        "rejected world-anchor patches must not bump the world revision"
    );
}

#[test]
fn derived_mass_properties_are_validated_before_world_state_mutates() {
    let mut world = World::new(WorldDesc::default());
    let body = world
        .create_body(BodyDesc::default())
        .expect("body should be created");

    let shape_overflow_revision = world.revision();
    let shape_overflow_error = world
        .create_collider(
            body,
            ColliderDesc {
                shape: SharedShape::circle(1.0e20),
                density: 1.0,
                ..ColliderDesc::default()
            },
        )
        .expect_err("finite inputs with overflowing mass formulas must be rejected");
    assert!(matches!(
        shape_overflow_error,
        WorldError::Validation(ValidationError::ColliderDesc {
            field: "mass_properties",
        })
    ));
    assert_eq!(
        world.revision(),
        shape_overflow_revision,
        "shape mass overflow must not bump the world revision"
    );
    assert_eq!(
        world
            .colliders_for_body(body)
            .expect("body should still resolve")
            .count(),
        0,
        "rejected collider descriptors must not attach collider handles"
    );

    world
        .create_collider(
            body,
            ColliderDesc {
                shape: SharedShape::circle(1.0),
                density: 1.0,
                ..ColliderDesc::default()
            },
        )
        .expect("finite base collider should be created");
    let finite_mass = world
        .body(body)
        .expect("body should resolve")
        .mass_properties();
    let aggregate_overflow_revision = world.revision();
    let aggregate_overflow_error = world
        .create_collider(
            body,
            ColliderDesc {
                shape: SharedShape::circle(1.0),
                local_pose: Pose::from_xy_angle(1.0e20, 0.0, 0.0),
                density: 1.0,
                ..ColliderDesc::default()
            },
        )
        .expect_err("finite collider offset can overflow aggregate inertia");
    assert!(matches!(
        aggregate_overflow_error,
        WorldError::Validation(ValidationError::ColliderDesc {
            field: "mass_properties",
        })
    ));
    assert_eq!(
        world.revision(),
        aggregate_overflow_revision,
        "aggregate mass overflow must not bump the world revision"
    );
    assert_eq!(
        world
            .colliders_for_body(body)
            .expect("body should still resolve")
            .count(),
        1,
        "rejected aggregate mass must not allocate or attach another collider"
    );
    assert_eq!(
        world
            .body(body)
            .expect("body should resolve")
            .mass_properties(),
        finite_mass,
        "rejected aggregate mass must preserve authoritative body mass facts"
    );

    world
        .create_collider(
            body,
            ColliderDesc {
                shape: SharedShape::circle(1.0),
                local_pose: Pose::from_xy_angle(1.0, 0.0, 0.0),
                density: 1.0,
                ..ColliderDesc::default()
            },
        )
        .expect("second finite collider should be created");
    let finite_mass_after_second = world
        .body(body)
        .expect("body should resolve")
        .mass_properties();
    let collider = world
        .colliders_for_body(body)
        .expect("body should still resolve")
        .next()
        .expect("base collider should exist");
    let patch_revision = world.revision();
    let patch_error = world
        .apply_collider_patch(
            collider,
            ColliderPatch {
                local_pose: Some(Pose::from_xy_angle(1.0e20, 0.0, 0.0)),
                ..ColliderPatch::default()
            },
        )
        .expect_err("patches must validate prospective aggregate mass before mutation");
    assert!(matches!(
        patch_error,
        WorldError::Validation(ValidationError::ColliderPatch {
            field: "mass_properties",
        })
    ));
    assert_eq!(
        world.revision(),
        patch_revision,
        "rejected mass-property patches must not bump the world revision"
    );
    assert_eq!(
        world
            .collider(collider)
            .expect("collider should resolve")
            .local_pose(),
        Pose::default(),
        "rejected mass-property patches must not mutate collider slots"
    );
    assert_eq!(
        world
            .body(body)
            .expect("body should resolve")
            .mass_properties(),
        finite_mass_after_second,
        "rejected patches must preserve authoritative body mass facts"
    );
}

#[test]
fn zero_area_regular_polygons_are_rejected_before_world_state_mutates() {
    let mut world = World::new(WorldDesc::default());
    let body = world
        .create_body(BodyDesc::default())
        .expect("body should be created");

    let revision = world.revision();
    let error = world
        .create_collider(
            body,
            ColliderDesc {
                shape: SharedShape::regular_polygon(6, 0.0),
                density: 0.0,
                ..ColliderDesc::default()
            },
        )
        .expect_err("zero-area regular polygons must be rejected even at zero density");
    assert!(matches!(
        error,
        WorldError::Validation(ValidationError::ColliderDesc {
            field: "shape.radius",
        })
    ));
    assert_eq!(
        world.revision(),
        revision,
        "rejected zero-area regular polygons must not bump revision"
    );
    assert_eq!(
        world
            .colliders_for_body(body)
            .expect("body should still resolve")
            .count(),
        0,
        "rejected zero-area regular polygons must not attach collider handles"
    );
}

#[test]
fn stale_reads_are_explicit_instead_of_collapsing_into_absence() {
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
                shape: SharedShape::circle(0.5),
                ..ColliderDesc::default()
            },
        )
        .expect("collider should be created");
    let joint = world
        .create_joint(JointDesc::WorldAnchor(WorldAnchorJointDesc {
            body: body_a,
            ..WorldAnchorJointDesc::default()
        }))
        .expect("joint should be created");

    world
        .destroy_body(body_a)
        .expect("body should be destroyable");

    assert!(matches!(
        world.try_body(body_a),
        Err(WorldError::Handle(HandleError::StaleBody { .. }))
    ));
    assert!(matches!(
        world.try_collider(collider),
        Err(WorldError::Handle(HandleError::StaleCollider { .. }))
    ));
    assert!(matches!(
        world.try_joint(joint),
        Err(WorldError::Handle(HandleError::StaleJoint { .. }))
    ));
    assert!(matches!(
        world.try_colliders_for_body(body_a),
        Err(WorldError::Handle(HandleError::StaleBody { .. }))
    ));
    assert!(world
        .try_body(body_b)
        .expect("live handles should still resolve")
        .handle()
        .is_valid());
}

#[test]
fn step_emits_numeric_warnings_without_committing_non_finite_body_state() {
    let mut world = World::new(WorldDesc {
        gravity: (f32::NAN, 0.0).into(),
        enable_sleep: false,
    });
    let body = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(0.0, 0.0, 0.0),
            ..BodyDesc::default()
        })
        .expect("body should be created");

    let mut pipeline = SimulationPipeline::new(StepConfig::default());
    let report = pipeline.step(&mut world);

    assert!(
        report.stats.numeric_warnings > 0,
        "non-finite intermediates should be surfaced as warnings"
    );
    assert!(report.events.iter().any(|event| {
        matches!(event, WorldEvent::NumericsWarning(warning) if warning.phase == "integrate")
    }));

    let body = world
        .try_body(body)
        .expect("body should remain addressable");
    assert!(
        body.pose().translation().x().is_finite()
            && body.pose().translation().y().is_finite()
            && body.linear_velocity().x().is_finite()
            && body.linear_velocity().y().is_finite(),
        "explicit numerics handling should prevent NaN state from leaking into retained world facts"
    );
}

#[test]
fn simulation_step_emits_lifecycle_and_contact_events_with_nonzero_stats() {
    let mut world = World::new(WorldDesc {
        gravity: (0.0, 0.0).into(),
        enable_sleep: true,
    });
    let ground = world
        .create_body(BodyDesc {
            body_type: BodyType::Static,
            pose: Pose::from_xy_angle(0.0, 0.0, 0.0),
            ..BodyDesc::default()
        })
        .expect("ground should be created");
    let sleeper = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(4.0, 4.0, 0.0),
            ..BodyDesc::default()
        })
        .expect("sleeper should be created");
    let overlapping = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(0.0, 0.0, 0.0),
            ..BodyDesc::default()
        })
        .expect("dynamic body should be created");

    let ground_collider = world
        .create_collider(
            ground,
            ColliderDesc {
                shape: SharedShape::rect(4.0, 1.0),
                ..ColliderDesc::default()
            },
        )
        .expect("ground collider should be created");
    let overlapping_collider = world
        .create_collider(
            overlapping,
            ColliderDesc {
                shape: SharedShape::circle(0.75),
                ..ColliderDesc::default()
            },
        )
        .expect("dynamic collider should be created");
    let anchor_joint = world
        .create_joint(JointDesc::WorldAnchor(WorldAnchorJointDesc {
            body: overlapping,
            world_anchor: (2.0, 0.0).into(),
            stiffness: 1.0,
            ..WorldAnchorJointDesc::default()
        }))
        .expect("joint should be created");

    let mut pipeline = SimulationPipeline::new(StepConfig::default());
    let first_report = pipeline.step(&mut world);

    assert!(
        first_report.stats.contact_count > 0,
        "overlapping colliders must contribute contact stats"
    );
    assert!(
        first_report.stats.manifold_count > 0,
        "active contacts must also surface manifolds"
    );
    assert!(first_report
        .events
        .iter()
        .any(|event| matches!(event, WorldEvent::BodyCreated { body } if *body == ground)));
    assert!(first_report.events.iter().any(
        |event| matches!(event, WorldEvent::JointCreated { joint } if *joint == anchor_joint)
    ));
    assert!(first_report.events.iter().any(|event| {
        matches!(
            event,
            WorldEvent::ContactStarted(contact)
                if (contact.collider_a == ground_collider
                    && contact.collider_b == overlapping_collider)
                    || (contact.collider_a == overlapping_collider
                        && contact.collider_b == ground_collider)
        )
    }));

    let sleeper_eventually_slept = (0..40).any(|_| {
        pipeline.step(&mut world).events.iter().any(|event| {
            matches!(
                event,
                WorldEvent::SleepChanged(sleep)
                    if sleep.body == sleeper && sleep.is_sleeping
            )
        })
    });
    assert!(
        sleeper_eventually_slept,
        "sleep events should still emit after the stability window"
    );
}

#[test]
fn circle_contacts_keep_normals_toward_ordered_body_after_collider_slot_reuse() {
    let mut world = World::new(WorldDesc {
        gravity: (0.0, 0.0).into(),
        enable_sleep: false,
    });
    let temp_body = world
        .create_body(BodyDesc {
            body_type: BodyType::Static,
            ..BodyDesc::default()
        })
        .expect("temporary body should be created");
    let recycled_slot_collider = world
        .create_collider(
            temp_body,
            ColliderDesc {
                shape: SharedShape::circle(0.5),
                ..ColliderDesc::default()
            },
        )
        .expect("temporary collider should be created");
    world
        .destroy_collider(recycled_slot_collider)
        .expect("collider slot should be reusable");

    let left_body = world
        .create_body(BodyDesc {
            body_type: BodyType::Static,
            pose: Pose::from_xy_angle(0.0, 0.0, 0.0),
            ..BodyDesc::default()
        })
        .expect("left body should be created");
    let left_collider = world
        .create_collider(
            left_body,
            ColliderDesc {
                shape: SharedShape::circle(1.0),
                ..ColliderDesc::default()
            },
        )
        .expect("left collider should reuse the old slot generation");
    let right_body = world
        .create_body(BodyDesc {
            body_type: BodyType::Static,
            pose: Pose::from_xy_angle(1.5, 0.0, 0.0),
            ..BodyDesc::default()
        })
        .expect("right body should be created");
    let right_collider = world
        .create_collider(
            right_body,
            ColliderDesc {
                shape: SharedShape::circle(1.0),
                ..ColliderDesc::default()
            },
        )
        .expect("right collider should be created after the recycled slot");

    assert!(
        right_collider < left_collider,
        "generation bits should make handle order diverge from live snapshot order"
    );

    let mut pipeline = SimulationPipeline::new(StepConfig::default());
    let report = pipeline.step(&mut world);
    let contact = report
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::ContactStarted(contact)
                if contact.collider_a == right_collider && contact.collider_b == left_collider =>
            {
                Some(contact)
            }
            _ => None,
        })
        .expect("overlapping circles should emit an ordered contact");

    assert_eq!(contact.body_a, right_body);
    assert_eq!(contact.body_b, left_body);
    assert!(
        contact.normal.x() > 0.0 && contact.normal.y().abs() <= f32::EPSILON,
        "normal should point toward ordered body_a even when handle order diverges; got {:?}",
        contact.normal
    );
}

#[test]
fn world_anchor_joints_affect_body_motion_during_step() {
    let mut world = World::new(WorldDesc {
        gravity: (0.0, 0.0).into(),
        enable_sleep: false,
    });
    let body = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(0.0, 0.0, 0.0),
            ..BodyDesc::default()
        })
        .expect("body should be created");
    world
        .create_joint(JointDesc::WorldAnchor(WorldAnchorJointDesc {
            body,
            world_anchor: (3.0, 0.0).into(),
            stiffness: 1.0,
            ..WorldAnchorJointDesc::default()
        }))
        .expect("joint should be created");

    let before = world
        .try_body(body)
        .expect("live body must resolve")
        .pose()
        .translation()
        .x();

    let mut pipeline = SimulationPipeline::new(StepConfig::default());
    let _ = pipeline.step(&mut world);

    let after = world
        .try_body(body)
        .expect("live body must resolve")
        .pose()
        .translation()
        .x();
    assert!(
        after > before,
        "world-anchor joint should move the body toward the anchor"
    );
}
