use picea::debug::DebugStats;
use picea::math::{point::Point, vector::Vector};
use picea::prelude::{
    BodyDesc, BodyHandle, BodyType, ColliderDesc, ColliderHandle, CollisionFilter, ContactEvent,
    ContactFeatureId, ContactId, ContactReductionReason, DebugBody, DebugContact, DebugIsland,
    DebugManifold, DebugManifoldPoint, DebugSnapshot, DebugSnapshotOptions, EpaTerminationReason,
    GenericConvexFallbackReason, GenericConvexTrace, GjkTerminationReason, ManifoldId, Material,
    Pose, QueryFilter, QueryPipeline, SharedShape, SimulationPipeline, SleepEvent,
    SleepTransitionReason, StepConfig, StepReport, StepStats, WarmStartCacheReason, World,
    WorldDesc, WorldEvent,
};

fn step_once(world: &mut World) -> StepReport {
    SimulationPipeline::new(StepConfig::default()).step(world)
}

#[test]
fn debug_snapshot_defaults_to_empty_stable_fact_layers() {
    let snapshot = DebugSnapshot::default();

    assert!(snapshot.bodies.is_empty());
    assert!(snapshot.colliders.is_empty());
    assert!(snapshot.joints.is_empty());
    assert!(snapshot.contacts.is_empty());
    assert!(snapshot.manifolds.is_empty());
    assert!(snapshot.islands.is_empty());
    assert!(snapshot.primitives.is_empty());
}

#[test]
fn query_pipeline_returns_no_hits_before_sync() {
    let query = QueryPipeline::new();

    assert!(query
        .intersect_point(Point::new(0.0, 0.0), QueryFilter::default())
        .is_empty());
}

#[test]
fn debug_snapshot_with_step_report_preserves_step_facts_and_collider_semantics() {
    let mut world = World::new(WorldDesc::default());
    let body_a = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(0.0, 0.0, 0.0),
            ..BodyDesc::default()
        })
        .expect("body should be created");
    let body_b = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(3.0, 0.0, 0.0),
            ..BodyDesc::default()
        })
        .expect("body should be created");

    let material_a = Material {
        friction: 0.7,
        restitution: 0.3,
    };
    let filter_a = CollisionFilter {
        memberships: 0b0001,
        collides_with: 0b0010,
    };
    let collider_a = world
        .create_collider(
            body_a,
            ColliderDesc {
                shape: SharedShape::circle(1.0),
                density: 2.5,
                material: material_a,
                filter: filter_a,
                ..ColliderDesc::default()
            },
        )
        .expect("first collider should be created");
    let collider_b = world
        .create_collider(
            body_b,
            ColliderDesc {
                shape: SharedShape::rect(1.0, 2.0),
                density: 3.0,
                material: Material {
                    friction: 0.2,
                    restitution: 0.1,
                },
                filter: CollisionFilter {
                    memberships: 0b0010,
                    collides_with: 0b0001,
                },
                ..ColliderDesc::default()
            },
        )
        .expect("second collider should be created");

    let report = StepReport {
        step_index: 3,
        simulated_time: 0.05,
        dt: 1.0 / 60.0,
        revision: world.revision(),
        stats: StepStats {
            step_index: 3,
            body_count: 2,
            collider_count: 2,
            active_body_count: 2,
            broadphase_candidate_count: 4,
            broadphase_update_count: 2,
            broadphase_stale_proxy_drop_count: 1,
            broadphase_same_body_drop_count: 1,
            broadphase_filter_drop_count: 1,
            broadphase_narrowphase_drop_count: 1,
            broadphase_rebuild_count: 1,
            broadphase_tree_depth: 3,
            contact_count: 1,
            manifold_count: 1,
            warm_start_hit_count: 1,
            warm_start_miss_count: 2,
            warm_start_drop_count: 3,
            ccd_candidate_count: 4,
            ccd_hit_count: 2,
            ccd_miss_count: 1,
            ccd_clamp_count: 2,
            ..StepStats::default()
        },
        events: vec![WorldEvent::ContactStarted(ContactEvent {
            contact_id: ContactId::default(),
            manifold_id: ManifoldId::default(),
            body_a,
            body_b,
            collider_a,
            collider_b,
            feature_id: ContactFeatureId::default(),
            point: Point::new(1.0, 0.0),
            normal: Vector::new(-1.0, 0.0),
            depth: 0.125,
            reduction_reason: ContactReductionReason::Clipped,
            warm_start_reason: WarmStartCacheReason::Hit,
            warm_start_normal_impulse: 1.25,
            warm_start_tangent_impulse: -0.5,
            solver_normal_impulse: 1.5,
            solver_tangent_impulse: -0.25,
            normal_impulse_clamped: false,
            tangent_impulse_clamped: true,
            restitution_velocity_threshold: 1.75,
            restitution_applied: true,
            generic_convex_trace: Some(GenericConvexTrace {
                fallback_reason: GenericConvexFallbackReason::GenericConvexFallback,
                gjk_termination: GjkTerminationReason::Intersect,
                epa_termination: EpaTerminationReason::Converged,
                gjk_iterations: 3,
                epa_iterations: 2,
                simplex_len: 3,
            }),
            ccd_trace: None,
        })],
    };

    let snapshot = DebugSnapshot::from_world_with_step_report(
        &world,
        &report,
        &DebugSnapshotOptions::default(),
    );

    assert_eq!(snapshot.meta.dt, report.dt);
    assert_eq!(snapshot.meta.simulated_time, report.simulated_time);
    assert_eq!(snapshot.stats.step_index, report.step_index);
    assert_eq!(snapshot.stats.broadphase_candidate_count, 4);
    assert_eq!(snapshot.stats.broadphase_update_count, 2);
    assert_eq!(snapshot.stats.broadphase_stale_proxy_drop_count, 1);
    assert_eq!(snapshot.stats.broadphase_same_body_drop_count, 1);
    assert_eq!(snapshot.stats.broadphase_filter_drop_count, 1);
    assert_eq!(snapshot.stats.broadphase_narrowphase_drop_count, 1);
    assert_eq!(snapshot.stats.broadphase_rebuild_count, 1);
    assert_eq!(snapshot.stats.broadphase_tree_depth, 3);
    assert_eq!(snapshot.stats.warm_start_hit_count, 1);
    assert_eq!(snapshot.stats.warm_start_miss_count, 2);
    assert_eq!(snapshot.stats.warm_start_drop_count, 3);
    assert_eq!(snapshot.stats.ccd_candidate_count, 4);
    assert_eq!(snapshot.stats.ccd_hit_count, 2);
    assert_eq!(snapshot.stats.ccd_miss_count, 1);
    assert_eq!(snapshot.stats.ccd_clamp_count, 2);
    assert_eq!(snapshot.contacts.len(), 1);
    assert_eq!(snapshot.manifolds.len(), 1);
    assert!(!snapshot.primitives.is_empty());

    let body = snapshot
        .bodies
        .iter()
        .find(|body| body.handle == body_a)
        .expect("snapshot should contain body mass properties");
    assert!((body.mass_properties.mass - 2.5 * std::f32::consts::PI).abs() <= 1e-4);
    assert!(body.mass_properties.inverse_mass > 0.0);

    let collider = snapshot
        .colliders
        .iter()
        .find(|collider| collider.handle == collider_a)
        .expect("snapshot should contain collider semantics");
    assert_eq!(collider.density, 2.5);
    assert_eq!(collider.material, material_a);
    assert_eq!(collider.filter, filter_a);

    assert_eq!(snapshot.contacts[0].colliders, [collider_a, collider_b]);
    assert_eq!(snapshot.contacts[0].feature_id, ContactFeatureId::default());
    assert_eq!(
        snapshot.contacts[0].reduction_reason,
        ContactReductionReason::Clipped
    );
    assert_eq!(
        snapshot.contacts[0].warm_start_reason,
        WarmStartCacheReason::Hit
    );
    assert_eq!(snapshot.contacts[0].normal_impulse, 1.25);
    assert_eq!(snapshot.contacts[0].tangent_impulse, -0.5);
    assert_eq!(snapshot.contacts[0].solver_normal_impulse, 1.5);
    assert_eq!(snapshot.contacts[0].solver_tangent_impulse, -0.25);
    assert!(!snapshot.contacts[0].normal_impulse_clamped);
    assert!(snapshot.contacts[0].tangent_impulse_clamped);
    assert_eq!(snapshot.contacts[0].restitution_velocity_threshold, 1.75);
    assert!(snapshot.contacts[0].restitution_applied);
    assert_eq!(
        snapshot.contacts[0]
            .generic_convex_trace
            .expect("debug contact should preserve generic trace")
            .fallback_reason,
        GenericConvexFallbackReason::GenericConvexFallback
    );
    assert_eq!(
        snapshot.manifolds[0].contact_ids,
        vec![ContactId::default()]
    );
    assert_eq!(snapshot.manifolds[0].points.len(), 1);
    assert_eq!(snapshot.manifolds[0].normal, Vector::new(-1.0, 0.0));
    assert_eq!(snapshot.manifolds[0].depth, 0.125);
    assert_eq!(snapshot.manifolds[0].warm_start_hit_count, 1);
    assert_eq!(snapshot.manifolds[0].warm_start_miss_count, 0);
    assert_eq!(snapshot.manifolds[0].warm_start_drop_count, 0);
}

#[test]
fn warm_start_debug_snapshot_exposes_cache_facts() {
    let mut world = World::new(WorldDesc {
        gravity: Vector::default(),
        enable_sleep: false,
    });
    let left = world
        .create_body(BodyDesc {
            body_type: BodyType::Static,
            pose: Pose::from_xy_angle(0.0, 0.0, 0.0),
            can_sleep: false,
            ..BodyDesc::default()
        })
        .expect("left body should be created");
    let right = world
        .create_body(BodyDesc {
            body_type: BodyType::Static,
            pose: Pose::from_xy_angle(1.5, 0.0, 0.0),
            can_sleep: false,
            ..BodyDesc::default()
        })
        .expect("right body should be created");
    world
        .create_collider(
            left,
            ColliderDesc {
                shape: SharedShape::rect(2.0, 2.0),
                ..ColliderDesc::default()
            },
        )
        .expect("left collider should be created");
    world
        .create_collider(
            right,
            ColliderDesc {
                shape: SharedShape::rect(2.0, 2.0),
                ..ColliderDesc::default()
            },
        )
        .expect("right collider should be created");
    let mut pipeline = SimulationPipeline::new(StepConfig::default());
    let first = pipeline.step(&mut world);
    let second = pipeline.step(&mut world);

    let first_snapshot = DebugSnapshot::from_world_with_step_report(
        &world,
        &first,
        &DebugSnapshotOptions::default(),
    );
    let second_snapshot = DebugSnapshot::from_world_with_step_report(
        &world,
        &second,
        &DebugSnapshotOptions::default(),
    );

    assert_eq!(
        first_snapshot.stats.warm_start_miss_count,
        first_snapshot.contacts.len()
    );
    assert_eq!(
        second_snapshot.stats.warm_start_hit_count,
        second_snapshot.contacts.len()
    );
    assert!(second_snapshot
        .contacts
        .iter()
        .all(|contact| contact.warm_start_reason == WarmStartCacheReason::Hit));
    assert_eq!(
        second_snapshot.manifolds[0].warm_start_hit_count,
        second_snapshot.contacts.len()
    );
}

#[test]
fn warm_start_new_picea_payload_fields_default_when_deserializing_older_json() {
    let mut contact_value =
        serde_json::to_value(ContactEvent::default()).expect("contact event should serialize");
    remove_json_fields(
        &mut contact_value,
        &[
            "warm_start_reason",
            "warm_start_normal_impulse",
            "warm_start_tangent_impulse",
            "solver_normal_impulse",
            "solver_tangent_impulse",
            "normal_impulse_clamped",
            "tangent_impulse_clamped",
            "restitution_velocity_threshold",
            "restitution_applied",
            "generic_convex_trace",
            "ccd_trace",
        ],
    );
    let contact: ContactEvent =
        serde_json::from_value(contact_value).expect("older contact event should deserialize");
    assert_eq!(
        contact.warm_start_reason,
        WarmStartCacheReason::MissNoPrevious
    );
    assert_eq!(contact.warm_start_normal_impulse, 0.0);
    assert_eq!(contact.warm_start_tangent_impulse, 0.0);
    assert_eq!(contact.solver_normal_impulse, 0.0);
    assert_eq!(contact.solver_tangent_impulse, 0.0);
    assert!(!contact.normal_impulse_clamped);
    assert!(!contact.tangent_impulse_clamped);
    assert_eq!(contact.restitution_velocity_threshold, 0.0);
    assert!(!contact.restitution_applied);
    assert_eq!(contact.generic_convex_trace, None);
    assert_eq!(contact.ccd_trace, None);

    let mut stats_value =
        serde_json::to_value(StepStats::default()).expect("step stats should serialize");
    remove_json_fields(
        &mut stats_value,
        &[
            "warm_start_hit_count",
            "warm_start_miss_count",
            "warm_start_drop_count",
            "ccd_candidate_count",
            "ccd_hit_count",
            "ccd_miss_count",
            "ccd_clamp_count",
        ],
    );
    let stats: StepStats =
        serde_json::from_value(stats_value).expect("older step stats should deserialize");
    assert_eq!(stats.warm_start_hit_count, 0);
    assert_eq!(stats.warm_start_miss_count, 0);
    assert_eq!(stats.warm_start_drop_count, 0);
    assert_eq!(stats.ccd_candidate_count, 0);
    assert_eq!(stats.ccd_hit_count, 0);
    assert_eq!(stats.ccd_miss_count, 0);
    assert_eq!(stats.ccd_clamp_count, 0);

    let mut debug_stats_value =
        serde_json::to_value(DebugStats::default()).expect("debug stats should serialize");
    remove_json_fields(
        &mut debug_stats_value,
        &[
            "warm_start_hit_count",
            "warm_start_miss_count",
            "warm_start_drop_count",
            "ccd_candidate_count",
            "ccd_hit_count",
            "ccd_miss_count",
            "ccd_clamp_count",
        ],
    );
    let debug_stats: DebugStats =
        serde_json::from_value(debug_stats_value).expect("older debug stats should deserialize");
    assert_eq!(debug_stats.warm_start_hit_count, 0);
    assert_eq!(debug_stats.warm_start_miss_count, 0);
    assert_eq!(debug_stats.warm_start_drop_count, 0);
    assert_eq!(debug_stats.ccd_candidate_count, 0);
    assert_eq!(debug_stats.ccd_hit_count, 0);
    assert_eq!(debug_stats.ccd_miss_count, 0);
    assert_eq!(debug_stats.ccd_clamp_count, 0);

    let debug_contact = DebugContact {
        id: ContactId::default(),
        bodies: [BodyHandle::default(), BodyHandle::default()],
        colliders: [ColliderHandle::default(), ColliderHandle::default()],
        feature_id: ContactFeatureId::default(),
        point: Point::new(0.0, 0.0),
        normal: Vector::new(0.0, 1.0),
        depth: 0.0,
        reduction_reason: ContactReductionReason::SinglePoint,
        warm_start_reason: WarmStartCacheReason::Hit,
        normal_impulse: 1.0,
        tangent_impulse: -1.0,
        solver_normal_impulse: 1.5,
        solver_tangent_impulse: -0.25,
        normal_impulse_clamped: false,
        tangent_impulse_clamped: true,
        restitution_velocity_threshold: 1.75,
        restitution_applied: true,
        generic_convex_trace: Some(GenericConvexTrace {
            fallback_reason: GenericConvexFallbackReason::GenericConvexFallback,
            gjk_termination: GjkTerminationReason::Intersect,
            epa_termination: EpaTerminationReason::Converged,
            gjk_iterations: 3,
            epa_iterations: 2,
            simplex_len: 3,
        }),
        ccd_trace: None,
    };
    let mut debug_contact_value =
        serde_json::to_value(debug_contact).expect("debug contact should serialize");
    remove_json_fields(
        &mut debug_contact_value,
        &[
            "warm_start_reason",
            "normal_impulse",
            "tangent_impulse",
            "solver_normal_impulse",
            "solver_tangent_impulse",
            "normal_impulse_clamped",
            "tangent_impulse_clamped",
            "restitution_velocity_threshold",
            "restitution_applied",
            "generic_convex_trace",
            "ccd_trace",
        ],
    );
    let decoded_debug_contact: DebugContact = serde_json::from_value(debug_contact_value)
        .expect("older debug contact should deserialize");
    assert_eq!(
        decoded_debug_contact.warm_start_reason,
        WarmStartCacheReason::MissNoPrevious
    );
    assert_eq!(decoded_debug_contact.normal_impulse, 0.0);
    assert_eq!(decoded_debug_contact.tangent_impulse, 0.0);
    assert_eq!(decoded_debug_contact.solver_normal_impulse, 0.0);
    assert_eq!(decoded_debug_contact.solver_tangent_impulse, 0.0);
    assert!(!decoded_debug_contact.normal_impulse_clamped);
    assert!(!decoded_debug_contact.tangent_impulse_clamped);
    assert_eq!(decoded_debug_contact.restitution_velocity_threshold, 0.0);
    assert!(!decoded_debug_contact.restitution_applied);
    assert_eq!(decoded_debug_contact.generic_convex_trace, None);
    assert_eq!(decoded_debug_contact.ccd_trace, None);

    let debug_manifold = DebugManifold {
        id: ManifoldId::default(),
        bodies: [BodyHandle::default(), BodyHandle::default()],
        colliders: [ColliderHandle::default(), ColliderHandle::default()],
        contact_ids: vec![ContactId::default()],
        points: vec![DebugManifoldPoint {
            contact_id: ContactId::default(),
            feature_id: ContactFeatureId::default(),
            point: Point::new(0.0, 0.0),
            depth: 0.0,
        }],
        normal: Vector::new(0.0, 1.0),
        depth: 0.0,
        reduction_reason: ContactReductionReason::SinglePoint,
        warm_start_hit_count: 1,
        warm_start_miss_count: 2,
        warm_start_drop_count: 3,
        generic_convex_trace: Some(GenericConvexTrace {
            fallback_reason: GenericConvexFallbackReason::GenericConvexFallback,
            gjk_termination: GjkTerminationReason::Intersect,
            epa_termination: EpaTerminationReason::Converged,
            gjk_iterations: 3,
            epa_iterations: 2,
            simplex_len: 3,
        }),
        active: true,
    };
    let mut debug_manifold_value =
        serde_json::to_value(debug_manifold).expect("debug manifold should serialize");
    remove_json_fields(
        &mut debug_manifold_value,
        &[
            "warm_start_hit_count",
            "warm_start_miss_count",
            "warm_start_drop_count",
            "generic_convex_trace",
        ],
    );
    let decoded_debug_manifold: DebugManifold = serde_json::from_value(debug_manifold_value)
        .expect("older debug manifold should deserialize");
    assert_eq!(decoded_debug_manifold.warm_start_hit_count, 0);
    assert_eq!(decoded_debug_manifold.warm_start_miss_count, 0);
    assert_eq!(decoded_debug_manifold.warm_start_drop_count, 0);
    assert_eq!(decoded_debug_manifold.generic_convex_trace, None);

    let mut sleep_value = serde_json::to_value(SleepEvent {
        body: BodyHandle::default(),
        is_sleeping: false,
        island_id: 7,
        reason: SleepTransitionReason::Impact,
    })
    .expect("sleep event should serialize");
    remove_json_fields(&mut sleep_value, &["island_id", "reason"]);
    let decoded_sleep: SleepEvent =
        serde_json::from_value(sleep_value).expect("older sleep event should deserialize");
    assert_eq!(decoded_sleep.island_id, 0);
    assert_eq!(decoded_sleep.reason, SleepTransitionReason::Unknown);

    let mut debug_body_value = serde_json::to_value(DebugBody {
        handle: BodyHandle::default(),
        body_type: BodyType::Dynamic,
        transform: Default::default(),
        mass_properties: Default::default(),
        linear_velocity: Vector::default(),
        angular_velocity: 0.0,
        sleeping: false,
        island_id: Some(1),
        user_data: 0,
    })
    .expect("debug body should serialize");
    remove_json_fields(&mut debug_body_value, &["island_id"]);
    let decoded_debug_body: DebugBody =
        serde_json::from_value(debug_body_value).expect("older debug body should deserialize");
    assert_eq!(decoded_debug_body.island_id, None);

    let mut debug_island_value = serde_json::to_value(DebugIsland {
        id: 1,
        bodies: vec![BodyHandle::default()],
        sleeping: false,
        reason: SleepTransitionReason::Impact,
    })
    .expect("debug island should serialize");
    remove_json_fields(&mut debug_island_value, &["reason"]);
    let decoded_debug_island: DebugIsland =
        serde_json::from_value(debug_island_value).expect("older debug island should deserialize");
    assert_eq!(decoded_debug_island.reason, SleepTransitionReason::Unknown);

    let mut snapshot_value =
        serde_json::to_value(DebugSnapshot::default()).expect("debug snapshot should serialize");
    remove_json_fields(&mut snapshot_value, &["islands"]);
    let decoded_snapshot: DebugSnapshot =
        serde_json::from_value(snapshot_value).expect("older debug snapshot should deserialize");
    assert!(decoded_snapshot.islands.is_empty());
}

fn remove_json_fields(value: &mut serde_json::Value, fields: &[&str]) {
    let object = value
        .as_object_mut()
        .expect("test payload should be a JSON object");
    for field in fields {
        object.remove(*field);
    }
}

#[test]
fn broadphase_drop_reason_facts_explain_candidate_filtering() {
    let mut same_body_world = World::new(WorldDesc::default());
    let body = same_body_world
        .create_body(BodyDesc {
            body_type: BodyType::Static,
            ..BodyDesc::default()
        })
        .expect("body should be created");
    for x in [0.0, 0.25] {
        same_body_world
            .create_collider(
                body,
                ColliderDesc {
                    shape: SharedShape::rect(1.0, 1.0),
                    local_pose: Pose::from_xy_angle(x, 0.0, 0.0),
                    ..ColliderDesc::default()
                },
            )
            .expect("collider should be created");
    }
    let same_body_report = step_once(&mut same_body_world);
    assert_eq!(same_body_report.stats.broadphase_candidate_count, 1);
    assert_eq!(same_body_report.stats.broadphase_same_body_drop_count, 1);

    let mut filtered_world = World::new(WorldDesc::default());
    let first = filtered_world
        .create_body(BodyDesc {
            body_type: BodyType::Static,
            ..BodyDesc::default()
        })
        .expect("first body should be created");
    let second = filtered_world
        .create_body(BodyDesc {
            body_type: BodyType::Static,
            ..BodyDesc::default()
        })
        .expect("second body should be created");
    let blocked_filter = CollisionFilter {
        memberships: 0b0001,
        collides_with: 0b0010,
    };
    for body in [first, second] {
        filtered_world
            .create_collider(
                body,
                ColliderDesc {
                    shape: SharedShape::rect(1.0, 1.0),
                    filter: blocked_filter,
                    ..ColliderDesc::default()
                },
            )
            .expect("collider should be created");
    }
    let filtered_report = step_once(&mut filtered_world);
    assert_eq!(filtered_report.stats.broadphase_candidate_count, 1);
    assert_eq!(filtered_report.stats.broadphase_filter_drop_count, 1);

    let mut narrowphase_world = World::new(WorldDesc::default());
    for (x, y) in [(0.0, 0.0), (1.5, 1.5)] {
        let body = narrowphase_world
            .create_body(BodyDesc {
                body_type: BodyType::Static,
                pose: Pose::from_xy_angle(x, y, 0.0),
                ..BodyDesc::default()
            })
            .expect("body should be created");
        narrowphase_world
            .create_collider(
                body,
                ColliderDesc {
                    shape: SharedShape::circle(1.0),
                    ..ColliderDesc::default()
                },
            )
            .expect("collider should be created");
    }
    let narrowphase_report = step_once(&mut narrowphase_world);
    assert_eq!(narrowphase_report.stats.broadphase_candidate_count, 1);
    assert_eq!(
        narrowphase_report.stats.broadphase_narrowphase_drop_count,
        1
    );
}

#[test]
fn query_pipeline_can_filter_by_collision_groups_from_debug_facts() {
    let mut world = World::new(WorldDesc::default());
    let body = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(0.0, 0.0, 0.0),
            ..BodyDesc::default()
        })
        .expect("body should be created");

    world
        .create_collider(
            body,
            ColliderDesc {
                shape: SharedShape::circle(1.0),
                filter: CollisionFilter {
                    memberships: 0b0001,
                    collides_with: 0b0010,
                },
                ..ColliderDesc::default()
            },
        )
        .expect("collider should be created");

    let snapshot = world.debug_snapshot(&DebugSnapshotOptions::for_query());
    let mut query = QueryPipeline::new();
    query.sync(&snapshot);

    let allowed = query.intersect_point(
        Point::new(0.0, 0.0),
        QueryFilter::default().colliding_with(CollisionFilter {
            memberships: 0b0010,
            collides_with: 0b0001,
        }),
    );
    let blocked = query.intersect_point(
        Point::new(0.0, 0.0),
        QueryFilter::default().colliding_with(CollisionFilter {
            memberships: 0b0100,
            collides_with: 0b0001,
        }),
    );

    assert_eq!(allowed.len(), 1);
    assert!(blocked.is_empty());
}
