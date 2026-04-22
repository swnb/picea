use picea::prelude::{
    BodyDesc, BodyPatch, BodyType, ColliderDesc, JointDesc, Pose, SharedShape, SimulationPipeline,
    StepConfig, World, WorldAnchorJointDesc, WorldDesc, WorldError, WorldEvent,
};

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
        WorldError::InvalidBodyDesc {
            field: "pose.translation.x",
        }
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
        WorldError::InvalidBodyPatch {
            field: "gravity_scale",
        }
    ));
    assert_eq!(
        world.revision(),
        patch_revision,
        "rejected patches must not bump world revision"
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
        Err(WorldError::StaleBodyHandle { .. })
    ));
    assert!(matches!(
        world.try_collider(collider),
        Err(WorldError::StaleColliderHandle { .. })
    ));
    assert!(matches!(
        world.try_joint(joint),
        Err(WorldError::StaleJointHandle { .. })
    ));
    assert!(matches!(
        world.try_colliders_for_body(body_a),
        Err(WorldError::StaleBodyHandle { .. })
    ));
    assert!(world
        .try_body(body_b)
        .expect("live handles should still resolve")
        .handle()
        .is_valid());
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
    let report = pipeline.step(&mut world);

    assert!(
        report.stats.contact_count > 0,
        "overlapping colliders must contribute contact stats"
    );
    assert!(
        report.stats.manifold_count > 0,
        "active contacts must also surface manifolds"
    );
    assert!(report
        .events
        .iter()
        .any(|event| matches!(event, WorldEvent::BodyCreated { body } if *body == ground)));
    assert!(report.events.iter().any(
        |event| matches!(event, WorldEvent::JointCreated { joint } if *joint == anchor_joint)
    ));
    assert!(report.events.iter().any(|event| {
        matches!(
            event,
            WorldEvent::ContactStarted(contact)
                if (contact.collider_a == ground_collider
                    && contact.collider_b == overlapping_collider)
                    || (contact.collider_a == overlapping_collider
                        && contact.collider_b == ground_collider)
        )
    }));
    assert!(report.events.iter().any(|event| {
        matches!(
            event,
            WorldEvent::SleepChanged(sleep)
                if sleep.body == sleeper && sleep.is_sleeping
        )
    }));
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
