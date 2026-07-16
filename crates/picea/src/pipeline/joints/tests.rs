use std::collections::BTreeMap;

use crate::{
    body::{BodyDesc, BodyPatch, BodyType, Pose},
    events::SleepTransitionReason,
    math::{point::Point, vector::Vector},
    world::World,
};

#[test]
fn revolute_point_constraint_inverts_off_diagonal_effective_mass() {
    let inverse = super::invert_revolute_effective_mass(4.0, 1.0, 3.0)
        .expect("a finite positive-definite 2x2 matrix must be invertible");

    assert!((inverse[0][0] - 3.0 / 11.0).abs() <= 1.0e-6);
    assert!((inverse[0][1] + 1.0 / 11.0).abs() <= 1.0e-6);
    assert!((inverse[1][0] + 1.0 / 11.0).abs() <= 1.0e-6);
    assert!((inverse[1][1] - 4.0 / 11.0).abs() <= 1.0e-6);
}

#[test]
fn revolute_point_constraint_uses_clockwise_positive_cross_sign() {
    let lambda = Vector::new(0.0, 2.0);
    let r_a = Vector::new(1.0, 0.0);
    let r_b = Vector::new(-1.0, 0.0);
    let deltas = super::revolute_position_deltas(1.0, 0.5, r_a, 2.0, 0.25, r_b, lambda)
        .expect("finite point-constraint deltas should be produced atomically");

    assert_eq!(deltas.translation_a, Vector::new(0.0, -2.0));
    assert_eq!(deltas.translation_b, Vector::new(0.0, 4.0));
    assert_eq!(deltas.angle_a, 1.0);
    assert_eq!(deltas.angle_b, 0.5);
}

#[test]
fn revolute_point_constraint_static_static_and_singular_rows_fail_closed() {
    assert_eq!(super::invert_revolute_effective_mass(0.0, 0.0, 0.0), None);
    assert_eq!(super::invert_revolute_effective_mass(1.0, 1.0, 1.0), None);
}

#[test]
fn revolute_point_constraint_nonfinite_rows_fail_closed() {
    for matrix in [
        (f32::NAN, 0.0, 1.0),
        (1.0, f32::INFINITY, 1.0),
        (1.0, 0.0, f32::NEG_INFINITY),
    ] {
        assert_eq!(
            super::invert_revolute_effective_mass(matrix.0, matrix.1, matrix.2),
            None
        );
    }
}

#[test]
fn revolute_point_constraint_rebuilds_origin_from_corrected_world_com() {
    let eval_pose = Pose::from_xy_angle(3.0, -2.0, 0.4);
    let local_center_of_mass = Point::new(0.75, -0.25);
    let corrected_world_com = Point::new(4.0, 1.5);
    let corrected_angle = 1.1;
    let sampled_translation_advance = Vector::new(0.2, -0.1);
    let sampled_angle_advance = 0.3;

    let current = super::rebuild_revolute_current_pose_from_world_com(
        eval_pose,
        local_center_of_mass,
        corrected_world_com,
        corrected_angle,
        sampled_translation_advance,
        sampled_angle_advance,
    )
    .expect("finite COM reconstruction should produce a finite current pose");
    let final_eval = Pose::from_xy_angle(
        current.translation().x() + sampled_translation_advance.x(),
        current.translation().y() + sampled_translation_advance.y(),
        current.angle() + sampled_angle_advance,
    );

    assert!((final_eval.angle() - corrected_angle).abs() <= 1.0e-6);
    assert!(
        (final_eval.transform_point(local_center_of_mass) - corrected_world_com).length() <= 1.0e-6
    );
}

#[test]
fn revolute_point_constraint_atomic_apply_rejects_partial_nonfinite_update() {
    let mut world = World::default();
    let body_a = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(-1.0, 0.0, 0.0),
            ..BodyDesc::default()
        })
        .expect("body A should be created");
    let body_b = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(1.0, 0.0, 0.0),
            ..BodyDesc::default()
        })
        .expect("body B should be created");
    let before_a = world.try_body(body_a).expect("body A should exist").pose();
    let before_b = world.try_body(body_b).expect("body B should exist").pose();
    let mut wake_reasons = BTreeMap::new();

    let applied = super::apply_revolute_pose_pair_atomically(
        &mut world,
        body_a,
        Pose::from_xy_angle(-0.5, 0.0, 0.2),
        body_b,
        Pose::from_xy_angle(f32::NAN, 0.0, -0.2),
        &mut wake_reasons,
    );

    assert!(!applied);
    assert_eq!(
        world.try_body(body_a).expect("body A should exist").pose(),
        before_a
    );
    assert_eq!(
        world.try_body(body_b).expect("body B should exist").pose(),
        before_b
    );
    assert!(wake_reasons.is_empty());
}

#[test]
fn revolute_point_constraint_zero_delta_does_not_wake_sleeping_endpoint() {
    let mut world = World::default();
    let sleeping = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            sleeping: true,
            ..BodyDesc::default()
        })
        .expect("sleeping endpoint should be created");
    let awake = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            ..BodyDesc::default()
        })
        .expect("awake endpoint should be created");
    let mut wake_reasons = BTreeMap::new();
    let sleeping_pose = world
        .try_body(sleeping)
        .expect("sleeping endpoint should exist")
        .pose();
    let awake_pose = world
        .try_body(awake)
        .expect("awake endpoint should exist")
        .pose();

    let applied = super::apply_revolute_pose_pair_atomically(
        &mut world,
        sleeping,
        sleeping_pose,
        awake,
        awake_pose,
        &mut wake_reasons,
    );

    assert!(applied);
    assert!(world
        .try_body(sleeping)
        .expect("sleeping endpoint should exist")
        .sleeping());
    assert!(!wake_reasons.contains_key(&sleeping));
    assert!(!wake_reasons
        .values()
        .any(|reason| { *reason == SleepTransitionReason::JointCorrection }));
}

#[test]
fn revolute_point_constraint_sub_epsilon_pose_change_wakes_sleeping_endpoint() {
    let tiny = f32::from_bits(1);
    assert!(tiny > 0.0 && tiny < f32::EPSILON);

    let mut world = World::default();
    let sleeping = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            sleeping: true,
            ..BodyDesc::default()
        })
        .expect("sleeping endpoint should be created");
    let awake = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            ..BodyDesc::default()
        })
        .expect("awake endpoint should be created");
    let awake_pose = world
        .try_body(awake)
        .expect("awake endpoint should exist")
        .pose();
    let mut wake_reasons = BTreeMap::new();

    let applied = super::apply_revolute_pose_pair_atomically(
        &mut world,
        sleeping,
        Pose::from_xy_angle(tiny, 0.0, 0.0),
        awake,
        awake_pose,
        &mut wake_reasons,
    );

    assert!(applied);
    let sleeping_view = world
        .try_body(sleeping)
        .expect("sleeping endpoint should exist");
    assert_eq!(
        sleeping_view.pose().translation().x().to_bits(),
        tiny.to_bits()
    );
    assert!(!sleeping_view.sleeping());
    assert_eq!(
        wake_reasons.get(&sleeping),
        Some(&SleepTransitionReason::JointCorrection)
    );
}

#[test]
fn revolute_point_constraint_sub_epsilon_velocity_wakes_only_for_actual_change() {
    let tiny = f32::from_bits(1);
    assert!(tiny > 0.0 && tiny < f32::EPSILON);

    let mut changed_world = World::default();
    let changed_sleeping = changed_world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            sleeping: true,
            ..BodyDesc::default()
        })
        .expect("sleeping endpoint should be created");
    let changed_peer = changed_world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            ..BodyDesc::default()
        })
        .expect("peer endpoint should be created");
    let mut changed_wake_reasons = BTreeMap::new();

    let applied = crate::solver::body_state::apply_revolute_velocity_pair_atomically(
        &mut changed_world,
        changed_sleeping,
        Vector::new(tiny, 0.0),
        0.0,
        changed_peer,
        Vector::default(),
        0.0,
        &mut changed_wake_reasons,
    );

    assert!(applied);
    let changed_view = changed_world
        .try_body(changed_sleeping)
        .expect("sleeping endpoint should exist");
    assert_eq!(changed_view.linear_velocity().x().to_bits(), tiny.to_bits());
    assert!(!changed_view.sleeping());
    assert_eq!(
        changed_wake_reasons.get(&changed_sleeping),
        Some(&SleepTransitionReason::JointCorrection)
    );

    let mut rounded_world = World::default();
    let rounded_sleeping = rounded_world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            linear_velocity: Vector::new(1.0, 0.0),
            sleeping: true,
            ..BodyDesc::default()
        })
        .expect("sleeping endpoint should be created");
    let rounded_peer = rounded_world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            ..BodyDesc::default()
        })
        .expect("peer endpoint should be created");
    let rounded_delta = f32::EPSILON * 0.25;
    assert_eq!(1.0 + rounded_delta, 1.0);
    let mut rounded_wake_reasons = BTreeMap::new();

    let applied = crate::solver::body_state::apply_revolute_velocity_pair_atomically(
        &mut rounded_world,
        rounded_sleeping,
        Vector::new(rounded_delta, 0.0),
        0.0,
        rounded_peer,
        Vector::default(),
        0.0,
        &mut rounded_wake_reasons,
    );

    assert!(applied);
    let rounded_view = rounded_world
        .try_body(rounded_sleeping)
        .expect("sleeping endpoint should exist");
    assert_eq!(rounded_view.linear_velocity(), Vector::new(1.0, 0.0));
    assert!(rounded_view.sleeping());
    assert!(!rounded_wake_reasons.contains_key(&rounded_sleeping));
}

#[test]
fn revolute_point_constraint_sub_epsilon_post_demand_is_nonzero() {
    let tiny = f32::from_bits(1);
    assert!(tiny > 0.0 && tiny < f32::EPSILON);

    assert!(super::correction_is_nonzero(Vector::new(tiny, 0.0), 0.0));
    assert!(super::correction_is_nonzero(Vector::default(), tiny));
    assert!(!super::correction_is_nonzero(Vector::default(), 0.0));
    assert!(!super::correction_is_nonzero(
        Vector::new(f32::NAN, 0.0),
        0.0
    ));
}

#[test]
fn revolute_point_constraint_test_fixture_can_restore_sleep_without_user_wake() {
    let mut world = World::default();
    let body = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            ..BodyDesc::default()
        })
        .expect("body should be created");
    world
        .apply_body_patch(
            body,
            BodyPatch {
                sleeping: Some(true),
                ..BodyPatch::default()
            },
        )
        .expect("test fixture should be able to mark an awake body sleeping");
    assert!(world.try_body(body).expect("body should exist").sleeping());
}
