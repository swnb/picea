use picea::prelude::{
    BodyDesc, BodyType, ColliderDesc, CollisionFilter, DebugSnapshotOptions, DistanceJointDesc,
    JointDesc, Material, Pose, QueryFilter, QueryPipeline, SharedShape, SimulationPipeline,
    StepConfig, World, WorldAnchorJointDesc, WorldDesc,
};

#[test]
fn world_api_supports_create_step_query_and_debug_snapshot() {
    let mut world = World::new(WorldDesc::default());

    let ground = world
        .create_body(BodyDesc {
            body_type: BodyType::Static,
            pose: Pose::from_xy_angle(0.0, -10.0, 0.0),
            ..BodyDesc::default()
        })
        .expect("ground body should be created");
    let ball = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            pose: Pose::from_xy_angle(0.0, 0.0, 0.0),
            ..BodyDesc::default()
        })
        .expect("ball body should be created");

    let ground_collider = world
        .create_collider(
            ground,
            ColliderDesc {
                shape: SharedShape::rect(40.0, 2.0),
                material: Material::default(),
                filter: CollisionFilter::default(),
                ..ColliderDesc::default()
            },
        )
        .expect("ground collider should be created");
    let ball_collider = world
        .create_collider(
            ball,
            ColliderDesc {
                shape: SharedShape::circle(1.0),
                material: Material::default(),
                filter: CollisionFilter::default(),
                ..ColliderDesc::default()
            },
        )
        .expect("ball collider should be created");

    world
        .create_joint(JointDesc::Distance(DistanceJointDesc {
            body_a: ground,
            body_b: ball,
            ..DistanceJointDesc::default()
        }))
        .expect("distance joint should be created");
    world
        .create_joint(JointDesc::WorldAnchor(WorldAnchorJointDesc {
            body: ball,
            ..WorldAnchorJointDesc::default()
        }))
        .expect("world-anchor joint should be created");

    let mut pipeline = SimulationPipeline::new(StepConfig {
        dt: 1.0 / 60.0,
        ..StepConfig::default()
    });
    let report = pipeline.step(&mut world);

    assert_eq!(report.dt, 1.0 / 60.0);
    assert!(world.try_body(ground).is_ok());
    assert!(world.try_body(ball).is_ok());
    assert!(world.try_collider(ground_collider).is_ok());
    assert!(world.try_collider(ball_collider).is_ok());

    let mut query = QueryPipeline::new();
    query.sync(&world);
    let query_hits = query.intersect_point(
        picea::math::point::Point::new(0.0, 0.0),
        QueryFilter::default(),
    );
    assert!(!query_hits.is_empty());

    let snapshot = world.debug_snapshot(&DebugSnapshotOptions::default());
    assert_eq!(snapshot.bodies.len(), 2);
    assert_eq!(snapshot.colliders.len(), 2);
    assert_eq!(snapshot.joints.len(), 2);
    assert_eq!(snapshot.stats.step_index, report.step_index);
}
