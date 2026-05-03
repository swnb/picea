use picea::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut world = World::new(WorldDesc {
        gravity: Vector::new(0.0, 9.8),
        ..WorldDesc::default()
    });

    let floor = world.create_body(BodyDesc {
        body_type: BodyType::Static,
        pose: Pose::from_xy_angle(0.0, 1.0, 0.0),
        ..BodyDesc::default()
    })?;
    world.create_collider(
        floor,
        ColliderDesc {
            shape: SharedShape::rect(6.0, 0.25),
            ..ColliderDesc::default()
        },
    )?;

    let falling_box = world.create_body(BodyDesc {
        body_type: BodyType::Dynamic,
        pose: Pose::from_xy_angle(0.0, -1.0, 0.0),
        can_sleep: false,
        ..BodyDesc::default()
    })?;
    world.create_collider(
        falling_box,
        ColliderDesc {
            shape: SharedShape::rect(0.5, 0.5),
            ..ColliderDesc::default()
        },
    )?;

    let mut pipeline = SimulationPipeline::new(StepConfig::default());
    let report = pipeline.step(&mut world);
    let snapshot = DebugSnapshot::from_world_with_step_report(
        &world,
        &report,
        &DebugSnapshotOptions::default(),
    );

    let mut query = QueryPipeline::new();
    query.sync(&world);
    let floor_hits = query.intersect_point(Point::new(0.0, 1.0), QueryFilter::default());

    let recipe_result = WorldRecipe::new(WorldDesc::default())
        .with_scene_body(
            BodyBundle::dynamic()
                .with_pose(Pose::from_xy_angle(0.0, -2.0, 0.0))
                .with_collider(ColliderBundle::new(SharedShape::circle(0.25))),
        )
        .instantiate()
        .expect("the public beta smoke recipe should be valid");

    println!(
        "step={} bodies={} query_hits={} recipe_bodies={}",
        report.stats.step_index,
        snapshot.bodies.len(),
        floor_hits.len(),
        recipe_result.world.bodies().count()
    );

    Ok(())
}
