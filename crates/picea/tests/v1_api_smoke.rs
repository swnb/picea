use picea::prelude::{
    BodyBundle, BodyDesc, BodyType, ColliderBundle, ColliderDesc, CollisionFilter,
    CollisionLayerPreset, DebugSnapshotOptions, DistanceJointDesc, JointBundle, JointDesc,
    Material, MaterialPreset, Pose, QueryFilter, QueryPipeline, SharedShape, SimulationPipeline,
    StepConfig, World, WorldAnchorJointDesc, WorldDesc, WorldRecipe,
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

#[test]
fn recipe_api_creates_a_world_that_can_step_query_and_debug() {
    let recipe = WorldRecipe::new(WorldDesc::default())
        .with_body(
            BodyBundle::static_body()
                .with_pose(Pose::from_xy_angle(0.0, -2.0, 0.0))
                .with_collider(
                    ColliderBundle::new(SharedShape::rect(8.0, 1.0))
                        .with_material(MaterialPreset::Rough)
                        .with_filter(CollisionLayerPreset::StaticGeometry),
                ),
        )
        .with_body(
            BodyBundle::dynamic()
                .with_pose(Pose::from_xy_angle(0.0, 0.0, 0.0))
                .with_collider(
                    ColliderBundle::new(SharedShape::circle(0.5))
                        .with_material(MaterialPreset::Bouncy)
                        .with_filter(CollisionLayerPreset::DynamicBody),
                ),
        );

    let mut created = recipe
        .instantiate()
        .expect("valid recipe should create a world");
    assert_eq!(created.created.body_handles.len(), 2);
    assert_eq!(created.created.collider_handles.len(), 2);
    assert_eq!(
        created
            .world
            .collider(created.created.collider_handles[1])
            .expect("dynamic collider should resolve")
            .material(),
        Material::preset(MaterialPreset::Bouncy)
    );
    assert!(
        CollisionFilter::preset(CollisionLayerPreset::DynamicBody).allows(
            &CollisionFilter::preset(CollisionLayerPreset::StaticGeometry)
        ),
        "default dynamic/static layer presets should interact"
    );

    let mut pipeline = SimulationPipeline::new(StepConfig::default());
    let report = pipeline.step(&mut created.world);
    assert_eq!(report.stats.body_count, 2);
    assert_eq!(report.stats.collider_count, 2);

    let mut query = QueryPipeline::new();
    query.sync(&created.world);
    assert!(!query
        .intersect_point((0.0, 0.0).into(), QueryFilter::default())
        .is_empty());

    let snapshot = created
        .world
        .debug_snapshot(&DebugSnapshotOptions::default());
    assert_eq!(snapshot.bodies.len(), 2);
    assert_eq!(snapshot.colliders.len(), 2);
}

#[test]
fn recipe_api_can_declare_joints_between_recipe_bodies() {
    let recipe = WorldRecipe::new(WorldDesc::default())
        .with_body(BodyBundle::static_body().with_pose(Pose::from_xy_angle(0.0, -1.0, 0.0)))
        .with_body(BodyBundle::dynamic().with_pose(Pose::from_xy_angle(0.0, 1.0, 0.0)))
        .with_joint(JointBundle::distance(0, 1).with_rest_length(2.0))
        .with_joint(JointBundle::world_anchor(1).with_world_anchor((0.0, 1.5).into()));

    let created = recipe
        .instantiate()
        .expect("valid recipe joints should instantiate");

    assert_eq!(created.created.body_handles.len(), 2);
    assert_eq!(created.created.joint_handles.len(), 2);
    assert_eq!(created.world.joints().count(), 2);
    assert_eq!(
        created
            .world
            .debug_snapshot(&DebugSnapshotOptions::default())
            .stats
            .active_joint_count,
        2
    );
}
