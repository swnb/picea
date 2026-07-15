use picea::prelude::{
    BodyDesc, BodyHandle, BodyPatch, BodyType, ColliderDesc, ColliderPatch, DistanceJointDesc,
    DistanceJointPatch, JointDesc, JointPatch, Pose, RevoluteJointDesc, RevoluteJointPatch,
    SharedShape, SimulationPipeline, SleepTransitionReason, StepConfig, Vector, World,
    WorldAnchorJointDesc, WorldAnchorJointPatch, WorldCommand, WorldCommandEvent, WorldDesc,
    WorldError, WorldEvent,
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

fn radial_velocity_joint_world() -> (World, BodyHandle, BodyHandle) {
    let mut world = World::new(WorldDesc {
        gravity: Vector::default(),
        enable_sleep: false,
    });
    let body_a = world
        .create_body(BodyDesc {
            pose: Pose::from_xy_angle(0.0, 0.0, 0.0),
            linear_velocity: (-1.0, 0.0).into(),
            can_sleep: false,
            ..BodyDesc::default()
        })
        .expect("body_a should be created");
    let body_b = world
        .create_body(BodyDesc {
            pose: Pose::from_xy_angle(1.0, 0.0, 0.0),
            linear_velocity: (1.0, 0.0).into(),
            can_sleep: false,
            ..BodyDesc::default()
        })
        .expect("body_b should be created");

    for body in [body_a, body_b] {
        world
            .create_collider(
                body,
                ColliderDesc {
                    shape: SharedShape::circle(0.1),
                    ..ColliderDesc::default()
                },
            )
            .expect("mass-bearing collider should be created");
    }
    world
        .create_joint(JointDesc::Distance(DistanceJointDesc {
            body_a,
            body_b,
            rest_length: 1.0,
            stiffness: 1.0,
            ..DistanceJointDesc::default()
        }))
        .expect("distance joint should be created");
    (world, body_a, body_b)
}

fn distance_joint_radial_speed(world: &World, body_a: BodyHandle, body_b: BodyHandle) -> f32 {
    let body_a = world
        .try_body(body_a)
        .expect("body_a should remain live after step");
    let body_b = world
        .try_body(body_b)
        .expect("body_b should remain live after step");
    let direction =
        (body_b.pose().translation() - body_a.pose().translation()).normalized_or_zero();
    (body_b.linear_velocity() - body_a.linear_velocity()).dot(direction)
}

#[derive(Clone, Copy, Debug)]
struct ContactWokenDistanceProjectionFacts {
    radial_speed: f32,
    joint_row_count: usize,
}

fn contact_woken_distance_projection_facts(
    joint_velocity_projection: bool,
) -> ContactWokenDistanceProjectionFacts {
    let mut world = World::new(WorldDesc {
        gravity: Vector::default(),
        enable_sleep: true,
    });
    let body_a = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(0.0, 0.0, 0.0),
            can_sleep: true,
            ..BodyDesc::default()
        })
        .expect("contact-woken Distance body_a should be created");
    let body_b = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(1.0, 0.0, 0.0),
            can_sleep: true,
            ..BodyDesc::default()
        })
        .expect("contact-woken Distance body_b should be created");
    let body_a_collider = world
        .create_collider(
            body_a,
            ColliderDesc {
                shape: SharedShape::circle(0.2),
                ..ColliderDesc::default()
            },
        )
        .expect("contact-woken Distance body_a collider should be created");
    world
        .create_collider(
            body_b,
            ColliderDesc {
                shape: SharedShape::circle(0.2),
                ..ColliderDesc::default()
            },
        )
        .expect("contact-woken Distance body_b collider should be created");
    world
        .create_joint(JointDesc::Distance(DistanceJointDesc {
            body_a,
            body_b,
            rest_length: 1.0,
            stiffness: 1.0,
            ..DistanceJointDesc::default()
        }))
        .expect("contact-woken Distance joint should be created");

    // Consume lifecycle wakes before establishing the sleeping precondition. The actual test
    // step then starts with an inactive Distance island, so only its contact can activate it.
    let mut preparation_pipeline = SimulationPipeline::new(StepConfig {
        joint_velocity_projection: false,
        enable_sleep: true,
        ..StepConfig::default()
    });
    preparation_pipeline.step(&mut world);
    for body in [body_a, body_b] {
        world
            .apply_body_patch(
                body,
                BodyPatch {
                    sleeping: Some(true),
                    ..BodyPatch::default()
                },
            )
            .expect("Distance endpoint should enter the sleeping precondition");
    }
    assert!(
        world
            .try_body(body_a)
            .expect("body_a stays live")
            .sleeping()
            && world
                .try_body(body_b)
                .expect("body_b stays live")
                .sleeping(),
        "S5_HARNESS_BOUNDARY:contact_woken_distance_sleeping_precondition"
    );

    let striker = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(-0.35, 0.0, 0.0),
            linear_velocity: Vector::new(4.0, 0.0),
            can_sleep: false,
            ..BodyDesc::default()
        })
        .expect("contact-woken Distance striker should be created");
    let striker_collider = world
        .create_collider(
            striker,
            ColliderDesc {
                shape: SharedShape::circle(0.2),
                ..ColliderDesc::default()
            },
        )
        .expect("contact-woken Distance striker collider should be created");
    let mut pipeline = SimulationPipeline::new(StepConfig {
        joint_velocity_projection,
        enable_sleep: true,
        ..StepConfig::default()
    });
    let report = pipeline.step(&mut world);
    let normal_impulse = report
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::ContactStarted(contact) | WorldEvent::ContactPersisted(contact)
                if (contact.collider_a == body_a_collider
                    && contact.collider_b == striker_collider)
                    || (contact.collider_a == striker_collider
                        && contact.collider_b == body_a_collider) =>
            {
                Some(contact.solver_normal_impulse)
            }
            _ => None,
        })
        .expect("S5_HARNESS_BOUNDARY:contact_woken_distance_contact_missing");
    let radial_speed = distance_joint_radial_speed(&world, body_a, body_b).abs();
    let body_a_awake = !world
        .try_body(body_a)
        .expect("body_a stays live")
        .sleeping();
    let body_b_awake = !world
        .try_body(body_b)
        .expect("body_b stays live")
        .sleeping();
    let body_a_contact_wake = report.events.iter().any(|event| {
        matches!(
            event,
            WorldEvent::SleepChanged(sleep)
                if sleep.body == body_a
                    && !sleep.is_sleeping
                    && matches!(
                        sleep.reason,
                        SleepTransitionReason::Impact
                            | SleepTransitionReason::ContactImpulse
                    )
        )
    });
    let body_b_island_wake = report.events.iter().any(|event| {
        matches!(
            event,
            WorldEvent::SleepChanged(sleep) if sleep.body == body_b && !sleep.is_sleeping
        )
    });
    println!(
        "S5_DISTANCE_CONTACT_WAKE_FACT:projection={joint_velocity_projection};radial_speed={radial_speed:.9};normal_impulse={normal_impulse:.9};contacts={};contact_rows={};joint_rows={};sleep_transitions={};body_a_contact_wake={body_a_contact_wake};body_b_island_wake={body_b_island_wake};body_a_awake={body_a_awake};body_b_awake={body_b_awake};numeric_warnings={}",
        report.stats.contact_count,
        report.stats.contact_row_count,
        report.stats.joint_row_count,
        report.stats.sleep_transition_count,
        report.stats.numeric_warnings
    );
    assert!(
        radial_speed.is_finite()
            && normal_impulse.is_finite()
            && normal_impulse > 0.0
            && report.stats.contact_count > 0
            && report.stats.contact_row_count > 0
            && body_a_contact_wake
            && body_b_island_wake
            && body_a_awake
            && body_b_awake
            && report.stats.numeric_warnings == 0,
        "S5_HARNESS_BOUNDARY:contact_woken_distance_requires_real_contact_and_wake"
    );
    ContactWokenDistanceProjectionFacts {
        radial_speed,
        joint_row_count: report.stats.joint_row_count,
    }
}

#[test]
fn joint_velocity_projection_flag_controls_radial_velocity_projection() {
    let (mut disabled_world, disabled_a, disabled_b) = radial_velocity_joint_world();
    let mut disabled_pipeline = SimulationPipeline::new(StepConfig {
        joint_velocity_projection: false,
        enable_sleep: false,
        ..StepConfig::default()
    });
    disabled_pipeline.step(&mut disabled_world);
    let disabled_radial_speed =
        distance_joint_radial_speed(&disabled_world, disabled_a, disabled_b).abs();
    assert!(
        disabled_radial_speed > 1.0,
        "disabled projection should leave the outgoing radial velocity visible, got {disabled_radial_speed}"
    );

    let (mut enabled_world, enabled_a, enabled_b) = radial_velocity_joint_world();
    let mut enabled_pipeline = SimulationPipeline::new(StepConfig {
        joint_velocity_projection: true,
        enable_sleep: false,
        ..StepConfig::default()
    });
    enabled_pipeline.step(&mut enabled_world);
    let enabled_radial_speed =
        distance_joint_radial_speed(&enabled_world, enabled_a, enabled_b).abs();
    assert!(
        enabled_radial_speed <= 1.0e-4,
        "enabled projection should remove the distance-joint radial velocity, got {enabled_radial_speed}"
    );
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
fn contact_gathering_geometry_cache_invalidates_after_body_and_collider_edits() {
    let mut world = World::new(WorldDesc {
        gravity: (0.0, 0.0).into(),
        enable_sleep: false,
    });
    let left_body = world
        .create_body(BodyDesc {
            body_type: BodyType::Static,
            pose: Pose::from_xy_angle(0.0, 0.0, 0.0),
            ..BodyDesc::default()
        })
        .expect("left body should be created");
    world
        .create_collider(
            left_body,
            ColliderDesc {
                shape: SharedShape::rect(2.0, 2.0),
                ..ColliderDesc::default()
            },
        )
        .expect("left collider should be created");
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
                shape: SharedShape::rect(2.0, 2.0),
                ..ColliderDesc::default()
            },
        )
        .expect("right collider should be created");

    let mut pipeline = SimulationPipeline::new(StepConfig::default());
    let first = pipeline.step(&mut world);
    assert!(
        first.stats.contact_count > 0,
        "initial overlap should warm the derived geometry cache through contact gathering"
    );

    world
        .apply_body_patch(
            right_body,
            BodyPatch {
                pose: Some(Pose::from_xy_angle(10.0, 0.0, 0.0)),
                ..BodyPatch::default()
            },
        )
        .expect("body pose patch should move the collider away");
    let after_body_move = pipeline.step(&mut world);
    assert_eq!(
        after_body_move.stats.contact_count, 0,
        "contact gathering must not reuse old world-space AABB/vertices after a body pose edit"
    );

    world
        .apply_body_patch(
            right_body,
            BodyPatch {
                pose: Some(Pose::from_xy_angle(1.5, 0.0, 0.0)),
                ..BodyPatch::default()
            },
        )
        .expect("body pose patch should move the collider back");
    let warmed_again = pipeline.step(&mut world);
    assert!(
        warmed_again.stats.contact_count > 0,
        "returning to overlap should warm the cache at the new transform"
    );

    world
        .apply_collider_patch(
            right_collider,
            ColliderPatch {
                local_pose: Some(Pose::from_xy_angle(20.0, 0.0, 0.0)),
                shape: Some(SharedShape::rect(1.0, 1.0)),
                ..ColliderPatch::default()
            },
        )
        .expect("collider geometry patch should move the shape away");
    let after_collider_edit = pipeline.step(&mut world);
    assert_eq!(
        after_collider_edit.stats.contact_count, 0,
        "contact gathering must not reuse old derived geometry after collider shape/local-pose edits"
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

#[test]
fn dynamic_static_distance_joints_solve_through_dense_island_slots() {
    let mut world = World::new(WorldDesc {
        gravity: (0.0, 0.0).into(),
        enable_sleep: false,
    });
    let static_right = world
        .create_body(BodyDesc {
            body_type: BodyType::Static,
            pose: Pose::from_xy_angle(4.0, 0.0, 0.0),
            ..BodyDesc::default()
        })
        .expect("right static endpoint should be created");
    let dynamic_to_right = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(0.0, 0.0, 0.0),
            ..BodyDesc::default()
        })
        .expect("dynamic endpoint should be created");
    let static_left = world
        .create_body(BodyDesc {
            body_type: BodyType::Static,
            pose: Pose::from_xy_angle(-4.0, 0.0, 0.0),
            ..BodyDesc::default()
        })
        .expect("left static endpoint should be created");
    let dynamic_to_left = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(0.0, 0.0, 0.0),
            ..BodyDesc::default()
        })
        .expect("dynamic endpoint should be created");

    world
        .create_joint(JointDesc::Distance(DistanceJointDesc {
            body_a: dynamic_to_right,
            body_b: static_right,
            rest_length: 1.0,
            stiffness: 1.0,
            ..DistanceJointDesc::default()
        }))
        .expect("dynamic-static distance joint should be created");
    world
        .create_joint(JointDesc::Distance(DistanceJointDesc {
            body_a: static_left,
            body_b: dynamic_to_left,
            rest_length: 1.0,
            stiffness: 1.0,
            ..DistanceJointDesc::default()
        }))
        .expect("static-dynamic distance joint should be created");

    let right_before = world
        .try_body(dynamic_to_right)
        .expect("dynamic endpoint must resolve")
        .pose()
        .translation()
        .x();
    let left_before = world
        .try_body(dynamic_to_left)
        .expect("dynamic endpoint must resolve")
        .pose()
        .translation()
        .x();
    let static_right_before = world
        .try_body(static_right)
        .expect("static endpoint must resolve")
        .pose()
        .translation()
        .x();
    let static_left_before = world
        .try_body(static_left)
        .expect("static endpoint must resolve")
        .pose()
        .translation()
        .x();

    let mut pipeline = SimulationPipeline::new(StepConfig::default());
    let report = pipeline.step(&mut world);

    let right_after = world
        .try_body(dynamic_to_right)
        .expect("dynamic endpoint must resolve")
        .pose()
        .translation()
        .x();
    let left_after = world
        .try_body(dynamic_to_left)
        .expect("dynamic endpoint must resolve")
        .pose()
        .translation()
        .x();
    assert_eq!(report.stats.joint_count, 2);
    assert!(
        right_after > right_before,
        "dynamic body_a should move toward its static distance-joint endpoint"
    );
    assert!(
        left_after < left_before,
        "dynamic body_b should move toward its static distance-joint endpoint"
    );
    assert_eq!(
        world
            .try_body(static_right)
            .expect("static endpoint must resolve")
            .pose()
            .translation()
            .x(),
        static_right_before,
        "static endpoint must remain fixed while the dense island row solves"
    );
    assert_eq!(
        world
            .try_body(static_left)
            .expect("static endpoint must resolve")
            .pose()
            .translation()
            .x(),
        static_left_before,
        "static endpoint must remain fixed while the dense island row solves"
    );
}

fn s5_lifecycle_world() -> World {
    World::new(WorldDesc {
        gravity: Vector::default(),
        enable_sleep: true,
    })
}

fn s5_dynamic_body(world: &mut World, sleeping: bool, case: &str) -> BodyHandle {
    world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            sleeping,
            ..BodyDesc::default()
        })
        .unwrap_or_else(|error| {
            panic!("S5_HARNESS_BOUNDARY:{case}: dynamic body setup failed: {error:?}")
        })
}

fn s5_static_body(world: &mut World, case: &str) -> BodyHandle {
    world
        .create_body(BodyDesc {
            body_type: BodyType::Static,
            ..BodyDesc::default()
        })
        .unwrap_or_else(|error| {
            panic!("S5_HARNESS_BOUNDARY:{case}: static body setup failed: {error:?}")
        })
}

fn s5_set_sleeping(world: &mut World, body: BodyHandle, case: &str) {
    world
        .apply_body_patch(
            body,
            BodyPatch {
                sleeping: Some(true),
                ..BodyPatch::default()
            },
        )
        .unwrap_or_else(|error| {
            panic!("S5_HARNESS_BOUNDARY:{case}: could not prepare sleeping dynamic: {error:?}")
        });
    assert!(
        world
            .body(body)
            .unwrap_or_else(|error| {
                panic!("S5_HARNESS_BOUNDARY:{case}: prepared body vanished: {error:?}")
            })
            .sleeping(),
        "S5_HARNESS_BOUNDARY:{case}: sleeping precondition was not established"
    );
}

fn s5_step_events(world: &mut World) -> Vec<WorldEvent> {
    SimulationPipeline::new(StepConfig::default())
        .step(world)
        .events
}

fn s5_has_user_patch_wake(events: &[WorldEvent], body: BodyHandle) -> bool {
    events.iter().any(|event| {
        matches!(
            event,
            WorldEvent::SleepChanged(sleep)
                if sleep.body == body
                    && !sleep.is_sleeping
                    && sleep.reason == SleepTransitionReason::UserPatch
        )
    })
}

fn s5_has_sleep_transition(events: &[WorldEvent], body: BodyHandle) -> bool {
    events
        .iter()
        .any(|event| matches!(event, WorldEvent::SleepChanged(sleep) if sleep.body == body))
}

fn s5_is_awake(world: &World, body: BodyHandle, case: &str) -> bool {
    !world
        .body(body)
        .unwrap_or_else(|error| {
            panic!("S5_HARNESS_BOUNDARY:{case}: live endpoint vanished: {error:?}")
        })
        .sleeping()
}

#[derive(Clone, Copy, Debug)]
struct S5WakeObservation {
    awake: bool,
    user_patch: bool,
}

impl S5WakeObservation {
    fn satisfied(self) -> bool {
        self.awake && self.user_patch
    }
}

fn s5_observe_wake(
    world: &World,
    events: &[WorldEvent],
    body: BodyHandle,
    case: &str,
) -> S5WakeObservation {
    S5WakeObservation {
        awake: s5_is_awake(world, body, case),
        user_patch: s5_has_user_patch_wake(events, body),
    }
}

fn s5_assert_joint_kinds_woke(
    case: &str,
    distance: S5WakeObservation,
    world_anchor: S5WakeObservation,
    revolute: S5WakeObservation,
) {
    println!(
        "S5_WAKE_OBSERVATION:{case}:Distance(awake={},user_patch={});WorldAnchor(awake={},user_patch={});Revolute(awake={},user_patch={})",
        distance.awake,
        distance.user_patch,
        world_anchor.awake,
        world_anchor.user_patch,
        revolute.awake,
        revolute.user_patch
    );
    if distance.satisfied() && world_anchor.satisfied() && revolute.satisfied() {
        return;
    }

    let expected_failure = match case {
        "create" => "Distance sleeping dynamic endpoint remained sleeping or lacked UserPatch",
        "constraint_patch" => "Distance endpoint remained sleeping after a constraint patch",
        "user_data_only" => "Distance positive constraint control remained sleeping",
        "remove" => "still-live Distance endpoint remained sleeping",
        _ => "existing-kind wake contract was not satisfied",
    };
    panic!(
        "S5_EXPECTED_WAKE:{case}: {expected_failure}; observed Distance(awake={},user_patch={}), WorldAnchor(awake={},user_patch={}), Revolute(awake={},user_patch={})",
        distance.awake,
        distance.user_patch,
        world_anchor.awake,
        world_anchor.user_patch,
        revolute.awake,
        revolute.user_patch
    );
}

#[test]
fn post_contact_revolute_reconciliation_does_not_repeat_existing_joint_rows() {
    let config = StepConfig {
        joint_velocity_projection: false,
        enable_sleep: false,
        ..StepConfig::default()
    };
    let dt = config.dt;
    let contact_woken_projection_enabled = contact_woken_distance_projection_facts(true);
    let contact_woken_projection_disabled = contact_woken_distance_projection_facts(false);

    let mut distance_world = World::new(WorldDesc {
        gravity: Vector::default(),
        enable_sleep: false,
    });
    let distance_dynamic = distance_world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(0.0, 0.0, 0.0),
            can_sleep: false,
            ..BodyDesc::default()
        })
        .expect("distance no-repeat dynamic body should be created");
    let distance_static = distance_world
        .create_body(BodyDesc {
            body_type: BodyType::Static,
            pose: Pose::from_xy_angle(4.0, 0.0, 0.0),
            ..BodyDesc::default()
        })
        .expect("distance no-repeat static body should be created");
    distance_world
        .create_joint(JointDesc::Distance(DistanceJointDesc {
            body_a: distance_dynamic,
            body_b: distance_static,
            rest_length: 1.0,
            stiffness: 1.0,
            ..DistanceJointDesc::default()
        }))
        .expect("distance no-repeat joint should be created");
    // A static counterpart doubles the existing Distance correction. Re-evaluating that row in a
    // generic post pass would use the already-corrected distance and land at `distance_repeated`.
    let distance_single = 2.0 * (4.0 - 1.0) * dt;
    let distance_repeated = distance_single + 2.0 * ((4.0 - distance_single) - 1.0) * dt;
    let mut distance_pipeline = SimulationPipeline::new(config);
    let distance_report = distance_pipeline.step(&mut distance_world);
    let distance_actual = distance_world
        .try_body(distance_dynamic)
        .expect("distance dynamic body survives")
        .pose()
        .translation()
        .x();

    let mut anchor_world = World::new(WorldDesc {
        gravity: Vector::default(),
        enable_sleep: false,
    });
    let anchor_dynamic = anchor_world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(0.0, 0.0, 0.0),
            linear_velocity: (1.0, 0.0).into(),
            can_sleep: false,
            ..BodyDesc::default()
        })
        .expect("world-anchor no-repeat body should be created");
    anchor_world
        .create_collider(
            anchor_dynamic,
            ColliderDesc {
                shape: SharedShape::circle(0.1),
                density: 1.0,
                ..ColliderDesc::default()
            },
        )
        .expect("world-anchor damping control needs finite inverse mass");
    let anchor_damping = 30.0;
    anchor_world
        .create_joint(JointDesc::WorldAnchor(WorldAnchorJointDesc {
            body: anchor_dynamic,
            world_anchor: (3.0, 0.0).into(),
            stiffness: 1.0,
            damping: anchor_damping,
            ..WorldAnchorJointDesc::default()
        }))
        .expect("world-anchor no-repeat joint should be created");
    // Each WorldAnchor evaluation predicts with the velocity then applies correction and damping.
    // A forbidden second solve therefore changes both the current pose and velocity a second time.
    let damping_strength = (anchor_damping * dt).clamp(0.0, 1.0);
    let first_eval = dt;
    let first_correction = (3.0 - first_eval) * dt;
    let first_velocity = 1.0 * (1.0 - damping_strength);
    let anchor_single = first_correction + first_velocity * dt;
    let second_eval = first_correction + first_velocity * dt;
    let second_correction = (3.0 - second_eval) * dt;
    let second_velocity = first_velocity * (1.0 - damping_strength);
    let anchor_repeated = first_correction + second_correction + second_velocity * dt;
    let mut anchor_pipeline = SimulationPipeline::new(config);
    let anchor_report = anchor_pipeline.step(&mut anchor_world);
    let anchor_actual = anchor_world
        .try_body(anchor_dynamic)
        .expect("world-anchor body survives")
        .pose()
        .translation()
        .x();
    let anchor_actual_velocity = anchor_world
        .try_body(anchor_dynamic)
        .expect("world-anchor body survives")
        .linear_velocity()
        .x();

    println!(
        "S5_REVOLUTE_REPLAN_BOUNDARY:no_repeat_existing_rows:Distance(actual={distance_actual:.6},single={distance_single:.6},repeated={distance_repeated:.6},rows={});WorldAnchor(actual={anchor_actual:.6},single={anchor_single:.6},repeated={anchor_repeated:.6},velocity={anchor_actual_velocity:.6},single_velocity={first_velocity:.6},repeated_velocity={second_velocity:.6},rows={})",
        distance_report.stats.joint_row_count,
        anchor_report.stats.joint_row_count
    );
    println!(
        "S5_REVOLUTE_REPLAN_BOUNDARY:contact_woken_distance_projection:enabled={contact_woken_projection_enabled:?};disabled={contact_woken_projection_disabled:?}"
    );
    assert_eq!(distance_report.stats.joint_row_count, 1);
    assert_eq!(anchor_report.stats.joint_row_count, 1);
    assert!(
        contact_woken_projection_enabled.joint_row_count == 0
            && contact_woken_projection_disabled.joint_row_count == 0
            && contact_woken_projection_enabled.radial_speed <= 1.0e-4
            && contact_woken_projection_disabled.radial_speed > 1.0e-3,
        "S5_REVOLUTE_REPLAN_BOUNDARY:contact_woken_distance_projection: enabled radial speed={:.9}, disabled radial speed={:.9}, enabled rows={}, disabled rows={}",
        contact_woken_projection_enabled.radial_speed,
        contact_woken_projection_disabled.radial_speed,
        contact_woken_projection_enabled.joint_row_count,
        contact_woken_projection_disabled.joint_row_count
    );
    assert!(
        (distance_actual - distance_single).abs() <= 1.0e-6
            && (distance_actual - distance_repeated).abs() > 0.01,
        "Distance row must run exactly once per step"
    );
    assert!(
        (anchor_actual - anchor_single).abs() <= 1.0e-6
            && (anchor_actual - anchor_repeated).abs() > 0.01,
        "WorldAnchor correction must run exactly once per step"
    );
    assert!(
        (anchor_actual_velocity - first_velocity).abs() <= 1.0e-6
            && (anchor_actual_velocity - second_velocity).abs() > 0.1,
        "WorldAnchor damping must run exactly once per step"
    );
}

#[test]
fn joint_lifecycle_wake_create_contract() {
    let mut world = s5_lifecycle_world();
    let static_endpoint = s5_static_body(&mut world, "create");
    let distance_dynamic = s5_dynamic_body(&mut world, true, "create");
    let anchor_dynamic = s5_dynamic_body(&mut world, true, "create");
    let revolute_dynamic = s5_dynamic_body(&mut world, true, "create");

    let distance_joint = world
        .create_joint(JointDesc::Distance(DistanceJointDesc {
            body_a: distance_dynamic,
            body_b: static_endpoint,
            ..DistanceJointDesc::default()
        }))
        .expect("S5_HARNESS_BOUNDARY:create: Distance setup must succeed");
    let anchor_joint = world
        .create_joint(JointDesc::WorldAnchor(WorldAnchorJointDesc {
            body: anchor_dynamic,
            ..WorldAnchorJointDesc::default()
        }))
        .expect("S5_HARNESS_BOUNDARY:create: WorldAnchor setup must succeed");
    let revolute_joint = world
        .create_joint(JointDesc::Revolute(RevoluteJointDesc {
            body_a: revolute_dynamic,
            body_b: static_endpoint,
            ..RevoluteJointDesc::default()
        }))
        .expect("S5_HARNESS_BOUNDARY:create: Revolute setup must succeed");

    let events = s5_step_events(&mut world);
    assert!(
        events.iter().any(
            |event| matches!(event, WorldEvent::JointCreated { joint } if *joint == distance_joint)
        ) && events.iter().any(
            |event| matches!(event, WorldEvent::JointCreated { joint } if *joint == anchor_joint)
        ) && events.iter().any(
            |event| matches!(event, WorldEvent::JointCreated { joint } if *joint == revolute_joint)
        ),
        "S5_HARNESS_BOUNDARY:create: all three joint kinds must execute"
    );
    assert!(
        !s5_has_sleep_transition(&events, static_endpoint),
        "S5_HARNESS_BOUNDARY:create: static endpoint must not emit a sleep transition"
    );
    let distance_observation = s5_observe_wake(&world, &events, distance_dynamic, "create");
    let world_anchor_observation = s5_observe_wake(&world, &events, anchor_dynamic, "create");
    let revolute_observation = s5_observe_wake(&world, &events, revolute_dynamic, "create");
    println!("S5_WAKE_CASE:create");
    s5_assert_joint_kinds_woke(
        "create",
        distance_observation,
        world_anchor_observation,
        revolute_observation,
    );
}

#[test]
fn joint_lifecycle_wake_constraint_patch_contract() {
    let mut world = s5_lifecycle_world();
    let static_endpoint = s5_static_body(&mut world, "constraint_patch");
    let distance_dynamic = s5_dynamic_body(&mut world, false, "constraint_patch");
    let anchor_dynamic = s5_dynamic_body(&mut world, false, "constraint_patch");
    let revolute_dynamic = s5_dynamic_body(&mut world, false, "constraint_patch");
    let distance_joint = world
        .create_joint(JointDesc::Distance(DistanceJointDesc {
            body_a: distance_dynamic,
            body_b: static_endpoint,
            ..DistanceJointDesc::default()
        }))
        .expect("S5_HARNESS_BOUNDARY:constraint_patch: Distance setup must succeed");
    let anchor_joint = world
        .create_joint(JointDesc::WorldAnchor(WorldAnchorJointDesc {
            body: anchor_dynamic,
            ..WorldAnchorJointDesc::default()
        }))
        .expect("S5_HARNESS_BOUNDARY:constraint_patch: WorldAnchor setup must succeed");
    let revolute_joint = world
        .create_joint(JointDesc::Revolute(RevoluteJointDesc {
            body_a: revolute_dynamic,
            body_b: static_endpoint,
            ..RevoluteJointDesc::default()
        }))
        .expect("S5_HARNESS_BOUNDARY:constraint_patch: Revolute setup must succeed");
    let _setup_events = s5_step_events(&mut world);
    s5_set_sleeping(&mut world, distance_dynamic, "constraint_patch");
    s5_set_sleeping(&mut world, anchor_dynamic, "constraint_patch");
    s5_set_sleeping(&mut world, revolute_dynamic, "constraint_patch");

    world
        .apply_joint_patch(
            distance_joint,
            JointPatch::Distance(DistanceJointPatch {
                stiffness: Some(0.75),
                ..DistanceJointPatch::default()
            }),
        )
        .expect("S5_HARNESS_BOUNDARY:constraint_patch: Distance patch must succeed");
    world
        .apply_joint_patch(
            anchor_joint,
            JointPatch::WorldAnchor(WorldAnchorJointPatch {
                stiffness: Some(0.75),
                ..WorldAnchorJointPatch::default()
            }),
        )
        .expect("S5_HARNESS_BOUNDARY:constraint_patch: WorldAnchor patch must succeed");
    world
        .apply_joint_patch(
            revolute_joint,
            JointPatch::Revolute(RevoluteJointPatch {
                local_anchor_a: Some((0.25, 0.0).into()),
                ..RevoluteJointPatch::default()
            }),
        )
        .expect("S5_HARNESS_BOUNDARY:constraint_patch: Revolute patch must succeed");

    let events = s5_step_events(&mut world);
    assert!(
        !s5_has_sleep_transition(&events, static_endpoint),
        "S5_HARNESS_BOUNDARY:constraint_patch: static endpoint must not transition"
    );
    let distance_observation =
        s5_observe_wake(&world, &events, distance_dynamic, "constraint_patch");
    let world_anchor_observation =
        s5_observe_wake(&world, &events, anchor_dynamic, "constraint_patch");
    let revolute_observation =
        s5_observe_wake(&world, &events, revolute_dynamic, "constraint_patch");
    println!("S5_WAKE_CASE:constraint_patch");
    s5_assert_joint_kinds_woke(
        "constraint_patch",
        distance_observation,
        world_anchor_observation,
        revolute_observation,
    );
}

#[test]
fn joint_lifecycle_wake_user_data_only_contract() {
    let mut world = s5_lifecycle_world();
    let static_endpoint = s5_static_body(&mut world, "user_data_only");
    let distance_dynamic = s5_dynamic_body(&mut world, false, "user_data_only");
    let anchor_dynamic = s5_dynamic_body(&mut world, false, "user_data_only");
    let revolute_dynamic = s5_dynamic_body(&mut world, false, "user_data_only");
    let distance_joint = world
        .create_joint(JointDesc::Distance(DistanceJointDesc {
            body_a: distance_dynamic,
            body_b: static_endpoint,
            ..DistanceJointDesc::default()
        }))
        .expect("S5_HARNESS_BOUNDARY:user_data_only: Distance setup must succeed");
    let anchor_joint = world
        .create_joint(JointDesc::WorldAnchor(WorldAnchorJointDesc {
            body: anchor_dynamic,
            ..WorldAnchorJointDesc::default()
        }))
        .expect("S5_HARNESS_BOUNDARY:user_data_only: WorldAnchor setup must succeed");
    let revolute_joint = world
        .create_joint(JointDesc::Revolute(RevoluteJointDesc {
            body_a: revolute_dynamic,
            body_b: static_endpoint,
            ..RevoluteJointDesc::default()
        }))
        .expect("S5_HARNESS_BOUNDARY:user_data_only: Revolute setup must succeed");
    let _setup_events = s5_step_events(&mut world);
    s5_set_sleeping(&mut world, distance_dynamic, "user_data_only");
    s5_set_sleeping(&mut world, anchor_dynamic, "user_data_only");
    s5_set_sleeping(&mut world, revolute_dynamic, "user_data_only");

    world
        .apply_joint_patch(
            distance_joint,
            JointPatch::Distance(DistanceJointPatch::default()),
        )
        .expect("S5_HARNESS_BOUNDARY:user_data_only: empty Distance patch must succeed");
    world
        .apply_joint_patch(
            anchor_joint,
            JointPatch::WorldAnchor(WorldAnchorJointPatch::default()),
        )
        .expect("S5_HARNESS_BOUNDARY:user_data_only: empty WorldAnchor patch must succeed");
    world
        .apply_joint_patch(
            revolute_joint,
            JointPatch::Revolute(RevoluteJointPatch::default()),
        )
        .expect("S5_HARNESS_BOUNDARY:user_data_only: empty Revolute patch must succeed");
    assert!(
        world
            .body(distance_dynamic)
            .expect("S5_HARNESS_BOUNDARY:user_data_only: Distance after empty patch")
            .sleeping()
            && world
                .body(anchor_dynamic)
                .expect("S5_HARNESS_BOUNDARY:user_data_only: WorldAnchor after empty patch")
                .sleeping()
            && world
                .body(revolute_dynamic)
                .expect("S5_HARNESS_BOUNDARY:user_data_only: Revolute after empty patch")
                .sleeping(),
        "S5_HARNESS_BOUNDARY:user_data_only: empty patch must not wake any joint kind"
    );

    world
        .apply_joint_patch(
            distance_joint,
            JointPatch::Distance(DistanceJointPatch {
                user_data: Some(41),
                ..DistanceJointPatch::default()
            }),
        )
        .expect("S5_HARNESS_BOUNDARY:user_data_only: Distance metadata patch must succeed");
    world
        .apply_joint_patch(
            anchor_joint,
            JointPatch::WorldAnchor(WorldAnchorJointPatch {
                user_data: Some(42),
                ..WorldAnchorJointPatch::default()
            }),
        )
        .expect("S5_HARNESS_BOUNDARY:user_data_only: WorldAnchor metadata patch must succeed");
    world
        .apply_joint_patch(
            revolute_joint,
            JointPatch::Revolute(RevoluteJointPatch {
                user_data: Some(43),
                ..RevoluteJointPatch::default()
            }),
        )
        .expect("S5_HARNESS_BOUNDARY:user_data_only: Revolute metadata patch must succeed");
    let metadata_kept_distance_sleeping = world
        .body(distance_dynamic)
        .expect("S5_HARNESS_BOUNDARY:user_data_only: Distance endpoint must remain live")
        .sleeping();
    let metadata_kept_anchor_sleeping = world
        .body(anchor_dynamic)
        .expect("S5_HARNESS_BOUNDARY:user_data_only: WorldAnchor endpoint must remain live")
        .sleeping();
    let metadata_kept_revolute_sleeping = world
        .body(revolute_dynamic)
        .expect("S5_HARNESS_BOUNDARY:user_data_only: Revolute endpoint must remain live")
        .sleeping();

    world
        .apply_joint_patch(
            distance_joint,
            JointPatch::Distance(DistanceJointPatch {
                stiffness: Some(0.5),
                ..DistanceJointPatch::default()
            }),
        )
        .expect("S5_HARNESS_BOUNDARY:user_data_only: Distance positive control must succeed");
    world
        .apply_joint_patch(
            anchor_joint,
            JointPatch::WorldAnchor(WorldAnchorJointPatch {
                stiffness: Some(0.5),
                ..WorldAnchorJointPatch::default()
            }),
        )
        .expect("S5_HARNESS_BOUNDARY:user_data_only: WorldAnchor positive control must succeed");
    world
        .apply_joint_patch(
            revolute_joint,
            JointPatch::Revolute(RevoluteJointPatch {
                local_anchor_b: Some((0.0, 0.5).into()),
                ..RevoluteJointPatch::default()
            }),
        )
        .expect("S5_HARNESS_BOUNDARY:user_data_only: Revolute positive control must succeed");

    let events = s5_step_events(&mut world);
    assert!(
        metadata_kept_distance_sleeping
            && metadata_kept_anchor_sleeping
            && metadata_kept_revolute_sleeping,
        "S5_HARNESS_BOUNDARY:user_data_only: metadata-only patch must not wake any joint kind"
    );
    assert!(
        !s5_has_sleep_transition(&events, static_endpoint),
        "S5_HARNESS_BOUNDARY:user_data_only: static endpoint must not transition"
    );
    let distance_observation = s5_observe_wake(&world, &events, distance_dynamic, "user_data_only");
    let world_anchor_observation =
        s5_observe_wake(&world, &events, anchor_dynamic, "user_data_only");
    let revolute_observation = s5_observe_wake(&world, &events, revolute_dynamic, "user_data_only");
    println!("S5_WAKE_CASE:user_data_only");
    s5_assert_joint_kinds_woke(
        "user_data_only",
        distance_observation,
        world_anchor_observation,
        revolute_observation,
    );
}

#[test]
fn joint_lifecycle_wake_remove_contract() {
    let mut world = s5_lifecycle_world();
    let static_endpoint = s5_static_body(&mut world, "remove");
    let distance_dynamic = s5_dynamic_body(&mut world, false, "remove");
    let anchor_dynamic = s5_dynamic_body(&mut world, false, "remove");
    let revolute_dynamic = s5_dynamic_body(&mut world, false, "remove");
    let distance_joint = world
        .create_joint(JointDesc::Distance(DistanceJointDesc {
            body_a: distance_dynamic,
            body_b: static_endpoint,
            ..DistanceJointDesc::default()
        }))
        .expect("S5_HARNESS_BOUNDARY:remove: Distance setup must succeed");
    let anchor_joint = world
        .create_joint(JointDesc::WorldAnchor(WorldAnchorJointDesc {
            body: anchor_dynamic,
            ..WorldAnchorJointDesc::default()
        }))
        .expect("S5_HARNESS_BOUNDARY:remove: WorldAnchor setup must succeed");
    let revolute_joint = world
        .create_joint(JointDesc::Revolute(RevoluteJointDesc {
            body_a: revolute_dynamic,
            body_b: static_endpoint,
            ..RevoluteJointDesc::default()
        }))
        .expect("S5_HARNESS_BOUNDARY:remove: Revolute setup must succeed");
    let _setup_events = s5_step_events(&mut world);
    s5_set_sleeping(&mut world, distance_dynamic, "remove");
    s5_set_sleeping(&mut world, anchor_dynamic, "remove");
    s5_set_sleeping(&mut world, revolute_dynamic, "remove");

    world
        .destroy_joint(distance_joint)
        .expect("S5_HARNESS_BOUNDARY:remove: Distance remove must succeed");
    world
        .destroy_joint(anchor_joint)
        .expect("S5_HARNESS_BOUNDARY:remove: WorldAnchor remove must succeed");
    world
        .destroy_joint(revolute_joint)
        .expect("S5_HARNESS_BOUNDARY:remove: Revolute remove must succeed");
    let events = s5_step_events(&mut world);
    assert!(
        events.iter().any(
            |event| matches!(event, WorldEvent::JointRemoved { joint } if *joint == distance_joint)
        ) && events.iter().any(
            |event| matches!(event, WorldEvent::JointRemoved { joint } if *joint == anchor_joint)
        ) && events.iter().any(
            |event| matches!(event, WorldEvent::JointRemoved { joint } if *joint == revolute_joint)
        ),
        "S5_HARNESS_BOUNDARY:remove: all three kinds must emit JointRemoved"
    );
    assert!(
        !s5_has_sleep_transition(&events, static_endpoint),
        "S5_HARNESS_BOUNDARY:remove: static endpoint must not transition"
    );
    let distance_observation = s5_observe_wake(&world, &events, distance_dynamic, "remove");
    let world_anchor_observation = s5_observe_wake(&world, &events, anchor_dynamic, "remove");
    let revolute_observation = s5_observe_wake(&world, &events, revolute_dynamic, "remove");
    println!("S5_WAKE_CASE:remove");
    s5_assert_joint_kinds_woke(
        "remove",
        distance_observation,
        world_anchor_observation,
        revolute_observation,
    );
}

#[test]
fn joint_lifecycle_wake_body_cascade_contract() {
    let mut world = s5_lifecycle_world();
    let deleted_endpoint = s5_dynamic_body(&mut world, false, "body_cascade");
    let distance_survivor = s5_dynamic_body(&mut world, false, "body_cascade");
    let revolute_survivor = s5_dynamic_body(&mut world, false, "body_cascade");
    let distance_joint = world
        .create_joint(JointDesc::Distance(DistanceJointDesc {
            body_a: deleted_endpoint,
            body_b: distance_survivor,
            ..DistanceJointDesc::default()
        }))
        .expect("S5_HARNESS_BOUNDARY:body_cascade: Distance setup must succeed");
    let revolute_joint = world
        .create_joint(JointDesc::Revolute(RevoluteJointDesc {
            body_a: deleted_endpoint,
            body_b: revolute_survivor,
            ..RevoluteJointDesc::default()
        }))
        .expect("S5_HARNESS_BOUNDARY:body_cascade: Revolute setup must succeed");
    let _setup_events = s5_step_events(&mut world);
    s5_set_sleeping(&mut world, deleted_endpoint, "body_cascade");
    s5_set_sleeping(&mut world, distance_survivor, "body_cascade");
    s5_set_sleeping(&mut world, revolute_survivor, "body_cascade");

    world
        .destroy_body(deleted_endpoint)
        .expect("S5_HARNESS_BOUNDARY:body_cascade: body removal must succeed");
    assert!(
        matches!(
            world.body(deleted_endpoint),
            Err(WorldError::Handle(HandleError::StaleBody { .. }))
        ),
        "S5_HARNESS_BOUNDARY:body_cascade: deleted endpoint must be stale"
    );
    let events = s5_step_events(&mut world);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, WorldEvent::JointRemoved { joint: removed } if *removed == distance_joint))
            && events
                .iter()
                .any(|event| matches!(event, WorldEvent::JointRemoved { joint: removed } if *removed == revolute_joint)),
        "S5_HARNESS_BOUNDARY:body_cascade: both cascaded pair joints must emit JointRemoved"
    );
    assert!(
        !s5_has_sleep_transition(&events, deleted_endpoint),
        "S5_HARNESS_BOUNDARY:body_cascade: deleted endpoint must not receive a wake transition"
    );
    println!("S5_WAKE_CASE:body_cascade");

    let distance_observation = s5_observe_wake(&world, &events, distance_survivor, "body_cascade");
    let revolute_observation = s5_observe_wake(&world, &events, revolute_survivor, "body_cascade");
    println!(
        "S5_WAKE_OBSERVATION:body_cascade:Distance(awake={},user_patch={});Revolute(awake={},user_patch={})",
        distance_observation.awake,
        distance_observation.user_patch,
        revolute_observation.awake,
        revolute_observation.user_patch
    );
    assert!(
        distance_observation.satisfied() && revolute_observation.satisfied(),
        "S5_EXPECTED_WAKE:body_cascade: surviving Distance/Revolute counterpart remained sleeping"
    );
}

#[test]
fn joint_lifecycle_wake_rejected_transaction_contract() {
    let mut world = s5_lifecycle_world();
    let static_endpoint = s5_static_body(&mut world, "rejected_transaction");
    let dynamic_endpoint = s5_dynamic_body(&mut world, true, "rejected_transaction");
    let revolute_endpoint = s5_dynamic_body(&mut world, true, "rejected_transaction");
    let setup_events = s5_step_events(&mut world);
    assert!(
        matches!(
            setup_events.as_slice(),
            [
                WorldEvent::BodyCreated { body: first },
                WorldEvent::BodyCreated { body: second },
                WorldEvent::BodyCreated { body: third }
            ] if *first == static_endpoint
                && *second == dynamic_endpoint
                && *third == revolute_endpoint
        ),
        "S5_HARNESS_BOUNDARY:rejected_transaction: setup events must be fully drained before the rejected batch"
    );

    let mut untouched_control = world.clone();
    let initial_revision = world.revision();
    let initial_static = world
        .body(static_endpoint)
        .expect("S5_HARNESS_BOUNDARY:rejected_transaction: static setup view");
    let initial_dynamic = world
        .body(dynamic_endpoint)
        .expect("S5_HARNESS_BOUNDARY:rejected_transaction: dynamic setup view");
    let initial_revolute = world
        .body(revolute_endpoint)
        .expect("S5_HARNESS_BOUNDARY:rejected_transaction: revolute setup view");

    let error = world
        .commands()
        .apply([
            WorldCommand::CreateJoint {
                desc: JointDesc::Distance(DistanceJointDesc {
                    body_a: dynamic_endpoint,
                    body_b: static_endpoint,
                    ..DistanceJointDesc::default()
                }),
            },
            WorldCommand::CreateJoint {
                desc: JointDesc::Revolute(RevoluteJointDesc {
                    body_a: revolute_endpoint,
                    body_b: static_endpoint,
                    ..RevoluteJointDesc::default()
                }),
            },
            WorldCommand::CreateJoint {
                desc: JointDesc::WorldAnchor(WorldAnchorJointDesc {
                    body: BodyHandle::INVALID,
                    ..WorldAnchorJointDesc::default()
                }),
            },
        ])
        .expect_err("S5_HARNESS_BOUNDARY:rejected_transaction: invalid scratch batch must reject");
    let rejected_revision = world.revision();
    let rejected_joint_handles = world.joints().collect::<Vec<_>>();

    assert_eq!(
        error.command_index, 2,
        "S5_HARNESS_BOUNDARY:rejected_transaction: third command must be the rejection point"
    );
    assert_eq!(
        rejected_revision, initial_revision,
        "S5_HARNESS_BOUNDARY:rejected_transaction: rejected scratch batch leaked a revision"
    );
    assert_eq!(
        rejected_joint_handles,
        Vec::new(),
        "S5_HARNESS_BOUNDARY:rejected_transaction: rejected scratch batch leaked a joint"
    );
    assert_eq!(
        world
            .body(static_endpoint)
            .expect("S5_HARNESS_BOUNDARY:rejected_transaction: static endpoint after rejection"),
        initial_static,
        "S5_HARNESS_BOUNDARY:rejected_transaction: rejected scratch batch changed static body facts"
    );
    assert_eq!(
        world
            .body(dynamic_endpoint)
            .expect("S5_HARNESS_BOUNDARY:rejected_transaction: dynamic endpoint after rejection"),
        initial_dynamic,
        "S5_HARNESS_BOUNDARY:rejected_transaction: rejected scratch batch changed dynamic body facts"
    );
    assert_eq!(
        world
            .body(revolute_endpoint)
            .expect("S5_HARNESS_BOUNDARY:rejected_transaction: revolute endpoint after rejection"),
        initial_revolute,
        "S5_HARNESS_BOUNDARY:rejected_transaction: rejected scratch batch changed Revolute body facts"
    );

    let rejected_events = s5_step_events(&mut world);
    let control_events = s5_step_events(&mut untouched_control);
    assert!(
        rejected_events.is_empty(),
        "S5_HARNESS_BOUNDARY:rejected_transaction: authoritative event receipt must be completely empty after rejection, got {rejected_events:?}"
    );
    assert!(
        control_events.is_empty(),
        "S5_HARNESS_BOUNDARY:rejected_transaction: untouched control unexpectedly emitted events"
    );
    assert_eq!(
        world.revision(),
        untouched_control.revision(),
        "S5_HARNESS_BOUNDARY:rejected_transaction: rejected world revision diverged from untouched control"
    );
    assert_eq!(
        world
            .body(static_endpoint)
            .expect("S5_HARNESS_BOUNDARY:rejected_transaction: static post-step view"),
        untouched_control
            .body(static_endpoint)
            .expect("S5_HARNESS_BOUNDARY:rejected_transaction: control static post-step view"),
        "S5_HARNESS_BOUNDARY:rejected_transaction: static body diverged from untouched control"
    );
    assert_eq!(
        world
            .body(dynamic_endpoint)
            .expect("S5_HARNESS_BOUNDARY:rejected_transaction: dynamic post-step view"),
        untouched_control
            .body(dynamic_endpoint)
            .expect("S5_HARNESS_BOUNDARY:rejected_transaction: control dynamic post-step view"),
        "S5_HARNESS_BOUNDARY:rejected_transaction: dynamic body diverged from untouched control"
    );
    assert_eq!(
        world
            .body(revolute_endpoint)
            .expect("S5_HARNESS_BOUNDARY:rejected_transaction: revolute post-step view"),
        untouched_control
            .body(revolute_endpoint)
            .expect("S5_HARNESS_BOUNDARY:rejected_transaction: control revolute post-step view"),
        "S5_HARNESS_BOUNDARY:rejected_transaction: revolute body diverged from untouched control"
    );

    let successful = world
        .commands()
        .apply([
            WorldCommand::CreateJoint {
                desc: JointDesc::WorldAnchor(WorldAnchorJointDesc {
                    body: dynamic_endpoint,
                    ..WorldAnchorJointDesc::default()
                }),
            },
            WorldCommand::CreateJoint {
                desc: JointDesc::Revolute(RevoluteJointDesc {
                    body_a: revolute_endpoint,
                    body_b: static_endpoint,
                    ..RevoluteJointDesc::default()
                }),
            },
        ])
        .expect("S5_HARNESS_BOUNDARY:rejected_transaction: positive transaction must succeed");
    let control_successful = untouched_control
        .commands()
        .apply([
            WorldCommand::CreateJoint {
                desc: JointDesc::WorldAnchor(WorldAnchorJointDesc {
                    body: dynamic_endpoint,
                    ..WorldAnchorJointDesc::default()
                }),
            },
            WorldCommand::CreateJoint {
                desc: JointDesc::Revolute(RevoluteJointDesc {
                    body_a: revolute_endpoint,
                    body_b: static_endpoint,
                    ..RevoluteJointDesc::default()
                }),
            },
        ])
        .expect("S5_HARNESS_BOUNDARY:rejected_transaction: untouched control create must succeed");
    assert_eq!(
        successful.joint_handles.len(),
        2,
        "S5_HARNESS_BOUNDARY:rejected_transaction: successful control must create two joints"
    );
    assert_eq!(
        control_successful.joint_handles.len(),
        2,
        "S5_HARNESS_BOUNDARY:rejected_transaction: untouched control must create two joints"
    );
    assert_eq!(
        successful.joint_handles, control_successful.joint_handles,
        "S5_HARNESS_BOUNDARY:rejected_transaction: rejected scratch consumed authoritative joint-handle state"
    );
    let actual_anchor = successful.joint_handles[0];
    let actual_revolute = successful.joint_handles[1];
    assert!(
        matches!(
            successful.events.as_slice(),
            [
                WorldCommandEvent::JointCreated { joint: first },
                WorldCommandEvent::JointCreated { joint: second }
            ] if *first == actual_anchor && *second == actual_revolute
        ) && matches!(
            control_successful.events.as_slice(),
            [
                WorldCommandEvent::JointCreated { joint: first },
                WorldCommandEvent::JointCreated { joint: second }
            ] if *first == actual_anchor && *second == actual_revolute
        ),
        "S5_HARNESS_BOUNDARY:rejected_transaction: successful reports must identify their authoritative handles"
    );
    assert_eq!(
        world.joints().collect::<Vec<_>>(),
        successful.joint_handles,
        "S5_HARNESS_BOUNDARY:rejected_transaction: actual handles must be the authoritative joints"
    );
    assert_eq!(
        untouched_control.joints().collect::<Vec<_>>(),
        control_successful.joint_handles,
        "S5_HARNESS_BOUNDARY:rejected_transaction: control handles must be the authoritative joints"
    );
    for handle in successful.joint_handles.iter().copied() {
        assert_eq!(
            world
                .joint(handle)
                .expect("S5_HARNESS_BOUNDARY:rejected_transaction: report handle must resolve")
                .desc(),
            untouched_control
                .joint(handle)
                .expect("S5_HARNESS_BOUNDARY:rejected_transaction: control handle must resolve")
                .desc(),
            "S5_HARNESS_BOUNDARY:rejected_transaction: successful descriptor diverged from untouched control"
        );
    }
    assert!(
        matches!(world.joint(actual_anchor).expect("anchor").desc(), JointDesc::WorldAnchor(desc) if desc.body == dynamic_endpoint)
            && matches!(world.joint(actual_revolute).expect("revolute").desc(), JointDesc::Revolute(desc) if desc.body_a == revolute_endpoint && desc.body_b == static_endpoint),
        "S5_HARNESS_BOUNDARY:rejected_transaction: authoritative descriptors must preserve WorldAnchor/Revolute commands"
    );

    let successful_events = s5_step_events(&mut world);
    assert!(
        !s5_has_sleep_transition(&successful_events, static_endpoint),
        "S5_HARNESS_BOUNDARY:rejected_transaction: failed scratch static endpoint must not transition"
    );
    println!(
        "S5_REJECTED_TRANSACTION_BOUNDARY:events_empty=true;handle_control={:?};descriptor_control=true",
        successful.joint_handles
    );
    println!("S5_WAKE_CASE:rejected_transaction");

    let world_anchor_observation = s5_observe_wake(
        &world,
        &successful_events,
        dynamic_endpoint,
        "rejected_transaction",
    );
    let revolute_observation = s5_observe_wake(
        &world,
        &successful_events,
        revolute_endpoint,
        "rejected_transaction",
    );
    println!(
        "S5_WAKE_OBSERVATION:rejected_transaction:WorldAnchor(awake={},user_patch={});Revolute(awake={},user_patch={})",
        world_anchor_observation.awake,
        world_anchor_observation.user_patch,
        revolute_observation.awake,
        revolute_observation.user_patch
    );
    assert!(
        world_anchor_observation.satisfied() && revolute_observation.satisfied(),
        "S5_EXPECTED_WAKE:rejected_transaction: successful WorldAnchor/Revolute transaction controls did not wake with UserPatch"
    );
}
