use std::collections::BTreeSet;

use picea::events::CcdTargetKind;
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

fn attach_shape_with_local_pose(
    world: &mut World,
    body: BodyHandle,
    shape: SharedShape,
    local_pose: Pose,
    material: Material,
) -> ColliderHandle {
    world
        .create_collider(
            body,
            ColliderDesc {
                shape,
                local_pose,
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

fn raw_feature_parts(feature_id: ContactFeatureId) -> (u32, u32) {
    let raw = serde_json::to_value(feature_id)
        .expect("contact feature id should serialize")
        .as_u64()
        .expect("contact feature id should serialize as its raw integer");
    (raw as u32, (raw >> 32) as u32)
}

fn decoded_raw_feature(feature_id: ContactFeatureId) -> (u32, u32, u32, u32) {
    let (index, slot) = raw_feature_parts(feature_id);
    (
        (index >> 24) & 0xff,
        (index >> 12) & 0xfff,
        index & 0xfff,
        slot,
    )
}

fn contact_local_witnesses(contact: &ContactEvent, pose_a: Pose, pose_b: Pose) -> (Point, Point) {
    let half_depth = contact.normal * (contact.depth * 0.5);
    (
        pose_a.inverse_transform_point(contact.point - half_depth),
        pose_b.inverse_transform_point(contact.point + half_depth),
    )
}

fn dual_local_witness_drift(
    first: &ContactEvent,
    first_pose_a: Pose,
    first_pose_b: Pose,
    second: &ContactEvent,
    second_pose_a: Pose,
    second_pose_b: Pose,
) -> f32 {
    let (first_a, first_b) = contact_local_witnesses(first, first_pose_a, first_pose_b);
    let (second_a, second_b) = contact_local_witnesses(second, second_pose_a, second_pose_b);
    (first_a - second_a)
        .length()
        .max((first_b - second_b).length())
}

fn unique_two_point_local_witness_correspondence(
    first: &[ContactEvent],
    first_pose_a: Pose,
    first_pose_b: Pose,
    second: &[ContactEvent],
    second_pose_a: Pose,
    second_pose_b: Pose,
) -> [(usize, usize); 2] {
    assert_eq!(first.len(), 2);
    assert_eq!(second.len(), 2);
    let direct_drifts = [
        dual_local_witness_drift(
            &first[0],
            first_pose_a,
            first_pose_b,
            &second[0],
            second_pose_a,
            second_pose_b,
        ),
        dual_local_witness_drift(
            &first[1],
            first_pose_a,
            first_pose_b,
            &second[1],
            second_pose_a,
            second_pose_b,
        ),
    ];
    let crossed_drifts = [
        dual_local_witness_drift(
            &first[0],
            first_pose_a,
            first_pose_b,
            &second[1],
            second_pose_a,
            second_pose_b,
        ),
        dual_local_witness_drift(
            &first[1],
            first_pose_a,
            first_pose_b,
            &second[0],
            second_pose_a,
            second_pose_b,
        ),
    ];
    let direct_score = (
        direct_drifts.into_iter().fold(0.0_f32, f32::max),
        direct_drifts.into_iter().sum::<f32>(),
    );
    let crossed_score = (
        crossed_drifts.into_iter().fold(0.0_f32, f32::max),
        crossed_drifts.into_iter().sum::<f32>(),
    );
    assert_ne!(
        direct_score, crossed_score,
        "the fixture must have a unique two-point local-witness correspondence"
    );
    if direct_score.0 < crossed_score.0
        || (direct_score.0 == crossed_score.0 && direct_score.1 < crossed_score.1)
    {
        [(0, 0), (1, 1)]
    } else {
        [(0, 1), (1, 0)]
    }
}

#[derive(Debug)]
struct StackBehaviorFrame {
    report: StepReport,
    state_hash: String,
    penetration_max: f32,
    penetration_sum: f32,
    max_linear_speed: f32,
    max_angular_speed: f32,
    contact_enter_count: usize,
    contact_exit_count: usize,
}

fn build_stack_4_world() -> World {
    let mut world = World::new(WorldDesc {
        gravity: Vector::new(0.0, 9.8),
        enable_sleep: true,
    });
    let floor = create_body(&mut world, BodyType::Static, 0.0, 2.5, Vector::default());
    attach_shape(
        &mut world,
        floor,
        SharedShape::rect(10.0, 0.5),
        Material::default(),
    );
    for index in 0..4 {
        let body = world
            .create_body(BodyDesc {
                body_type: BodyType::Dynamic,
                pose: Pose::from_xy_angle(0.0, 1.7 - index as f32, 0.0),
                can_sleep: true,
                ..BodyDesc::default()
            })
            .expect("stack body should be created");
        attach_shape(
            &mut world,
            body,
            SharedShape::rect(0.9, 0.9),
            Material::default(),
        );
    }
    world
}

fn build_dense_position_correction_stack_world(dynamic_layers: usize) -> (World, Vec<BodyHandle>) {
    let mut world = no_gravity_world();
    let floor = create_body(&mut world, BodyType::Static, 0.0, 0.5, Vector::default());
    let material = Material {
        friction: 0.0,
        restitution: 0.0,
    };
    attach_shape(&mut world, floor, SharedShape::rect(10.0, 1.0), material);

    let mut bodies = Vec::with_capacity(dynamic_layers);
    for index in 0..dynamic_layers {
        // Reuse the same face-manifold geometry as the smaller correction tests,
        // but stack enough slightly overlapping pairs to cross the dense
        // position-correction threshold in a real solver step.
        let body = create_body(
            &mut world,
            BodyType::Dynamic,
            0.0,
            -0.45 - index as f32 * 0.95,
            Vector::default(),
        );
        attach_shape(&mut world, body, SharedShape::rect(1.0, 1.0), material);
        bodies.push(body);
    }

    (world, bodies)
}

fn collect_stack_4_frames(frames: usize) -> Vec<StackBehaviorFrame> {
    let mut world = build_stack_4_world();
    let mut pipeline = SimulationPipeline::new(fixed_step_config());
    let mut captured = Vec::with_capacity(frames);

    for _ in 0..frames {
        let report = pipeline.step(&mut world);
        let snapshot = DebugSnapshot::from_world_with_step_report(
            &world,
            &report,
            &DebugSnapshotOptions::default(),
        );
        let (penetration_max, penetration_sum) = snapshot.manifolds.iter().fold(
            (0.0_f32, 0.0_f32),
            |(max_depth, total_depth), manifold| {
                let depth = manifold.depth.max(0.0);
                (max_depth.max(depth), total_depth + depth)
            },
        );
        let (max_linear_speed, max_angular_speed) = snapshot
            .bodies
            .iter()
            .filter(|body| body.body_type == BodyType::Dynamic)
            .fold((0.0_f32, 0.0_f32), |(max_linear, max_angular), body| {
                (
                    max_linear.max(body.linear_velocity.length()),
                    max_angular.max(body.angular_velocity.abs()),
                )
            });
        let contact_enter_count = report
            .events
            .iter()
            .filter(|event| matches!(event, WorldEvent::ContactStarted(_)))
            .count();
        let contact_exit_count = report
            .events
            .iter()
            .filter(|event| matches!(event, WorldEvent::ContactEnded(_)))
            .count();

        captured.push(StackBehaviorFrame {
            state_hash: stable_snapshot_hash(&snapshot),
            report,
            penetration_max,
            penetration_sum,
            max_linear_speed,
            max_angular_speed,
            contact_enter_count,
            contact_exit_count,
        });
    }

    captured
}

fn stable_snapshot_hash(snapshot: &DebugSnapshot) -> String {
    let bytes = serde_json::to_vec(snapshot).expect("debug snapshot should serialize for hashing");
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
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
fn stack_4_behavior_lock_stays_deterministic_and_quiet_after_settling() {
    let first_run = collect_stack_4_frames(300);
    let second_run = collect_stack_4_frames(300);
    let final_first = first_run
        .last()
        .expect("first stack run should have a final frame");
    let final_second = second_run
        .last()
        .expect("second stack run should have a final frame");

    assert_eq!(
        final_first.state_hash, final_second.state_hash,
        "stack_4 should keep a deterministic final state hash for behavior-lock comparisons"
    );
    assert!(
        first_run
            .iter()
            .all(|frame| frame.report.stats.numeric_warnings == 0),
        "stack_4 behavior lock must stay numerically safe across the whole settling window"
    );

    let settled_penetration_max = first_run
        .iter()
        .skip(120)
        .map(|frame| frame.penetration_max)
        .fold(0.0_f32, f32::max);
    let settled_penetration_sum = first_run
        .iter()
        .skip(120)
        .map(|frame| frame.penetration_sum)
        .fold(0.0_f32, f32::max);
    assert!(
        settled_penetration_max <= 0.04,
        "stack_4 should keep settled penetration within the D2 max-depth target; got {settled_penetration_max}"
    );
    assert!(
        settled_penetration_sum <= 0.16,
        "stack_4 should keep settled penetration sum within the D2 target; got {settled_penetration_sum}"
    );

    // D2's quiet window accepts a low-motion settled stack even before the
    // island sleep heuristic decides to flip the sleeping bit.
    let quiet_windows_after_frame_240 = first_run[240..].windows(24);
    let (quiet_max_linear_speed, quiet_max_angular_speed) = quiet_windows_after_frame_240.fold(
        (0.0_f32, 0.0_f32),
        |(max_linear, max_angular), window| {
            let window_max_linear = window
                .iter()
                .map(|frame| frame.max_linear_speed)
                .fold(0.0_f32, f32::max);
            let window_max_angular = window
                .iter()
                .map(|frame| frame.max_angular_speed)
                .fold(0.0_f32, f32::max);
            (
                max_linear.max(window_max_linear),
                max_angular.max(window_max_angular),
            )
        },
    );
    assert!(
        quiet_max_linear_speed <= 0.04,
        "all stack_4 rolling windows after frame 240 should stay within the D2 quiet linear-velocity target; got {quiet_max_linear_speed}"
    );
    assert!(
        quiet_max_angular_speed <= 0.08,
        "all stack_4 rolling windows after frame 240 should stay within the D2 quiet angular-velocity target; got {quiet_max_angular_speed}"
    );

    assert!(
        first_run
            .iter()
            .skip(180)
            .all(|frame| frame.report.stats.warm_start_drop_count == 0),
        "continuing stack contacts should stop dropping trusted warm-start cache entries after settling"
    );
    assert!(
        first_run
            .iter()
            .skip(180)
            .collect::<Vec<_>>()
            .windows(24)
            .all(|window| {
                window
                    .iter()
                    .map(|frame| frame.contact_enter_count + frame.contact_exit_count)
                    .sum::<usize>()
                    <= 2
            }),
        "late stack_4 windows should not keep churning contact identity after settling"
    );
}

#[test]
fn stack_4_long_window_retains_vertical_support_chain() {
    let mut world = build_stack_4_world();
    let mut pipeline = SimulationPipeline::new(fixed_step_config());
    let mut final_snapshot = None;
    for _ in 0..1200 {
        let report = pipeline.step(&mut world);
        final_snapshot = Some(DebugSnapshot::from_world_with_step_report(
            &world,
            &report,
            &DebugSnapshotOptions::default(),
        ));
    }
    let final_snapshot = final_snapshot.expect("long stack run should produce a final snapshot");
    let dynamic_support_links = final_snapshot
        .contacts
        .iter()
        .filter(|contact| {
            contact.bodies.iter().all(|handle| {
                final_snapshot
                    .bodies
                    .iter()
                    .find(|body| body.handle == *handle)
                    .map(|body| body.body_type == BodyType::Dynamic)
                    .unwrap_or(false)
            })
        })
        .count();
    let dynamic_x_positions = final_snapshot
        .bodies
        .iter()
        .filter(|body| body.body_type == BodyType::Dynamic)
        .map(|body| body.transform.translation.x())
        .collect::<Vec<_>>();
    let x_spread = dynamic_x_positions
        .iter()
        .copied()
        .fold(f32::NEG_INFINITY, f32::max)
        - dynamic_x_positions
            .iter()
            .copied()
            .fold(f32::INFINITY, f32::min);

    assert!(
        dynamic_support_links >= 3,
        "stack_4 should retain the three dynamic support links of a vertical four-box stack; dynamic_support_links={dynamic_support_links}, contacts={:?}",
        final_snapshot.contacts
    );
    assert!(
        x_spread <= 0.35,
        "stack_4 should not quietly collapse into floor-spread boxes; x_positions={dynamic_x_positions:?}, spread={x_spread}"
    );
}

#[test]
fn stack_4_long_window_enters_sleep_after_stable_vertical_stack() {
    let mut world = build_stack_4_world();
    let mut pipeline = SimulationPipeline::new(fixed_step_config());
    for _ in 0..1200 {
        pipeline.step(&mut world);
    }
    let snapshot = DebugSnapshot::from_world(&world, &DebugSnapshotOptions::default());
    let awake_dynamic_bodies = snapshot
        .bodies
        .iter()
        .filter(|body| body.body_type == BodyType::Dynamic && !body.sleeping)
        .map(|body| body.handle)
        .collect::<Vec<_>>();

    assert!(
        awake_dynamic_bodies.is_empty(),
        "stack_4 should enter sleep once the vertical support chain is stable; awake_dynamic_bodies={awake_dynamic_bodies:?}"
    );
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
    assert_eq!(
        second_contact.warm_start_reason,
        WarmStartCacheReason::Hit,
        "trustworthy same-feature contact should transfer cached normal impulse: {second_contact:?}"
    );
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

    assert_eq!(
        second_contact.warm_start_reason,
        WarmStartCacheReason::Hit,
        "static contact should still expose a warm-start cache hit even when it has no solver row: {second_contact:?}"
    );
    assert!(
        second_contact.warm_start_normal_impulse > 0.0,
        "warm-start facts should still describe the transferred cache: {second_contact:?}"
    );
    assert_eq!(second_contact.solver_normal_impulse, 0.0);
    assert_eq!(second_contact.solver_tangent_impulse, 0.0);
}

#[test]
fn zero_velocity_iterations_cannot_restitute_or_confirm_warm_started_contact() {
    let mut world = no_gravity_world();
    let moving = create_body(
        &mut world,
        BodyType::Dynamic,
        -0.975,
        0.0,
        Vector::new(12.0, 0.0),
    );
    let wall = create_body(&mut world, BodyType::Static, 0.975, 0.0, Vector::default());
    let material = Material {
        friction: 0.0,
        restitution: 0.5,
    };
    attach_shape(&mut world, moving, SharedShape::circle(1.0), material);
    attach_shape(&mut world, wall, SharedShape::circle(1.0), material);

    let first = step_world(&mut world, 1);
    assert!(
        active_contact_events(&first)
            .iter()
            .any(|contact| contact.solver_normal_impulse > 0.0),
        "fixture must establish a positive warm-start cache"
    );

    world
        .apply_body_patch(
            moving,
            BodyPatch {
                pose: Some(Pose::from_xy_angle(-0.975, 0.0, 0.0)),
                linear_velocity: Some(Vector::new(12.0, 0.0)),
                wake: true,
                ..BodyPatch::default()
            },
        )
        .expect("second step should reuse the same contact feature");
    let separated_report = step_world_with_config(
        &mut world,
        StepConfig {
            velocity_iterations: 0,
            ..fixed_step_config()
        },
        1,
    );
    let separated_contacts = active_contact_events(&separated_report);
    assert!(
        separated_contacts.is_empty(),
        "final-separated contact must not survive solely because a cached impulse was applied: {separated_contacts:?}"
    );
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
fn sat_edge_swap_candidate_persists_lifecycle_without_auto_warm_start_impulse() {
    let mut world = no_gravity_world();
    let lower = create_body(&mut world, BodyType::Static, 0.0, 0.0, Vector::default());
    let upper = create_body(&mut world, BodyType::Static, -0.25, 1.8, Vector::default());
    attach_shape(
        &mut world,
        lower,
        SharedShape::rect(2.0, 2.0),
        Material::default(),
    );
    attach_shape(
        &mut world,
        upper,
        SharedShape::rect(2.0, 2.0),
        Material::default(),
    );
    let lower_pose = Pose::from_xy_angle(0.0, 0.0, -0.12);
    let first_upper_pose = Pose::from_xy_angle(-0.25, 1.8, -0.17);
    let second_upper_pose = Pose::from_xy_angle(-0.25, 1.8, -0.15);
    world
        .apply_body_patch(
            lower,
            BodyPatch {
                pose: Some(lower_pose),
                wake: true,
                ..BodyPatch::default()
            },
        )
        .expect("lower rectangle pose should be patched");
    world
        .apply_body_patch(
            upper,
            BodyPatch {
                pose: Some(first_upper_pose),
                wake: true,
                ..BodyPatch::default()
            },
        )
        .expect("upper rectangle pose should be patched");
    let mut pipeline = SimulationPipeline::new(fixed_step_config());

    let first = pipeline.step(&mut world);
    world
        .apply_body_patch(
            upper,
            BodyPatch {
                pose: Some(second_upper_pose),
                wake: true,
                ..BodyPatch::default()
            },
        )
        .expect("upper rectangle should nudge across the SAT reference/incident edge swap");
    let second = pipeline.step(&mut world);
    let first_contacts = active_contact_events(&first);
    let second_contacts = active_contact_events(&second);

    assert_eq!(first_contacts.len(), 2);
    assert_eq!(second_contacts.len(), 2);
    assert!(
        first_contacts.iter().any(|first| {
            second_contacts.iter().any(|second| {
                let first_anchor_a = lower_pose.inverse_transform_point(first.point);
                let first_anchor_b = first_upper_pose.inverse_transform_point(first.point);
                let second_anchor_a = lower_pose.inverse_transform_point(second.point);
                let second_anchor_b = second_upper_pose.inverse_transform_point(second.point);
                let local_anchor_drift = (first_anchor_a - second_anchor_a)
                    .length()
                    .max((first_anchor_b - second_anchor_b).length());
                first.contact_id == second.contact_id
                    && first.manifold_id == second.manifold_id
                    && first.normal.dot(second.normal) > 0.999
                    && local_anchor_drift < 0.25
            })
        }),
        "E4b identity-only candidate should preserve lifecycle ids across SAT edge-role swaps with local-anchor continuity and without relying on raw feature-id equality; first={first_contacts:?}, second={second_contacts:?}"
    );
    assert!(
        second_contacts.iter().all(|contact| {
            contact.warm_start_reason != WarmStartCacheReason::Hit
                && contact.warm_start_normal_impulse == 0.0
                && contact.warm_start_tangent_impulse == 0.0
        }),
        "first-stage persistent manifold identity must not automatically migrate solver impulse; second={second_contacts:?}"
    );
}

#[test]
fn manifold_persistence_sat_role_swap_preserves_both_contact_ids_by_local_anchor() {
    let mut world = no_gravity_world();
    let lower = create_body(&mut world, BodyType::Static, 0.0, 0.0, Vector::default());
    let upper = create_body(&mut world, BodyType::Static, -0.25, 1.8, Vector::default());
    let lower_collider = attach_shape(
        &mut world,
        lower,
        SharedShape::rect(2.0, 2.0),
        Material::default(),
    );
    let upper_collider = attach_shape(
        &mut world,
        upper,
        SharedShape::rect(2.0, 2.0),
        Material::default(),
    );
    let lower_pose = Pose::from_xy_angle(0.0, 0.0, -0.12);
    let first_upper_pose = Pose::from_xy_angle(-0.25, 1.8, -0.17);
    let second_upper_pose = Pose::from_xy_angle(-0.25, 1.8, -0.15);
    world
        .apply_body_patch(
            lower,
            BodyPatch {
                pose: Some(lower_pose),
                wake: true,
                ..BodyPatch::default()
            },
        )
        .expect("lower rectangle pose should be patched");
    world
        .apply_body_patch(
            upper,
            BodyPatch {
                pose: Some(first_upper_pose),
                wake: true,
                ..BodyPatch::default()
            },
        )
        .expect("upper rectangle pose should be patched");
    let mut pipeline = SimulationPipeline::new(fixed_step_config());

    let first = pipeline.step(&mut world);
    world
        .apply_body_patch(
            upper,
            BodyPatch {
                pose: Some(second_upper_pose),
                wake: true,
                ..BodyPatch::default()
            },
        )
        .expect("upper rectangle should cross the SAT reference/incident role swap");
    let second = pipeline.step(&mut world);
    let first_contacts = active_contact_events(&first);
    let second_contacts = active_contact_events(&second);

    assert_eq!(first_contacts.len(), 2);
    assert_eq!(second_contacts.len(), 2);
    assert!(first_contacts.iter().all(|contact| {
        contact.collider_a == lower_collider.min(upper_collider)
            && contact.collider_b == lower_collider.max(upper_collider)
    }));
    let correspondence = unique_two_point_local_witness_correspondence(
        &first_contacts,
        lower_pose,
        first_upper_pose,
        &second_contacts,
        lower_pose,
        second_upper_pose,
    );
    assert_ne!(first_contacts[0].contact_id, first_contacts[1].contact_id);
    for (first_index, second_index) in correspondence {
        let other_first_index = 1 - first_index;
        assert_eq!(
            second_contacts[second_index].contact_id, first_contacts[first_index].contact_id,
            "each current point must inherit the id of its unique local-witness predecessor"
        );
        assert_eq!(
            second_contacts[second_index].manifold_id,
            first_contacts[first_index].manifold_id
        );
        assert_ne!(
            second_contacts[second_index].contact_id, first_contacts[other_first_index].contact_id,
            "left/right point identities must not interchange"
        );
    }
    assert_eq!(
        second_contacts
            .iter()
            .map(|contact| contact.manifold_id)
            .collect::<BTreeSet<_>>()
            .len(),
        1
    );
    let first_feature_indices = first_contacts
        .iter()
        .map(|contact| raw_feature_parts(contact.feature_id).0)
        .collect::<BTreeSet<_>>();
    let second_feature_indices = second_contacts
        .iter()
        .map(|contact| raw_feature_parts(contact.feature_id).0)
        .collect::<BTreeSet<_>>();
    assert_ne!(first_feature_indices, second_feature_indices);
    assert_eq!(
        second_contacts
            .iter()
            .map(|contact| raw_feature_parts(contact.feature_id).1)
            .collect::<BTreeSet<_>>()
            .len(),
        2,
        "the second frame must retain two distinct raw point slots from current geometry"
    );
    assert!(second_contacts.iter().all(|contact| {
        contact.normal.dot(first_contacts[0].normal) > 0.999
            && contact.warm_start_reason != WarmStartCacheReason::Hit
            && contact.warm_start_normal_impulse == 0.0
            && contact.warm_start_tangent_impulse == 0.0
    }));
}

#[test]
fn manifold_persistence_geometry_patch_invalidates_all_history_consumers() {
    fn run_patch(patch: ColliderPatch) -> (Vec<ContactEvent>, Vec<ContactEvent>) {
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
        let first = active_contact_events(&pipeline.step(&mut world));
        world
            .apply_collider_patch(right_collider, patch)
            .expect("geometry patch should succeed");
        let second = active_contact_events(&pipeline.step(&mut world));
        (first, second)
    }

    let cases = [
        (
            "shape",
            run_patch(ColliderPatch {
                shape: Some(SharedShape::rect(2.02, 2.0)),
                ..ColliderPatch::default()
            }),
        ),
        (
            "local_pose",
            run_patch(ColliderPatch {
                local_pose: Some(Pose::from_xy_angle(0.01, 0.0, 0.0)),
                ..ColliderPatch::default()
            }),
        ),
    ];
    let mut results = Vec::new();
    for (kind, (first_contacts, second_contacts)) in cases {
        assert_eq!(first_contacts.len(), 2, "{kind} first frame");
        assert_eq!(second_contacts.len(), 2, "{kind} second frame");
        let first_contact_ids = first_contacts
            .iter()
            .map(|contact| contact.contact_id)
            .collect::<BTreeSet<_>>();
        let first_manifold_ids = first_contacts
            .iter()
            .map(|contact| contact.manifold_id)
            .collect::<BTreeSet<_>>();
        let second_contact_ids = second_contacts
            .iter()
            .map(|contact| contact.contact_id)
            .collect::<BTreeSet<_>>();
        let second_manifold_ids = second_contacts
            .iter()
            .map(|contact| contact.manifold_id)
            .collect::<BTreeSet<_>>();
        let accepted = second_manifold_ids.len() == 1
            && first_contact_ids.is_disjoint(&second_contact_ids)
            && first_manifold_ids.is_disjoint(&second_manifold_ids)
            && second_contacts.iter().all(|contact| {
                contact.lifecycle_reason == ContactLifecycleReason::Started
                    && contact.warm_start_reason == WarmStartCacheReason::MissFeatureId
                    && contact.warm_start_normal_impulse == 0.0
                    && contact.warm_start_tangent_impulse == 0.0
                    && !contact.source_row_continuity_candidate
                    && contact.source_row_continuity_reason != SourceRowContinuityReason::Candidate
            });
        results.push((kind, accepted, second_contacts));
    }
    assert!(
        results.iter().all(|(_, accepted, _)| *accepted),
        "shape and local-pose revisions must invalidate every history consumer; results={:?}",
        results
            .iter()
            .map(|(kind, accepted, contacts)| (
                kind,
                accepted,
                contacts
                    .iter()
                    .map(|contact| (
                        contact.lifecycle_reason,
                        contact.warm_start_reason,
                        contact.source_row_continuity_candidate,
                        contact.source_row_continuity_reason,
                    ))
                    .collect::<Vec<_>>(),
            ))
            .collect::<Vec<_>>()
    );
}

#[test]
fn manifold_persistence_world_commands_geometry_patch_is_atomic() {
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
    let first_contacts = active_contact_events(&first);
    assert_eq!(first_contacts.len(), 2);
    assert!(!first_contacts.is_empty());
    let first_contact_ids = first_contacts
        .iter()
        .map(|contact| contact.contact_id)
        .collect::<BTreeSet<_>>();
    let first_manifold_ids = first_contacts
        .iter()
        .map(|contact| contact.manifold_id)
        .collect::<BTreeSet<_>>();

    let error = world
        .commands()
        .apply([
            WorldCommand::PatchCollider {
                collider: right_collider,
                patch: ColliderPatch {
                    shape: Some(SharedShape::rect(2.02, 2.0)),
                    ..ColliderPatch::default()
                },
            },
            WorldCommand::PatchCollider {
                collider: right_collider,
                patch: ColliderPatch {
                    local_pose: Some(Pose::from_xy_angle(f32::NAN, 0.0, 0.0)),
                    ..ColliderPatch::default()
                },
            },
        ])
        .expect_err("invalid command must reject the scratch transaction");
    assert_eq!(error.command_index, 1);
    let after_rejection = pipeline.step(&mut world);
    let after_rejection_contacts = active_contact_events(&after_rejection);
    assert_eq!(
        after_rejection_contacts
            .iter()
            .map(|contact| contact.contact_id)
            .collect::<BTreeSet<_>>(),
        first_contact_ids,
        "a rejected scratch patch must leave contact history intact"
    );
    assert_eq!(
        after_rejection_contacts
            .iter()
            .map(|contact| contact.manifold_id)
            .collect::<BTreeSet<_>>(),
        first_manifold_ids
    );
    assert!(after_rejection_contacts.iter().all(|contact| {
        contact.lifecycle_reason == ContactLifecycleReason::ExactFeature
            && contact.warm_start_reason == WarmStartCacheReason::Hit
    }));

    world
        .commands()
        .apply_one(WorldCommand::PatchCollider {
            collider: right_collider,
            patch: ColliderPatch {
                shape: Some(SharedShape::rect(2.02, 2.0)),
                ..ColliderPatch::default()
            },
        })
        .expect("valid geometry command should commit");
    let after_commit = pipeline.step(&mut world);
    let after_commit_contacts = active_contact_events(&after_commit);
    assert_eq!(after_commit_contacts.len(), 2);
    let committed_contact_ids = after_commit_contacts
        .iter()
        .map(|contact| contact.contact_id)
        .collect::<BTreeSet<_>>();
    let committed_manifold_ids = after_commit_contacts
        .iter()
        .map(|contact| contact.manifold_id)
        .collect::<BTreeSet<_>>();
    let lifecycle_invalidated = first_contact_ids.is_disjoint(&committed_contact_ids)
        && first_manifold_ids.is_disjoint(&committed_manifold_ids)
        && after_commit_contacts
            .iter()
            .all(|contact| contact.lifecycle_reason == ContactLifecycleReason::Started);
    let warm_start_invalidated = after_commit_contacts.iter().all(|contact| {
        contact.warm_start_reason == WarmStartCacheReason::MissFeatureId
            && contact.warm_start_normal_impulse == 0.0
            && contact.warm_start_tangent_impulse == 0.0
    });
    let source_row_invalidated = after_commit_contacts.iter().all(|contact| {
        !contact.source_row_continuity_candidate
            && contact.source_row_continuity_reason != SourceRowContinuityReason::Candidate
    });
    assert!(
        lifecycle_invalidated && warm_start_invalidated && source_row_invalidated,
        "committed geometry patch must invalidate all retained contact history; lifecycle={lifecycle_invalidated}, warm_start={warm_start_invalidated}, source_row={source_row_invalidated}, facts={:?}",
        after_commit_contacts
            .iter()
            .map(|contact| (
                contact.lifecycle_reason,
                contact.warm_start_reason,
                contact.source_row_continuity_candidate,
                contact.source_row_continuity_reason,
            ))
            .collect::<Vec<_>>()
    );
}

#[test]
fn manifold_persistence_sensor_transitions_do_not_expand_edge_swap_identity() {
    fn role_swap_transition(
        first_sensor: bool,
        second_sensor: bool,
    ) -> (Vec<ContactEvent>, Vec<ContactEvent>) {
        let mut world = no_gravity_world();
        let lower = create_body(&mut world, BodyType::Static, 0.0, 0.0, Vector::default());
        let upper = create_body(&mut world, BodyType::Static, -0.25, 1.8, Vector::default());
        attach_shape(
            &mut world,
            lower,
            SharedShape::rect(2.0, 2.0),
            Material::default(),
        );
        let upper_collider = world
            .create_collider(
                upper,
                ColliderDesc {
                    shape: SharedShape::rect(2.0, 2.0),
                    is_sensor: first_sensor,
                    ..ColliderDesc::default()
                },
            )
            .expect("upper collider should be created");
        world
            .apply_body_patch(
                lower,
                BodyPatch {
                    pose: Some(Pose::from_xy_angle(0.0, 0.0, -0.12)),
                    wake: true,
                    ..BodyPatch::default()
                },
            )
            .expect("lower pose should be patched");
        world
            .apply_body_patch(
                upper,
                BodyPatch {
                    pose: Some(Pose::from_xy_angle(-0.25, 1.8, -0.17)),
                    wake: true,
                    ..BodyPatch::default()
                },
            )
            .expect("first upper pose should be patched");
        let mut pipeline = SimulationPipeline::new(fixed_step_config());
        let first = pipeline.step(&mut world);
        world
            .apply_collider_patch(
                upper_collider,
                ColliderPatch {
                    is_sensor: Some(second_sensor),
                    ..ColliderPatch::default()
                },
            )
            .expect("sensor transition should succeed");
        world
            .apply_body_patch(
                upper,
                BodyPatch {
                    pose: Some(Pose::from_xy_angle(-0.25, 1.8, -0.15)),
                    wake: true,
                    ..BodyPatch::default()
                },
            )
            .expect("second upper pose should be patched");
        let second = pipeline.step(&mut world);
        (
            active_contact_events(&first),
            active_contact_events(&second),
        )
    }

    fn exact_transition(
        first_sensor: bool,
        second_sensor: bool,
    ) -> (Vec<ContactEvent>, Vec<ContactEvent>) {
        let mut world = no_gravity_world();
        let left = create_body(&mut world, BodyType::Static, 0.0, 0.0, Vector::default());
        let right = create_body(&mut world, BodyType::Static, 1.5, 0.0, Vector::default());
        attach_shape(
            &mut world,
            left,
            SharedShape::circle(1.0),
            Material::default(),
        );
        let right_collider = world
            .create_collider(
                right,
                ColliderDesc {
                    shape: SharedShape::circle(1.0),
                    is_sensor: first_sensor,
                    ..ColliderDesc::default()
                },
            )
            .expect("exact-transition collider should be created");
        let mut pipeline = SimulationPipeline::new(fixed_step_config());
        let first = active_contact_events(&pipeline.step(&mut world));
        world
            .apply_collider_patch(
                right_collider,
                ColliderPatch {
                    is_sensor: Some(second_sensor),
                    ..ColliderPatch::default()
                },
            )
            .expect("exact sensor transition should succeed");
        let second = active_contact_events(&pipeline.step(&mut world));
        (first, second)
    }

    let mut role_swap_results = Vec::new();
    for (first_sensor, second_sensor) in [(false, true), (true, false), (true, true)] {
        let (first, second) = role_swap_transition(first_sensor, second_sensor);
        assert_eq!(first.len(), 2);
        assert_eq!(second.len(), 2);
        let old_ids = first
            .iter()
            .map(|contact| contact.contact_id)
            .collect::<BTreeSet<_>>();
        let current_ids = second
            .iter()
            .map(|contact| contact.contact_id)
            .collect::<BTreeSet<_>>();
        let old_manifold_ids = first
            .iter()
            .map(|contact| contact.manifold_id)
            .collect::<BTreeSet<_>>();
        let current_manifold_ids = second
            .iter()
            .map(|contact| contact.manifold_id)
            .collect::<BTreeSet<_>>();
        let expected_warm_reason = if second_sensor {
            WarmStartCacheReason::SkippedSensor
        } else {
            WarmStartCacheReason::MissFeatureId
        };
        role_swap_results.push((
            first_sensor,
            second_sensor,
            old_ids.is_disjoint(&current_ids)
                && old_manifold_ids == current_manifold_ids
                && second.iter().all(|contact| {
                    contact.lifecycle_reason == ContactLifecycleReason::Started
                        && contact.warm_start_reason == expected_warm_reason
                        && contact.warm_start_normal_impulse == 0.0
                        && contact.warm_start_tangent_impulse == 0.0
                }),
            second,
        ));
    }

    let mut exact_results = Vec::new();
    for (first_sensor, second_sensor, expected_warm_reason) in [
        (false, true, WarmStartCacheReason::SkippedSensor),
        (true, false, WarmStartCacheReason::MissPreviousSensor),
        (true, true, WarmStartCacheReason::SkippedSensor),
    ] {
        let (first, second) = exact_transition(first_sensor, second_sensor);
        assert_eq!(first.len(), 1);
        assert_eq!(second.len(), 1);
        exact_results.push((
            first_sensor,
            second_sensor,
            second[0].contact_id == first[0].contact_id
                && second[0].manifold_id == first[0].manifold_id
                && second[0].lifecycle_reason == ContactLifecycleReason::ExactFeature
                && second[0].warm_start_reason == expected_warm_reason
                && second[0].warm_start_normal_impulse == 0.0
                && second[0].warm_start_tangent_impulse == 0.0,
            second[0],
        ));
    }
    assert!(
        role_swap_results
            .iter()
            .all(|(_, _, accepted, _)| *accepted)
            && exact_results
                .iter()
                .all(|(_, _, accepted, _)| *accepted),
        "sensor matrix must preserve exact identity without broadening edge-swap identity; edge={:?}, exact={:?}",
        role_swap_results
            .iter()
            .map(|(first_sensor, second_sensor, accepted, contacts)| (
                first_sensor,
                second_sensor,
                accepted,
                contacts
                    .iter()
                    .map(|contact| (
                        contact.lifecycle_reason,
                        contact.warm_start_reason,
                        contact.contact_id,
                        contact.manifold_id,
                    ))
                    .collect::<Vec<_>>(),
            ))
            .collect::<Vec<_>>(),
        exact_results
            .iter()
            .map(|(first_sensor, second_sensor, accepted, contact)| (
                first_sensor,
                second_sensor,
                accepted,
                contact.lifecycle_reason,
                contact.warm_start_reason,
                contact.contact_id,
                contact.manifold_id,
            ))
            .collect::<Vec<_>>()
    );
}

#[test]
fn manifold_persistence_normalizes_geometric_a_b_order_with_revisions() {
    fn run_fixture(patched_is_min: bool) -> (bool, bool, Vec<ContactEvent>) {
        let mut world = no_gravity_world();
        let lower = create_body(&mut world, BodyType::Static, 0.0, 0.0, Vector::default());
        let upper = create_body(&mut world, BodyType::Static, -0.25, 1.8, Vector::default());
        let (lower_collider, upper_collider) = if patched_is_min {
            let upper_collider = attach_shape(
                &mut world,
                upper,
                SharedShape::rect(2.0, 2.0),
                Material::default(),
            );
            let lower_collider = attach_shape(
                &mut world,
                lower,
                SharedShape::rect(2.0, 2.0),
                Material::default(),
            );
            (lower_collider, upper_collider)
        } else {
            let lower_collider = attach_shape(
                &mut world,
                lower,
                SharedShape::rect(2.0, 2.0),
                Material::default(),
            );
            let upper_collider = attach_shape(
                &mut world,
                upper,
                SharedShape::rect(2.0, 2.0),
                Material::default(),
            );
            (lower_collider, upper_collider)
        };
        assert_eq!(
            upper_collider == lower_collider.min(upper_collider),
            patched_is_min
        );
        world
            .apply_collider_patch(
                upper_collider,
                ColliderPatch {
                    local_pose: Some(Pose::default()),
                    ..ColliderPatch::default()
                },
            )
            .expect("pre-contact revision patch should succeed");
        let lower_pose = Pose::from_xy_angle(0.0, 0.0, -0.12);
        let first_upper_pose = Pose::from_xy_angle(-0.25, 1.8, -0.17);
        let second_upper_pose = Pose::from_xy_angle(-0.25, 1.8, -0.15);
        world
            .apply_body_patch(
                lower,
                BodyPatch {
                    pose: Some(lower_pose),
                    wake: true,
                    ..BodyPatch::default()
                },
            )
            .expect("lower pose should be patched");
        world
            .apply_body_patch(
                upper,
                BodyPatch {
                    pose: Some(first_upper_pose),
                    wake: true,
                    ..BodyPatch::default()
                },
            )
            .expect("first upper pose should be patched");
        let mut pipeline = SimulationPipeline::new(fixed_step_config());
        let first = active_contact_events(&pipeline.step(&mut world));
        world
            .apply_body_patch(
                upper,
                BodyPatch {
                    pose: Some(second_upper_pose),
                    wake: true,
                    ..BodyPatch::default()
                },
            )
            .expect("upper pose should cross the SAT role swap");
        let second = active_contact_events(&pipeline.step(&mut world));
        assert_eq!(first.len(), 2);
        assert_eq!(second.len(), 2);
        let (first_pose_a, first_pose_b, second_pose_a, second_pose_b) =
            if lower_collider < upper_collider {
                (lower_pose, first_upper_pose, lower_pose, second_upper_pose)
            } else {
                (first_upper_pose, lower_pose, second_upper_pose, lower_pose)
            };
        let correspondence = unique_two_point_local_witness_correspondence(
            &first,
            first_pose_a,
            first_pose_b,
            &second,
            second_pose_a,
            second_pose_b,
        );
        let role_swap_continuity = correspondence.iter().all(|(first_index, second_index)| {
            second[*second_index].contact_id == first[*first_index].contact_id
                && second[*second_index].manifold_id == first[*first_index].manifold_id
        });

        let previous_contact_ids = second
            .iter()
            .map(|contact| contact.contact_id)
            .collect::<BTreeSet<_>>();
        let previous_manifold_ids = second
            .iter()
            .map(|contact| contact.manifold_id)
            .collect::<BTreeSet<_>>();
        world
            .apply_collider_patch(
                upper_collider,
                ColliderPatch {
                    local_pose: Some(Pose::from_xy_angle(0.01, 0.0, 0.0)),
                    ..ColliderPatch::default()
                },
            )
            .expect("post-continuity revision patch should succeed");
        let after_patch = active_contact_events(&pipeline.step(&mut world));
        assert_eq!(after_patch.len(), 2);
        let current_contact_ids = after_patch
            .iter()
            .map(|contact| contact.contact_id)
            .collect::<BTreeSet<_>>();
        let current_manifold_ids = after_patch
            .iter()
            .map(|contact| contact.manifold_id)
            .collect::<BTreeSet<_>>();
        let patch_invalidated = previous_contact_ids.is_disjoint(&current_contact_ids)
            && previous_manifold_ids.is_disjoint(&current_manifold_ids)
            && after_patch.iter().all(|contact| {
                contact.lifecycle_reason == ContactLifecycleReason::Started
                    && contact.warm_start_reason == WarmStartCacheReason::MissFeatureId
                    && !contact.source_row_continuity_candidate
                    && contact.source_row_continuity_reason != SourceRowContinuityReason::Candidate
            });
        (role_swap_continuity, patch_invalidated, after_patch)
    }

    let fixtures = [
        ("patched_min", run_fixture(true)),
        ("patched_max", run_fixture(false)),
    ];
    assert!(
        fixtures
            .iter()
            .all(|(_, (continuity, invalidated, _))| *continuity && *invalidated),
        "ordered revisions must survive SAT role swaps and invalidate either ordered side after patch; results={:?}",
        fixtures
            .iter()
            .map(|(label, (continuity, invalidated, contacts))| (
                label,
                continuity,
                invalidated,
                contacts
                    .iter()
                    .map(|contact| (
                        contact.lifecycle_reason,
                        contact.warm_start_reason,
                        contact.source_row_continuity_candidate,
                        contact.source_row_continuity_reason,
                    ))
                    .collect::<Vec<_>>(),
            ))
            .collect::<Vec<_>>()
    );
}

#[test]
fn manifold_persistence_two_to_one_to_two_preserves_only_surviving_point() {
    let mut world = no_gravity_world();
    let left = create_body(&mut world, BodyType::Static, 0.0, 0.0, Vector::default());
    let first_right_pose = Pose::from_xy_angle(1.7, 0.0, 0.0);
    let single_right_pose = Pose::from_xy_angle(1.9, 0.0, 0.2);
    let right = world
        .create_body(BodyDesc {
            body_type: BodyType::Static,
            pose: first_right_pose,
            can_sleep: false,
            ..BodyDesc::default()
        })
        .expect("right body should be created");
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
    let left_pose = Pose::default();
    let mut pipeline = SimulationPipeline::new(fixed_step_config());

    let first_report = pipeline.step(&mut world);
    let first = active_contact_events(&first_report);
    assert_eq!(first.len(), 2);
    assert_eq!(
        first
            .iter()
            .map(|contact| contact.manifold_id)
            .collect::<BTreeSet<_>>()
            .len(),
        1
    );
    world
        .apply_body_patch(
            right,
            BodyPatch {
                pose: Some(single_right_pose),
                wake: true,
                ..BodyPatch::default()
            },
        )
        .expect("right body should move to the one-point pose");
    let second_report = pipeline.step(&mut world);
    let second = active_contact_events(&second_report);
    assert_eq!(second.len(), 1);
    assert_eq!(
        second_report
            .events
            .iter()
            .filter(|event| matches!(event, WorldEvent::ContactPersisted(_)))
            .count(),
        1
    );
    assert_eq!(
        second_report
            .events
            .iter()
            .filter(|event| matches!(event, WorldEvent::ContactStarted(_)))
            .count(),
        0
    );
    let second_contact = second[0];
    let first_drifts = first
        .iter()
        .map(|contact| {
            dual_local_witness_drift(
                contact,
                left_pose,
                first_right_pose,
                &second_contact,
                left_pose,
                single_right_pose,
            )
        })
        .collect::<Vec<_>>();
    let surviving_first_index = if first_drifts[0] < first_drifts[1] {
        0
    } else {
        1
    };
    assert_ne!(first_drifts[0], first_drifts[1]);
    assert!(first_drifts[surviving_first_index] < 0.25);
    let ended_first_index = 1 - surviving_first_index;
    assert_eq!(
        second_contact.contact_id,
        first[surviving_first_index].contact_id
    );
    assert_eq!(
        second_contact.manifold_id,
        first[surviving_first_index].manifold_id
    );
    assert_eq!(
        second_contact.lifecycle_reason,
        ContactLifecycleReason::ExactFeature
    );
    let ended = second_report
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::ContactEnded(contact) => Some(*contact),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(ended.len(), 1);
    assert_eq!(ended[0].contact_id, first[ended_first_index].contact_id);

    world
        .apply_body_patch(
            right,
            BodyPatch {
                pose: Some(first_right_pose),
                wake: true,
                ..BodyPatch::default()
            },
        )
        .expect("right body should return to the two-point pose");
    let third_report = pipeline.step(&mut world);
    let third = active_contact_events(&third_report);
    assert_eq!(third.len(), 2);
    let third_drifts = third
        .iter()
        .map(|contact| {
            dual_local_witness_drift(
                &second_contact,
                left_pose,
                single_right_pose,
                contact,
                left_pose,
                first_right_pose,
            )
        })
        .collect::<Vec<_>>();
    let surviving_third_index = if third_drifts[0] < third_drifts[1] {
        0
    } else {
        1
    };
    assert_ne!(third_drifts[0], third_drifts[1]);
    let returning_third_index = 1 - surviving_third_index;
    let surviving = third[surviving_third_index];
    let returning = third[returning_third_index];
    assert_eq!(surviving.contact_id, second_contact.contact_id);
    assert_eq!(surviving.manifold_id, second_contact.manifold_id);
    assert_ne!(returning.contact_id, first[ended_first_index].contact_id);
    assert_ne!(returning.contact_id, surviving.contact_id);
    assert_eq!(returning.manifold_id, surviving.manifold_id);
    assert_eq!(returning.lifecycle_reason, ContactLifecycleReason::Started);
    assert_ne!(returning.warm_start_reason, WarmStartCacheReason::Hit);
    assert_eq!(returning.warm_start_normal_impulse, 0.0);
    assert_eq!(returning.warm_start_tangent_impulse, 0.0);
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
fn warm_start_cache_survives_small_tangential_slip_on_same_face() {
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
                pose: Some(Pose::from_xy_angle(1.5, 0.08, 0.0)),
                wake: true,
                ..BodyPatch::default()
            },
        )
        .expect("body should slide a small amount along the same contact face");
    let second = pipeline.step(&mut world);
    let second_contacts = active_contact_events(&second);

    assert_eq!(first.stats.contact_count, 2);
    assert_eq!(second.stats.contact_count, 2);
    assert!(
        second_contacts
            .iter()
            .all(|contact| contact.warm_start_reason == WarmStartCacheReason::Hit),
        "small same-face tangential slip should not drop trusted warm-start points: {second_contacts:?}"
    );
    assert_eq!(second.stats.warm_start_hit_count, second_contacts.len());
    assert_eq!(second.stats.warm_start_drop_count, 0);
}

#[test]
fn warm_start_cache_transfers_tangent_impulse_across_small_tangential_slip() {
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
    let mut pipeline = SimulationPipeline::new(StepConfig {
        velocity_iterations: 10,
        position_iterations: 0,
        ..fixed_step_config()
    });

    let first = pipeline.step(&mut world);
    let first_contact = active_contact_events(&first)
        .into_iter()
        .max_by(|a, b| {
            a.solver_tangent_impulse
                .abs()
                .partial_cmp(&b.solver_tangent_impulse.abs())
                .unwrap()
        })
        .expect("first step should solve a sliding contact");
    assert!(
        first_contact.solver_tangent_impulse.abs() > 1.0e-4,
        "first step should cache a real friction impulse: {first_contact:?}"
    );

    let slider_position = body_position(&world, slider);
    world
        .apply_body_patch(
            slider,
            BodyPatch {
                pose: Some(Pose::from_xy_angle(
                    slider_position.x() + 0.08,
                    slider_position.y(),
                    0.0,
                )),
                linear_velocity: Some(Vector::default()),
                wake: true,
                ..BodyPatch::default()
            },
        )
        .expect("slider should move a small tangential amount along the same face");
    let second = pipeline.step(&mut world);
    let second_contact = active_contact_events(&second)
        .into_iter()
        .max_by(|a, b| {
            a.warm_start_tangent_impulse
                .abs()
                .partial_cmp(&b.warm_start_tangent_impulse.abs())
                .unwrap()
        })
        .expect("second step should keep the sliding contact");

    assert_eq!(second_contact.warm_start_reason, WarmStartCacheReason::Hit);
    assert!(
        second_contact.warm_start_tangent_impulse.abs() > 1.0e-4,
        "sub-threshold tangential slip should transfer the cached tangent impulse: {second_contact:?}"
    );
    assert_eq!(second.stats.warm_start_drop_count, 0);
}

#[test]
fn warm_start_cache_drops_tangent_impulse_after_large_tangential_slip() {
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
    let mut pipeline = SimulationPipeline::new(StepConfig {
        velocity_iterations: 10,
        position_iterations: 0,
        ..fixed_step_config()
    });

    let first = pipeline.step(&mut world);
    assert!(
        active_contact_events(&first)
            .iter()
            .any(|contact| contact.solver_tangent_impulse.abs() > 1.0e-4),
        "first step should cache a real friction impulse"
    );

    let slider_position = body_position(&world, slider);
    world
        .apply_body_patch(
            slider,
            BodyPatch {
                pose: Some(Pose::from_xy_angle(
                    slider_position.x() + 0.20,
                    slider_position.y(),
                    0.0,
                )),
                linear_velocity: Some(Vector::default()),
                wake: true,
                ..BodyPatch::default()
            },
        )
        .expect("slider should move beyond the trusted tangential drift window");
    let second = pipeline.step(&mut world);
    let second_contacts = active_contact_events(&second);

    assert!(
        second_contacts
            .iter()
            .any(|contact| contact.warm_start_reason == WarmStartCacheReason::DroppedPointDrift),
        "over-threshold curved tangential slip should drop cached impulses: {second_contacts:?}"
    );
    assert!(
        second_contacts
            .iter()
            .all(|contact| contact.warm_start_tangent_impulse == 0.0),
        "dropped drift should not expose stale tangent impulses: {second_contacts:?}"
    );
    assert!(second.stats.warm_start_drop_count > 0);
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
fn ramp_friction_suppresses_downhill_sliding_more_than_low_friction() {
    // In this setup the actual downhill motion projects onto the ramp's negative tangent, so we
    // lock assertions to that signed downhill vector instead of erasing direction with `abs()`.
    fn downhill_progress_after_contact(friction: f32) -> (f32, f32) {
        let mut world = World::new(WorldDesc {
            gravity: Vector::new(0.0, 9.81),
            enable_sleep: false,
        });
        let ramp_angle: f32 = 0.4;
        let ramp_tangent = Vector::new(ramp_angle.cos(), ramp_angle.sin());
        let downhill_tangent = -ramp_tangent;
        let surface_normal = Vector::new(ramp_angle.sin(), -ramp_angle.cos());
        let ramp_origin = Vector::new(0.0, 1.0);
        let slider_origin = ramp_origin + ramp_tangent * -1.5 + surface_normal * 0.73;
        let material = Material {
            friction,
            restitution: 0.0,
        };

        let ramp = world
            .create_body(BodyDesc {
                body_type: BodyType::Static,
                pose: Pose::from_xy_angle(ramp_origin.x(), ramp_origin.y(), ramp_angle),
                can_sleep: false,
                ..BodyDesc::default()
            })
            .expect("ramp should be created");
        let slider = world
            .create_body(BodyDesc {
                body_type: BodyType::Dynamic,
                pose: Pose::from_xy_angle(slider_origin.x(), slider_origin.y(), 0.0),
                can_sleep: false,
                ..BodyDesc::default()
            })
            .expect("slider should be created");
        attach_shape(&mut world, ramp, SharedShape::rect(8.0, 0.5), material);
        attach_shape(&mut world, slider, SharedShape::circle(0.5), material);

        step_world(&mut world, 90);

        let downhill_progress =
            (body_position(&world, slider) - slider_origin).dot(downhill_tangent);
        let downhill_speed = body_velocity(&world, slider).dot(downhill_tangent);
        (downhill_progress, downhill_speed)
    }

    let (low_downhill_progress, low_downhill_speed) = downhill_progress_after_contact(0.05);
    let (high_downhill_progress, high_downhill_speed) = downhill_progress_after_contact(1.2);

    assert!(
        low_downhill_progress > 0.0,
        "low-friction slider should still move downhill; progress={low_downhill_progress}"
    );
    assert!(
        high_downhill_progress > 0.0,
        "high-friction slider should still move downhill, just less; progress={high_downhill_progress}"
    );
    assert!(
        low_downhill_speed > 0.0,
        "low-friction slider should still have downhill speed; speed={low_downhill_speed}"
    );
    assert!(
        high_downhill_speed > 0.0,
        "high-friction slider should still have downhill speed, just less; speed={high_downhill_speed}"
    );
    assert!(
        low_downhill_progress > high_downhill_progress + 0.2,
        "low-friction ramp should allow more downhill travel; low={low_downhill_progress}, high={high_downhill_progress}"
    );
    assert!(
        low_downhill_speed > high_downhill_speed + 0.2,
        "low-friction ramp should retain more downhill speed; low={low_downhill_speed}, high={high_downhill_speed}"
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
fn two_point_sliding_face_contact_keeps_friction_torque_balanced() {
    // A centered box sliding across a flat face should get friction without a
    // large artificial spin. This locks the manifold-level behavior that dense
    // stacks need before E4 can safely retain shallow dynamic support rows.
    let mut world = World::new(WorldDesc {
        gravity: Vector::default(),
        enable_sleep: false,
    });
    let floor = create_body(&mut world, BodyType::Static, 0.0, 0.5, Vector::default());
    let box_body = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(0.0, -0.45, 0.006),
            linear_velocity: Vector::new(6.0, 3.0),
            can_sleep: false,
            ..BodyDesc::default()
        })
        .expect("box should be created");
    let material = Material {
        friction: 1.0,
        restitution: 0.0,
    };
    attach_shape(&mut world, floor, SharedShape::rect(10.0, 1.0), material);
    attach_shape(&mut world, box_body, SharedShape::rect(1.0, 1.0), material);

    let report = step_world_with_config(
        &mut world,
        StepConfig {
            velocity_iterations: 10,
            position_iterations: 4,
            ..fixed_step_config()
        },
        1,
    );
    let contacts = active_contact_events(&report);

    assert!(
        contacts.len() >= 2,
        "centered face contact should expose a two-point manifold: {contacts:?}"
    );
    assert!(
        body_velocity(&world, box_body).x().abs() < 6.0,
        "friction should still reduce tangential sliding speed; velocity={:?}",
        body_velocity(&world, box_body)
    );
    assert!(
        body_angular_velocity(&world, box_body).abs() <= 0.02,
        "two-point face friction should not inject large artificial spin; angular_velocity={}, contacts={contacts:?}",
        body_angular_velocity(&world, box_body)
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
fn contact_position_correction_does_not_double_apply_face_manifold_points() {
    // A face manifold usually exports two contact points. Residual position
    // correction should treat those points as one geometric overlap for pair
    // separation; applying the full correction once per point injects extra
    // motion into stacks and churns the next frame's contact identity.
    let mut world = no_gravity_world();
    let floor = create_body(&mut world, BodyType::Static, 0.0, 0.5, Vector::default());
    let box_body = create_body(&mut world, BodyType::Dynamic, 0.0, -0.45, Vector::default());
    let material = Material {
        friction: 0.0,
        restitution: 0.0,
    };
    attach_shape(&mut world, floor, SharedShape::rect(10.0, 1.0), material);
    attach_shape(&mut world, box_body, SharedShape::rect(1.0, 1.0), material);
    let start_y = body_position(&world, box_body).y();

    let report = step_world_with_config(
        &mut world,
        StepConfig {
            velocity_iterations: 0,
            position_iterations: 1,
            ..fixed_step_config()
        },
        1,
    );
    let contacts = active_contact_events(&report);
    let end_y = body_position(&world, box_body).y();
    let correction_distance = (end_y - start_y).abs();

    assert!(
        contacts.len() >= 2,
        "fixture should expose a two-point face manifold: {contacts:?}"
    );
    assert!(
        correction_distance <= 0.09,
        "one frame should not apply an obviously unbounded correction per manifold point; moved {correction_distance}, contacts={contacts:?}"
    );
    assert!(
        report.stats.position_correction_max_translation <= 0.09,
        "position correction stats should report the bounded dynamic-body translation; stats={:?}",
        report.stats
    );
}

#[test]
fn contact_position_correction_reports_input_depth_and_applied_translation() {
    // E3 observability lock: the contact manifold depth is gathered before
    // residual position correction mutates poses. Solver tuning needs both the
    // pre-correction overlap and the actual pose translation to avoid mistaking
    // stale manifold facts for post-correction geometry.
    let mut world = no_gravity_world();
    let floor = create_body(&mut world, BodyType::Static, 0.0, 0.5, Vector::default());
    let box_body = create_body(&mut world, BodyType::Dynamic, 0.0, -0.45, Vector::default());
    let material = Material {
        friction: 0.0,
        restitution: 0.0,
    };
    attach_shape(&mut world, floor, SharedShape::rect(10.0, 1.0), material);
    attach_shape(&mut world, box_body, SharedShape::rect(1.0, 1.0), material);
    let start_y = body_position(&world, box_body).y();

    let report = step_world_with_config(
        &mut world,
        StepConfig {
            velocity_iterations: 0,
            position_iterations: 1,
            ..fixed_step_config()
        },
        1,
    );
    let correction_contacts = active_contact_events(&report);
    let end_y = body_position(&world, box_body).y();
    let correction_distance = (end_y - start_y).abs();

    assert!(
        report.stats.position_correction_input_max_depth > 0.04,
        "position correction should report the pre-correction overlap that drove the solve"
    );
    assert_eq!(
        report.stats.position_correction_body_count, 1,
        "fixture has only one dynamic body eligible for residual position correction"
    );
    assert!(
        (report.stats.position_correction_max_translation - correction_distance).abs() < 1.0e-5,
        "reported correction translation should match the actual body pose delta"
    );
    assert!(
        report.stats.position_correction_total_translation + 1.0e-5 >= correction_distance,
        "total correction should include the applied dynamic body translation; reported={}, actual={}",
        report.stats.position_correction_total_translation,
        correction_distance
    );
    assert!(
        correction_contacts
            .iter()
            .any(|contact| contact.solver_position_correction_depth > 0.0),
        "contact events should expose row-level residual correction depth: {correction_contacts:?}"
    );
    assert!(
        correction_contacts.iter().any(|contact| {
            contact.solver_position_correction_body_a_translation > 0.0
                || contact.solver_position_correction_body_b_translation > 0.0
        }),
        "contact events should expose per-body correction translation: {correction_contacts:?}"
    );
}

#[test]
fn contact_position_correction_dense_stack_re_evaluates_row_depth_on_step_surface() {
    // E3 dense behavior lock: a real step must cross the dense correction
    // threshold, and the row-level correction facts should show that later
    // rows consumed less pseudo-depth than an unre-evaluated stale pass would.
    let (mut world, bodies) = build_dense_position_correction_stack_world(8);
    let start_positions = bodies
        .iter()
        .map(|&body| body_position(&world, body))
        .collect::<Vec<_>>();

    let report = step_world_with_config(
        &mut world,
        StepConfig {
            velocity_iterations: 0,
            position_iterations: 4,
            ..fixed_step_config()
        },
        1,
    );
    let correction_contacts = active_contact_events(&report)
        .into_iter()
        .filter(|contact| contact.solver_position_correction_depth > 0.0)
        .collect::<Vec<_>>();
    let moved_distances = bodies
        .iter()
        .zip(start_positions.iter())
        .map(|(&body, &start)| (body_position(&world, body) - start).length())
        .collect::<Vec<_>>();
    let min_input_depth = correction_contacts
        .iter()
        .map(|contact| contact.depth)
        .fold(f32::INFINITY, f32::min);
    let max_input_depth = correction_contacts
        .iter()
        .map(|contact| contact.depth)
        .fold(f32::NEG_INFINITY, f32::max);
    let min_consumed_depth = correction_contacts
        .iter()
        .map(|contact| contact.solver_position_correction_depth)
        .fold(f32::INFINITY, f32::min);
    let max_consumed_depth = correction_contacts
        .iter()
        .map(|contact| contact.solver_position_correction_depth)
        .fold(f32::NEG_INFINITY, f32::max);
    let moved_body_count = moved_distances
        .iter()
        .filter(|distance| **distance > 1.0e-4)
        .count();
    let max_linear_speed = bodies
        .iter()
        .map(|&body| body_velocity(&world, body).length())
        .fold(0.0_f32, f32::max);

    assert!(
        report.stats.position_correction_input_contact_count >= 16,
        "fixture should cross the dense threshold on the real step surface; stats={:?}",
        report.stats
    );
    assert!(
        correction_contacts.len() >= 16,
        "dense stack should keep at least sixteen correcting contact rows in StepReport events: {correction_contacts:?}"
    );
    assert!(
        report.stats.contact_count >= correction_contacts.len(),
        "StepReport contact count should cover the correcting rows; stats={:?}, contacts={correction_contacts:?}",
        report.stats
    );
    assert!(
        max_input_depth - min_input_depth <= 0.02,
        "fixture should start from a near-uniform overlap so row-depth spread comes from the solve, not uneven setup; min={min_input_depth}, max={max_input_depth}, contacts={correction_contacts:?}"
    );
    assert!(
        max_consumed_depth > min_consumed_depth + 0.02,
        "dense pseudo-depth re-evaluation should make later rows consume materially less correction depth than earlier rows; min={min_consumed_depth}, max={max_consumed_depth}, contacts={correction_contacts:?}"
    );
    assert!(
        report.stats.position_correction_total_translation
            < report.stats.position_correction_input_total_depth * 0.78,
        "dense correction should stay below a stale raw-depth over-correction budget; stats={:?}",
        report.stats
    );
    assert!(
        moved_body_count >= 6,
        "dense position correction should move most dynamic layers through queued pose updates; moved={moved_distances:?}"
    );
    assert!(
        max_linear_speed <= 1.0e-5,
        "position-only dense correction should not inject linear velocity when velocity iterations are disabled; max_speed={max_linear_speed}"
    );
    assert!(
        report.stats.position_correction_total_translation
            > moved_distances.iter().sum::<f32>() * 3.0,
        "dense row correction work should materially exceed the final net pose drift because the same bodies participate in multiple stack rows; stats={:?}, moved={moved_distances:?}",
        report.stats
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
fn sleep_accepts_the_d2_quiet_window_but_rejects_faster_motion() {
    let mut quiet_world = no_gravity_world();
    let quiet_body = quiet_world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            linear_velocity: Vector::new(0.02, 0.0),
            angular_velocity: 0.04,
            can_sleep: true,
            ..BodyDesc::default()
        })
        .expect("quiet body should be created");

    step_world(&mut quiet_world, 31);

    assert!(
        quiet_world
            .try_body(quiet_body)
            .expect("quiet body should exist")
            .sleeping(),
        "D2 quiet-window motion should be eligible for sleep after the stability window"
    );

    let mut moving_world = no_gravity_world();
    let moving_body = moving_world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            linear_velocity: Vector::new(0.08, 0.0),
            angular_velocity: 0.12,
            can_sleep: true,
            ..BodyDesc::default()
        })
        .expect("moving body should be created");

    step_world(&mut moving_world, 31);

    assert!(
        !moving_world
            .try_body(moving_body)
            .expect("moving body should exist")
            .sleeping(),
        "motion above the D2 quiet window should not be force-slept"
    );
}

#[test]
fn resting_static_contact_can_accumulate_sleep_window() {
    // E5 behavior lock: a genuinely low-motion resting contact should be able
    // to accumulate the island sleep window. Position correction may keep tiny
    // overlaps bounded, but it must not behave like a perpetual external wake.
    let mut world = World::new(WorldDesc {
        gravity: Vector::new(0.0, 9.8),
        enable_sleep: true,
    });
    let floor = create_body(&mut world, BodyType::Static, 0.0, 0.5, Vector::default());
    let body = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(0.0, -0.5, 0.0),
            can_sleep: true,
            ..BodyDesc::default()
        })
        .expect("resting body should be created");
    attach_shape(
        &mut world,
        floor,
        SharedShape::rect(10.0, 1.0),
        Material::default(),
    );
    attach_shape(
        &mut world,
        body,
        SharedShape::rect(1.0, 1.0),
        Material::default(),
    );

    step_world(&mut world, 90);

    assert!(
        world.try_body(body).expect("body should exist").sleeping(),
        "resting static contact should sleep once the stability window has elapsed"
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
fn step_broadphase_counters_report_traversal_and_prune_work() {
    let mut world = no_gravity_world();
    let near_left = create_body(&mut world, BodyType::Static, 0.0, 0.0, Vector::default());
    let near_right = create_body(&mut world, BodyType::Static, 1.5, 0.0, Vector::default());
    let far = create_body(&mut world, BodyType::Static, 20.0, 20.0, Vector::default());
    for body in [near_left, near_right, far] {
        attach_shape(
            &mut world,
            body,
            SharedShape::circle(1.0),
            Material::default(),
        );
    }

    let report = step_world(&mut world, 1);

    assert_eq!(report.stats.broadphase_candidate_count, 1);
    assert_eq!(report.stats.broadphase_traversal_count, 4);
    assert_eq!(report.stats.broadphase_pruned_count, 2);
    assert_eq!(report.stats.contact_count, 1);

    let snapshot = DebugSnapshot::from_world_with_step_report(
        &world,
        &report,
        &DebugSnapshotOptions::default(),
    );
    assert_eq!(
        snapshot.stats.broadphase_traversal_count,
        report.stats.broadphase_traversal_count
    );
    assert_eq!(
        snapshot.stats.broadphase_pruned_count,
        report.stats.broadphase_pruned_count
    );
}

#[test]
fn joint_damping_reduces_peak_radial_speed_vs_zero_damping() {
    fn peak_post_step_radial_speed(damping: f32) -> f32 {
        let mut world = no_gravity_world();
        let body = create_body(
            &mut world,
            BodyType::Dynamic,
            2.0,
            0.0,
            Vector::new(6.0, 0.0),
        );
        attach_shape(
            &mut world,
            body,
            SharedShape::rect(1.0, 1.0),
            Material::default(),
        );
        world
            .create_joint(JointDesc::WorldAnchor(WorldAnchorJointDesc {
                body,
                local_anchor: Point::default(),
                world_anchor: Point::default(),
                stiffness: 4.0,
                damping,
                ..WorldAnchorJointDesc::default()
            }))
            .expect("world-anchor joint should be created");

        let mut pipeline = SimulationPipeline::new(StepConfig {
            dt: DT,
            joint_velocity_projection: false,
            ..StepConfig::default()
        });
        let mut peak = 0.0_f32;
        for _ in 0..12 {
            pipeline.step(&mut world);
            let position = body_position(&world, body);
            let radial_axis = Vector::new(-position.x(), -position.y()).normalized_or_zero();
            assert!(
                radial_axis.length() > f32::EPSILON,
                "body anchor should remain separated from the fixed world anchor"
            );
            let radial_speed = body_velocity(&world, body).dot(radial_axis).abs();
            assert!(radial_speed.is_finite(), "radial speed must stay finite");
            peak = peak.max(radial_speed);
        }
        peak
    }

    let zero_damping_peak = peak_post_step_radial_speed(0.0);
    let high_damping_peak = peak_post_step_radial_speed(30.0);

    assert!(
        high_damping_peak < zero_damping_peak * 0.75,
        "high world-anchor damping should materially reduce post-step radial speed: \
         zero_damping_peak={zero_damping_peak}, high_damping_peak={high_damping_peak}"
    );
}

#[test]
fn world_anchor_damping_reduces_offset_anchor_radial_speed_from_rotation() {
    fn post_step_anchor_radial_speed(damping: f32) -> f32 {
        let mut world = no_gravity_world();
        let body = world
            .create_body(BodyDesc {
                body_type: BodyType::Dynamic,
                angular_velocity: 6.0,
                can_sleep: false,
                ..BodyDesc::default()
            })
            .expect("rotating body should be created");
        attach_shape(
            &mut world,
            body,
            SharedShape::rect(2.0, 2.0),
            Material::default(),
        );
        let local_anchor = Point::new(1.0, 0.0);
        let world_anchor = Point::new(1.0, 0.0);
        world
            .create_joint(JointDesc::WorldAnchor(WorldAnchorJointDesc {
                body,
                local_anchor,
                world_anchor,
                stiffness: 0.0,
                damping,
                ..WorldAnchorJointDesc::default()
            }))
            .expect("world-anchor joint should be created");

        let mut pipeline = SimulationPipeline::new(StepConfig {
            dt: DT,
            joint_velocity_projection: false,
            ..StepConfig::default()
        });
        pipeline.step(&mut world);

        let view = world.try_body(body).expect("body should still exist");
        let pose = view.pose();
        let anchor = pose.transform_point(local_anchor);
        let offset = anchor - pose.point();
        // Picea uses clockwise-positive angular velocity in screen space.
        let anchor_velocity = view.linear_velocity()
            + Vector::new(
                view.angular_velocity() * offset.y(),
                -view.angular_velocity() * offset.x(),
            );
        let axis = (world_anchor - anchor).normalized_or_zero();
        assert!(
            axis.length() > f32::EPSILON,
            "integration should separate the rotating local anchor from its fixed target"
        );
        let radial_speed = anchor_velocity.dot(axis).abs();
        assert!(radial_speed.is_finite(), "anchor speed must stay finite");
        radial_speed
    }

    let zero_damping_speed = post_step_anchor_radial_speed(0.0);
    let high_damping_speed = post_step_anchor_radial_speed(30.0);

    assert!(
        high_damping_speed < zero_damping_speed * 0.75,
        "high damping should reduce radial point velocity at an offset anchor: \
         zero_damping_speed={zero_damping_speed}, high_damping_speed={high_damping_speed}"
    );
}

#[test]
fn world_anchor_damping_uses_clamped_per_step_strength_and_preserves_tangent() {
    fn post_step_velocity(damping: f32) -> (Vector, Vector) {
        let initial_velocity = Vector::new(6.0, 3.0);
        let mut world = no_gravity_world();
        let body = create_body(&mut world, BodyType::Dynamic, 2.0, 0.0, initial_velocity);
        attach_shape(
            &mut world,
            body,
            SharedShape::rect(1.0, 1.0),
            Material::default(),
        );
        let world_anchor = Point::new(0.0, initial_velocity.y() * DT);
        world
            .create_joint(JointDesc::WorldAnchor(WorldAnchorJointDesc {
                body,
                world_anchor,
                stiffness: 0.0,
                damping,
                ..WorldAnchorJointDesc::default()
            }))
            .expect("world-anchor joint should be created");

        let mut pipeline = SimulationPipeline::new(StepConfig {
            dt: DT,
            joint_velocity_projection: false,
            ..StepConfig::default()
        });
        pipeline.step(&mut world);

        let axis = (world_anchor
            - world
                .try_body(body)
                .expect("body should still exist")
                .pose()
                .point())
        .normalized_or_zero();
        assert!(axis.length() > f32::EPSILON);
        (body_velocity(&world, body), axis)
    }

    fn assert_near(actual: f32, expected: f32, label: &str) {
        let tolerance = 1.0e-5;
        assert!(
            (actual - expected).abs() <= tolerance,
            "{label}: expected {expected}, got {actual}"
        );
    }

    let (zero_velocity, axis) = post_step_velocity(0.0);
    let (half_velocity, half_axis) = post_step_velocity(30.0);
    let (clamped_velocity, clamped_axis) = post_step_velocity(120.0);
    let tangent = axis.perp();
    let zero_radial = zero_velocity.dot(axis);
    let zero_tangent = zero_velocity.dot(tangent);

    assert_near(zero_radial, -6.0, "zero damping radial velocity");
    assert_near(zero_tangent, 3.0, "zero damping tangent velocity");
    assert_near(
        half_velocity.dot(half_axis),
        zero_radial * (1.0 - (30.0 * DT).clamp(0.0, 1.0)),
        "damping=30 radial velocity",
    );
    assert_near(
        half_velocity.dot(half_axis.perp()),
        zero_tangent,
        "damping=30 tangent velocity",
    );
    assert_near(
        clamped_velocity.dot(clamped_axis),
        0.0,
        "clamped damping radial velocity",
    );
    assert_near(
        clamped_velocity.dot(clamped_axis.perp()),
        zero_tangent,
        "clamped damping tangent velocity",
    );
}

#[test]
fn world_anchor_damping_at_zero_length_has_no_axis_bias() {
    fn post_step_velocity(initial_velocity: Vector) -> Vector {
        let mut world = no_gravity_world();
        let body = create_body(
            &mut world,
            BodyType::Dynamic,
            -initial_velocity.x() * DT,
            -initial_velocity.y() * DT,
            initial_velocity,
        );
        attach_shape(
            &mut world,
            body,
            SharedShape::rect(1.0, 1.0),
            Material::default(),
        );
        world
            .create_joint(JointDesc::WorldAnchor(WorldAnchorJointDesc {
                body,
                stiffness: 0.0,
                damping: 120.0,
                ..WorldAnchorJointDesc::default()
            }))
            .expect("world-anchor joint should be created");

        let mut pipeline = SimulationPipeline::new(StepConfig {
            dt: DT,
            joint_velocity_projection: false,
            ..StepConfig::default()
        });
        pipeline.step(&mut world);

        let position = body_position(&world, body);
        assert!(
            position.length() <= 1.0e-6,
            "integration should place the local anchor exactly on the world anchor"
        );
        body_velocity(&world, body)
    }

    let x_velocity = Vector::new(6.0, 0.0);
    let y_velocity = Vector::new(0.0, 6.0);
    assert_eq!(
        post_step_velocity(x_velocity),
        x_velocity,
        "a coincident anchor has no x-axis damping direction"
    );
    assert_eq!(
        post_step_velocity(y_velocity),
        y_velocity,
        "a coincident anchor has no y-axis damping direction"
    );
}

#[test]
fn jointed_active_island_reports_joint_rows_without_contact_rows() {
    let mut world = no_gravity_world();
    let left = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(-0.5, 0.0, 0.0),
            can_sleep: false,
            ..BodyDesc::default()
        })
        .expect("left body should be created");
    let right = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(0.75, 0.0, 0.0),
            can_sleep: false,
            ..BodyDesc::default()
        })
        .expect("right body should be created");
    world
        .create_joint(JointDesc::Distance(DistanceJointDesc {
            body_a: left,
            body_b: right,
            rest_length: 1.0,
            stiffness: 4.0,
            damping: 0.25,
            ..DistanceJointDesc::default()
        }))
        .expect("distance joint should be created");

    let report = step_world(&mut world, 1);

    assert_eq!(report.stats.island_count, 1);
    assert_eq!(report.stats.active_island_count, 1);
    assert_eq!(report.stats.sleeping_island_skip_count, 0);
    assert_eq!(report.stats.solver_body_slot_count, 2);
    assert_eq!(report.stats.contact_row_count, 0);
    assert_eq!(report.stats.joint_row_count, 1);

    let snapshot = DebugSnapshot::from_world_with_step_report(
        &world,
        &report,
        &DebugSnapshotOptions::default(),
    );
    assert_eq!(snapshot.stats.joint_row_count, report.stats.joint_row_count);
    assert_eq!(
        snapshot.stats.contact_row_count,
        report.stats.contact_row_count
    );
}

#[test]
fn velocity_first_contact_uses_preintegrated_pose_before_final_position_integration() {
    let mut world = no_gravity_world();
    let target = create_body(&mut world, BodyType::Static, 0.0, 0.0, Vector::default());
    let mover = create_body(
        &mut world,
        BodyType::Dynamic,
        -1.1,
        0.0,
        Vector::new(12.0, 0.0),
    );
    let material = Material {
        friction: 0.0,
        restitution: 0.0,
    };
    // A static circle deliberately avoids the static-convex CCD clamp path. The
    // ordinary contact phase must still detect the overlap at the mover's
    // preintegrated pose without committing that pose to authoritative state.
    attach_shape(&mut world, target, SharedShape::circle(0.5), material);
    attach_shape(&mut world, mover, SharedShape::circle(0.5), material);
    let report = step_world_with_config(
        &mut world,
        StepConfig {
            velocity_iterations: 10,
            position_iterations: 0,
            ..fixed_step_config()
        },
        1,
    );

    assert!(
        report.events.iter().any(|event| matches!(
            event,
            WorldEvent::ContactStarted(_) | WorldEvent::ContactPersisted(_)
        )),
        "velocity-first contact generation must see the ordinary body's preintegrated pose in the same frame"
    );
    let solved_velocity = body_velocity(&world, mover).x();
    assert!(
        solved_velocity < 6.0,
        "contact velocity solve must substantially reduce the incoming speed before final position integration; vx={solved_velocity}"
    );
    let final_x = body_position(&world, mover).x();
    assert!(
        final_x < -1.0,
        "final position must integrate the solved velocity instead of advancing to the unsolved x=-0.9 pose; x={final_x}"
    );
}

#[test]
fn speculative_contact_does_not_apply_predicted_depth_as_position_correction() {
    let mut world = no_gravity_world();
    let target = create_body(&mut world, BodyType::Static, 0.0, 0.0, Vector::default());
    let mover = create_body(
        &mut world,
        BodyType::Dynamic,
        -1.1,
        0.0,
        Vector::new(12.0, 0.0),
    );
    let material = Material {
        friction: 0.0,
        restitution: 0.0,
    };
    // Circle versus circle bypasses the static-convex CCD clamp, isolating the
    // speculative contact path while the authoritative poses remain separated.
    attach_shape(&mut world, target, SharedShape::circle(0.5), material);
    attach_shape(&mut world, mover, SharedShape::circle(0.5), material);

    let report = step_world_with_config(
        &mut world,
        StepConfig {
            velocity_iterations: 0,
            position_iterations: 1,
            ..fixed_step_config()
        },
        1,
    );

    assert!(
        report.stats.contact_row_count > 0,
        "the predicted overlap should still construct a speculative velocity row"
    );
    assert_eq!(
        report.stats.position_correction_input_contact_count, 0,
        "predicted overlap depth must not enter authoritative residual position correction"
    );
    assert_eq!(
        report.stats.position_correction_body_count, 0,
        "separated authoritative poses must not be translated by residual correction"
    );
    assert!(
        report.stats.position_correction_total_translation.abs() <= 1.0e-6,
        "speculative depth must not move authoritative poses; stats={:?}",
        report.stats
    );
}

#[test]
fn contact_event_geometry_matches_final_authoritative_circle_poses() {
    let mut world = no_gravity_world();
    let target = create_body(&mut world, BodyType::Static, 0.0, 0.0, Vector::default());
    let mover = create_body(
        &mut world,
        BodyType::Dynamic,
        -1.1,
        0.0,
        Vector::new(12.0, 0.0),
    );
    let material = Material {
        friction: 0.0,
        restitution: 0.0,
    };
    // Circle versus circle bypasses the static-convex CCD clamp, so exported
    // geometry can be checked against the poses committed after the solve.
    attach_shape(&mut world, target, SharedShape::circle(0.5), material);
    attach_shape(&mut world, mover, SharedShape::circle(0.5), material);

    let report = step_world_with_config(
        &mut world,
        StepConfig {
            velocity_iterations: 10,
            position_iterations: 0,
            ..fixed_step_config()
        },
        1,
    );
    let contact = active_contact_events(&report)
        .into_iter()
        .next()
        .expect("the speculative velocity constraint should finalize one contact event");
    let center_a = body_position(&world, contact.body_a);
    let center_b = body_position(&world, contact.body_b);
    let offset_to_a = center_a - center_b;
    let distance = offset_to_a.length();
    let expected_normal = offset_to_a / distance;
    let point_on_a = center_a - expected_normal * 0.5;
    let point_on_b = center_b + expected_normal * 0.5;
    let expected_point = Point::from((point_on_a + point_on_b) * 0.5);
    let expected_depth = (1.0 - distance).max(0.0);

    assert!(contact.depth.is_finite());
    assert!(contact.point.x().is_finite() && contact.point.y().is_finite());
    assert!(contact.normal.x().is_finite() && contact.normal.y().is_finite());
    assert!(
        (contact.depth - expected_depth).abs() <= 1.0e-5,
        "event depth must describe final authoritative circle poses: contact={contact:?}, expected_depth={expected_depth}"
    );
    assert!(
        (contact.point - expected_point).length() <= 1.0e-5,
        "event point must be the midpoint of final authoritative surface points: contact={contact:?}, expected_point={expected_point:?}"
    );
    assert!(
        (contact.normal - expected_normal).length() <= 1.0e-5,
        "event normal must point toward body A in final authoritative geometry: contact={contact:?}, expected_normal={expected_normal:?}"
    );
}

#[test]
fn finalized_speculative_contact_cache_preserves_next_step_warm_start() {
    let mut world = no_gravity_world();
    let target = create_body(&mut world, BodyType::Static, 0.0, 0.0, Vector::default());
    let mover = create_body(
        &mut world,
        BodyType::Dynamic,
        -1.1,
        0.0,
        Vector::new(12.0, 0.0),
    );
    let material = Material {
        friction: 0.0,
        restitution: 0.0,
    };
    // Circle versus circle bypasses the static-convex CCD clamp, making the
    // second frame exercise the cache finalized from a speculative first hit.
    attach_shape(&mut world, target, SharedShape::circle(0.5), material);
    attach_shape(&mut world, mover, SharedShape::circle(0.5), material);
    let mut pipeline = SimulationPipeline::new(StepConfig {
        velocity_iterations: 10,
        position_iterations: 0,
        ..fixed_step_config()
    });

    let first = pipeline.step(&mut world);
    let first_contact = active_contact_events(&first)
        .into_iter()
        .next()
        .expect("the first predicted hit should finalize a contact");
    assert!(
        first_contact.solver_normal_impulse > 0.0,
        "the first frame must cache a solved normal impulse: {first_contact:?}"
    );

    let second = pipeline.step(&mut world);
    let second_contact = second
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::ContactPersisted(contact) => Some(*contact),
            _ => None,
        })
        .expect("the finalized first-frame contact should persist on the next step");

    assert_eq!(second_contact.contact_id, first_contact.contact_id);
    assert_eq!(second_contact.manifold_id, first_contact.manifold_id);
    assert_eq!(second_contact.warm_start_reason, WarmStartCacheReason::Hit);
    assert_eq!(second.stats.warm_start_drop_count, 0);
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
    let target_velocity = body_velocity(&world, target);
    assert!(
        target_velocity.length() > 1.0e-4,
        "impact wake should write solved contact velocity back to the sleeping target; got {target_velocity:?}"
    );
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
fn sleeping_unrelated_island_keeps_contact_facts_but_skips_solver_rows() {
    let mut world = no_gravity_world();
    let active = create_body(
        &mut world,
        BodyType::Dynamic,
        -2.0,
        0.0,
        Vector::new(1.0, 0.0),
    );
    let active_support = create_body(&mut world, BodyType::Static, -1.25, 0.0, Vector::default());
    let sleeper = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(2.0, 0.0, 0.0),
            can_sleep: true,
            sleeping: true,
            ..BodyDesc::default()
        })
        .expect("sleeping body should be created");
    let sleeper_support = create_body(&mut world, BodyType::Static, 2.75, 0.0, Vector::default());
    for body in [active, active_support, sleeper, sleeper_support] {
        attach_shape(
            &mut world,
            body,
            SharedShape::circle(0.5),
            Material::default(),
        );
    }

    let report = step_world(&mut world, 1);
    let contacts = active_contact_events(&report);
    let active_contacts = contacts
        .iter()
        .filter(|contact| contact.body_a == active || contact.body_b == active)
        .collect::<Vec<_>>();
    let sleeping_contacts = contacts
        .iter()
        .filter(|contact| contact.body_a == sleeper || contact.body_b == sleeper)
        .collect::<Vec<_>>();

    assert!(
        active_contacts
            .iter()
            .any(|contact| contact.solver_normal_impulse > 0.0),
        "the active island should still solve its contact row: {active_contacts:?}"
    );
    assert_eq!(
        sleeping_contacts.len(),
        1,
        "the sleeping island should keep deterministic contact facts"
    );
    assert_eq!(sleeping_contacts[0].solver_normal_impulse, 0.0);
    assert_eq!(sleeping_contacts[0].solver_tangent_impulse, 0.0);
    assert!(
        world.try_body(sleeper).expect("sleeper exists").sleeping(),
        "an unrelated sleeping island must remain asleep"
    );
    assert_eq!(
        report.stats.contact_count,
        contacts.len(),
        "StepStats contact count should still match emitted active contact facts"
    );
    assert_eq!(report.stats.island_count, 2);
    assert_eq!(report.stats.active_island_count, 1);
    assert_eq!(report.stats.sleeping_island_skip_count, 1);
    assert_eq!(report.stats.solver_body_slot_count, 2);
    assert_eq!(report.stats.contact_row_count, 1);
    assert_eq!(report.stats.joint_row_count, 0);

    let snapshot = DebugSnapshot::from_world_with_step_report(
        &world,
        &report,
        &DebugSnapshotOptions::default(),
    );
    let sleeper_island = snapshot
        .bodies
        .iter()
        .find(|body| body.handle == sleeper)
        .and_then(|body| body.island_id)
        .expect("sleeping dynamic body should keep an island fact");
    assert!(
        snapshot
            .islands
            .iter()
            .any(|island| island.id == sleeper_island && island.sleeping),
        "DebugSnapshot should keep the unrelated sleeping island visible"
    );
    assert_eq!(snapshot.stats.island_count, report.stats.island_count);
    assert_eq!(
        snapshot.stats.active_island_count,
        report.stats.active_island_count
    );
    assert_eq!(
        snapshot.stats.sleeping_island_skip_count,
        report.stats.sleeping_island_skip_count
    );
    assert_eq!(
        snapshot.stats.contact_row_count,
        report.stats.contact_row_count
    );
    let snapshot_sleep_contact = snapshot
        .contacts
        .iter()
        .find(|contact| contact.bodies.contains(&sleeper))
        .expect("DebugSnapshot should retain the sleeping island contact fact");
    assert_eq!(snapshot_sleep_contact.solver_normal_impulse, 0.0);
    assert_eq!(snapshot_sleep_contact.solver_tangent_impulse, 0.0);
}

#[test]
fn fast_small_body_does_not_tunnel_through_thin_wall() {
    assert_fast_small_body_does_not_tunnel_through_thin_wall();
}

#[test]
fn ccd_fast_small_body_does_not_tunnel_through_thin_wall() {
    assert_fast_small_body_does_not_tunnel_through_thin_wall();
}

#[test]
fn ccd_clamped_dynamic_circle_preserves_full_step_angular_integration() {
    let mut world = no_gravity_world();
    let wall = create_body(&mut world, BodyType::Static, 0.0, 0.0, Vector::default());
    let angular_velocity = 6.0;
    let spinner = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(-1.0, 0.0, 0.0),
            linear_velocity: Vector::new(200.0, 0.0),
            angular_velocity,
            can_sleep: false,
            ..BodyDesc::default()
        })
        .expect("spinning body should be created");
    let frictionless = Material {
        friction: 0.0,
        restitution: 0.0,
    };
    attach_shape(&mut world, wall, SharedShape::rect(0.1, 10.0), frictionless);
    attach_shape(&mut world, spinner, SharedShape::circle(0.05), frictionless);

    let report = step_world(&mut world, 1);
    let spinner = world
        .try_body(spinner)
        .expect("spinning body should still exist");
    let angle = spinner.pose().angle();
    let expected_angle = angular_velocity * DT;

    assert_eq!(report.stats.ccd_clamp_count, 1);
    assert!(
        (angle - expected_angle).abs() < 1.0e-4,
        "CCD-clamped body should retain full-step angular integration; angle={angle}, expected_angle={expected_angle}, angular_velocity={}",
        spinner.angular_velocity()
    );
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

#[test]
fn ccd_dynamic_convex_body_does_not_tunnel_through_thin_wall() {
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
        SharedShape::rect(0.1, 0.1),
        Material::default(),
    );

    let report = step_world(&mut world, 1);
    let position = body_position(&world, bullet);
    let contact = active_contact_events(&report)
        .into_iter()
        .find(|contact| contact.ccd_trace.is_some())
        .expect("CCD should report the swept convex contact with the wall");
    let trace = contact.ccd_trace.expect("contact should carry CCD trace");

    assert_eq!(report.stats.ccd_candidate_count, 1);
    assert_eq!(report.stats.ccd_hit_count, 1);
    assert_eq!(report.stats.ccd_miss_count, 0);
    assert_eq!(report.stats.ccd_clamp_count, 1);
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
    assert!(trace.toi_point.y().abs() < 1.0e-3);
    assert!(
        position.x() <= -0.09,
        "CCD should keep the dynamic convex body on the pre-impact side; x={}",
        position.x()
    );
}

#[test]
fn ccd_dynamic_convex_missed_sweep_does_not_false_positive_or_clamp() {
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
        SharedShape::rect(0.1, 0.1),
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
        "missed convex sweep should keep the integrated end pose; y={}",
        position.y()
    );
}

#[test]
fn ccd_static_convex_cache_invalidates_after_body_and_collider_edits() {
    fn assert_no_stale_wall_hit(mut world: World) {
        let bullet = create_body(
            &mut world,
            BodyType::Dynamic,
            -1.0,
            0.0,
            Vector::new(200.0, 0.0),
        );
        attach_shape(
            &mut world,
            bullet,
            SharedShape::rect(0.1, 0.1),
            Material::default(),
        );

        let report = step_world(&mut world, 1);
        let position = body_position(&world, bullet);

        assert_eq!(report.stats.contact_count, 0);
        assert_eq!(report.stats.ccd_candidate_count, 0);
        assert_eq!(report.stats.ccd_hit_count, 0);
        assert_eq!(report.stats.ccd_clamp_count, 0);
        assert!(
            position.x() > 2.0,
            "stale CCD geometry would clamp the bullet at the old wall; x={}",
            position.x()
        );
    }

    let mut body_edit_world = no_gravity_world();
    let body_moved_wall = create_body(
        &mut body_edit_world,
        BodyType::Static,
        0.0,
        0.0,
        Vector::default(),
    );
    attach_shape(
        &mut body_edit_world,
        body_moved_wall,
        SharedShape::rect(0.1, 10.0),
        Material::default(),
    );
    step_world(&mut body_edit_world, 1);
    body_edit_world
        .apply_body_patch(
            body_moved_wall,
            BodyPatch {
                pose: Some(Pose::from_xy_angle(100.0, 0.0, 0.0)),
                ..BodyPatch::default()
            },
        )
        .expect("static wall body should move away");
    assert_no_stale_wall_hit(body_edit_world);

    let mut collider_edit_world = no_gravity_world();
    let collider_moved_wall = create_body(
        &mut collider_edit_world,
        BodyType::Static,
        0.0,
        0.0,
        Vector::default(),
    );
    let collider = attach_shape(
        &mut collider_edit_world,
        collider_moved_wall,
        SharedShape::rect(0.1, 10.0),
        Material::default(),
    );
    step_world(&mut collider_edit_world, 1);
    collider_edit_world
        .apply_collider_patch(
            collider,
            ColliderPatch {
                local_pose: Some(Pose::from_xy_angle(100.0, 0.0, 0.0)),
                shape: Some(SharedShape::rect(0.05, 10.0)),
                ..ColliderPatch::default()
            },
        )
        .expect("static wall collider geometry should move away");
    assert_no_stale_wall_hit(collider_edit_world);
}

#[test]
fn ccd_dynamic_convex_multi_hit_budget_selects_earliest_static_hit() {
    let mut world = no_gravity_world();
    let near_wall = create_body(&mut world, BodyType::Static, 0.0, 0.0, Vector::default());
    let far_wall = create_body(&mut world, BodyType::Static, 0.8, 0.0, Vector::default());
    let bullet = create_body(
        &mut world,
        BodyType::Dynamic,
        -1.0,
        0.0,
        Vector::new(200.0, 0.0),
    );
    let near_collider = attach_shape(
        &mut world,
        near_wall,
        SharedShape::rect(0.1, 10.0),
        Material::default(),
    );
    let far_collider = attach_shape(
        &mut world,
        far_wall,
        SharedShape::rect(0.1, 10.0),
        Material::default(),
    );
    attach_shape(
        &mut world,
        bullet,
        SharedShape::rect(0.1, 0.1),
        Material::default(),
    );

    let report = step_world(&mut world, 1);
    let contact = active_contact_events(&report)
        .into_iter()
        .find(|contact| contact.ccd_trace.is_some())
        .expect("CCD should emit the selected earliest hit");
    let trace = contact.ccd_trace.expect("contact should carry CCD trace");

    assert_eq!(report.stats.ccd_candidate_count, 2);
    assert_eq!(report.stats.ccd_hit_count, 2);
    assert_eq!(report.stats.ccd_miss_count, 0);
    assert_eq!(report.stats.ccd_clamp_count, 1);
    assert_eq!(trace.static_body, near_wall);
    assert_eq!(trace.static_collider, near_collider);
    assert_ne!(trace.static_collider, far_collider);
    assert!(
        body_position(&world, bullet).x() <= -0.09,
        "CCD budget should clamp at the first static hit before reaching the farther wall"
    );
}

#[test]
fn ccd_dynamic_convex_pair_clamps_both_bodies_without_tunneling() {
    let mut world = no_gravity_world();
    let left = create_body(
        &mut world,
        BodyType::Dynamic,
        -1.0,
        0.0,
        Vector::new(200.0, 0.0),
    );
    let right = create_body(
        &mut world,
        BodyType::Dynamic,
        1.0,
        0.0,
        Vector::new(-200.0, 0.0),
    );
    let left_collider = attach_shape(
        &mut world,
        left,
        SharedShape::rect(0.1, 0.1),
        Material::default(),
    );
    let right_collider = attach_shape(
        &mut world,
        right,
        SharedShape::rect(0.1, 0.1),
        Material::default(),
    );

    let report = step_world(&mut world, 1);
    let left_position = body_position(&world, left);
    let right_position = body_position(&world, right);
    let contact = active_contact_events(&report)
        .into_iter()
        .find(|contact| contact.ccd_trace.is_some())
        .expect("dynamic-vs-dynamic CCD should emit the selected swept contact");
    let trace = contact.ccd_trace.expect("contact should carry CCD trace");

    assert_eq!(report.stats.ccd_candidate_count, 1);
    assert_eq!(report.stats.ccd_hit_count, 1);
    assert_eq!(report.stats.ccd_miss_count, 0);
    assert_eq!(report.stats.ccd_clamp_count, 2);
    assert_eq!(trace.moving_body, left);
    assert_eq!(trace.static_body, right);
    assert_eq!(trace.moving_collider, left_collider);
    assert_eq!(trace.static_collider, right_collider);
    assert_eq!(trace.target_kind, CcdTargetKind::Dynamic);
    assert_eq!(trace.target_swept_start, Point::new(1.0, 0.0));
    assert!(trace.target_swept_end.x() < -2.0);
    assert!(trace.toi > 0.0 && trace.toi < 1.0);
    assert!(trace.clamp > 0.0);
    assert!(trace.target_clamp > 0.0);
    assert!(
        left_position.x() < right_position.x(),
        "dynamic CCD should stop the bodies before they exchange sides; left_x={}, right_x={}",
        left_position.x(),
        right_position.x()
    );
}

#[test]
fn ccd_dynamic_convex_hits_stationary_dynamic_convex_target() {
    let mut world = no_gravity_world();
    let bullet = create_body(
        &mut world,
        BodyType::Dynamic,
        -1.0,
        0.0,
        Vector::new(200.0, 0.0),
    );
    let target = create_body(
        &mut world,
        BodyType::Dynamic,
        0.0,
        0.0,
        Vector::new(0.0, 0.0),
    );
    let bullet_collider = attach_shape(
        &mut world,
        bullet,
        SharedShape::rect(0.1, 0.1),
        Material::default(),
    );
    let target_collider = attach_shape(
        &mut world,
        target,
        SharedShape::rect(0.1, 0.1),
        Material::default(),
    );

    let report = step_world(&mut world, 1);
    let bullet_position = body_position(&world, bullet);
    let target_position = body_position(&world, target);
    let contact = active_contact_events(&report)
        .into_iter()
        .find(|contact| contact.ccd_trace.is_some())
        .expect("dynamic CCD should include stationary dynamic convex targets");
    let trace = contact.ccd_trace.expect("contact should carry CCD trace");

    assert_eq!(report.stats.ccd_candidate_count, 1);
    assert_eq!(report.stats.ccd_hit_count, 1);
    assert_eq!(report.stats.ccd_miss_count, 0);
    assert_eq!(report.stats.ccd_clamp_count, 2);
    assert_eq!(trace.moving_body, bullet);
    assert_eq!(trace.static_body, target);
    assert_eq!(trace.moving_collider, bullet_collider);
    assert_eq!(trace.static_collider, target_collider);
    assert_eq!(trace.target_kind, CcdTargetKind::Dynamic);
    assert_eq!(trace.target_swept_start, Point::new(0.0, 0.0));
    assert_eq!(trace.target_swept_end, Point::new(0.0, 0.0));
    assert!(trace.toi > 0.0 && trace.toi < 1.0);
    assert!(trace.clamp > 0.0);
    assert_eq!(trace.target_clamp, 0.0);
    assert!(
        bullet_position.x() < target_position.x(),
        "dynamic CCD should stop the bullet before it tunnels through a stationary dynamic target; bullet_x={}, target_x={}",
        bullet_position.x(),
        target_position.x()
    );
}

#[test]
fn ccd_dynamic_convex_pair_missed_sweep_does_not_false_positive_or_clamp() {
    let mut world = no_gravity_world();
    let diamond = create_body(
        &mut world,
        BodyType::Dynamic,
        0.0,
        0.0,
        Vector::new(0.1, 0.0),
    );
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
        SharedShape::rect(0.1, 0.1),
        Material::default(),
    );

    let report = step_world(&mut world, 1);
    let bullet_position = body_position(&world, bullet);

    assert_eq!(report.stats.contact_count, 0);
    assert_eq!(report.stats.ccd_candidate_count, 1);
    assert_eq!(report.stats.ccd_hit_count, 0);
    assert_eq!(report.stats.ccd_miss_count, 1);
    assert_eq!(report.stats.ccd_clamp_count, 0);
    assert!(
        (bullet_position.y() - 0.9).abs() < 1.0e-4,
        "missed dynamic sweep should keep the integrated end pose; y={}",
        bullet_position.y()
    );
}

#[test]
fn ccd_dynamic_convex_multi_hit_budget_selects_earliest_dynamic_hit() {
    let mut world = no_gravity_world();
    let bullet = create_body(
        &mut world,
        BodyType::Dynamic,
        -1.0,
        0.0,
        Vector::new(200.0, 0.0),
    );
    let near = create_body(
        &mut world,
        BodyType::Dynamic,
        0.0,
        0.0,
        Vector::new(0.1, 0.0),
    );
    let far = create_body(
        &mut world,
        BodyType::Dynamic,
        0.8,
        0.0,
        Vector::new(0.1, 0.0),
    );
    let near_collider = attach_shape(
        &mut world,
        near,
        SharedShape::rect(0.1, 0.1),
        Material::default(),
    );
    let far_collider = attach_shape(
        &mut world,
        far,
        SharedShape::rect(0.1, 0.1),
        Material::default(),
    );
    attach_shape(
        &mut world,
        bullet,
        SharedShape::rect(0.1, 0.1),
        Material::default(),
    );

    let report = step_world(&mut world, 1);
    let contact = active_contact_events(&report)
        .into_iter()
        .find(|contact| contact.ccd_trace.is_some())
        .expect("CCD should emit the selected earliest dynamic hit");
    let trace = contact.ccd_trace.expect("contact should carry CCD trace");

    assert_eq!(report.stats.ccd_candidate_count, 2);
    assert_eq!(report.stats.ccd_hit_count, 2);
    assert_eq!(report.stats.ccd_miss_count, 0);
    assert_eq!(report.stats.ccd_clamp_count, 2);
    assert_eq!(trace.static_body, near);
    assert_eq!(trace.static_collider, near_collider);
    assert_ne!(trace.static_collider, far_collider);
    assert_eq!(trace.target_kind, CcdTargetKind::Dynamic);
    assert_eq!(trace.target_swept_start, Point::new(0.0, 0.0));
    assert!(trace.target_swept_end.x() > trace.target_swept_start.x());
    assert!(trace.target_clamp > 0.0);
    assert!(
        body_position(&world, bullet).x() < body_position(&world, far).x(),
        "CCD budget should select the nearer dynamic target before the farther hit"
    );
}

#[test]
fn ccd_dynamic_compound_body_clamps_on_earliest_piece_hit() {
    let mut world = no_gravity_world();
    let wall = create_body(&mut world, BodyType::Static, 0.0, 0.25, Vector::default());
    let compound = create_body(
        &mut world,
        BodyType::Dynamic,
        -1.0,
        0.0,
        Vector::new(200.0, 0.0),
    );
    let wall_collider = attach_shape(
        &mut world,
        wall,
        SharedShape::rect(0.1, 0.12),
        Material::default(),
    );
    let upper_piece = attach_shape_with_local_pose(
        &mut world,
        compound,
        SharedShape::rect(0.1, 0.1),
        Pose::from_xy_angle(0.0, 0.25, 0.0),
        Material::default(),
    );
    let lower_piece = attach_shape_with_local_pose(
        &mut world,
        compound,
        SharedShape::rect(0.1, 0.1),
        Pose::from_xy_angle(0.0, -0.25, 0.0),
        Material::default(),
    );

    let report = step_world(&mut world, 1);
    let compound_position = body_position(&world, compound);
    let contact = active_contact_events(&report)
        .into_iter()
        .find(|contact| contact.ccd_trace.is_some())
        .expect("dynamic compound CCD should emit the selected swept piece contact");
    let trace = contact.ccd_trace.expect("contact should carry CCD trace");

    assert_eq!(report.stats.ccd_candidate_count, 1);
    assert_eq!(report.stats.ccd_hit_count, 1);
    assert_eq!(report.stats.ccd_miss_count, 0);
    assert_eq!(report.stats.ccd_clamp_count, 1);
    assert_eq!(trace.target_kind, CcdTargetKind::Static);
    assert_eq!(trace.moving_body, compound);
    assert_eq!(trace.static_body, wall);
    assert_eq!(trace.moving_collider, upper_piece);
    assert_eq!(trace.static_collider, wall_collider);
    assert_ne!(trace.moving_collider, lower_piece);
    assert!(trace.toi > 0.0 && trace.toi < 1.0);
    assert!(
        compound_position.x() <= -0.09,
        "compound body should clamp before the upper piece crosses the wall; x={}",
        compound_position.x()
    );
}

#[test]
fn ccd_dynamic_convex_pair_skips_rotating_bodies() {
    let mut world = no_gravity_world();
    let rotating = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(-1.0, 0.0, 0.0),
            linear_velocity: Vector::new(200.0, 0.0),
            angular_velocity: 1.0,
            can_sleep: false,
            ..BodyDesc::default()
        })
        .expect("rotating body should be created");
    let target = create_body(
        &mut world,
        BodyType::Dynamic,
        1.0,
        0.0,
        Vector::new(-200.0, 0.0),
    );
    attach_shape(
        &mut world,
        rotating,
        SharedShape::rect(0.1, 0.1),
        Material::default(),
    );
    attach_shape(
        &mut world,
        target,
        SharedShape::rect(0.1, 0.1),
        Material::default(),
    );

    let report = step_world(&mut world, 1);

    assert_eq!(report.stats.ccd_candidate_count, 0);
    assert_eq!(report.stats.ccd_hit_count, 0);
    assert_eq!(report.stats.ccd_clamp_count, 0);
    assert!(
        active_contact_events(&report)
            .iter()
            .all(|contact| contact.ccd_trace.is_none()),
        "rotational CCD is outside the M19 translational slice"
    );
}

#[test]
fn manifold_persistence_sat_role_swap_reports_persistent_edge_swap() {
    let mut world = no_gravity_world();
    let lower = create_body(&mut world, BodyType::Static, 0.0, 0.0, Vector::default());
    let upper = create_body(&mut world, BodyType::Static, -0.25, 1.8, Vector::default());
    attach_shape(
        &mut world,
        lower,
        SharedShape::rect(2.0, 2.0),
        Material::default(),
    );
    attach_shape(
        &mut world,
        upper,
        SharedShape::rect(2.0, 2.0),
        Material::default(),
    );
    let lower_pose = Pose::from_xy_angle(0.0, 0.0, -0.12);
    let first_upper_pose = Pose::from_xy_angle(-0.25, 1.8, -0.17);
    let second_upper_pose = Pose::from_xy_angle(-0.25, 1.8, -0.15);
    world
        .apply_body_patch(
            lower,
            BodyPatch {
                pose: Some(lower_pose),
                wake: true,
                ..BodyPatch::default()
            },
        )
        .expect("lower rectangle pose should be patched");
    world
        .apply_body_patch(
            upper,
            BodyPatch {
                pose: Some(first_upper_pose),
                wake: true,
                ..BodyPatch::default()
            },
        )
        .expect("first upper rectangle pose should be patched");
    let mut pipeline = SimulationPipeline::new(fixed_step_config());

    let first = pipeline.step(&mut world);
    world
        .apply_body_patch(
            upper,
            BodyPatch {
                pose: Some(second_upper_pose),
                wake: true,
                ..BodyPatch::default()
            },
        )
        .expect("upper rectangle should cross the SAT reference/incident role swap");
    let second = pipeline.step(&mut world);
    let first_contacts = active_contact_events(&first);
    let second_contacts = active_contact_events(&second);

    assert_eq!(first_contacts.len(), 2);
    assert_eq!(second_contacts.len(), 2);
    assert_eq!(
        second
            .events
            .iter()
            .filter(|event| matches!(event, WorldEvent::ContactPersisted(_)))
            .count(),
        2,
        "both role-swapped points must use the persisted event variant"
    );
    assert!(second_contacts.iter().all(|current| {
        first_contacts
            .iter()
            .all(|previous| previous.feature_id != current.feature_id)
    }));
    let correspondence = unique_two_point_local_witness_correspondence(
        &first_contacts,
        lower_pose,
        first_upper_pose,
        &second_contacts,
        lower_pose,
        second_upper_pose,
    );
    assert_ne!(first_contacts[0].contact_id, first_contacts[1].contact_id);
    assert_ne!(second_contacts[0].contact_id, second_contacts[1].contact_id);
    for (first_index, second_index) in correspondence {
        let previous = first_contacts[first_index];
        let current = second_contacts[second_index];
        let (previous_kind, previous_reference, previous_incident, _) =
            decoded_raw_feature(previous.feature_id);
        let (current_kind, current_reference, current_incident, _) =
            decoded_raw_feature(current.feature_id);
        assert_ne!(previous.feature_id, current.feature_id);
        assert_eq!(previous_kind, current_kind);
        assert_eq!(previous_reference, current_incident);
        assert_eq!(previous_incident, current_reference);
        assert_ne!(previous_reference, previous_incident);
        assert_eq!(current.contact_id, previous.contact_id);
        assert_eq!(current.manifold_id, previous.manifold_id);
        assert_eq!(
            current.lifecycle_reason,
            ContactLifecycleReason::PersistentEdgeSwap
        );
    }
    assert_eq!(
        second_contacts
            .iter()
            .map(|contact| contact.manifold_id)
            .collect::<BTreeSet<_>>()
            .len(),
        1
    );
}

#[test]
fn manifold_persistence_symmetric_edge_index_is_not_role_swap() {
    let mut world = no_gravity_world();
    let lower = create_body(&mut world, BodyType::Static, 0.0, 0.0, Vector::default());
    let upper = create_body(&mut world, BodyType::Static, -0.2, 2.0, Vector::default());
    attach_shape(
        &mut world,
        lower,
        SharedShape::rect(2.0, 2.0),
        Material::default(),
    );
    attach_shape(
        &mut world,
        upper,
        SharedShape::rect(2.0, 2.0),
        Material::default(),
    );
    let first_pose = Pose::from_xy_angle(-0.2, 2.0, std::f32::consts::PI - 0.22);
    let second_pose = Pose::from_xy_angle(-0.2, 2.0, std::f32::consts::PI - 0.18);
    world
        .apply_body_patch(
            upper,
            BodyPatch {
                pose: Some(first_pose),
                wake: true,
                ..BodyPatch::default()
            },
        )
        .expect("first rectangle pose should be patched");
    let mut pipeline = SimulationPipeline::new(fixed_step_config());

    let first = active_contact_events(&pipeline.step(&mut world));
    world
        .apply_body_patch(
            upper,
            BodyPatch {
                pose: Some(second_pose),
                wake: true,
                ..BodyPatch::default()
            },
        )
        .expect("second rectangle pose should be patched");
    let second = active_contact_events(&pipeline.step(&mut world));

    assert_eq!(first.len(), 1);
    assert_eq!(second.len(), 1);
    let previous = first[0];
    let current = second[0];
    let (previous_index, previous_slot) = raw_feature_parts(previous.feature_id);
    let (current_index, current_slot) = raw_feature_parts(current.feature_id);
    let (_, reference_edge, incident_edge, decoded_current_slot) =
        decoded_raw_feature(current.feature_id);
    assert_ne!(previous.feature_id, current.feature_id);
    assert_eq!(previous_index, current_index);
    assert_eq!(reference_edge, incident_edge);
    assert_eq!(decoded_current_slot, current_slot);
    assert_ne!(previous_slot, current_slot);
    assert_eq!(
        current.warm_start_reason,
        WarmStartCacheReason::MissFeatureId
    );
    assert!(
        current.lifecycle_reason != ContactLifecycleReason::PersistentEdgeSwap
            && current.source_row_continuity_reason != SourceRowContinuityReason::EdgeSwap,
        "a symmetric raw edge index with only a point-slot change is not a SAT role swap; previous={previous:?}, current={current:?}"
    );
}

fn create_revolute(
    world: &mut World,
    body_a: BodyHandle,
    body_b: BodyHandle,
    local_anchor_a: Point,
    local_anchor_b: Point,
) -> JointHandle {
    world
        .create_joint(JointDesc::Revolute(RevoluteJointDesc {
            body_a,
            body_b,
            local_anchor_a,
            local_anchor_b,
            ..RevoluteJointDesc::default()
        }))
        .expect("revolute joint should be created")
}

fn revolute_anchor(world: &World, body: BodyHandle, local_anchor: Point) -> Point {
    world
        .try_body(body)
        .expect("revolute endpoint should remain live")
        .pose()
        .transform_point(local_anchor)
}

fn revolute_anchor_drift(
    world: &World,
    body_a: BodyHandle,
    local_anchor_a: Point,
    body_b: BodyHandle,
    local_anchor_b: Point,
) -> f32 {
    (revolute_anchor(world, body_b, local_anchor_b)
        - revolute_anchor(world, body_a, local_anchor_a))
    .length()
}

fn assert_revolute_body_finite(world: &World, body: BodyHandle, label: &str) {
    let body = world
        .try_body(body)
        .unwrap_or_else(|_| panic!("{label} should remain live"));
    let pose = body.pose();
    let mass = body.mass_properties();
    assert!(
        pose.translation().x().is_finite()
            && pose.translation().y().is_finite()
            && pose.angle().is_finite()
            && body.linear_velocity().x().is_finite()
            && body.linear_velocity().y().is_finite()
            && body.angular_velocity().is_finite()
            && mass.mass.is_finite()
            && mass.inverse_mass.is_finite()
            && mass.inertia.is_finite()
            && mass.inverse_inertia.is_finite()
            && mass.local_center_of_mass.x().is_finite()
            && mass.local_center_of_mass.y().is_finite(),
        "S5_HARNESS_BOUNDARY:{label}_must_stay_finite"
    );
}

#[test]
fn revolute_joint_two_dynamic_preserves_anchor_coincidence() {
    let mut world = no_gravity_world();
    let body_a = create_body(
        &mut world,
        BodyType::Dynamic,
        -1.0,
        0.0,
        Vector::new(-1.0, 0.0),
    );
    let body_b = create_body(
        &mut world,
        BodyType::Dynamic,
        1.0,
        0.0,
        Vector::new(1.0, 0.0),
    );
    attach_shape(
        &mut world,
        body_a,
        SharedShape::rect(0.5, 0.5),
        Material::default(),
    );
    attach_shape(
        &mut world,
        body_b,
        SharedShape::rect(0.5, 0.5),
        Material::default(),
    );
    let local_anchor_a = Point::new(1.0, 0.0);
    let local_anchor_b = Point::new(-1.0, 0.0);
    create_revolute(&mut world, body_a, body_b, local_anchor_a, local_anchor_b);

    let mut pipeline = SimulationPipeline::new(fixed_step_config());
    let mut max_drift = 0.0_f32;
    let mut warning_count = 0usize;
    for _ in 0..240 {
        let report = pipeline.step(&mut world);
        warning_count += report.stats.numeric_warnings;
        max_drift = max_drift.max(revolute_anchor_drift(
            &world,
            body_a,
            local_anchor_a,
            body_b,
            local_anchor_b,
        ));
    }
    let final_drift = revolute_anchor_drift(&world, body_a, local_anchor_a, body_b, local_anchor_b);
    println!(
        "S5_REVOLUTE_FACT:two_dynamic:max_drift={max_drift:.6};final_drift={final_drift:.6};numeric_warnings={warning_count}"
    );

    assert_eq!(
        warning_count, 0,
        "S5_HARNESS_BOUNDARY:unexpected_numeric_warning"
    );
    assert!(
        max_drift <= 0.03,
        "S5_REVOLUTE_SOLVER_ASSERT:two_dynamic_max_drift:max_drift={max_drift:.6};limit=0.03"
    );
    assert!(
        final_drift <= 0.01,
        "S5_REVOLUTE_SOLVER_ASSERT:two_dynamic_final_drift:final_drift={final_drift:.6};limit=0.01"
    );
}

#[test]
fn revolute_joint_leaves_relative_rotation_free() {
    let mut world = no_gravity_world();
    let body_a = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            angular_velocity: 1.0,
            can_sleep: false,
            ..BodyDesc::default()
        })
        .expect("body A should be created");
    let body_b = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            angular_velocity: -1.0,
            can_sleep: false,
            ..BodyDesc::default()
        })
        .expect("body B should be created");
    let no_pair_contact = CollisionFilter {
        memberships: 1,
        collides_with: 0,
    };
    for body in [body_a, body_b] {
        world
            .create_collider(
                body,
                ColliderDesc {
                    shape: SharedShape::rect(1.0, 0.5),
                    density: 1.0,
                    filter: no_pair_contact,
                    ..ColliderDesc::default()
                },
            )
            .expect("free-rotation endpoint should have finite positive mass and inertia");
    }
    create_revolute(
        &mut world,
        body_a,
        body_b,
        Point::default(),
        Point::default(),
    );
    let mass_a = world
        .try_body(body_a)
        .expect("body A exists")
        .mass_properties();
    let mass_b = world
        .try_body(body_b)
        .expect("body B exists")
        .mass_properties();
    assert!(
        mass_a.inverse_mass > 0.0
            && mass_b.inverse_mass > 0.0
            && mass_a.inverse_inertia > 0.0
            && mass_b.inverse_inertia > 0.0,
        "S5_HARNESS_BOUNDARY:free_rotation_requires_positive_inverse_mass_and_inertia"
    );
    // Both anchors and both centers of mass are at the body origins, so rA=rB=0 and
    // K=(inv_mass_a+inv_mass_b)I. This proves the boundary is not passing via a singular row.
    let k_diagonal = mass_a.inverse_mass + mass_b.inverse_mass;
    let k_determinant = k_diagonal * k_diagonal;
    assert!(
        k_determinant.is_finite() && k_determinant > f32::EPSILON,
        "S5_HARNESS_BOUNDARY:free_rotation_requires_invertible_effective_mass"
    );
    let initial_relative_angle = world
        .try_body(body_b)
        .expect("body B exists")
        .pose()
        .angle()
        - world
            .try_body(body_a)
            .expect("body A exists")
            .pose()
            .angle();
    let mut pipeline = SimulationPipeline::new(fixed_step_config());
    let mut baseline_joint_rows = Vec::new();
    for _ in 0..60 {
        let report = pipeline.step(&mut world);
        baseline_joint_rows.push(report.stats.joint_row_count);
    }
    let final_relative_angle = world
        .try_body(body_b)
        .expect("body B exists")
        .pose()
        .angle()
        - world
            .try_body(body_a)
            .expect("body A exists")
            .pose()
            .angle();
    let relative_change = (final_relative_angle - initial_relative_angle).abs();
    println!(
        "S5_REVOLUTE_BOUNDARY:free_rotation:inverse_mass=({:.6},{:.6});inverse_inertia=({:.6},{:.6});k_determinant={k_determinant:.6};relative_change={relative_change:.6};baseline_joint_rows={baseline_joint_rows:?}",
        mass_a.inverse_mass,
        mass_b.inverse_mass,
        mass_a.inverse_inertia,
        mass_b.inverse_inertia
    );

    assert!(
        relative_change >= 1.0,
        "S5_HARNESS_BOUNDARY:relative_rotation_was_locked:change={relative_change:.6}"
    );
}

#[test]
fn revolute_joint_static_dynamic_preserves_static_pose() {
    let mut world = no_gravity_world();
    let static_body = create_body(&mut world, BodyType::Static, 0.0, 0.0, Vector::default());
    let dynamic_body = create_body(
        &mut world,
        BodyType::Dynamic,
        1.0,
        0.0,
        Vector::new(2.0, 0.0),
    );
    attach_shape(
        &mut world,
        dynamic_body,
        SharedShape::rect(1.0, 1.0),
        Material::default(),
    );
    let static_anchor = Point::default();
    let dynamic_anchor = Point::new(-1.0, 0.0);
    create_revolute(
        &mut world,
        static_body,
        dynamic_body,
        static_anchor,
        dynamic_anchor,
    );
    let static_pose_before = world
        .try_body(static_body)
        .expect("static endpoint should exist")
        .pose();

    let report = step_world(&mut world, 60);
    let static_pose_after = world
        .try_body(static_body)
        .expect("static endpoint should exist")
        .pose();
    let drift = revolute_anchor_drift(
        &world,
        static_body,
        static_anchor,
        dynamic_body,
        dynamic_anchor,
    );
    println!(
        "S5_REVOLUTE_FACT:static_dynamic:drift={drift:.6};numeric_warnings={}",
        report.stats.numeric_warnings
    );

    assert_eq!(static_pose_after, static_pose_before);
    assert!(
        drift <= 0.01,
        "S5_REVOLUTE_SOLVER_ASSERT:static_dynamic_pivot_drift:drift={drift:.6};limit=0.01"
    );
}

#[test]
fn revolute_joint_off_center_anchor_uses_rotational_inertia() {
    let mut world = no_gravity_world();
    let static_body = create_body(&mut world, BodyType::Static, 0.0, 0.0, Vector::default());
    let dynamic_body = create_body(
        &mut world,
        BodyType::Dynamic,
        1.0,
        0.0,
        Vector::new(0.0, 4.0),
    );
    attach_shape(
        &mut world,
        dynamic_body,
        SharedShape::rect(1.0, 0.5),
        Material::default(),
    );
    create_revolute(
        &mut world,
        static_body,
        dynamic_body,
        Point::default(),
        Point::new(-1.0, 0.0),
    );

    let report = step_world(&mut world, 60);
    let rotation = world
        .try_body(dynamic_body)
        .expect("dynamic endpoint should exist")
        .pose()
        .angle()
        .abs();
    println!(
        "S5_REVOLUTE_FACT:off_center:rotation={rotation:.6};numeric_warnings={}",
        report.stats.numeric_warnings
    );

    assert!(
        rotation >= 0.5,
        "S5_REVOLUTE_SOLVER_ASSERT:off_center_requires_rotational_inertia:rotation={rotation:.6};minimum=0.5"
    );
}

#[test]
fn revolute_joint_is_deterministic_and_finite() {
    fn run() -> (Vec<String>, Vec<usize>) {
        let mut world = no_gravity_world();
        let body_a = create_body(
            &mut world,
            BodyType::Dynamic,
            -1.0,
            0.0,
            Vector::new(0.25, -0.5),
        );
        let body_b = world
            .create_body(BodyDesc {
                body_type: BodyType::Dynamic,
                pose: Pose::from_xy_angle(1.0, 0.0, 0.0),
                linear_velocity: Vector::new(-0.5, 0.25),
                angular_velocity: 1.5,
                can_sleep: false,
                ..BodyDesc::default()
            })
            .expect("body B should be created");
        for body in [body_a, body_b] {
            attach_shape(
                &mut world,
                body,
                SharedShape::rect(0.5, 0.75),
                Material::default(),
            );
        }
        create_revolute(
            &mut world,
            body_a,
            body_b,
            Point::new(1.0, 0.0),
            Point::new(-1.0, 0.0),
        );
        let mut pipeline = SimulationPipeline::new(fixed_step_config());
        let mut hashes = Vec::new();
        let mut joint_rows = Vec::new();
        for _ in 0..120 {
            let report = pipeline.step(&mut world);
            assert_eq!(report.stats.numeric_warnings, 0);
            assert_revolute_body_finite(&world, body_a, "determinism_body_a");
            assert_revolute_body_finite(&world, body_b, "determinism_body_b");
            hashes.push(stable_snapshot_hash(
                &world.debug_snapshot(&DebugSnapshotOptions::default()),
            ));
            joint_rows.push(report.stats.joint_row_count);
        }
        (hashes, joint_rows)
    }

    let first = run();
    let second = run();
    println!(
        "S5_REVOLUTE_BOUNDARY:deterministic_finite:frames={};baseline_joint_rows={:?}",
        first.0.len(),
        first.1
    );
    assert_eq!(first.0, second.0, "revolute scenario must be deterministic");
    assert_eq!(first.1, second.1, "solver-row facts must be deterministic");
}

fn build_connected_contact_revolute_world() -> (World, BodyHandle, BodyHandle) {
    let mut world = no_gravity_world();
    let body_a = create_body(&mut world, BodyType::Dynamic, 0.0, 0.0, Vector::default());
    let body_b = create_body(&mut world, BodyType::Dynamic, 0.75, 0.0, Vector::default());
    for body in [body_a, body_b] {
        attach_shape(
            &mut world,
            body,
            SharedShape::circle(0.5),
            Material::default(),
        );
    }
    create_revolute(
        &mut world,
        body_a,
        body_b,
        Point::new(0.375, 0.0),
        Point::new(-0.375, 0.0),
    );
    (world, body_a, body_b)
}

#[test]
fn revolute_joint_mixed_contact_island_keeps_separate_logical_rows() {
    let (mut world, body_a, body_b) = build_connected_contact_revolute_world();
    let report = step_world(&mut world, 1);
    let pair_contact_count = active_contact_events(&report)
        .iter()
        .filter(|contact| {
            [contact.body_a, contact.body_b].contains(&body_a)
                && [contact.body_a, contact.body_b].contains(&body_b)
        })
        .count();
    println!(
        "S5_REVOLUTE_FACT:mixed_rows:pair_contacts={pair_contact_count};contact_rows={};joint_rows={}",
        report.stats.contact_row_count, report.stats.joint_row_count
    );

    assert!(
        pair_contact_count > 0,
        "S5_HARNESS_BOUNDARY:mixed_fixture_has_no_contact"
    );
    assert!(
        report.stats.contact_row_count > 0,
        "S5_HARNESS_BOUNDARY:mixed_fixture_has_no_contact_row"
    );
    assert_eq!(
        report.stats.joint_row_count, 1,
        "S5_REVOLUTE_SOLVER_ASSERT:mixed_island_requires_one_logical_joint_row"
    );
}

#[test]
fn revolute_joint_connected_bodies_still_contact() {
    let (mut world, body_a, body_b) = build_connected_contact_revolute_world();
    let report = step_world(&mut world, 1);
    let contacts = active_contact_events(&report);
    let pair_contacts = contacts
        .iter()
        .filter(|contact| {
            [contact.body_a, contact.body_b].contains(&body_a)
                && [contact.body_a, contact.body_b].contains(&body_b)
        })
        .count();
    println!(
        "S5_REVOLUTE_BOUNDARY:connected_contact:pair_contacts={pair_contacts};contact_rows={};baseline_joint_rows={}",
        report.stats.contact_row_count, report.stats.joint_row_count
    );
    assert!(
        pair_contacts > 0,
        "connected joint endpoints must still collide"
    );
    assert!(report.stats.contact_row_count > 0);
}

#[test]
fn revolute_joint_nonzero_local_center_of_mass_preserves_pivot() {
    let mut world = no_gravity_world();
    let static_body = create_body(&mut world, BodyType::Static, 1.25, 0.25, Vector::default());
    let dynamic_body = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            angular_velocity: 3.0,
            can_sleep: false,
            ..BodyDesc::default()
        })
        .expect("dynamic endpoint should be created");
    attach_shape_with_local_pose(
        &mut world,
        dynamic_body,
        SharedShape::rect(0.5, 0.75),
        Pose::from_xy_angle(0.75, 0.25, 0.0),
        Material::default(),
    );
    let local_anchor = Point::new(1.25, 0.25);
    create_revolute(
        &mut world,
        static_body,
        dynamic_body,
        Point::default(),
        local_anchor,
    );
    let mass = world
        .try_body(dynamic_body)
        .expect("dynamic endpoint should exist")
        .mass_properties();
    assert!(
        Vector::from(mass.local_center_of_mass).length() > 0.5,
        "S5_HARNESS_BOUNDARY:fixture_must_have_nonzero_local_com"
    );
    let anchor_from_center = local_anchor - mass.local_center_of_mass;
    assert!(
        anchor_from_center.length() >= 0.5 && mass.inverse_mass > 0.0 && mass.inverse_inertia > 0.0,
        "S5_HARNESS_BOUNDARY:nonzero_com_fixture_requires_nonzero_lever_and_solvable_mass"
    );

    let mut pipeline = SimulationPipeline::new(fixed_step_config());
    let mut max_drift = 0.0_f32;
    for _ in 0..120 {
        let report = pipeline.step(&mut world);
        assert_eq!(report.stats.numeric_warnings, 0);
        max_drift = max_drift.max(revolute_anchor_drift(
            &world,
            static_body,
            Point::default(),
            dynamic_body,
            local_anchor,
        ));
    }
    let final_drift = revolute_anchor_drift(
        &world,
        static_body,
        Point::default(),
        dynamic_body,
        local_anchor,
    );
    let endpoint_angular_velocity = body_angular_velocity(&world, dynamic_body);
    println!(
        "S5_REVOLUTE_FACT:nonzero_com:local_com={:?};local_anchor={local_anchor:?};lever={anchor_from_center:?};lever_length={:.6};inverse_mass={:.6};inverse_inertia={:.6};angular_velocity={endpoint_angular_velocity:.6};max_drift={max_drift:.6};final_drift={final_drift:.6}",
        mass.local_center_of_mass,
        anchor_from_center.length(),
        mass.inverse_mass,
        mass.inverse_inertia
    );
    assert!(endpoint_angular_velocity.abs() > 0.1);
    assert!(
        max_drift <= 0.03,
        "S5_REVOLUTE_SOLVER_ASSERT:nonzero_com_max_pivot_drift:max_drift={max_drift:.6};limit=0.03"
    );
    assert!(
        final_drift <= 0.01,
        "S5_REVOLUTE_SOLVER_ASSERT:nonzero_com_final_pivot_drift:final_drift={final_drift:.6};limit=0.01"
    );
}

const CCD_SPINNER_INITIAL_ANGULAR_VELOCITY: f32 = 6.0;

struct CcdSpinnerControl {
    final_angular_velocity: f32,
    mass_properties: MassProperties,
    report: StepReport,
    trace: CcdTrace,
}

struct CcdZeroAngularCorrectionOracle {
    target_anchor: Point,
    clamped_translation: Vector,
    sampled_eval_angle: f32,
    lever: Vector,
    radial_error: Vector,
    expected_position_delta_angle: f32,
}

fn assert_same_ccd_geometry(actual: &CcdTrace, control: &CcdTrace) {
    // Body and collider handles are allocated per world, so the control and joint worlds
    // cannot share identity values. Compare the CCD geometry/decision facts here; each test
    // separately binds the actual trace identities to the bodies and colliders it created.
    assert_eq!(actual.target_kind, control.target_kind);
    assert_eq!(actual.swept_start, control.swept_start);
    assert_eq!(actual.swept_end, control.swept_end);
    assert_eq!(actual.target_swept_start, control.target_swept_start);
    assert_eq!(actual.target_swept_end, control.target_swept_end);
    assert_eq!(actual.toi, control.toi);
    assert_eq!(actual.advancement, control.advancement);
    assert_eq!(actual.clamp, control.clamp);
    assert_eq!(actual.target_clamp, control.target_clamp);
    assert_eq!(actual.slop, control.slop);
    assert_eq!(actual.toi_point, control.toi_point);
}

fn run_ccd_spinner_without_joint(local_anchor: Point) -> CcdSpinnerControl {
    let mut world = no_gravity_world();
    let wall = create_body(&mut world, BodyType::Static, 0.0, 0.0, Vector::default());
    let spinner = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(-1.0, 0.0, 0.0),
            linear_velocity: Vector::new(200.0, 0.0),
            angular_velocity: CCD_SPINNER_INITIAL_ANGULAR_VELOCITY,
            can_sleep: false,
            ..BodyDesc::default()
        })
        .expect("CCD spinner should be created");
    let material = Material {
        friction: 1.0,
        restitution: 0.0,
    };
    attach_shape(&mut world, wall, SharedShape::rect(0.1, 10.0), material);
    attach_shape(&mut world, spinner, SharedShape::circle(0.05), material);
    let report = step_world(&mut world, 1);
    let body = world.try_body(spinner).expect("CCD spinner should survive");
    let final_pose = body.pose();
    let final_angular_velocity = body.angular_velocity();
    let mass_properties = body.mass_properties();
    let trace = active_contact_events(&report)
        .into_iter()
        .find_map(|contact| contact.ccd_trace)
        .expect("S5_HARNESS_BOUNDARY:control_CCD_clamp_must_emit_trace");
    assert!(final_pose.transform_point(local_anchor).x().is_finite());
    CcdSpinnerControl {
        final_angular_velocity,
        mass_properties,
        report,
        trace,
    }
}

fn ccd_zero_angular_correction_oracle(
    control: &CcdSpinnerControl,
    local_anchor: Point,
    radial_error_length: f32,
) -> CcdZeroAngularCorrectionOracle {
    let sweep = control.trace.swept_end - control.trace.swept_start;
    let sweep_direction = sweep.normalized_or_zero();
    assert!(
        sweep_direction.length() > f32::EPSILON,
        "S5_HARNESS_BOUNDARY:CCD_control_requires_nonzero_sweep"
    );
    let clamped_translation =
        Vector::from(control.trace.swept_end) - sweep_direction * control.trace.clamp;
    let sampled_eval_angle = CCD_SPINNER_INITIAL_ANGULAR_VELOCITY * DT;
    let eval_pose = Pose::from_xy_angle(
        clamped_translation.x(),
        clamped_translation.y(),
        sampled_eval_angle,
    );
    let eval_anchor = eval_pose.transform_point(local_anchor);
    let eval_world_com = eval_pose.transform_point(control.mass_properties.local_center_of_mass);
    let lever = eval_anchor - eval_world_com;
    let radial_error = lever.normalized_or_zero() * radial_error_length;
    let target_anchor = eval_anchor + radial_error;

    assert!(
        control.mass_properties.inverse_mass > 0.0
            && control.mass_properties.inverse_inertia > 0.0
            && lever.length() > f32::EPSILON
            && radial_error.length() > 0.01,
        "S5_HARNESS_BOUNDARY:CCD_oracle_requires_solvable_off_center_endpoint"
    );

    // With a static counterpart, K has `lever` as its translational eigenvector. A
    // constraint error parallel to `lever` therefore produces a parallel lambda and zero
    // cross product, proving the position phase's expected angular correction is exactly zero.
    let inverse_mass = control.mass_properties.inverse_mass;
    let inverse_inertia = control.mass_properties.inverse_inertia;
    let k11 = inverse_mass + inverse_inertia * lever.y() * lever.y();
    let k12 = -inverse_inertia * lever.x() * lever.y();
    let k22 = inverse_mass + inverse_inertia * lever.x() * lever.x();
    let determinant = k11 * k22 - k12 * k12;
    assert!(determinant.is_finite() && determinant > f32::EPSILON);
    let constraint_error = eval_anchor - target_anchor;
    let lambda = Vector::new(
        -(k22 * constraint_error.x() - k12 * constraint_error.y()) / determinant,
        -(-k12 * constraint_error.x() + k11 * constraint_error.y()) / determinant,
    );
    let expected_position_delta_angle = -inverse_inertia * lever.cross(lambda);
    assert!(
        expected_position_delta_angle.abs() <= 1.0e-6,
        "S5_HARNESS_BOUNDARY:CCD_oracle_position_delta_angle_must_be_zero:delta_angle={expected_position_delta_angle}"
    );

    CcdZeroAngularCorrectionOracle {
        target_anchor,
        clamped_translation,
        sampled_eval_angle,
        lever,
        radial_error,
        expected_position_delta_angle,
    }
}

#[test]
fn revolute_joint_ccd_clamped_rotating_endpoint_uses_final_angle() {
    let local_anchor = Point::new(0.0, 0.2);
    let control = run_ccd_spinner_without_joint(local_anchor);
    assert_eq!(control.report.stats.ccd_clamp_count, 1);
    assert!(control.report.stats.contact_count > 0);
    assert!(
        (control.final_angular_velocity - CCD_SPINNER_INITIAL_ANGULAR_VELOCITY).abs() > 1.0e-4,
        "S5_HARNESS_BOUNDARY:control_contact_must_mutate_angular_velocity"
    );
    let oracle = ccd_zero_angular_correction_oracle(&control, local_anchor, 0.05);
    let target_anchor = oracle.target_anchor;

    let mut world = no_gravity_world();
    let wall = create_body(&mut world, BodyType::Static, 0.0, 0.0, Vector::default());
    let target = create_body(
        &mut world,
        BodyType::Static,
        target_anchor.x(),
        target_anchor.y(),
        Vector::default(),
    );
    let spinner = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(-1.0, 0.0, 0.0),
            linear_velocity: Vector::new(200.0, 0.0),
            angular_velocity: CCD_SPINNER_INITIAL_ANGULAR_VELOCITY,
            can_sleep: false,
            ..BodyDesc::default()
        })
        .expect("CCD spinner should be created");
    let material = Material {
        friction: 1.0,
        restitution: 0.0,
    };
    let wall_collider = attach_shape(&mut world, wall, SharedShape::rect(0.1, 10.0), material);
    let spinner_collider = attach_shape(&mut world, spinner, SharedShape::circle(0.05), material);
    create_revolute(&mut world, target, spinner, Point::default(), local_anchor);

    let report = step_world(&mut world, 1);
    let spinner_view = world.try_body(spinner).expect("spinner should survive");
    let expected_final_angle =
        oracle.expected_position_delta_angle + spinner_view.angular_velocity() * DT;
    let drift = revolute_anchor_drift(&world, target, Point::default(), spinner, local_anchor);
    let actual_trace = active_contact_events(&report)
        .into_iter()
        .find_map(|contact| contact.ccd_trace)
        .expect("S5_HARNESS_BOUNDARY:CCD clamp must produce a contact fact");
    println!(
        "S5_REVOLUTE_FACT:ccd_final_angle:clamps={};contacts={};clamped_translation={:?};sampled_eval_angle={:.6};lever={:?};radial_error={:?};position_delta_angle={:.6};initial_omega={:.6};latest_omega={:.6};angle={:.6};expected_angle={expected_final_angle:.6};drift={drift:.6}",
        report.stats.ccd_clamp_count,
        report.stats.contact_count,
        oracle.clamped_translation,
        oracle.sampled_eval_angle,
        oracle.lever,
        oracle.radial_error,
        oracle.expected_position_delta_angle,
        CCD_SPINNER_INITIAL_ANGULAR_VELOCITY,
        spinner_view.angular_velocity(),
        spinner_view.pose().angle()
    );
    assert_eq!(
        report.stats.ccd_clamp_count, 1,
        "S5_HARNESS_BOUNDARY:ccd_did_not_clamp"
    );
    assert!(
        report.stats.contact_count > 0,
        "S5_HARNESS_BOUNDARY:ccd_contact_missing"
    );
    assert_eq!(actual_trace.moving_body, spinner);
    assert_eq!(actual_trace.static_body, wall);
    assert_eq!(actual_trace.moving_collider, spinner_collider);
    assert_eq!(actual_trace.static_collider, wall_collider);
    assert_same_ccd_geometry(&actual_trace, &control.trace);
    assert!(
        (spinner_view.angular_velocity() - CCD_SPINNER_INITIAL_ANGULAR_VELOCITY).abs() > 1.0e-4,
        "S5_HARNESS_BOUNDARY:contact_must_mutate_angular_velocity"
    );
    assert!(
        (spinner_view.pose().angle() - expected_final_angle).abs() <= 1.0e-4,
        "S5_HARNESS_BOUNDARY:ccd_must_preserve_latest_angular_advance"
    );
    assert!(
        drift <= 0.01,
        "S5_REVOLUTE_SOLVER_ASSERT:ccd_final_angle_pivot_drift:drift={drift:.6};limit=0.01"
    );
}

#[test]
fn revolute_joint_awake_sleeping_pair_uses_current_pose_and_wakes_on_correction() {
    // Inactive zero-correction and singular rows are boundary controls: they must not wake
    // sleeping endpoints merely because a Revolute descriptor exists.
    for (label, anchor_b, attach_mass) in [
        ("zero", Point::new(-1.0, 0.0), true),
        ("singular", Point::default(), false),
        ("inactive_skip", Point::new(-0.75, 0.0), true),
    ] {
        let mut boundary_world = no_gravity_world();
        let body_a = boundary_world
            .create_body(BodyDesc {
                body_type: BodyType::Dynamic,
                pose: Pose::from_xy_angle(-1.0, 0.0, 0.0),
                can_sleep: true,
                ..BodyDesc::default()
            })
            .expect("boundary body A should be created");
        let body_b = boundary_world
            .create_body(BodyDesc {
                body_type: BodyType::Dynamic,
                pose: Pose::from_xy_angle(1.0, 0.0, 0.0),
                can_sleep: true,
                ..BodyDesc::default()
            })
            .expect("boundary body B should be created");
        if attach_mass {
            for body in [body_a, body_b] {
                attach_shape(
                    &mut boundary_world,
                    body,
                    SharedShape::rect(0.5, 0.5),
                    Material::default(),
                );
            }
        } else {
            assert_eq!(
                boundary_world
                    .try_body(body_a)
                    .expect("body A exists")
                    .mass_properties()
                    .inverse_mass,
                0.0
            );
        }
        create_revolute(
            &mut boundary_world,
            body_a,
            body_b,
            Point::new(1.0, 0.0),
            anchor_b,
        );
        step_world(&mut boundary_world, 31);
        assert!(boundary_world
            .try_body(body_a)
            .expect("body A exists")
            .sleeping());
        assert!(boundary_world
            .try_body(body_b)
            .expect("body B exists")
            .sleeping());
        let report = step_world(&mut boundary_world, 1);
        println!(
            "S5_REVOLUTE_BOUNDARY:{label}:body_a={:?};body_b={:?};events={:?};joint_rows={}",
            boundary_world
                .try_body(body_a)
                .expect("body A exists")
                .status(),
            boundary_world
                .try_body(body_b)
                .expect("body B exists")
                .status(),
            report.events,
            report.stats.joint_row_count
        );
        assert!(boundary_world
            .try_body(body_a)
            .expect("body A exists")
            .sleeping());
        assert!(boundary_world
            .try_body(body_b)
            .expect("body B exists")
            .sleeping());
        assert!(!report.events.iter().any(|event| matches!(
            event,
            WorldEvent::SleepChanged(event)
                if !event.is_sleeping
                    && event.reason == picea::events::SleepTransitionReason::JointCorrection
        )));
        println!(
            "S5_REVOLUTE_BOUNDARY:{label}:sleeping=true;joint_rows={}",
            report.stats.joint_row_count
        );
    }

    let mut world = no_gravity_world();
    let sleeper = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(-1.0, 0.0, 0.0),
            can_sleep: true,
            ..BodyDesc::default()
        })
        .expect("sleeping endpoint should be created");
    let awake = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(1.0, 0.0, 0.0),
            can_sleep: true,
            ..BodyDesc::default()
        })
        .expect("awake endpoint should be created");
    for body in [sleeper, awake] {
        attach_shape(
            &mut world,
            body,
            SharedShape::rect(0.5, 0.5),
            Material::default(),
        );
    }
    create_revolute(
        &mut world,
        sleeper,
        awake,
        Point::new(1.0, 0.0),
        Point::new(-1.0, 0.0),
    );
    step_world(&mut world, 1);
    world
        .apply_body_patch(
            sleeper,
            BodyPatch {
                linear_velocity: Some(Vector::new(-4.0, 0.0)),
                angular_velocity: Some(2.0),
                sleeping: Some(true),
                ..BodyPatch::default()
            },
        )
        .expect("sleeper fixture patch should apply without a pending wake");
    world
        .apply_body_patch(
            awake,
            BodyPatch {
                linear_velocity: Some(Vector::new(4.0, 0.0)),
                ..BodyPatch::default()
            },
        )
        .expect("awake fixture velocity should apply");
    let sleeper_before = world.try_body(sleeper).expect("sleeper exists").pose();
    assert!(world.try_body(sleeper).expect("sleeper exists").sleeping());

    let report = step_world(&mut world, 1);
    let sleeper_after = world.try_body(sleeper).expect("sleeper exists").pose();
    let correction = (sleeper_after.translation() - sleeper_before.translation()).length()
        + (sleeper_after.angle() - sleeper_before.angle()).abs();
    let correction_wake = report.events.iter().find_map(|event| match event {
        WorldEvent::SleepChanged(event)
            if event.body == sleeper
                && !event.is_sleeping
                && event.reason == picea::events::SleepTransitionReason::JointCorrection =>
        {
            Some(*event)
        }
        _ => None,
    });
    println!(
        "S5_REVOLUTE_FACT:awake_sleeping:finite_correction={correction:.6};sleeping_after={};wake={correction_wake:?};events={:?}",
        world.try_body(sleeper).expect("sleeper exists").sleeping(),
        report.events
    );
    assert_revolute_body_finite(&world, sleeper, "awake_sleeping_sleeper");
    assert_revolute_body_finite(&world, awake, "awake_sleeping_awake");
    assert!(
        correction_wake.is_some(),
        "S5_REVOLUTE_SOLVER_ASSERT:finite_nonzero_correction_must_wake_with_joint_correction"
    );
    assert!(
        correction > f32::EPSILON,
        "S5_REVOLUTE_SOLVER_ASSERT:awake_sleeping_pair_requires_nonzero_correction"
    );
}

struct RevoluteContactWindow {
    max_drift: f32,
    final_drift: f32,
    contact_frames: usize,
    numeric_warnings: usize,
    final_joint_rows: usize,
}

fn run_revolute_contact_window(joint_velocity_projection: bool) -> RevoluteContactWindow {
    let mut world = World::new(WorldDesc {
        gravity: Vector::new(0.0, 9.8),
        ..WorldDesc::default()
    });
    let floor = create_body(&mut world, BodyType::Static, 0.0, 2.5, Vector::default());
    let pendulum = create_body(
        &mut world,
        BodyType::Dynamic,
        0.0,
        0.0,
        Vector::new(0.5, 0.0),
    );
    let material = Material {
        friction: 0.8,
        restitution: 0.0,
    };
    attach_shape(&mut world, floor, SharedShape::rect(8.0, 0.5), material);
    attach_shape(&mut world, pendulum, SharedShape::rect(1.0, 2.0), material);
    let floor_anchor = Point::new(0.0, -3.5);
    let pendulum_anchor = Point::new(0.0, -1.0);
    create_revolute(&mut world, floor, pendulum, floor_anchor, pendulum_anchor);
    let mut pipeline = SimulationPipeline::new(StepConfig {
        joint_velocity_projection,
        ..fixed_step_config()
    });
    let mut facts = RevoluteContactWindow {
        max_drift: 0.0,
        final_drift: 0.0,
        contact_frames: 0,
        numeric_warnings: 0,
        final_joint_rows: 0,
    };
    for _ in 0..240 {
        let report = pipeline.step(&mut world);
        facts.contact_frames += usize::from(report.stats.contact_count > 0);
        facts.numeric_warnings += report.stats.numeric_warnings;
        facts.final_joint_rows = report.stats.joint_row_count;
        facts.max_drift = facts.max_drift.max(revolute_anchor_drift(
            &world,
            floor,
            floor_anchor,
            pendulum,
            pendulum_anchor,
        ));
    }
    facts.final_drift =
        revolute_anchor_drift(&world, floor, floor_anchor, pendulum, pendulum_anchor);
    facts
}

fn assert_revolute_contact_window(facts: &RevoluteContactWindow, projection: &str) {
    println!(
        "S5_REVOLUTE_FACT:contact_window:{projection}:contact_frames={};max_drift={:.6};final_drift={:.6};numeric_warnings={};final_joint_rows={}",
        facts.contact_frames,
        facts.max_drift,
        facts.final_drift,
        facts.numeric_warnings,
        facts.final_joint_rows
    );
    assert!(
        facts.contact_frames > 0,
        "S5_HARNESS_BOUNDARY:{projection}_contact_never_occurred"
    );
    assert_eq!(
        facts.numeric_warnings, 0,
        "S5_HARNESS_BOUNDARY:{projection}_numeric_warnings"
    );
    assert!(
        facts.max_drift <= 0.03,
        "S5_REVOLUTE_SOLVER_ASSERT:{projection}_contact_max_drift:max_drift={:.6};limit=0.03",
        facts.max_drift
    );
    assert!(
        facts.final_drift <= 0.01,
        "S5_REVOLUTE_SOLVER_ASSERT:{projection}_contact_final_drift:final_drift={:.6};limit=0.01",
        facts.final_drift
    );
}

#[test]
fn revolute_joint_contact_full_step_preserves_pivot_with_projection_enabled() {
    let facts = run_revolute_contact_window(true);
    assert_revolute_contact_window(&facts, "projection_enabled");
}

#[test]
fn revolute_joint_contact_full_step_preserves_pivot_with_projection_disabled() {
    let facts = run_revolute_contact_window(false);
    assert_revolute_contact_window(&facts, "projection_disabled");
}

#[test]
fn revolute_joint_ccd_clamped_contact_full_step_preserves_pivot() {
    let local_anchor = Point::new(0.0, 0.2);
    let control = run_ccd_spinner_without_joint(local_anchor);
    assert_eq!(control.report.stats.ccd_clamp_count, 1);
    assert!(control.report.stats.contact_count > 0);
    assert!(
        (control.final_angular_velocity - CCD_SPINNER_INITIAL_ANGULAR_VELOCITY).abs() > 1.0e-4,
        "S5_HARNESS_BOUNDARY:control_contact_must_mutate_angular_velocity"
    );
    let oracle = ccd_zero_angular_correction_oracle(&control, local_anchor, 0.02);
    let target_anchor = oracle.target_anchor;

    let mut world = no_gravity_world();
    let wall = create_body(&mut world, BodyType::Static, 0.0, 0.0, Vector::default());
    let target = create_body(
        &mut world,
        BodyType::Static,
        target_anchor.x(),
        target_anchor.y(),
        Vector::default(),
    );
    let spinner = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(-1.0, 0.0, 0.0),
            linear_velocity: Vector::new(200.0, 0.0),
            angular_velocity: CCD_SPINNER_INITIAL_ANGULAR_VELOCITY,
            can_sleep: false,
            ..BodyDesc::default()
        })
        .expect("CCD spinner should be created");
    let material = Material {
        friction: 1.0,
        restitution: 0.0,
    };
    let wall_collider = attach_shape(&mut world, wall, SharedShape::rect(0.1, 10.0), material);
    let spinner_collider = attach_shape(&mut world, spinner, SharedShape::circle(0.05), material);
    create_revolute(&mut world, target, spinner, Point::default(), local_anchor);

    let report = step_world(&mut world, 1);
    let spinner_view = world.try_body(spinner).expect("spinner should survive");
    let drift = revolute_anchor_drift(&world, target, Point::default(), spinner, local_anchor);
    let ccd_contact = active_contact_events(&report)
        .into_iter()
        .find(|contact| contact.ccd_trace.is_some())
        .expect("S5_HARNESS_BOUNDARY:CCD clamp must produce a contact fact");
    let actual_trace = ccd_contact
        .ccd_trace
        .expect("S5_HARNESS_BOUNDARY:CCD contact must preserve its trace");
    let expected_angle =
        oracle.expected_position_delta_angle + spinner_view.angular_velocity() * DT;
    println!(
        "S5_REVOLUTE_FACT:ccd_contact_full_step:clamps={};contacts={};solver_normal_impulse={:.6};clamped_translation={:?};sampled_eval_angle={:.6};lever={:?};radial_error={:?};position_delta_angle={:.6};initial_omega={:.6};final_omega={:.6};angle={:.6};expected_angle={expected_angle:.6};drift={drift:.6}",
        report.stats.ccd_clamp_count,
        report.stats.contact_count,
        ccd_contact.solver_normal_impulse,
        oracle.clamped_translation,
        oracle.sampled_eval_angle,
        oracle.lever,
        oracle.radial_error,
        oracle.expected_position_delta_angle,
        CCD_SPINNER_INITIAL_ANGULAR_VELOCITY,
        spinner_view.angular_velocity(),
        spinner_view.pose().angle()
    );
    assert_eq!(
        report.stats.ccd_clamp_count, 1,
        "S5_HARNESS_BOUNDARY:ccd_did_not_clamp"
    );
    assert!(
        report.stats.contact_count > 0,
        "S5_HARNESS_BOUNDARY:ccd_contact_missing"
    );
    assert!(
        ccd_contact.solver_normal_impulse > 0.0,
        "S5_HARNESS_BOUNDARY:ccd_contact_must_reach_solver"
    );
    assert_eq!(actual_trace.moving_body, spinner);
    assert_eq!(actual_trace.static_body, wall);
    assert_eq!(actual_trace.moving_collider, spinner_collider);
    assert_eq!(actual_trace.static_collider, wall_collider);
    assert_same_ccd_geometry(&actual_trace, &control.trace);
    assert!(
        (spinner_view.angular_velocity() - CCD_SPINNER_INITIAL_ANGULAR_VELOCITY).abs() > 1.0e-4,
        "S5_HARNESS_BOUNDARY:contact_must_mutate_angular_velocity"
    );
    assert!(
        (spinner_view.pose().angle() - expected_angle).abs() <= 1.0e-4,
        "S5_HARNESS_BOUNDARY:ccd_angular_integration_must_use_latest_velocity"
    );
    assert!(
        drift <= 0.01,
        "S5_REVOLUTE_SOLVER_ASSERT:ccd_contact_full_step_pivot_drift:drift={drift:.6};limit=0.01"
    );
}
