use picea::math::{point::Point, vector::Vector};
use picea::prelude::{
    BodyDesc, BodyType, ColliderDesc, CollisionFilter, ContactEvent, ContactFeatureId, ContactId,
    ContactReductionReason, DebugSnapshot, DebugSnapshotOptions, ManifoldId, Material, Pose,
    QueryFilter, QueryPipeline, SharedShape, SimulationPipeline, StepConfig, StepReport, StepStats,
    World, WorldDesc, WorldEvent,
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
    assert_eq!(snapshot.contacts.len(), 1);
    assert_eq!(snapshot.manifolds.len(), 1);
    assert!(!snapshot.primitives.is_empty());

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
        snapshot.manifolds[0].contact_ids,
        vec![ContactId::default()]
    );
    assert_eq!(snapshot.manifolds[0].points.len(), 1);
    assert_eq!(snapshot.manifolds[0].normal, Vector::new(-1.0, 0.0));
    assert_eq!(snapshot.manifolds[0].depth, 0.125);
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
