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
    step_world_with_config(world, fixed_step_config(), steps)
}

fn step_world_with_config(world: &mut World, config: StepConfig, steps: usize) -> StepReport {
    let mut pipeline = SimulationPipeline::new(config);
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

fn body_angular_velocity(world: &World, body: BodyHandle) -> f32 {
    world
        .try_body(body)
        .expect("body should still exist")
        .angular_velocity()
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
fn generic_convex_segment_rectangle_contact_reports_gjk_epa_trace() {
    let mut world = no_gravity_world();
    let segment_body = create_body(&mut world, BodyType::Static, 0.0, 0.0, Vector::default());
    let rect_body = create_body(&mut world, BodyType::Static, 0.25, 0.0, Vector::default());
    attach_shape(
        &mut world,
        segment_body,
        SharedShape::segment(Point::new(-1.0, 0.0), Point::new(1.0, 0.0)),
        Material::default(),
    );
    attach_shape(
        &mut world,
        rect_body,
        SharedShape::rect(1.0, 1.0),
        Material::default(),
    );

    let report = step_world(&mut world, 1);
    let contact = active_contact_events(&report)
        .into_iter()
        .next()
        .expect("generic convex fallback should produce a contact");
    let trace = contact
        .generic_convex_trace
        .expect("generic fallback contact should carry trace facts");

    assert_eq!(
        contact.reduction_reason,
        ContactReductionReason::GenericConvexFallback
    );
    assert_eq!(
        trace.fallback_reason,
        GenericConvexFallbackReason::GenericConvexFallback
    );
    assert_eq!(trace.gjk_termination, GjkTerminationReason::Intersect);
    assert_eq!(trace.epa_termination, EpaTerminationReason::Converged);
    assert!(trace.gjk_iterations > 0);
    assert!(trace.simplex_len > 0);
}

#[test]
fn generic_convex_segment_rectangle_identity_is_stable_across_steps() {
    let mut world = no_gravity_world();
    let segment_body = create_body(&mut world, BodyType::Static, 0.0, 0.0, Vector::default());
    let rect_body = create_body(&mut world, BodyType::Static, 0.25, 0.0, Vector::default());
    attach_shape(
        &mut world,
        segment_body,
        SharedShape::segment(Point::new(-1.0, 0.0), Point::new(1.0, 0.0)),
        Material::default(),
    );
    attach_shape(
        &mut world,
        rect_body,
        SharedShape::rect(1.0, 1.0),
        Material::default(),
    );
    let mut pipeline = SimulationPipeline::new(fixed_step_config());

    let first = pipeline.step(&mut world);
    let second = pipeline.step(&mut world);
    let first_contacts = active_contact_events(&first);
    let second_contacts = active_contact_events(&second);

    assert_eq!(first_contacts.len(), 1);
    assert_eq!(second_contacts.len(), 1);
    assert_eq!(
        (
            first_contacts[0].contact_id,
            first_contacts[0].manifold_id,
            first_contacts[0].feature_id,
        ),
        (
            second_contacts[0].contact_id,
            second_contacts[0].manifold_id,
            second_contacts[0].feature_id,
        )
    );
    assert_eq!(
        second_contacts[0].warm_start_reason,
        WarmStartCacheReason::Hit
    );
    assert_eq!(second.stats.warm_start_hit_count, 1);
    assert_eq!(second.stats.warm_start_miss_count, 0);
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
fn solver_impulse_facts_zero_when_warm_start_hit_has_no_solvable_row() {
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
    attach_shape(
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
        .expect("first step should solve one dynamic contact");
    assert!(
        first_contact.solver_normal_impulse > 0.0,
        "first contact should solve and cache a normal impulse: {first_contact:?}"
    );
    world
        .apply_body_patch(
            moving,
            BodyPatch {
                body_type: Some(BodyType::Static),
                pose: Some(Pose::from_xy_angle(-0.2, 0.0, 0.0)),
                wake: true,
                ..BodyPatch::default()
            },
        )
        .expect("body should become static while preserving the contact feature");

    let second = pipeline.step(&mut world);
    let second_contact = active_contact_events(&second)
        .into_iter()
        .next()
        .expect("second step should still report the static contact");

    assert_eq!(second_contact.warm_start_reason, WarmStartCacheReason::Hit);
    assert!(
        second_contact.warm_start_normal_impulse > 0.0,
        "warm-start facts should still describe the transferred cache: {second_contact:?}"
    );
    assert_eq!(second_contact.solver_normal_impulse, 0.0);
    assert_eq!(second_contact.solver_tangent_impulse, 0.0);
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
fn sequential_impulse_solves_all_manifold_rows_for_stacked_contact() {
    let mut world = World::new(WorldDesc {
        gravity: Vector::default(),
        enable_sleep: false,
    });
    let floor = create_body(&mut world, BodyType::Static, 0.0, 0.5, Vector::default());
    let box_body = create_body(
        &mut world,
        BodyType::Dynamic,
        0.0,
        -0.45,
        Vector::new(0.0, 3.0),
    );
    let material = Material {
        friction: 0.4,
        restitution: 0.0,
    };
    attach_shape(&mut world, floor, SharedShape::rect(10.0, 1.0), material);
    attach_shape(&mut world, box_body, SharedShape::rect(1.0, 1.0), material);

    let report = step_world_with_config(
        &mut world,
        StepConfig {
            velocity_iterations: 12,
            position_iterations: 6,
            ..fixed_step_config()
        },
        1,
    );
    let contacts = active_contact_events(&report);

    assert!(
        contacts.len() >= 2,
        "face contact should expose multiple manifold rows: {contacts:?}"
    );
    assert!(
        contacts
            .iter()
            .all(|contact| contact.solver_normal_impulse >= 0.0),
        "normal impulses must be non-negative on every contact row: {contacts:?}"
    );
    assert!(
        contacts
            .iter()
            .filter(|contact| contact.solver_normal_impulse > 1.0e-4)
            .count()
            >= 2,
        "M5 must solve all non-sensor rows instead of only the deepest row per pair: {contacts:?}"
    );
    assert!(
        body_velocity(&world, box_body).y() <= 0.25,
        "velocity iterations should remove most closing speed; velocity={:?}",
        body_velocity(&world, box_body)
    );
}

#[test]
fn tangent_impulse_is_clamped_by_coulomb_friction_budget() {
    let mut world = World::new(WorldDesc {
        gravity: Vector::default(),
        enable_sleep: false,
    });
    let floor = create_body(&mut world, BodyType::Static, 0.0, 0.5, Vector::default());
    let slider = create_body(
        &mut world,
        BodyType::Dynamic,
        0.0,
        -0.45,
        Vector::new(12.0, 3.0),
    );
    let material = Material {
        friction: 0.25,
        restitution: 0.0,
    };
    attach_shape(&mut world, floor, SharedShape::rect(10.0, 1.0), material);
    attach_shape(&mut world, slider, SharedShape::circle(0.5), material);

    let report = step_world_with_config(
        &mut world,
        StepConfig {
            velocity_iterations: 10,
            position_iterations: 4,
            ..fixed_step_config()
        },
        1,
    );
    let contact = active_contact_events(&report)
        .into_iter()
        .max_by(|a, b| {
            a.solver_tangent_impulse
                .abs()
                .partial_cmp(&b.solver_tangent_impulse.abs())
                .unwrap()
        })
        .expect("slider should contact the floor");
    let friction_budget = material.friction * contact.solver_normal_impulse;

    assert!(
        contact.solver_normal_impulse > 0.0,
        "normal impulse should create a Coulomb friction budget: {contact:?}"
    );
    assert!(
        contact.solver_tangent_impulse.abs() <= friction_budget + 1.0e-4,
        "tangent impulse must stay inside +/-mu * normal impulse; budget={friction_budget}, contact={contact:?}"
    );
    assert!(
        contact.tangent_impulse_clamped,
        "large tangential speed should hit the Coulomb clamp: {contact:?}"
    );
}

#[test]
fn restitution_uses_configurable_velocity_threshold() {
    fn impact(speed: f32, threshold: f32) -> (Vector, ContactEvent) {
        let mut world = World::new(WorldDesc {
            gravity: Vector::default(),
            enable_sleep: false,
        });
        let floor = create_body(&mut world, BodyType::Static, 0.0, 0.5, Vector::default());
        let ball = create_body(
            &mut world,
            BodyType::Dynamic,
            0.0,
            -0.45,
            Vector::new(0.0, speed),
        );
        let material = Material {
            friction: 0.0,
            restitution: 1.0,
        };
        attach_shape(&mut world, floor, SharedShape::rect(10.0, 1.0), material);
        attach_shape(&mut world, ball, SharedShape::circle(0.5), material);

        let report = step_world_with_config(
            &mut world,
            StepConfig {
                restitution_velocity_threshold: threshold,
                velocity_iterations: 8,
                position_iterations: 3,
                ..fixed_step_config()
            },
            1,
        );
        let contact = active_contact_events(&report)
            .into_iter()
            .next()
            .expect("impact should emit contact facts");
        (body_velocity(&world, ball), contact)
    }

    let (slow_velocity, slow_contact) = impact(1.0, 2.0);
    let (fast_velocity, fast_contact) = impact(4.0, 2.0);

    assert!(
        !slow_contact.restitution_applied,
        "low-speed impact should be below the configured bounce threshold: {slow_contact:?}"
    );
    assert_eq!(slow_contact.restitution_velocity_threshold, 2.0);
    assert!(
        slow_velocity.y() >= -0.1,
        "low-speed contact should not bounce upward; velocity={slow_velocity:?}"
    );
    assert!(
        fast_contact.restitution_applied,
        "fast impact should apply restitution above the configured threshold: {fast_contact:?}"
    );
    assert_eq!(fast_contact.restitution_velocity_threshold, 2.0);
    assert!(
        fast_velocity.y() < -2.5,
        "elastic impact should reverse enough speed above threshold; velocity={fast_velocity:?}"
    );
}

#[test]
fn off_center_contact_produces_angular_velocity() {
    let mut world = World::new(WorldDesc {
        gravity: Vector::default(),
        enable_sleep: false,
    });
    let obstacle = create_body(&mut world, BodyType::Static, 0.0, 0.0, Vector::default());
    let striking_box = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(-0.8, -0.35, 0.0),
            linear_velocity: Vector::new(4.0, 0.0),
            can_sleep: false,
            ..BodyDesc::default()
        })
        .expect("striking box should be created");
    let material = Material {
        friction: 0.0,
        restitution: 0.0,
    };
    attach_shape(&mut world, obstacle, SharedShape::circle(0.5), material);
    attach_shape(
        &mut world,
        striking_box,
        SharedShape::rect(1.0, 1.0),
        material,
    );

    let report = step_world_with_config(
        &mut world,
        StepConfig {
            velocity_iterations: 10,
            position_iterations: 4,
            ..fixed_step_config()
        },
        1,
    );

    assert!(
        active_contact_events(&report)
            .iter()
            .any(|contact| contact.solver_normal_impulse > 0.0),
        "off-center impact should solve a normal impulse: {:?}",
        active_contact_events(&report)
    );
    assert!(
        body_angular_velocity(&world, striking_box) < -0.1,
        "off-center rightward impact above the body's center should produce clockwise spin; angular_velocity={}",
        body_angular_velocity(&world, striking_box)
    );
}

#[test]
fn collider_density_changes_sequential_impulse_response_through_inverse_mass() {
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
fn contact_position_correction_preserves_spin_after_velocity_solve() {
    // Physical behavior: residual position correction must not delete angular velocity written
    // by the velocity solver or authored on the body before the step.
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
fn jointed_island_sleeps_together_with_island_reason_facts() {
    let mut world = no_gravity_world();
    let left = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(-0.5, 0.0, 0.0),
            can_sleep: true,
            ..BodyDesc::default()
        })
        .expect("left body should be created");
    let right = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(0.5, 0.0, 0.0),
            can_sleep: true,
            ..BodyDesc::default()
        })
        .expect("right body should be created");
    world
        .create_joint(JointDesc::Distance(DistanceJointDesc {
            body_a: left,
            body_b: right,
            rest_length: 1.0,
            ..DistanceJointDesc::default()
        }))
        .expect("resting distance joint should be created");

    let mut pipeline = SimulationPipeline::new(fixed_step_config());
    let mut reports = Vec::new();
    for _ in 0..31 {
        reports.push(pipeline.step(&mut world));
    }
    let sleep_events = reports
        .iter()
        .flat_map(|report| report.events.iter())
        .filter_map(|event| match event {
            WorldEvent::SleepChanged(event) => Some(event),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(sleep_events.len(), 2);
    assert!(sleep_events.iter().all(|event| event.is_sleeping));
    assert!(sleep_events
        .iter()
        .all(|event| event.reason == picea::events::SleepTransitionReason::StabilityWindow));
    assert_eq!(sleep_events[0].island_id, sleep_events[1].island_id);

    let snapshot = world.debug_snapshot(&DebugSnapshotOptions::default());
    let left_debug = snapshot
        .bodies
        .iter()
        .find(|body| body.handle == left)
        .expect("left body should be in debug snapshot");
    let right_debug = snapshot
        .bodies
        .iter()
        .find(|body| body.handle == right)
        .expect("right body should be in debug snapshot");
    assert_eq!(left_debug.island_id, right_debug.island_id);
    let island_id = left_debug
        .island_id
        .expect("dynamic bodies should have an island");
    let island = snapshot
        .islands
        .iter()
        .find(|island| island.id == island_id)
        .expect("debug snapshot should expose the sleeping island");
    assert!(island.sleeping);
    assert_eq!(island.bodies, vec![left, right]);
}

#[test]
fn transform_patch_wakes_the_touched_sleeping_island_only() {
    let mut world = no_gravity_world();
    let first = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(-2.0, 0.0, 0.0),
            can_sleep: true,
            ..BodyDesc::default()
        })
        .expect("first body should be created");
    let partner = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(-1.0, 0.0, 0.0),
            can_sleep: true,
            ..BodyDesc::default()
        })
        .expect("partner body should be created");
    let unrelated = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(2.0, 0.0, 0.0),
            can_sleep: true,
            ..BodyDesc::default()
        })
        .expect("unrelated body should be created");
    world
        .create_joint(JointDesc::Distance(DistanceJointDesc {
            body_a: first,
            body_b: partner,
            rest_length: 1.0,
            ..DistanceJointDesc::default()
        }))
        .expect("joint should connect the first island");

    step_world(&mut world, 31);
    assert!(world.try_body(first).expect("first exists").sleeping());
    assert!(world.try_body(partner).expect("partner exists").sleeping());
    assert!(world
        .try_body(unrelated)
        .expect("unrelated exists")
        .sleeping());

    world
        .apply_body_patch(
            first,
            BodyPatch {
                pose: Some(Pose::from_xy_angle(-1.5, 0.0, 0.0)),
                ..BodyPatch::default()
            },
        )
        .expect("pose patch should apply");

    let report = step_world(&mut world, 1);
    let wake_events = report
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::SleepChanged(event) if !event.is_sleeping => Some(event),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(wake_events.len(), 2);
    assert!(wake_events.iter().any(|event| event.body == first));
    assert!(wake_events.iter().any(|event| event.body == partner));
    assert!(wake_events
        .iter()
        .all(|event| event.reason == picea::events::SleepTransitionReason::TransformEdit));
    assert_eq!(wake_events[0].island_id, wake_events[1].island_id);
    assert!(!world.try_body(first).expect("first exists").sleeping());
    assert!(!world.try_body(partner).expect("partner exists").sleeping());
    assert!(
        world
            .try_body(unrelated)
            .expect("unrelated exists")
            .sleeping(),
        "an unrelated sleeping island should stay asleep"
    );
}

#[test]
fn static_contacts_do_not_bridge_dynamic_sleep_islands() {
    let mut world = no_gravity_world();
    let floor = create_body(&mut world, BodyType::Static, 0.0, 0.0, Vector::default());
    let left = create_body(&mut world, BodyType::Dynamic, -2.0, 0.0, Vector::default());
    let right = create_body(&mut world, BodyType::Dynamic, 2.0, 0.0, Vector::default());
    attach_shape(
        &mut world,
        floor,
        SharedShape::rect(8.0, 1.0),
        Material::default(),
    );
    attach_shape(
        &mut world,
        left,
        SharedShape::rect(0.5, 0.5),
        Material::default(),
    );
    attach_shape(
        &mut world,
        right,
        SharedShape::rect(0.5, 0.5),
        Material::default(),
    );

    step_world(&mut world, 1);
    let snapshot = world.debug_snapshot(&DebugSnapshotOptions::default());
    let left_island = snapshot
        .bodies
        .iter()
        .find(|body| body.handle == left)
        .and_then(|body| body.island_id)
        .expect("left dynamic body should have an island");
    let right_island = snapshot
        .bodies
        .iter()
        .find(|body| body.handle == right)
        .and_then(|body| body.island_id)
        .expect("right dynamic body should have an island");

    assert_ne!(
        left_island, right_island,
        "one static body must not bridge otherwise unrelated dynamic islands"
    );
}

#[test]
fn sleeping_body_wakes_on_contact_solver_impact() {
    let mut world = no_gravity_world();
    let target = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(0.0, 0.0, 0.0),
            can_sleep: true,
            ..BodyDesc::default()
        })
        .expect("target should be created");
    let partner = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(0.0, 2.0, 0.0),
            can_sleep: true,
            ..BodyDesc::default()
        })
        .expect("partner should be created");
    world
        .create_joint(JointDesc::Distance(DistanceJointDesc {
            body_a: target,
            body_b: partner,
            rest_length: 2.0,
            ..DistanceJointDesc::default()
        }))
        .expect("joint should connect target and partner");
    attach_shape(
        &mut world,
        target,
        SharedShape::circle(0.5),
        Material::default(),
    );

    step_world(&mut world, 31);
    assert!(world.try_body(target).expect("target exists").sleeping());
    assert!(world.try_body(partner).expect("partner exists").sleeping());

    let bullet = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(-1.0, 0.0, 0.0),
            linear_velocity: Vector::new(10.0, 0.0),
            can_sleep: false,
            ..BodyDesc::default()
        })
        .expect("bullet should be created");
    attach_shape(
        &mut world,
        bullet,
        SharedShape::circle(0.5),
        Material::default(),
    );

    let report = step_world(&mut world, 1);
    let first_contact_index = report
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                WorldEvent::ContactStarted(_) | WorldEvent::ContactPersisted(_)
            )
        })
        .expect("impact should emit contact facts");
    let first_wake_index = report
        .events
        .iter()
        .position(|event| matches!(event, WorldEvent::SleepChanged(event) if !event.is_sleeping))
        .expect("impact should wake the sleeping island");
    assert!(
        first_contact_index < first_wake_index,
        "contact facts should precede the resulting island wake events"
    );
    let wake_events = report
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::SleepChanged(event) if !event.is_sleeping => Some(event),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(wake_events.len(), 2);
    assert!(wake_events.iter().any(|event| event.body == target));
    assert!(wake_events.iter().any(|event| event.body == partner));
    assert!(wake_events
        .iter()
        .all(|event| event.reason == picea::events::SleepTransitionReason::Impact));
    assert_eq!(wake_events[0].island_id, wake_events[1].island_id);
}

#[test]
fn sleeping_body_resting_on_static_contact_stays_asleep() {
    let mut world = no_gravity_world();
    let target = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(0.0, 0.0, 0.0),
            can_sleep: true,
            sleeping: true,
            ..BodyDesc::default()
        })
        .expect("sleeping target should be created");
    let support = create_body(&mut world, BodyType::Static, 0.75, 0.0, Vector::default());
    attach_shape(
        &mut world,
        target,
        SharedShape::circle(0.5),
        Material::default(),
    );
    attach_shape(
        &mut world,
        support,
        SharedShape::circle(0.5),
        Material::default(),
    );

    let report = step_world(&mut world, 1);

    assert!(
        world.try_body(target).expect("target exists").sleeping(),
        "static support contact should not be classified as an impact wake"
    );
    assert!(
        !report.events.iter().any(|event| matches!(
            event,
            WorldEvent::SleepChanged(event) if event.body == target && !event.is_sleeping
        )),
        "resting static contact must not emit a wake transition"
    );
}

#[test]
fn fast_small_body_does_not_tunnel_through_thin_wall() {
    assert_fast_small_body_does_not_tunnel_through_thin_wall();
}

#[test]
fn ccd_fast_small_body_does_not_tunnel_through_thin_wall() {
    assert_fast_small_body_does_not_tunnel_through_thin_wall();
}

fn assert_fast_small_body_does_not_tunnel_through_thin_wall() {
    // Physical behavior: CCD should sweep fast bodies between poses and stop at the first time
    // of impact instead of relying only on the final sampled pose after integration.
    let mut world = no_gravity_world();
    let wall = create_body(&mut world, BodyType::Static, 0.0, 0.0, Vector::default());
    let bullet = create_body(
        &mut world,
        BodyType::Dynamic,
        -1.0,
        0.0,
        Vector::new(200.0, 0.0),
    );
    let wall_collider = attach_shape(
        &mut world,
        wall,
        SharedShape::rect(0.1, 10.0),
        Material::default(),
    );
    let bullet_collider = attach_shape(
        &mut world,
        bullet,
        SharedShape::circle(0.05),
        Material::default(),
    );

    let report = step_world(&mut world, 1);
    let position = body_position(&world, bullet);
    let contact = active_contact_events(&report)
        .into_iter()
        .find(|contact| contact.ccd_trace.is_some())
        .expect("CCD should report the swept contact with the wall");
    let trace = contact.ccd_trace.expect("contact should carry CCD trace");

    assert!(
        report.stats.contact_count > 0,
        "CCD should report the swept contact with the wall"
    );
    assert_eq!(report.stats.ccd_candidate_count, 1);
    assert_eq!(report.stats.ccd_hit_count, 1);
    assert_eq!(report.stats.ccd_miss_count, 0);
    assert_eq!(report.stats.ccd_clamp_count, 1);
    assert_eq!(
        report.stats.contact_count,
        active_contact_events(&report).len(),
        "StepStats contact count should match the active contact events emitted by the step"
    );
    assert_eq!(trace.moving_body, bullet);
    assert_eq!(trace.static_body, wall);
    assert_eq!(trace.moving_collider, bullet_collider);
    assert_eq!(trace.static_collider, wall_collider);
    assert_eq!(trace.swept_start, Point::new(-1.0, 0.0));
    assert!(trace.swept_end.x() > 2.0);
    assert!(trace.toi > 0.0 && trace.toi < 1.0);
    assert!(trace.advancement >= trace.toi && trace.advancement <= 1.0);
    assert!(trace.clamp > 0.0);
    assert!(trace.slop > 0.0);
    assert!((trace.toi_point.x() + 0.05).abs() < 1.0e-3);
    assert!(
        position.x() <= -0.05,
        "CCD should keep the bullet on the pre-impact side of the wall; x={}",
        position.x()
    );

    let snapshot = DebugSnapshot::from_world_with_step_report(
        &world,
        &report,
        &DebugSnapshotOptions::default(),
    );
    assert_eq!(snapshot.stats.ccd_candidate_count, 1);
    assert_eq!(snapshot.stats.ccd_hit_count, 1);
    assert_eq!(snapshot.stats.ccd_miss_count, 0);
    assert_eq!(snapshot.stats.ccd_clamp_count, 1);
    assert_eq!(snapshot.stats.contact_count, report.stats.contact_count);
    assert!(
        snapshot
            .contacts
            .iter()
            .any(|contact| contact.ccd_trace == Some(trace)),
        "debug contacts should retain the CCD trace"
    );
}

#[test]
fn ccd_missed_sweep_does_not_emit_false_positive_or_clamp() {
    let mut world = no_gravity_world();
    let diamond = create_body(&mut world, BodyType::Static, 0.0, 0.0, Vector::default());
    let bullet = create_body(
        &mut world,
        BodyType::Dynamic,
        -0.9,
        2.0,
        Vector::new(0.0, -66.0),
    );
    attach_shape(
        &mut world,
        diamond,
        SharedShape::convex_polygon(vec![
            Point::new(0.0, -1.0),
            Point::new(1.0, 0.0),
            Point::new(0.0, 1.0),
            Point::new(-1.0, 0.0),
        ]),
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

    assert_eq!(report.stats.contact_count, 0);
    assert_eq!(report.stats.ccd_candidate_count, 1);
    assert_eq!(report.stats.ccd_hit_count, 0);
    assert_eq!(report.stats.ccd_miss_count, 1);
    assert_eq!(report.stats.ccd_clamp_count, 0);
    assert!(
        (position.y() - 0.9).abs() < 1.0e-4,
        "missed sweep should keep the integrated end pose; y={}",
        position.y()
    );
}

#[test]
fn ccd_dynamic_circle_hits_static_convex_polygon() {
    let mut world = no_gravity_world();
    let diamond = create_body(&mut world, BodyType::Static, 0.0, 0.0, Vector::default());
    let bullet = create_body(
        &mut world,
        BodyType::Dynamic,
        -1.0,
        0.0,
        Vector::new(200.0, 0.0),
    );
    attach_shape(
        &mut world,
        diamond,
        SharedShape::convex_polygon(vec![
            Point::new(0.0, -1.0),
            Point::new(0.25, 0.0),
            Point::new(0.0, 1.0),
            Point::new(-0.25, 0.0),
        ]),
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
    let contact = active_contact_events(&report)
        .into_iter()
        .find(|contact| contact.ccd_trace.is_some())
        .expect("CCD should report the swept contact with static convex geometry");
    let trace = contact.ccd_trace.expect("contact should carry CCD trace");

    assert_eq!(report.stats.ccd_candidate_count, 1);
    assert_eq!(report.stats.ccd_hit_count, 1);
    assert_eq!(report.stats.ccd_miss_count, 0);
    assert_eq!(report.stats.ccd_clamp_count, 1);
    assert!(trace.toi > 0.0 && trace.toi < 1.0);
    assert!(
        position.x() < -0.2,
        "bullet should be clamped before crossing the convex polygon; x={}",
        position.x()
    );
}
