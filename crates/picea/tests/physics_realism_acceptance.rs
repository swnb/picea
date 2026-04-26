use picea::prelude::*;

const DT: f32 = 1.0 / 60.0;

fn fixed_step_config() -> StepConfig {
    StepConfig {
        dt: DT,
        ..StepConfig::default()
    }
}

fn no_gravity_world() -> World {
    World::new(WorldDesc {
        gravity: Vector::default(),
        ..WorldDesc::default()
    })
}

fn create_body(
    world: &mut World,
    body_type: BodyType,
    x: f32,
    y: f32,
    linear_velocity: Vector,
) -> BodyHandle {
    world
        .create_body(BodyDesc {
            body_type,
            pose: Pose::from_xy_angle(x, y, 0.0),
            linear_velocity,
            can_sleep: false,
            ..BodyDesc::default()
        })
        .expect("body should be created")
}

fn attach_shape(
    world: &mut World,
    body: BodyHandle,
    shape: SharedShape,
    material: Material,
) -> ColliderHandle {
    attach_shape_with_density(world, body, shape, 1.0, material)
}

fn attach_shape_with_density(
    world: &mut World,
    body: BodyHandle,
    shape: SharedShape,
    density: f32,
    material: Material,
) -> ColliderHandle {
    world
        .create_collider(
            body,
            ColliderDesc {
                shape,
                density,
                material,
                ..ColliderDesc::default()
            },
        )
        .expect("collider should be created")
}

fn step_world(world: &mut World, steps: usize) -> StepReport {
    let mut pipeline = SimulationPipeline::new(fixed_step_config());
    let mut report = StepReport::default();
    for _ in 0..steps {
        report = pipeline.step(world);
    }
    report
}

fn body_velocity(world: &World, body: BodyHandle) -> Vector {
    world
        .try_body(body)
        .expect("body should still exist")
        .linear_velocity()
}

fn body_position(world: &World, body: BodyHandle) -> Vector {
    world
        .try_body(body)
        .expect("body should still exist")
        .pose()
        .translation()
}

fn active_contact_events(report: &StepReport) -> Vec<ContactEvent> {
    report
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::ContactStarted(contact) | WorldEvent::ContactPersisted(contact) => {
                Some(*contact)
            }
            _ => None,
        })
        .collect()
}

#[test]
fn sat_polygon_manifold_reports_two_points_with_stable_features() {
    let mut world = no_gravity_world();
    let left = create_body(&mut world, BodyType::Static, 0.0, 0.0, Vector::default());
    let right = create_body(&mut world, BodyType::Static, 1.5, 0.0, Vector::default());
    attach_shape(
        &mut world,
        left,
        SharedShape::rect(2.0, 2.0),
        Material::default(),
    );
    attach_shape(
        &mut world,
        right,
        SharedShape::convex_polygon(vec![
            Point::new(-1.0, -1.0),
            Point::new(1.0, -1.0),
            Point::new(1.0, 1.0),
            Point::new(-1.0, 1.0),
        ]),
        Material::default(),
    );

    let first = step_world(&mut world, 1);
    let second = step_world(&mut world, 1);

    assert_eq!(first.stats.contact_count, 2);
    assert_eq!(first.stats.manifold_count, 1);
    let first_features = first
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::ContactStarted(contact) => Some(contact.feature_id),
            _ => None,
        })
        .collect::<Vec<_>>();
    let second_features = second
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::ContactPersisted(contact) => Some(contact.feature_id),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(first_features.len(), 2);
    assert_eq!(first_features, second_features);
    assert!(first.events.iter().all(|event| match event {
        WorldEvent::ContactStarted(contact) =>
            contact.normal == Vector::new(-1.0, 0.0) && (contact.depth - 0.5).abs() < 1.0e-4,
        _ => true,
    }));
}

#[test]
fn warm_start_cache_hits_continuing_contact_identity() {
    let mut world = no_gravity_world();
    let left = create_body(&mut world, BodyType::Static, 0.0, 0.0, Vector::default());
    let right = create_body(&mut world, BodyType::Static, 1.5, 0.0, Vector::default());
    attach_shape(
        &mut world,
        left,
        SharedShape::rect(2.0, 2.0),
        Material::default(),
    );
    attach_shape(
        &mut world,
        right,
        SharedShape::rect(2.0, 2.0),
        Material::default(),
    );
    let mut pipeline = SimulationPipeline::new(fixed_step_config());

    let first = pipeline.step(&mut world);
    let second = pipeline.step(&mut world);
    let first_contacts = active_contact_events(&first);
    let second_contacts = active_contact_events(&second);

    assert_eq!(first.stats.warm_start_miss_count, first_contacts.len());
    assert_eq!(first.stats.warm_start_hit_count, 0);
    assert_eq!(second.stats.warm_start_hit_count, second_contacts.len());
    assert_eq!(second.stats.warm_start_miss_count, 0);
    assert_eq!(second.stats.warm_start_drop_count, 0);
    assert_eq!(
        first_contacts
            .iter()
            .map(|contact| (contact.contact_id, contact.manifold_id, contact.feature_id))
            .collect::<Vec<_>>(),
        second_contacts
            .iter()
            .map(|contact| (contact.contact_id, contact.manifold_id, contact.feature_id))
            .collect::<Vec<_>>()
    );
    assert!(second_contacts
        .iter()
        .all(|contact| contact.warm_start_reason == WarmStartCacheReason::Hit));
}

#[test]
fn warm_start_cache_transfers_cached_impulse_after_trustworthy_match() {
    let mut world = no_gravity_world();
    let moving = create_body(
        &mut world,
        BodyType::Dynamic,
        -0.2,
        0.0,
        Vector::new(1.0, 0.0),
    );
    let wall = create_body(&mut world, BodyType::Static, 0.2, 0.0, Vector::default());
    let material = Material {
        friction: 0.5,
        restitution: 0.0,
    };
    attach_shape(&mut world, moving, SharedShape::circle(1.0), material);
    attach_shape(&mut world, wall, SharedShape::circle(1.0), material);
    let mut pipeline = SimulationPipeline::new(fixed_step_config());

    let first = pipeline.step(&mut world);
    world
        .apply_body_patch(
            moving,
            BodyPatch {
                pose: Some(Pose::from_xy_angle(-0.2, 0.0, 0.0)),
                linear_velocity: Some(Vector::default()),
                wake: true,
                ..BodyPatch::default()
            },
        )
        .expect("body patch should keep the next step on the same contact feature");
    let second = pipeline.step(&mut world);
    let first_contact = active_contact_events(&first)
        .into_iter()
        .next()
        .expect("first step should create one contact");
    let second_contact = active_contact_events(&second)
        .into_iter()
        .next()
        .expect("second step should persist one contact");

    assert_eq!(
        first_contact.warm_start_reason,
        WarmStartCacheReason::MissNoPrevious
    );
    assert_eq!(second_contact.warm_start_reason, WarmStartCacheReason::Hit);
    assert!(
        second_contact.warm_start_normal_impulse > 0.0,
        "the second step should transfer the first step's cached normal impulse: {second_contact:?}"
    );
    assert_eq!(second.stats.warm_start_hit_count, 1);
}

#[test]
fn warm_start_cache_drops_cached_impulse_when_normal_orientation_flips() {
    let mut world = no_gravity_world();
    let moving = create_body(
        &mut world,
        BodyType::Dynamic,
        -0.2,
        0.0,
        Vector::new(1.0, 0.0),
    );
    let anchor = create_body(&mut world, BodyType::Static, 0.2, 0.0, Vector::default());
    attach_shape(
        &mut world,
        moving,
        SharedShape::circle(1.0),
        Material::default(),
    );
    attach_shape(
        &mut world,
        anchor,
        SharedShape::circle(1.0),
        Material::default(),
    );
    let mut pipeline = SimulationPipeline::new(fixed_step_config());

    let first = pipeline.step(&mut world);
    assert_eq!(first.stats.warm_start_miss_count, 1);
    world
        .apply_body_patch(
            moving,
            BodyPatch {
                pose: Some(Pose::from_xy_angle(0.8, 0.0, 0.0)),
                linear_velocity: Some(Vector::default()),
                wake: true,
                ..BodyPatch::default()
            },
        )
        .expect("body patch should move the contact across the anchor");
    let second = pipeline.step(&mut world);
    let contact = active_contact_events(&second)
        .into_iter()
        .next()
        .expect("second step should keep the pair in contact");

    assert_eq!(
        contact.warm_start_reason,
        WarmStartCacheReason::DroppedNormalMismatch
    );
    assert_eq!(contact.warm_start_normal_impulse, 0.0);
    assert_eq!(contact.warm_start_tangent_impulse, 0.0);
    assert_eq!(second.stats.warm_start_drop_count, 1);
}

#[test]
fn warm_start_cache_does_not_survive_recontact_after_separation() {
    let mut world = no_gravity_world();
    let left = create_body(&mut world, BodyType::Static, 0.0, 0.0, Vector::default());
    let right = create_body(&mut world, BodyType::Static, 1.5, 0.0, Vector::default());
    attach_shape(
        &mut world,
        left,
        SharedShape::rect(2.0, 2.0),
        Material::default(),
    );
    attach_shape(
        &mut world,
        right,
        SharedShape::rect(2.0, 2.0),
        Material::default(),
    );
    let mut pipeline = SimulationPipeline::new(fixed_step_config());

    let first = pipeline.step(&mut world);
    world
        .apply_body_patch(
            right,
            BodyPatch {
                pose: Some(Pose::from_xy_angle(4.0, 0.0, 0.0)),
                wake: true,
                ..BodyPatch::default()
            },
        )
        .expect("body should move out of contact");
    let separated = pipeline.step(&mut world);
    world
        .apply_body_patch(
            right,
            BodyPatch {
                pose: Some(Pose::from_xy_angle(1.5, 0.0, 0.0)),
                wake: true,
                ..BodyPatch::default()
            },
        )
        .expect("body should move back into contact");
    let recontact = pipeline.step(&mut world);

    assert!(!active_contact_events(&first).is_empty());
    assert!(active_contact_events(&separated).is_empty());
    let recontact_contacts = active_contact_events(&recontact);
    assert!(!recontact_contacts.is_empty());
    assert!(recontact_contacts
        .iter()
        .all(|contact| contact.warm_start_reason == WarmStartCacheReason::MissNoPrevious));
    assert_eq!(recontact.stats.warm_start_hit_count, 0);
}

#[test]
fn warm_start_cache_uses_normalized_pair_identity_when_geometric_a_b_order_is_swapped() {
    let mut world = no_gravity_world();
    let right = create_body(&mut world, BodyType::Static, 1.5, 0.0, Vector::default());
    let left = create_body(&mut world, BodyType::Static, 0.0, 0.0, Vector::default());
    let right_collider = attach_shape(
        &mut world,
        right,
        SharedShape::rect(2.0, 2.0),
        Material::default(),
    );
    let left_collider = attach_shape(
        &mut world,
        left,
        SharedShape::rect(2.0, 2.0),
        Material::default(),
    );
    let mut pipeline = SimulationPipeline::new(fixed_step_config());

    let first = pipeline.step(&mut world);
    let second = pipeline.step(&mut world);
    let first_contacts = active_contact_events(&first);
    let second_contacts = active_contact_events(&second);

    assert!(!first_contacts.is_empty());
    assert_eq!(second.stats.warm_start_hit_count, second_contacts.len());
    assert!(second_contacts
        .iter()
        .all(|contact| contact.warm_start_reason == WarmStartCacheReason::Hit));
    assert!(second_contacts.iter().all(|contact| {
        contact.collider_a == right_collider.min(left_collider)
            && contact.collider_b == right_collider.max(left_collider)
    }));
}

#[test]
fn warm_start_cache_reports_feature_id_miss_when_pair_persists_on_different_features() {
    let mut world = no_gravity_world();
    let left = create_body(&mut world, BodyType::Static, 0.0, 0.0, Vector::default());
    let right = create_body(&mut world, BodyType::Static, 1.5, 0.0, Vector::default());
    attach_shape(
        &mut world,
        left,
        SharedShape::rect(2.0, 2.0),
        Material::default(),
    );
    let right_collider = attach_shape(
        &mut world,
        right,
        SharedShape::rect(2.0, 2.0),
        Material::default(),
    );
    let mut pipeline = SimulationPipeline::new(fixed_step_config());

    let first = pipeline.step(&mut world);
    world
        .apply_collider_patch(
            right_collider,
            ColliderPatch {
                shape: Some(SharedShape::circle(1.0)),
                ..ColliderPatch::default()
            },
        )
        .expect("collider should switch to a different feature family");
    let second = pipeline.step(&mut world);

    assert!(!active_contact_events(&first).is_empty());
    let reasons = active_contact_events(&second)
        .into_iter()
        .map(|contact| contact.warm_start_reason)
        .collect::<Vec<_>>();
    assert!(
        reasons.contains(&WarmStartCacheReason::MissFeatureId),
        "expected at least one point-level feature miss after clipped feature drift; reasons={reasons:?}"
    );
}

#[test]
fn warm_start_cache_drops_when_pair_anchor_relative_contact_point_drifts() {
    let mut world = no_gravity_world();
    let left = create_body(&mut world, BodyType::Static, 0.0, 0.0, Vector::default());
    let right = create_body(&mut world, BodyType::Static, 5.0, 0.0, Vector::default());
    attach_shape(
        &mut world,
        left,
        SharedShape::circle(10.0),
        Material::default(),
    );
    attach_shape(
        &mut world,
        right,
        SharedShape::circle(10.0),
        Material::default(),
    );
    let mut pipeline = SimulationPipeline::new(fixed_step_config());

    pipeline.step(&mut world);
    world
        .apply_body_patch(
            right,
            BodyPatch {
                pose: Some(Pose::from_xy_angle(5.0, 0.4, 0.0)),
                wake: true,
                ..BodyPatch::default()
            },
        )
        .expect("body should drift while staying in contact");
    let second = pipeline.step(&mut world);
    let contact = active_contact_events(&second)
        .into_iter()
        .next()
        .expect("pair should remain in contact after a small normal change");

    assert_eq!(
        contact.warm_start_reason,
        WarmStartCacheReason::DroppedPointDrift
    );
    assert_eq!(second.stats.warm_start_drop_count, 1);
}

#[test]
fn warm_start_cache_keeps_hit_when_touching_pair_translates_together() {
    let mut world = no_gravity_world();
    let left = create_body(&mut world, BodyType::Static, 0.0, 0.0, Vector::default());
    let right = create_body(&mut world, BodyType::Static, 1.5, 0.0, Vector::default());
    attach_shape(
        &mut world,
        left,
        SharedShape::rect(2.0, 2.0),
        Material::default(),
    );
    attach_shape(
        &mut world,
        right,
        SharedShape::rect(2.0, 2.0),
        Material::default(),
    );
    let mut pipeline = SimulationPipeline::new(fixed_step_config());

    pipeline.step(&mut world);
    world
        .apply_body_patch(
            left,
            BodyPatch {
                pose: Some(Pose::from_xy_angle(1.0, 0.0, 0.0)),
                wake: true,
                ..BodyPatch::default()
            },
        )
        .expect("left body should translate");
    world
        .apply_body_patch(
            right,
            BodyPatch {
                pose: Some(Pose::from_xy_angle(2.5, 0.0, 0.0)),
                wake: true,
                ..BodyPatch::default()
            },
        )
        .expect("right body should translate with the pair");
    let second = pipeline.step(&mut world);
    let contacts = active_contact_events(&second);

    assert!(!contacts.is_empty());
    assert!(contacts
        .iter()
        .all(|contact| contact.warm_start_reason == WarmStartCacheReason::Hit));
    assert_eq!(second.stats.warm_start_hit_count, contacts.len());
}

#[test]
fn warm_start_cache_does_not_hit_or_expose_stale_impulses_after_solid_contact_becomes_sensor() {
    let mut world = no_gravity_world();
    let moving = create_body(
        &mut world,
        BodyType::Dynamic,
        -0.2,
        0.0,
        Vector::new(1.0, 0.0),
    );
    let wall = create_body(&mut world, BodyType::Static, 0.2, 0.0, Vector::default());
    attach_shape(
        &mut world,
        moving,
        SharedShape::circle(1.0),
        Material::default(),
    );
    let wall_collider = attach_shape(
        &mut world,
        wall,
        SharedShape::circle(1.0),
        Material::default(),
    );
    let mut pipeline = SimulationPipeline::new(fixed_step_config());

    let first = pipeline.step(&mut world);
    let first_contact = active_contact_events(&first)
        .into_iter()
        .next()
        .expect("solid contact should exist");
    assert_eq!(
        first_contact.warm_start_reason,
        WarmStartCacheReason::MissNoPrevious
    );

    world
        .apply_collider_patch(
            wall_collider,
            ColliderPatch {
                is_sensor: Some(true),
                ..ColliderPatch::default()
            },
        )
        .expect("collider should become a sensor");
    world
        .apply_body_patch(
            moving,
            BodyPatch {
                pose: Some(Pose::from_xy_angle(-0.2, 0.0, 0.0)),
                linear_velocity: Some(Vector::default()),
                wake: true,
                ..BodyPatch::default()
            },
        )
        .expect("body should stay on the same contact feature");
    let second = pipeline.step(&mut world);
    let sensor_contact = active_contact_events(&second)
        .into_iter()
        .next()
        .expect("sensor contact should still be observable");

    assert_eq!(
        sensor_contact.warm_start_reason,
        WarmStartCacheReason::SkippedSensor
    );
    assert_eq!(sensor_contact.warm_start_normal_impulse, 0.0);
    assert_eq!(sensor_contact.warm_start_tangent_impulse, 0.0);
    assert_eq!(second.stats.warm_start_hit_count, 0);
}

#[test]
fn warm_start_cache_does_not_hit_after_sensor_contact_becomes_solid() {
    let mut world = no_gravity_world();
    let moving = create_body(
        &mut world,
        BodyType::Dynamic,
        -0.2,
        0.0,
        Vector::new(1.0, 0.0),
    );
    let wall = create_body(&mut world, BodyType::Static, 0.2, 0.0, Vector::default());
    attach_shape(
        &mut world,
        moving,
        SharedShape::circle(1.0),
        Material::default(),
    );
    let wall_collider = world
        .create_collider(
            wall,
            ColliderDesc {
                shape: SharedShape::circle(1.0),
                is_sensor: true,
                ..ColliderDesc::default()
            },
        )
        .expect("sensor collider should be created");
    let mut pipeline = SimulationPipeline::new(fixed_step_config());

    let first = pipeline.step(&mut world);
    let sensor_contact = active_contact_events(&first)
        .into_iter()
        .next()
        .expect("sensor contact should be observable");
    assert_eq!(
        sensor_contact.warm_start_reason,
        WarmStartCacheReason::SkippedSensor
    );

    world
        .apply_collider_patch(
            wall_collider,
            ColliderPatch {
                is_sensor: Some(false),
                ..ColliderPatch::default()
            },
        )
        .expect("collider should become solid");
    world
        .apply_body_patch(
            moving,
            BodyPatch {
                pose: Some(Pose::from_xy_angle(-0.2, 0.0, 0.0)),
                linear_velocity: Some(Vector::default()),
                wake: true,
                ..BodyPatch::default()
            },
        )
        .expect("body should stay on the same contact feature");
    let second = pipeline.step(&mut world);
    let solid_contact = active_contact_events(&second)
        .into_iter()
        .next()
        .expect("solid contact should continue from the sensor overlap");

    assert_ne!(solid_contact.warm_start_reason, WarmStartCacheReason::Hit);
    assert_eq!(
        solid_contact.warm_start_reason,
        WarmStartCacheReason::MissPreviousSensor
    );
    assert_eq!(solid_contact.warm_start_normal_impulse, 0.0);
    assert_eq!(solid_contact.warm_start_tangent_impulse, 0.0);
    assert_eq!(second.stats.warm_start_hit_count, 0);
    assert_eq!(second.stats.warm_start_miss_count, 1);
}

#[test]
fn circles_with_overlapping_aabbs_but_separated_geometry_do_not_contact() {
    // Physical behavior: broadphase AABB overlap should be followed by shape-level narrowing.
    // Two diagonal circles can have overlapping AABBs while their actual circle geometry is
    // separated, so the narrowphase must reject that broadphase-only false positive.
    let mut world = no_gravity_world();
    let first = create_body(&mut world, BodyType::Static, 0.0, 0.0, Vector::default());
    let second = create_body(&mut world, BodyType::Static, 1.5, 1.5, Vector::default());
    attach_shape(
        &mut world,
        first,
        SharedShape::circle(1.0),
        Material::default(),
    );
    attach_shape(
        &mut world,
        second,
        SharedShape::circle(1.0),
        Material::default(),
    );

    let report = step_world(&mut world, 1);

    assert_eq!(
        report.stats.contact_count, 0,
        "circle-circle narrowphase should reject diagonal AABB-only false positives"
    );
}

#[test]
fn restitution_changes_post_impact_bounce_velocity() {
    // Physical behavior: a dynamic body with restitution should rebound from static geometry.
    let mut world = no_gravity_world();
    let floor = create_body(&mut world, BodyType::Static, 0.0, 0.5, Vector::default());
    let ball = create_body(
        &mut world,
        BodyType::Dynamic,
        0.0,
        -2.0,
        Vector::new(0.0, 6.0),
    );
    attach_shape(
        &mut world,
        floor,
        SharedShape::rect(10.0, 1.0),
        Material {
            restitution: 1.0,
            friction: 0.0,
        },
    );
    attach_shape(
        &mut world,
        ball,
        SharedShape::circle(0.5),
        Material {
            restitution: 1.0,
            friction: 0.0,
        },
    );

    step_world(&mut world, 30);

    let velocity = body_velocity(&world, ball);
    assert!(
        velocity.y() < -3.0,
        "elastic impact should reverse vertical velocity; got {velocity:?}"
    );
}

#[test]
fn friction_changes_tangential_sliding_speed() {
    // Physical behavior: lower friction should preserve more tangential velocity than high
    // friction while sliding on the same surface.
    fn sliding_speed_after_contact(friction: f32) -> f32 {
        let mut world = no_gravity_world();
        let floor = create_body(&mut world, BodyType::Static, 0.0, 0.5, Vector::default());
        let slider = create_body(
            &mut world,
            BodyType::Dynamic,
            0.0,
            -0.45,
            Vector::new(4.0, 0.0),
        );
        let material = Material {
            friction,
            restitution: 0.0,
        };
        attach_shape(&mut world, floor, SharedShape::rect(10.0, 1.0), material);
        attach_shape(&mut world, slider, SharedShape::circle(0.5), material);

        step_world(&mut world, 10);
        body_velocity(&world, slider).x().abs()
    }

    let low_friction_speed = sliding_speed_after_contact(0.0);
    let high_friction_speed = sliding_speed_after_contact(1.0);

    assert!(
        low_friction_speed > high_friction_speed + 1.0,
        "low friction should retain more sliding speed; low={low_friction_speed}, high={high_friction_speed}"
    );
}

#[test]
fn collider_density_changes_interim_contact_velocity_response_through_inverse_mass() {
    fn post_contact_velocity(left_density: f32, right_density: f32) -> (Vector, Vector, f32, f32) {
        let mut world = no_gravity_world();
        let left = create_body(
            &mut world,
            BodyType::Dynamic,
            -0.4,
            0.0,
            Vector::new(1.0, 0.0),
        );
        let right = create_body(
            &mut world,
            BodyType::Dynamic,
            0.4,
            0.0,
            Vector::new(-1.0, 0.0),
        );
        let material = Material {
            friction: 0.0,
            restitution: 0.0,
        };
        attach_shape_with_density(
            &mut world,
            left,
            SharedShape::circle(0.5),
            left_density,
            material,
        );
        attach_shape_with_density(
            &mut world,
            right,
            SharedShape::circle(0.5),
            right_density,
            material,
        );

        step_world(&mut world, 1);

        let left_inverse_mass = world
            .body(left)
            .expect("left body should resolve")
            .mass_properties()
            .inverse_mass;
        let right_inverse_mass = world
            .body(right)
            .expect("right body should resolve")
            .mass_properties()
            .inverse_mass;
        (
            body_velocity(&world, left),
            body_velocity(&world, right),
            left_inverse_mass,
            right_inverse_mass,
        )
    }

    let (equal_left, equal_right, equal_left_inverse, equal_right_inverse) =
        post_contact_velocity(1.0, 1.0);
    let (light_left, heavy_right, light_inverse, heavy_inverse) = post_contact_velocity(1.0, 4.0);

    assert!(
        (equal_left_inverse - equal_right_inverse).abs() < 1.0e-5,
        "equal densities should produce equal inverse masses; left={equal_left_inverse}, right={equal_right_inverse}"
    );
    assert!(
        light_inverse > heavy_inverse * 3.9,
        "density-derived MassProperties should make the left body lighter; left inverse={light_inverse}, right inverse={heavy_inverse}"
    );
    assert!(
        equal_left.x().abs() < 0.1 && equal_right.x().abs() < 0.1,
        "equal-density inelastic contact should settle near zero velocity; left={equal_left:?}, right={equal_right:?}"
    );
    assert!(
        light_left.x() < -0.3 && heavy_right.x() < -0.3,
        "unequal density should move the interim response toward the heavier body's incoming velocity; left={light_left:?}, right={heavy_right:?}"
    );
}

#[test]
fn separating_overlap_does_not_apply_friction_impulse() {
    // Physical behavior: geometric overlap alone should not remove tangential velocity when the
    // bodies are already moving apart along the contact normal.
    let mut world = no_gravity_world();
    let floor = create_body(&mut world, BodyType::Static, 0.0, 0.5, Vector::default());
    let slider = create_body(
        &mut world,
        BodyType::Dynamic,
        0.0,
        -0.45,
        Vector::new(4.0, -1.0),
    );
    let material = Material {
        friction: 1.0,
        restitution: 0.0,
    };
    attach_shape(&mut world, floor, SharedShape::rect(10.0, 1.0), material);
    attach_shape(&mut world, slider, SharedShape::circle(0.5), material);

    step_world(&mut world, 1);

    let velocity = body_velocity(&world, slider);
    assert!(
        velocity.x() > 3.5,
        "separating contact should preserve tangential speed; got {velocity:?}"
    );
}

#[test]
fn contact_position_correction_preserves_spin_without_angular_solver() {
    // Physical behavior: until angular contact impulses are implemented, a zero-friction contact
    // should at least avoid deleting existing angular velocity as a side effect.
    let mut world = no_gravity_world();
    let floor = create_body(&mut world, BodyType::Static, 0.0, 0.5, Vector::default());
    let spinner = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(0.0, -0.45, 0.0),
            angular_velocity: 5.0,
            can_sleep: false,
            ..BodyDesc::default()
        })
        .expect("spinner should be created");
    let material = Material {
        friction: 0.0,
        restitution: 0.0,
    };
    attach_shape(&mut world, floor, SharedShape::rect(10.0, 1.0), material);
    attach_shape(&mut world, spinner, SharedShape::circle(0.5), material);

    step_world(&mut world, 1);

    let angular_velocity = world
        .try_body(spinner)
        .expect("spinner should still exist")
        .angular_velocity();
    assert!(
        (angular_velocity - 5.0).abs() < f32::EPSILON,
        "contact correction should not silently clear angular velocity; got {angular_velocity}"
    );
}

#[test]
fn sleep_requires_a_stability_window_before_a_body_sleeps() {
    // Physical behavior: sleeping should require sustained low motion over a stability window,
    // so bodies do not sleep after one quiet frame and miss near-future wake interactions.
    let mut world = no_gravity_world();
    let body = create_body(&mut world, BodyType::Dynamic, 0.0, 0.0, Vector::default());
    world
        .apply_body_patch(
            body,
            BodyPatch {
                can_sleep: Some(true),
                ..BodyPatch::default()
            },
        )
        .expect("body patch should apply");

    step_world(&mut world, 1);

    assert!(
        !world.try_body(body).expect("body should exist").sleeping(),
        "a single quiet step should not be enough to put a dynamic body to sleep"
    );

    step_world(&mut world, 31);

    assert!(
        world.try_body(body).expect("body should exist").sleeping(),
        "a body should sleep after remaining quiet for the stability window"
    );
}

#[test]
#[ignore = "known-red physics realism baseline: continuous collision detection is not implemented"]
fn fast_small_body_does_not_tunnel_through_thin_wall() {
    // Physical behavior: CCD should sweep fast bodies between poses and stop at the first time
    // of impact. Current implementation is expected to fail because collision is sampled only
    // after integration, so the circle tunnels completely through the thin wall.
    let mut world = no_gravity_world();
    let wall = create_body(&mut world, BodyType::Static, 0.0, 0.0, Vector::default());
    let bullet = create_body(
        &mut world,
        BodyType::Dynamic,
        -1.0,
        0.0,
        Vector::new(200.0, 0.0),
    );
    attach_shape(
        &mut world,
        wall,
        SharedShape::rect(0.1, 10.0),
        Material::default(),
    );
    attach_shape(
        &mut world,
        bullet,
        SharedShape::circle(0.05),
        Material::default(),
    );

    let report = step_world(&mut world, 1);
    let position = body_position(&world, bullet);

    assert!(
        report.stats.contact_count > 0,
        "CCD should report the swept contact with the wall"
    );
    assert!(
        position.x() <= -0.05,
        "CCD should keep the bullet on the pre-impact side of the wall; x={}",
        position.x()
    );
}
