use picea::debug::{DebugShape, DebugStats, DebugTransform};
use picea::math::{point::Point, vector::Vector};
use picea::prelude::{
    BodyDesc, BodyHandle, BodyPatch, BodyType, ColliderDesc, ColliderHandle, ColliderPatch,
    CollisionFilter, ContactEvent, ContactFeatureId, ContactId, ContactReductionReason, DebugBody,
    DebugCollider, DebugContact, DebugIsland, DebugManifold, DebugManifoldPoint, DebugSnapshot,
    DebugSnapshotOptions, EpaTerminationReason, GenericConvexFallbackReason, GenericConvexTrace,
    GjkTerminationReason, ManifoldId, Material, Pose, QueryFilter, QueryPipeline, QueryShape,
    QueryShapeError, QueryStats, SharedShape, SimulationPipeline, SleepEvent,
    SleepTransitionReason, StepConfig, StepReport, StepStats, WarmStartCacheReason, World,
    WorldDesc, WorldEvent,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

fn step_once(world: &mut World) -> StepReport {
    SimulationPipeline::new(StepConfig::default()).step(world)
}

fn assert_serialized_broadphase_tree_contract(
    nodes: &[serde_json::Value],
    root_id: u64,
    expected_leaf_colliders: &[ColliderHandle],
    expected_depth: u64,
) {
    let node_ids = nodes
        .iter()
        .map(|node| {
            node.get("id")
                .and_then(serde_json::Value::as_u64)
                .expect("broadphase tree nodes should carry ids")
        })
        .collect::<Vec<_>>();
    let unique_ids = node_ids.iter().copied().collect::<BTreeSet<_>>();
    let node_by_id = nodes
        .iter()
        .map(|node| {
            (
                node.get("id")
                    .and_then(serde_json::Value::as_u64)
                    .expect("broadphase tree nodes should carry ids"),
                node,
            )
        })
        .collect::<BTreeMap<_, _>>();

    assert_eq!(unique_ids.len(), nodes.len());
    let root = node_by_id
        .get(&root_id)
        .expect("root id should reference a reachable node");
    assert!(root.get("parent").is_none_or(serde_json::Value::is_null));

    let mut leaf_colliders = Vec::new();
    let mut visited = BTreeSet::new();
    let mut stack = vec![root_id];
    while let Some(node_id) = stack.pop() {
        assert!(
            visited.insert(node_id),
            "tree edges should not revisit the same node twice"
        );
        let node = node_by_id[&node_id];
        let parent_id = node.get("parent").and_then(serde_json::Value::as_u64);
        let left_id = node.get("left").and_then(serde_json::Value::as_u64);
        let right_id = node.get("right").and_then(serde_json::Value::as_u64);
        let collider = node.get("collider").filter(|value| !value.is_null());

        if let Some(parent_id) = parent_id {
            let parent = node_by_id
                .get(&parent_id)
                .expect("every parent id should reference another reachable node");
            let parent_left = parent.get("left").and_then(serde_json::Value::as_u64);
            let parent_right = parent.get("right").and_then(serde_json::Value::as_u64);
            assert!(
                parent_left == Some(node_id) || parent_right == Some(node_id),
                "parent should point back to each child"
            );
        }

        match (collider, left_id, right_id) {
            (Some(collider), None, None) => leaf_colliders.push(collider.clone()),
            (None, Some(left_id), Some(right_id)) => {
                assert_ne!(left_id, right_id);
                for child_id in [left_id, right_id] {
                    let child = node_by_id
                        .get(&child_id)
                        .expect("child ids should reference reachable nodes");
                    assert_eq!(
                        child.get("parent").and_then(serde_json::Value::as_u64),
                        Some(node_id),
                        "child should point back to its parent"
                    );
                    stack.push(child_id);
                }
            }
            _ => panic!("debug tree nodes should be leaves or binary internal nodes"),
        }
    }

    let mut expected_colliders = expected_leaf_colliders
        .iter()
        .copied()
        .map(|handle| serde_json::to_value(handle).expect("collider handle should serialize"))
        .collect::<Vec<_>>();
    leaf_colliders.sort_by_key(|value| value.to_string());
    expected_colliders.sort_by_key(|value| value.to_string());
    assert_eq!(leaf_colliders, expected_colliders);
    assert_eq!(visited.len(), nodes.len());

    let max_depth = nodes
        .iter()
        .map(|node| {
            node.get("depth")
                .and_then(serde_json::Value::as_u64)
                .expect("broadphase tree nodes should expose depth")
        })
        .max()
        .expect("reachable tree should contain nodes");
    assert_eq!(max_depth, expected_depth);
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
    assert!(snapshot.broadphase_tree.root.is_none());
    assert!(snapshot.broadphase_tree.nodes.is_empty());
    assert!(snapshot.primitives.is_empty());
}

#[test]
fn debug_snapshot_serializes_broadphase_tree_without_proxy_or_leaf_ids() {
    let mut world = World::new(WorldDesc::default());
    let mut colliders = Vec::new();

    for x in [0.0, 4.0, 8.0] {
        let body = world
            .create_body(BodyDesc {
                body_type: BodyType::Static,
                pose: Pose::from_xy_angle(x, 0.0, 0.0),
                ..BodyDesc::default()
            })
            .expect("body should be created");
        let collider = world
            .create_collider(
                body,
                ColliderDesc {
                    shape: SharedShape::circle(0.5),
                    ..ColliderDesc::default()
                },
            )
            .expect("collider should be created");
        colliders.push(collider);
    }

    let report = step_once(&mut world);
    let snapshot = DebugSnapshot::from_world_with_step_report(
        &world,
        &report,
        &DebugSnapshotOptions::default(),
    );
    let snapshot_value = serde_json::to_value(&snapshot).expect("debug snapshot should serialize");
    let tree = snapshot_value
        .get("broadphase_tree")
        .and_then(serde_json::Value::as_object)
        .expect("snapshot should carry a broadphase tree read model");
    let root = tree
        .get("root")
        .and_then(serde_json::Value::as_u64)
        .expect("reachable tree should expose a transient root node id");
    let depth = tree
        .get("depth")
        .and_then(serde_json::Value::as_u64)
        .expect("reachable tree should expose its depth");
    let nodes = tree
        .get("nodes")
        .and_then(serde_json::Value::as_array)
        .expect("reachable tree should expose nodes for visualization");

    assert_eq!(depth, report.stats.broadphase_tree_depth as u64);
    assert_eq!(nodes.len(), colliders.len() * 2 - 1);
    assert_serialized_broadphase_tree_contract(
        nodes,
        root,
        &colliders,
        report.stats.broadphase_tree_depth as u64,
    );

    for node in nodes.iter() {
        let object = node
            .as_object()
            .expect("broadphase tree nodes should serialize as objects");
        for internal_name in ["proxy_id", "proxy_index", "leaf_id", "leaf_index"] {
            assert!(
                !object.contains_key(internal_name),
                "debug tree must not expose private broadphase {internal_name}"
            );
        }
    }
}

#[test]
fn query_pipeline_returns_no_hits_before_sync() {
    let query = QueryPipeline::new();

    assert!(query
        .intersect_point(Point::new(0.0, 0.0), QueryFilter::default())
        .is_empty());
    assert_eq!(query.last_stats(), QueryStats::default());
}

#[test]
fn query_pipeline_records_candidate_filter_and_hit_counters() {
    let mut world = World::new(WorldDesc::default());
    let visible_body = world
        .create_body(BodyDesc {
            pose: Pose::from_xy_angle(0.0, 0.0, 0.0),
            ..BodyDesc::default()
        })
        .expect("visible body");
    world
        .create_collider(
            visible_body,
            ColliderDesc {
                shape: SharedShape::circle(1.0),
                ..ColliderDesc::default()
            },
        )
        .expect("visible collider");
    let sensor_body = world
        .create_body(BodyDesc {
            pose: Pose::from_xy_angle(0.0, 0.0, 0.0),
            ..BodyDesc::default()
        })
        .expect("sensor body");
    world
        .create_collider(
            sensor_body,
            ColliderDesc {
                shape: SharedShape::circle(1.0),
                is_sensor: true,
                ..ColliderDesc::default()
            },
        )
        .expect("sensor collider");

    let mut query = QueryPipeline::new();
    query.sync(&world);
    let hits = query.intersect_point(Point::new(0.0, 0.0), QueryFilter::default());

    assert_eq!(hits.len(), 1);
    assert_eq!(
        query.last_stats(),
        QueryStats {
            traversal_count: 3,
            candidate_count: 2,
            pruned_count: 0,
            filter_drop_count: 1,
            hit_count: 1,
        }
    );

    let misses = query.intersect_point(Point::new(20.0, 20.0), QueryFilter::default());

    assert!(misses.is_empty());
    assert_eq!(
        query.last_stats(),
        QueryStats {
            traversal_count: 1,
            candidate_count: 0,
            pruned_count: 1,
            filter_drop_count: 0,
            hit_count: 0,
        }
    );
}

#[test]
fn query_pipeline_last_stats_stays_coherent_under_shared_queries() {
    let mut world = World::new(WorldDesc::default());
    let visible_body = world
        .create_body(BodyDesc {
            pose: Pose::from_xy_angle(0.0, 0.0, 0.0),
            ..BodyDesc::default()
        })
        .expect("visible body");
    world
        .create_collider(
            visible_body,
            ColliderDesc {
                shape: SharedShape::circle(1.0),
                ..ColliderDesc::default()
            },
        )
        .expect("visible collider");
    let sensor_body = world
        .create_body(BodyDesc {
            pose: Pose::from_xy_angle(0.0, 0.0, 0.0),
            ..BodyDesc::default()
        })
        .expect("sensor body");
    world
        .create_collider(
            sensor_body,
            ColliderDesc {
                shape: SharedShape::circle(1.0),
                is_sensor: true,
                ..ColliderDesc::default()
            },
        )
        .expect("sensor collider");

    let mut query = QueryPipeline::new();
    query.sync(&world);
    let query = Arc::new(query);
    let stop = Arc::new(AtomicBool::new(false));
    let hit_stats = QueryStats {
        traversal_count: 3,
        candidate_count: 2,
        pruned_count: 0,
        filter_drop_count: 1,
        hit_count: 1,
    };
    let miss_stats = QueryStats {
        traversal_count: 1,
        candidate_count: 0,
        pruned_count: 1,
        filter_drop_count: 0,
        hit_count: 0,
    };

    std::thread::scope(|scope| {
        for point in [Point::new(0.0, 0.0), Point::new(20.0, 20.0)] {
            let query = Arc::clone(&query);
            let stop = Arc::clone(&stop);
            scope.spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    let _ = query.intersect_point(point, QueryFilter::default());
                    std::thread::yield_now();
                }
            });
        }

        let query = Arc::clone(&query);
        let stop = Arc::clone(&stop);
        scope.spawn(move || {
            for _ in 0..200_000 {
                let stats = query.last_stats();
                if stats == QueryStats::default() || stats == hit_stats || stats == miss_stats {
                    continue;
                }
                stop.store(true, Ordering::Relaxed);
                panic!("last_stats observed a torn snapshot: {stats:?}");
            }
            stop.store(true, Ordering::Relaxed);
        });
    });
}

#[test]
fn world_remains_send_sync_with_internal_query_and_geometry_caches() {
    fn assert_send_sync<T: Send + Sync>() {}

    assert_send_sync::<World>();
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
            broadphase_traversal_count: 8,
            broadphase_pruned_count: 5,
            broadphase_rebuild_count: 1,
            broadphase_tree_depth: 3,
            contact_count: 1,
            manifold_count: 1,
            island_count: 2,
            active_island_count: 1,
            sleeping_island_skip_count: 1,
            solver_body_slot_count: 3,
            contact_row_count: 2,
            joint_row_count: 1,
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
    assert_eq!(snapshot.stats.broadphase_traversal_count, 8);
    assert_eq!(snapshot.stats.broadphase_pruned_count, 5);
    assert_eq!(snapshot.stats.broadphase_rebuild_count, 1);
    assert_eq!(snapshot.stats.broadphase_tree_depth, 3);
    assert_eq!(snapshot.stats.island_count, 2);
    assert_eq!(snapshot.stats.active_island_count, 1);
    assert_eq!(snapshot.stats.sleeping_island_skip_count, 1);
    assert_eq!(snapshot.stats.solver_body_slot_count, 3);
    assert_eq!(snapshot.stats.contact_row_count, 2);
    assert_eq!(snapshot.stats.joint_row_count, 1);
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
            "broadphase_traversal_count",
            "broadphase_pruned_count",
            "island_count",
            "active_island_count",
            "sleeping_island_skip_count",
            "solver_body_slot_count",
            "contact_row_count",
            "joint_row_count",
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
    assert_eq!(stats.broadphase_traversal_count, 0);
    assert_eq!(stats.broadphase_pruned_count, 0);
    assert_eq!(stats.island_count, 0);
    assert_eq!(stats.active_island_count, 0);
    assert_eq!(stats.sleeping_island_skip_count, 0);
    assert_eq!(stats.solver_body_slot_count, 0);
    assert_eq!(stats.contact_row_count, 0);
    assert_eq!(stats.joint_row_count, 0);
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
            "broadphase_traversal_count",
            "broadphase_pruned_count",
            "island_count",
            "active_island_count",
            "sleeping_island_skip_count",
            "solver_body_slot_count",
            "contact_row_count",
            "joint_row_count",
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
    assert_eq!(debug_stats.broadphase_traversal_count, 0);
    assert_eq!(debug_stats.broadphase_pruned_count, 0);
    assert_eq!(debug_stats.island_count, 0);
    assert_eq!(debug_stats.active_island_count, 0);
    assert_eq!(debug_stats.sleeping_island_skip_count, 0);
    assert_eq!(debug_stats.solver_body_slot_count, 0);
    assert_eq!(debug_stats.contact_row_count, 0);
    assert_eq!(debug_stats.joint_row_count, 0);
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
    remove_json_fields(&mut snapshot_value, &["islands", "broadphase_tree"]);
    let decoded_snapshot: DebugSnapshot =
        serde_json::from_value(snapshot_value).expect("older debug snapshot should deserialize");
    assert!(decoded_snapshot.islands.is_empty());
    assert!(decoded_snapshot.broadphase_tree.nodes.is_empty());
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

    assert!(snapshot.broadphase_tree.nodes.is_empty());
    assert_eq!(allowed.len(), 1);
    assert!(blocked.is_empty());
}

#[test]
fn query_pipeline_preserves_ordering_and_filters_when_candidates_are_pruned() {
    let mut world = World::new(WorldDesc::default());
    let body_a = world
        .create_body(BodyDesc {
            body_type: BodyType::Static,
            pose: Pose::from_xy_angle(0.0, 0.0, 0.0),
            ..BodyDesc::default()
        })
        .expect("body should be created");
    let body_b = world
        .create_body(BodyDesc {
            body_type: BodyType::Static,
            pose: Pose::from_xy_angle(2.0, 0.0, 0.0),
            ..BodyDesc::default()
        })
        .expect("body should be created");
    let far_body = world
        .create_body(BodyDesc {
            body_type: BodyType::Static,
            pose: Pose::from_xy_angle(50.0, 0.0, 0.0),
            ..BodyDesc::default()
        })
        .expect("body should be created");
    let sensor_body = world
        .create_body(BodyDesc {
            body_type: BodyType::Static,
            pose: Pose::from_xy_angle(1.0, 0.0, 0.0),
            ..BodyDesc::default()
        })
        .expect("body should be created");

    let collider_a = world
        .create_collider(
            body_a,
            ColliderDesc {
                shape: SharedShape::circle(1.0),
                ..ColliderDesc::default()
            },
        )
        .expect("collider should be created");
    let collider_b = world
        .create_collider(
            body_b,
            ColliderDesc {
                shape: SharedShape::circle(1.0),
                ..ColliderDesc::default()
            },
        )
        .expect("collider should be created");
    world
        .create_collider(
            far_body,
            ColliderDesc {
                shape: SharedShape::circle(1.0),
                ..ColliderDesc::default()
            },
        )
        .expect("far collider should be created");
    let sensor_collider = world
        .create_collider(
            sensor_body,
            ColliderDesc {
                shape: SharedShape::circle(0.25),
                is_sensor: true,
                ..ColliderDesc::default()
            },
        )
        .expect("sensor collider should be created");

    let mut query = QueryPipeline::new();
    query.sync(&world);

    let aabb_hits = query.intersect_aabb(
        picea::debug::DebugAabb::new(Point::new(-1.5, -1.5), Point::new(3.5, 1.5)),
        QueryFilter::default(),
    );
    assert_eq!(
        aabb_hits.iter().map(|hit| hit.collider).collect::<Vec<_>>(),
        vec![collider_a, collider_b]
    );

    let default_point_hits = query.intersect_point(Point::new(1.0, 0.0), QueryFilter::default());
    assert_eq!(
        default_point_hits
            .iter()
            .map(|hit| hit.collider)
            .collect::<Vec<_>>(),
        vec![collider_a, collider_b]
    );

    let sensor_point_hits = query.intersect_point(
        Point::new(1.0, 0.0),
        QueryFilter::default().including_sensors(),
    );
    assert_eq!(
        sensor_point_hits
            .iter()
            .map(|hit| hit.collider)
            .collect::<Vec<_>>(),
        vec![collider_a, collider_b, sensor_collider]
    );

    let ray_hit = query
        .cast_ray(
            Point::new(-3.0, 0.0),
            Vector::new(1.0, 0.0),
            10.0,
            QueryFilter::default(),
        )
        .expect("ray should hit the first collider in snapshot order");
    assert_eq!(ray_hit.collider, collider_a);
}

#[test]
fn query_pipeline_sync_resets_last_stats_after_ray_and_aabb_queries() {
    let mut world = World::new(WorldDesc::default());
    let body = world
        .create_body(BodyDesc {
            body_type: BodyType::Static,
            ..BodyDesc::default()
        })
        .expect("body should be created");
    world
        .create_collider(
            body,
            ColliderDesc {
                shape: SharedShape::circle(1.0),
                ..ColliderDesc::default()
            },
        )
        .expect("collider should be created");

    let mut query = QueryPipeline::new();
    query.sync(&world);

    assert!(query
        .cast_ray(
            Point::new(-3.0, 0.0),
            Vector::new(1.0, 0.0),
            10.0,
            QueryFilter::default(),
        )
        .is_some());
    assert_ne!(query.last_stats(), QueryStats::default());

    query.sync(&world);
    assert_eq!(query.last_stats(), QueryStats::default());

    assert_eq!(
        query
            .intersect_aabb(
                picea::debug::DebugAabb::new(Point::new(-1.0, -1.0), Point::new(1.0, 1.0)),
                QueryFilter::default(),
            )
            .len(),
        1
    );
    assert_ne!(query.last_stats(), QueryStats::default());

    query.sync(&world);
    assert_eq!(query.last_stats(), QueryStats::default());
}

#[test]
fn query_pipeline_ray_and_aabb_queries_respect_sensor_and_collision_filters() {
    let mut world = World::new(WorldDesc::default());
    let visible_body = world
        .create_body(BodyDesc {
            body_type: BodyType::Static,
            pose: Pose::from_xy_angle(0.0, 0.0, 0.0),
            ..BodyDesc::default()
        })
        .expect("visible body should be created");
    let visible = world
        .create_collider(
            visible_body,
            ColliderDesc {
                shape: SharedShape::circle(0.5),
                filter: CollisionFilter {
                    memberships: 0b0001,
                    collides_with: 0b0001,
                },
                ..ColliderDesc::default()
            },
        )
        .expect("visible collider should be created");
    let sensor_body = world
        .create_body(BodyDesc {
            body_type: BodyType::Static,
            pose: Pose::from_xy_angle(-2.0, 0.0, 0.0),
            ..BodyDesc::default()
        })
        .expect("sensor body should be created");
    let sensor = world
        .create_collider(
            sensor_body,
            ColliderDesc {
                shape: SharedShape::circle(0.5),
                is_sensor: true,
                filter: CollisionFilter {
                    memberships: 0b0001,
                    collides_with: 0b0001,
                },
                ..ColliderDesc::default()
            },
        )
        .expect("sensor collider should be created");
    let blocked_body = world
        .create_body(BodyDesc {
            body_type: BodyType::Static,
            pose: Pose::from_xy_angle(2.0, 0.0, 0.0),
            ..BodyDesc::default()
        })
        .expect("blocked body should be created");
    let blocked = world
        .create_collider(
            blocked_body,
            ColliderDesc {
                shape: SharedShape::circle(0.5),
                filter: CollisionFilter {
                    memberships: 0b0010,
                    collides_with: 0b0010,
                },
                ..ColliderDesc::default()
            },
        )
        .expect("blocked collider should be created");

    let mut query = QueryPipeline::new();
    query.sync(&world);

    let all_bounds = picea::debug::DebugAabb::new(Point::new(-3.0, -1.0), Point::new(3.0, 1.0));
    assert_eq!(
        query
            .intersect_aabb(all_bounds, QueryFilter::default())
            .iter()
            .map(|hit| hit.collider)
            .collect::<Vec<_>>(),
        vec![visible, blocked]
    );
    assert_eq!(
        query
            .intersect_aabb(all_bounds, QueryFilter::default().including_sensors())
            .iter()
            .map(|hit| hit.collider)
            .collect::<Vec<_>>(),
        vec![visible, sensor, blocked]
    );
    assert_eq!(
        query
            .intersect_aabb(
                all_bounds,
                QueryFilter::default().colliding_with(CollisionFilter {
                    memberships: 0b0010,
                    collides_with: 0b0010,
                }),
            )
            .iter()
            .map(|hit| hit.collider)
            .collect::<Vec<_>>(),
        vec![blocked]
    );

    assert_eq!(
        query
            .cast_ray(
                Point::new(-4.0, 0.0),
                Vector::new(1.0, 0.0),
                10.0,
                QueryFilter::default(),
            )
            .expect("default ray should skip sensors and hit the visible collider")
            .collider,
        visible
    );
    assert_eq!(
        query
            .cast_ray(
                Point::new(-4.0, 0.0),
                Vector::new(1.0, 0.0),
                10.0,
                QueryFilter::default().including_sensors(),
            )
            .expect("sensor-inclusive ray should hit the nearest sensor")
            .collider,
        sensor
    );
    assert_eq!(
        query
            .cast_ray(
                Point::new(-4.0, 0.0),
                Vector::new(1.0, 0.0),
                10.0,
                QueryFilter::default().colliding_with(CollisionFilter {
                    memberships: 0b0010,
                    collides_with: 0b0010,
                }),
            )
            .expect("collision-filtered ray should hit only the matching collider")
            .collider,
        blocked
    );
}

#[test]
fn query_pipeline_ray_and_aabb_queries_require_sync_to_drop_stale_and_recycled_handles() {
    let mut world = World::new(WorldDesc::default());
    let body = world
        .create_body(BodyDesc {
            body_type: BodyType::Static,
            ..BodyDesc::default()
        })
        .expect("body should be created");
    let original = world
        .create_collider(
            body,
            ColliderDesc {
                shape: SharedShape::circle(0.5),
                ..ColliderDesc::default()
            },
        )
        .expect("original collider should be created");

    let mut query = QueryPipeline::new();
    query.sync(&world);

    let origin_bounds = picea::debug::DebugAabb::new(Point::new(-1.0, -1.0), Point::new(1.0, 1.0));
    assert_eq!(
        query
            .cast_ray(
                Point::new(-3.0, 0.0),
                Vector::new(1.0, 0.0),
                10.0,
                QueryFilter::default(),
            )
            .expect("original collider should be on the initial ray path")
            .collider,
        original
    );
    assert_eq!(
        query
            .intersect_aabb(origin_bounds, QueryFilter::default())
            .iter()
            .map(|hit| hit.collider)
            .collect::<Vec<_>>(),
        vec![original]
    );

    world
        .apply_body_patch(
            body,
            BodyPatch {
                pose: Some(Pose::from_xy_angle(10.0, 0.0, 0.0)),
                ..BodyPatch::default()
            },
        )
        .expect("body patch should succeed");

    assert_eq!(
        query
            .cast_ray(
                Point::new(-3.0, 0.0),
                Vector::new(1.0, 0.0),
                10.0,
                QueryFilter::default(),
            )
            .expect("stale cache should still reflect the old ray path before sync")
            .collider,
        original
    );
    assert_eq!(
        query
            .intersect_aabb(origin_bounds, QueryFilter::default())
            .iter()
            .map(|hit| hit.collider)
            .collect::<Vec<_>>(),
        vec![original]
    );

    query.sync(&world);
    assert!(
        query
            .cast_ray(
                Point::new(-3.0, 0.0),
                Vector::new(1.0, 0.0),
                10.0,
                QueryFilter::default(),
            )
            .is_none(),
        "resync should drop the old ray hit once the collider moved away"
    );
    assert!(query
        .intersect_aabb(origin_bounds, QueryFilter::default())
        .is_empty());

    world
        .destroy_collider(original)
        .expect("collider should be destroyed");
    let recycled = world
        .create_collider(
            body,
            ColliderDesc {
                shape: SharedShape::circle(0.5),
                local_pose: Pose::from_xy_angle(10.0, 0.0, 0.0),
                ..ColliderDesc::default()
            },
        )
        .expect("collider should reuse the old slot with a new generation");
    let recycled_bounds =
        picea::debug::DebugAabb::new(Point::new(19.0, -1.0), Point::new(21.0, 1.0));

    assert_eq!(
        query
            .cast_ray(
                Point::new(7.0, 0.0),
                Vector::new(1.0, 0.0),
                10.0,
                QueryFilter::default(),
            )
            .expect("stale cache should still point at the old generation before sync")
            .collider,
        original
    );
    assert_eq!(
        query
            .intersect_aabb(
                picea::debug::DebugAabb::new(Point::new(9.0, -1.0), Point::new(11.0, 1.0)),
                QueryFilter::default(),
            )
            .iter()
            .map(|hit| hit.collider)
            .collect::<Vec<_>>(),
        vec![original]
    );

    query.sync(&world);
    assert_eq!(
        query
            .cast_ray(
                Point::new(17.0, 0.0),
                Vector::new(1.0, 0.0),
                10.0,
                QueryFilter::default(),
            )
            .expect("resync should expose the recycled collider on its new ray path")
            .collider,
        recycled
    );
    assert_eq!(
        query
            .intersect_aabb(recycled_bounds, QueryFilter::default())
            .iter()
            .map(|hit| hit.collider)
            .collect::<Vec<_>>(),
        vec![recycled]
    );
    assert_ne!(original, recycled);
}

#[test]
fn query_pipeline_aabb_queries_include_touching_bounds_after_index_pruning() {
    let mut world = World::new(WorldDesc::default());
    let body = world
        .create_body(BodyDesc {
            body_type: BodyType::Static,
            pose: Pose::from_xy_angle(0.0, 0.0, 0.0),
            ..BodyDesc::default()
        })
        .expect("body should be created");
    let collider = world
        .create_collider(
            body,
            ColliderDesc {
                shape: SharedShape::rect(2.0, 2.0),
                ..ColliderDesc::default()
            },
        )
        .expect("collider should be created");

    let mut query = QueryPipeline::new();
    query.sync(&world);

    let touching_hits = query.intersect_aabb(
        picea::debug::DebugAabb::new(Point::new(1.0, -0.5), Point::new(2.0, 0.5)),
        QueryFilter::default(),
    );

    assert_eq!(
        touching_hits
            .iter()
            .map(|hit| hit.collider)
            .collect::<Vec<_>>(),
        vec![collider],
        "AABB query semantics are inclusive at the boundary; broadphase pruning must not drop touching bounds"
    );
}

#[test]
fn query_pipeline_sync_invalidates_cached_geometry_after_pose_and_shape_changes() {
    let mut world = World::new(WorldDesc::default());
    let body = world
        .create_body(BodyDesc {
            body_type: BodyType::Static,
            pose: Pose::from_xy_angle(0.0, 0.0, 0.0),
            ..BodyDesc::default()
        })
        .expect("body should be created");
    let collider = world
        .create_collider(
            body,
            ColliderDesc {
                shape: SharedShape::circle(1.0),
                ..ColliderDesc::default()
            },
        )
        .expect("collider should be created");
    let mut query = QueryPipeline::new();

    query.sync(&world);
    assert_eq!(
        query.intersect_point(Point::new(0.5, 0.0), QueryFilter::default())[0].collider,
        collider
    );

    world
        .apply_body_patch(
            body,
            BodyPatch {
                pose: Some(Pose::from_xy_angle(10.0, 0.0, 0.0)),
                ..BodyPatch::default()
            },
        )
        .expect("body pose patch should succeed");
    query.sync(&world);
    assert!(query
        .intersect_point(Point::new(0.5, 0.0), QueryFilter::default())
        .is_empty());
    assert_eq!(
        query.intersect_point(Point::new(10.5, 0.0), QueryFilter::default())[0].collider,
        collider
    );

    world
        .apply_collider_patch(
            collider,
            ColliderPatch {
                local_pose: Some(Pose::from_xy_angle(2.0, 0.0, 0.0)),
                shape: Some(SharedShape::rect(2.0, 2.0)),
                ..ColliderPatch::default()
            },
        )
        .expect("collider patch should succeed");
    query.sync(&world);
    assert!(query
        .intersect_point(Point::new(10.5, 0.0), QueryFilter::default())
        .is_empty());
    assert_eq!(
        query.intersect_point(Point::new(12.75, 0.0), QueryFilter::default())[0].collider,
        collider
    );
}

#[test]
fn query_pipeline_sync_does_not_leak_recycled_collider_candidates() {
    let mut world = World::new(WorldDesc::default());
    let body = world
        .create_body(BodyDesc {
            body_type: BodyType::Static,
            ..BodyDesc::default()
        })
        .expect("body should be created");
    let original = world
        .create_collider(
            body,
            ColliderDesc {
                shape: SharedShape::circle(1.0),
                ..ColliderDesc::default()
            },
        )
        .expect("original collider should be created");
    let mut query = QueryPipeline::new();
    query.sync(&world);
    assert_eq!(
        query.intersect_point(Point::new(0.0, 0.0), QueryFilter::default())[0].collider,
        original
    );

    world
        .destroy_collider(original)
        .expect("collider should be destroyed");
    let recycled = world
        .create_collider(
            body,
            ColliderDesc {
                shape: SharedShape::circle(1.0),
                local_pose: Pose::from_xy_angle(20.0, 0.0, 0.0),
                ..ColliderDesc::default()
            },
        )
        .expect("collider should reuse the old slot with a new generation");

    query.sync(&world);
    assert!(query
        .intersect_point(Point::new(0.0, 0.0), QueryFilter::default())
        .is_empty());
    assert_eq!(
        query.intersect_point(Point::new(20.0, 0.0), QueryFilter::default())[0].collider,
        recycled
    );
    assert_ne!(original, recycled);
}

#[test]
fn query_pipeline_shape_query_returns_deterministic_closest_hits_and_stats() {
    let mut world = World::new(WorldDesc::default());
    let temp_body = world
        .create_body(BodyDesc {
            body_type: BodyType::Static,
            ..BodyDesc::default()
        })
        .expect("temporary body should be created");
    let recycled_slot = world
        .create_collider(
            temp_body,
            ColliderDesc {
                shape: SharedShape::circle(0.5),
                ..ColliderDesc::default()
            },
        )
        .expect("temporary collider should be created");
    world
        .destroy_collider(recycled_slot)
        .expect("collider slot should be reusable");

    let left_body = world
        .create_body(BodyDesc {
            body_type: BodyType::Static,
            pose: Pose::from_xy_angle(-2.0, 0.0, 0.0),
            ..BodyDesc::default()
        })
        .expect("left body should be created");
    let left = world
        .create_collider(
            left_body,
            ColliderDesc {
                shape: SharedShape::circle(0.5),
                ..ColliderDesc::default()
            },
        )
        .expect("left collider should reuse the old slot generation");
    let right_body = world
        .create_body(BodyDesc {
            body_type: BodyType::Static,
            pose: Pose::from_xy_angle(2.0, 0.0, 0.0),
            ..BodyDesc::default()
        })
        .expect("right body should be created");
    let right = world
        .create_collider(
            right_body,
            ColliderDesc {
                shape: SharedShape::circle(0.5),
                ..ColliderDesc::default()
            },
        )
        .expect("right collider should be created after the recycled slot");
    let sensor_body = world
        .create_body(BodyDesc {
            body_type: BodyType::Static,
            pose: Pose::from_xy_angle(0.0, 2.0, 0.0),
            ..BodyDesc::default()
        })
        .expect("sensor body should be created");
    world
        .create_collider(
            sensor_body,
            ColliderDesc {
                shape: SharedShape::circle(0.5),
                is_sensor: true,
                ..ColliderDesc::default()
            },
        )
        .expect("sensor collider should be created");
    let filtered_body = world
        .create_body(BodyDesc {
            body_type: BodyType::Static,
            pose: Pose::from_xy_angle(0.0, -2.0, 0.0),
            ..BodyDesc::default()
        })
        .expect("filtered body should be created");
    world
        .create_collider(
            filtered_body,
            ColliderDesc {
                shape: SharedShape::circle(0.5),
                filter: CollisionFilter {
                    memberships: 0b0010,
                    collides_with: 0b0010,
                },
                ..ColliderDesc::default()
            },
        )
        .expect("filtered collider should be created");
    let far_body = world
        .create_body(BodyDesc {
            body_type: BodyType::Static,
            pose: Pose::from_xy_angle(10.0, 0.0, 0.0),
            ..BodyDesc::default()
        })
        .expect("far body should be created");
    world
        .create_collider(
            far_body,
            ColliderDesc {
                shape: SharedShape::circle(0.5),
                ..ColliderDesc::default()
            },
        )
        .expect("far collider should be created");

    assert!(
        right < left,
        "generation bits should make handle ordering diverge from snapshot order"
    );

    let mut query = QueryPipeline::new();
    query.sync(&world);

    let shape = QueryShape::circle(Point::new(0.0, 0.0), 0.5).expect("query shape should build");
    let hits = query.intersect_shape(
        &shape,
        1.0,
        QueryFilter::default().colliding_with(CollisionFilter {
            memberships: 0b0001,
            collides_with: 0b0001,
        }),
    );
    let closest = query.closest_shape(
        &shape,
        1.0,
        QueryFilter::default().colliding_with(CollisionFilter {
            memberships: 0b0001,
            collides_with: 0b0001,
        }),
    );
    let hits = hits.expect("shape query should succeed");
    let closest = closest
        .expect("closest shape query should succeed")
        .expect("closest hit should exist");

    assert_eq!(
        hits.iter().map(|hit| hit.collider).collect::<Vec<_>>(),
        vec![left, right]
    );
    assert!(hits.iter().all(|hit| (hit.distance - 1.0).abs() <= 1.0e-5));
    assert_eq!(closest.collider, left);
    assert_eq!(closest.body, left_body);
    assert_eq!(closest.query_point, Point::new(-0.5, 0.0));
    assert_eq!(closest.collider_point, Point::new(-1.5, 0.0));
    assert_eq!(closest.normal, Some((1.0, 0.0).into()));
    assert_eq!(query.last_stats().candidate_count, 4);
    assert_eq!(query.last_stats().filter_drop_count, 2);
    assert_eq!(query.last_stats().hit_count, 1);
    assert!(query.last_stats().traversal_count >= query.last_stats().candidate_count);
}

#[test]
fn query_pipeline_shape_query_respects_filters_sync_and_recycled_handles() {
    let mut world = World::new(WorldDesc::default());
    let body = world
        .create_body(BodyDesc {
            body_type: BodyType::Static,
            ..BodyDesc::default()
        })
        .expect("body should be created");
    let collider = world
        .create_collider(
            body,
            ColliderDesc {
                shape: SharedShape::circle(0.5),
                ..ColliderDesc::default()
            },
        )
        .expect("collider should be created");
    let sensor = world
        .create_collider(
            body,
            ColliderDesc {
                shape: SharedShape::circle(0.5),
                is_sensor: true,
                local_pose: Pose::from_xy_angle(0.0, 2.0, 0.0),
                ..ColliderDesc::default()
            },
        )
        .expect("sensor collider should be created");
    let mut query = QueryPipeline::new();
    query.sync(&world);

    let origin_shape =
        QueryShape::circle(Point::new(2.0, 0.0), 0.5).expect("query shape should build");
    assert_eq!(
        query
            .closest_shape(&origin_shape, 1.0, QueryFilter::default())
            .expect("shape query should succeed")
            .expect("query should hit")
            .collider,
        collider
    );
    assert_eq!(
        query
            .closest_shape(
                &QueryShape::circle(Point::new(0.0, 2.0), 0.5).expect("query shape should build"),
                0.0,
                QueryFilter::default().including_sensors(),
            )
            .expect("sensor query should succeed")
            .expect("sensor hit should exist")
            .collider,
        sensor
    );
    assert!(query
        .closest_shape(
            &QueryShape::circle(Point::new(0.0, 2.0), 0.5).expect("query shape should build"),
            0.0,
            QueryFilter::default(),
        )
        .expect("sensor-excluding query should succeed")
        .is_none());

    world
        .apply_body_patch(
            body,
            BodyPatch {
                pose: Some(Pose::from_xy_angle(10.0, 0.0, 0.0)),
                ..BodyPatch::default()
            },
        )
        .expect("body patch should succeed");
    query.sync(&world);
    assert!(query
        .closest_shape(&origin_shape, 1.0, QueryFilter::default())
        .expect("shape query should succeed")
        .is_none());

    let moved_body_shape =
        QueryShape::circle(Point::new(12.0, 0.0), 0.5).expect("query shape should build");
    assert_eq!(
        query
            .closest_shape(&moved_body_shape, 1.0, QueryFilter::default())
            .expect("shape query should succeed")
            .expect("query should hit after body patch")
            .collider,
        collider
    );

    world
        .apply_collider_patch(
            collider,
            ColliderPatch {
                local_pose: Some(Pose::from_xy_angle(5.0, 0.0, 0.0)),
                ..ColliderPatch::default()
            },
        )
        .expect("collider patch should succeed");
    query.sync(&world);
    assert!(query
        .closest_shape(&moved_body_shape, 1.0, QueryFilter::default())
        .expect("shape query should succeed")
        .is_none());

    let moved_collider_shape =
        QueryShape::circle(Point::new(17.0, 0.0), 0.5).expect("query shape should build");
    assert_eq!(
        query
            .closest_shape(&moved_collider_shape, 1.0, QueryFilter::default())
            .expect("shape query should succeed")
            .expect("query should hit after collider patch")
            .collider,
        collider
    );
    world
        .destroy_collider(collider)
        .expect("collider should be destroyed");
    let recycled = world
        .create_collider(
            body,
            ColliderDesc {
                shape: SharedShape::circle(0.5),
                local_pose: Pose::from_xy_angle(14.0, 0.0, 0.0),
                ..ColliderDesc::default()
            },
        )
        .expect("collider should reuse the old slot with a new generation");
    query.sync(&world);

    assert!(query
        .closest_shape(&moved_collider_shape, 1.0, QueryFilter::default())
        .expect("shape query should succeed after recycle")
        .is_none());
    assert_eq!(
        query
            .closest_shape(
                &QueryShape::circle(Point::new(24.0, 0.0), 0.5).expect("query shape should build"),
                0.0,
                QueryFilter::default(),
            )
            .expect("recycled collider query should succeed")
            .expect("recycled collider should be hit")
            .collider,
        recycled
    );
    assert_ne!(collider, recycled);
}

#[test]
fn query_shape_rejects_degenerate_and_unsupported_inputs() {
    assert!(matches!(
        QueryShape::from_shared_shape(
            &SharedShape::concave_polygon(vec![
                Point::new(-1.0, -1.0),
                Point::new(1.0, -1.0),
                Point::new(0.0, 0.0),
                Point::new(1.0, 1.0),
                Point::new(-1.0, 1.0),
            ]),
            Pose::default(),
        ),
        Err(QueryShapeError::UnsupportedShape { .. })
    ));
    assert!(matches!(
        QueryShape::circle(Point::new(0.0, 0.0), 0.0),
        Err(QueryShapeError::InvalidShape { .. })
    ));
    assert!(matches!(
        QueryShape::segment(Point::new(1.0, 1.0), Point::new(1.0, 1.0)),
        Err(QueryShapeError::InvalidShape { .. })
    ));
    assert!(matches!(
        QueryShape::polygon(vec![
            Point::new(f32::NAN, 0.0),
            Point::new(1.0, 0.0),
            Point::new(0.0, 1.0),
        ]),
        Err(QueryShapeError::InvalidShape { .. })
    ));
}

#[test]
fn query_shape_rejects_direct_concave_polygon_input() {
    assert_eq!(
        QueryShape::polygon(vec![
            Point::new(-2.0, -2.0),
            Point::new(2.0, -2.0),
            Point::new(0.0, 0.0),
            Point::new(2.0, 2.0),
            Point::new(-2.0, 2.0),
        ]),
        Err(QueryShapeError::UnsupportedShape {
            kind: "concave_polygon",
        })
    );
}

#[test]
fn query_shape_public_polygon_and_segment_constructors_build_supported_queries() {
    let mut world = World::new(WorldDesc::default());
    let body = world
        .create_body(BodyDesc {
            body_type: BodyType::Static,
            ..BodyDesc::default()
        })
        .expect("body should be created");
    let collider = world
        .create_collider(
            body,
            ColliderDesc {
                shape: SharedShape::circle(0.5),
                ..ColliderDesc::default()
            },
        )
        .expect("collider should be created");
    let mut query = QueryPipeline::new();
    query.sync(&world);

    let polygon = QueryShape::polygon(vec![
        Point::new(1.0, -0.5),
        Point::new(2.0, -0.5),
        Point::new(2.0, 0.5),
        Point::new(1.0, 0.5),
    ])
    .expect("convex polygon query should build");
    let segment = QueryShape::segment(Point::new(1.0, 0.0), Point::new(2.0, 0.0))
        .expect("segment query should build");

    assert_eq!(
        query
            .closest_shape(&polygon, 1.0, QueryFilter::default())
            .expect("polygon query should succeed")
            .expect("polygon query should hit")
            .collider,
        collider
    );
    assert_eq!(
        query
            .closest_shape(&segment, 1.0, QueryFilter::default())
            .expect("segment query should succeed")
            .expect("segment query should hit")
            .collider,
        collider
    );
}

#[test]
fn query_pipeline_shape_query_rejects_negative_max_distance() {
    let query = QueryPipeline::new();
    let shape = QueryShape::circle(Point::new(0.0, 0.0), 0.5).expect("query shape should build");

    assert_eq!(
        query.closest_shape(&shape, -0.1, QueryFilter::default()),
        Err(QueryShapeError::InvalidShape {
            kind: "max_distance",
            reason: "negative_or_non_finite",
        })
    );
}

#[test]
fn query_pipeline_shape_query_keeps_capsule_snapshot_distance_and_witness_points_consistent() {
    let mut snapshot = DebugSnapshot::default();
    snapshot.colliders.push(DebugCollider {
        handle: ColliderHandle::default(),
        body: BodyHandle::default(),
        local_transform: DebugTransform::default(),
        world_transform: DebugTransform::default(),
        aabb: None,
        shape: DebugShape::Segment {
            start: Point::new(0.0, -1.0),
            end: Point::new(0.0, 1.0),
            radius: 0.25,
        },
        density: 1.0,
        material: Material::default(),
        filter: CollisionFilter::default(),
        is_sensor: false,
        user_data: 0,
    });

    let mut query = QueryPipeline::new();
    query.sync(&snapshot);

    let polygon = QueryShape::polygon(vec![
        Point::new(1.0, -0.5),
        Point::new(2.0, -0.5),
        Point::new(2.0, 0.5),
        Point::new(1.0, 0.5),
    ])
    .expect("convex polygon query should build");
    let hit = query
        .closest_shape(&polygon, 1.0, QueryFilter::default())
        .expect("shape query should succeed")
        .expect("capsule snapshot should be hit");

    assert!((hit.distance - 0.75).abs() <= 1.0e-5);
    assert_eq!(hit.query_point.x(), 1.0);
    assert_eq!(hit.collider_point.x(), 0.25);
    assert!((hit.query_point.y() - hit.collider_point.y()).abs() <= 1.0e-5);
    assert!((hit.query_point.y().abs() - 0.5).abs() <= 1.0e-5);
    assert_eq!(hit.normal, Some((1.0, 0.0).into()));
}

#[test]
fn query_pipeline_shape_query_treats_capsule_radius_overlap_as_zero_distance() {
    let mut snapshot = DebugSnapshot::default();
    snapshot.colliders.push(DebugCollider {
        handle: ColliderHandle::default(),
        body: BodyHandle::default(),
        local_transform: DebugTransform::default(),
        world_transform: DebugTransform::default(),
        aabb: None,
        shape: DebugShape::Segment {
            start: Point::new(0.0, -1.0),
            end: Point::new(0.0, 1.0),
            radius: 0.25,
        },
        density: 1.0,
        material: Material::default(),
        filter: CollisionFilter::default(),
        is_sensor: false,
        user_data: 0,
    });

    let mut query = QueryPipeline::new();
    query.sync(&snapshot);

    let polygon = QueryShape::polygon(vec![
        Point::new(0.2, -0.5),
        Point::new(1.2, -0.5),
        Point::new(1.2, 0.5),
        Point::new(0.2, 0.5),
    ])
    .expect("convex polygon query should build");
    let hit = query
        .closest_shape(&polygon, 0.0, QueryFilter::default())
        .expect("shape query should succeed")
        .expect("capsule snapshot should overlap polygon");

    assert_eq!(hit.distance, 0.0);
    assert_eq!(hit.query_point, Point::new(0.2, -0.5));
    assert_eq!(hit.collider_point, Point::new(0.2, -0.5));
    assert_eq!(hit.normal, None);
}
