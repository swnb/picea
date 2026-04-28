use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use picea::prelude::{
    BodyBundle, BodyDesc, BodyType, ColliderBundle, ColliderDesc, CollisionLayerPreset,
    DistanceJointDesc, JointDesc, MaterialPreset, Point, Pose, QueryFilter, QueryPipeline,
    QueryStats, SharedShape, SimulationPipeline, StepConfig, StepReport, World, WorldCommands,
    WorldDesc,
};

const BROADPHASE_GRID: usize = 12;
const DENSE_GRID: usize = 9;
const STACK_HEIGHT: usize = 10;
const API_BATCH_SIZE: usize = 128;
const ISLAND_COUNT: usize = 32;

fn step_config() -> StepConfig {
    StepConfig {
        dt: 1.0 / 60.0,
        ..StepConfig::default()
    }
}

fn run_steps(mut world: World, steps: usize) -> StepReport {
    let mut pipeline = SimulationPipeline::new(step_config());
    let mut report = StepReport::default();
    for _ in 0..steps {
        report = pipeline.step(&mut world);
    }
    report
}

fn bench_step_scenario(
    c: &mut Criterion,
    group_name: &str,
    make_world: fn() -> World,
    steps: usize,
) {
    let baseline = run_steps(make_world(), steps);
    let stats = baseline.stats;
    // Criterion records timing; the benchmark id also records deterministic
    // engine counters so a local baseline explains what the timed step did
    // without turning early numbers into pass/fail thresholds.
    let id = BenchmarkId::new(
        "steps",
        format!(
            concat!(
                "n={}/bodies={}/colliders={}/candidates={}/",
                "broadphase_traversals={}/broadphase_pruned={}/",
                "contacts={}/contact_rows={}/joint_rows={}/",
                "islands={}/active_islands={}/sleep_skips={}/solver_slots={}/",
                "ccd_hits={}"
            ),
            steps,
            stats.body_count,
            stats.collider_count,
            stats.broadphase_candidate_count,
            stats.broadphase_traversal_count,
            stats.broadphase_pruned_count,
            stats.contact_count,
            stats.contact_row_count,
            stats.joint_row_count,
            stats.island_count,
            stats.active_island_count,
            stats.sleeping_island_skip_count,
            stats.solver_body_slot_count,
            stats.ccd_hit_count
        ),
    );

    let mut group = c.benchmark_group(group_name);
    group.bench_function(id, |bench| {
        bench.iter_batched(
            || make_world(),
            |world| black_box(run_steps(world, steps)),
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

fn many_small_islands_world() -> World {
    let mut world = World::new(WorldDesc {
        gravity: (0.0, 0.0).into(),
        enable_sleep: true,
    });
    let bodies = (0..ISLAND_COUNT)
        .map(|index| {
            BodyBundle::dynamic()
                .with_pose(Pose::from_xy_angle(index as f32 * 4.0, 0.0, 0.0))
                .with_collider(
                    ColliderBundle::new(SharedShape::circle(0.35))
                        .with_filter(CollisionLayerPreset::DynamicBody)
                        .with_material(MaterialPreset::Default),
                )
        })
        .collect::<Vec<_>>();
    world
        .commands()
        .create_bodies(bodies)
        .expect("many-islands setup should create");
    world
}

fn one_large_island_world() -> World {
    let mut world = World::new(WorldDesc {
        gravity: (0.0, 0.0).into(),
        enable_sleep: true,
    });
    let mut handles = Vec::new();
    for index in 0..ISLAND_COUNT {
        let body = world
            .create_body(BodyDesc {
                body_type: BodyType::Dynamic,
                pose: Pose::from_xy_angle(index as f32 * 0.8, 0.0, 0.0),
                gravity_scale: 0.0,
                ..BodyDesc::default()
            })
            .expect("large-island body should create");
        world
            .create_collider(
                body,
                ColliderDesc {
                    shape: SharedShape::circle(0.35),
                    ..ColliderDesc::default()
                },
            )
            .expect("large-island collider should create");
        handles.push(body);
    }
    for pair in handles.windows(2) {
        world
            .create_joint(JointDesc::Distance(DistanceJointDesc {
                body_a: pair[0],
                body_b: pair[1],
                rest_length: 0.8,
                stiffness: 1.0,
                damping: 0.0,
                ..DistanceJointDesc::default()
            }))
            .expect("large-island joint should create");
    }
    world
}

fn query_heavy_world() -> World {
    let mut world = World::new(WorldDesc::default());
    let bodies = (0..BROADPHASE_GRID)
        .flat_map(|row| {
            (0..BROADPHASE_GRID).map(move |col| {
                BodyBundle::static_body()
                    .with_pose(Pose::from_xy_angle(col as f32 * 1.5, row as f32 * 1.5, 0.0))
                    .with_collider(
                        ColliderBundle::new(SharedShape::rect(0.75, 0.75))
                            .with_filter(CollisionLayerPreset::StaticGeometry)
                            .with_material(MaterialPreset::Default),
                    )
            })
        })
        .collect::<Vec<_>>();
    world
        .commands()
        .create_bodies(bodies)
        .expect("query-heavy setup should create");
    world
}

fn query_points() -> Vec<Point> {
    (0..BROADPHASE_GRID)
        .flat_map(|row| {
            (0..BROADPHASE_GRID).map(move |col| Point::new(col as f32 * 1.5, row as f32 * 1.5))
        })
        .collect()
}

fn add_query_stats(total: &mut QueryStats, stats: QueryStats) {
    total.traversal_count += stats.traversal_count;
    total.candidate_count += stats.candidate_count;
    total.pruned_count += stats.pruned_count;
    total.filter_drop_count += stats.filter_drop_count;
    total.hit_count += stats.hit_count;
}

fn run_query_sweep(world: &World, points: &[Point]) -> (usize, QueryStats) {
    let mut query = QueryPipeline::new();
    query.sync(world);
    let mut hit_count = 0usize;
    let mut stats = QueryStats::default();
    for point in points {
        hit_count += query.intersect_point(*point, QueryFilter::default()).len();
        add_query_stats(&mut stats, query.last_stats());
    }
    (hit_count, stats)
}

fn bench_query_heavy(c: &mut Criterion) {
    let world = query_heavy_world();
    let points = query_points();
    let (hits, stats) = run_query_sweep(&world, &points);
    let id = BenchmarkId::new(
        "intersect_point_sweep",
        format!(
            "queries={}/hits={hits}/candidates={}/traversals={}/pruned={}/filter_drops={}",
            points.len(),
            stats.candidate_count,
            stats.traversal_count,
            stats.pruned_count,
            stats.filter_drop_count
        ),
    );

    let mut group = c.benchmark_group("query_heavy");
    group.bench_function(id, |bench| {
        bench.iter_batched(
            || (query_heavy_world(), query_points()),
            |(world, points)| black_box(run_query_sweep(&world, &points)),
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

fn sparse_broadphase_world() -> World {
    let mut world = World::new(WorldDesc::default());
    let bodies = (0..BROADPHASE_GRID)
        .flat_map(|row| {
            (0..BROADPHASE_GRID).map(move |col| {
                let x = col as f32 * 5.0;
                let y = row as f32 * 5.0;
                BodyBundle::static_body()
                    .with_pose(Pose::from_xy_angle(x, y, 0.0))
                    .with_collider(
                        ColliderBundle::new(SharedShape::circle(0.4))
                            .with_filter(CollisionLayerPreset::StaticGeometry)
                            .with_material(MaterialPreset::Ice),
                    )
            })
        })
        .collect::<Vec<_>>();
    world
        .commands()
        .create_bodies(bodies)
        .expect("sparse broadphase setup should create");
    world
}

fn dense_broadphase_world() -> World {
    let mut world = World::new(WorldDesc::default());
    let bodies = (0..DENSE_GRID)
        .flat_map(|row| {
            (0..DENSE_GRID).map(move |col| {
                let x = col as f32 * 0.45;
                let y = row as f32 * 0.45;
                BodyBundle::dynamic()
                    .with_pose(Pose::from_xy_angle(x, y, 0.0))
                    .with_collider(
                        ColliderBundle::new(SharedShape::circle(0.5))
                            .with_filter(CollisionLayerPreset::DynamicBody)
                            .with_material(MaterialPreset::Rough),
                    )
            })
        })
        .collect::<Vec<_>>();
    world
        .commands()
        .create_bodies(bodies)
        .expect("dense broadphase setup should create");
    world
}

fn stack_stability_world() -> World {
    let mut world = World::new(WorldDesc::default());
    let mut bodies = Vec::new();
    bodies.push(
        BodyBundle::static_body()
            .with_pose(Pose::from_xy_angle(0.0, 6.0, 0.0))
            .with_collider(
                ColliderBundle::new(SharedShape::rect(12.0, 1.0))
                    .with_filter(CollisionLayerPreset::StaticGeometry)
                    .with_material(MaterialPreset::Rough),
            ),
    );
    bodies.extend((0..STACK_HEIGHT).map(|index| {
        BodyBundle::dynamic()
            .with_pose(Pose::from_xy_angle(0.0, 5.0 - index as f32 * 1.05, 0.0))
            .with_collider(
                ColliderBundle::new(SharedShape::rect(1.0, 1.0))
                    .with_filter(CollisionLayerPreset::DynamicBody)
                    .with_material(MaterialPreset::Rough),
            )
    }));
    world
        .commands()
        .create_bodies(bodies)
        .expect("stack setup should create");
    world
}

fn ccd_bullet_world() -> World {
    let mut world = World::new(WorldDesc {
        gravity: (0.0, 0.0).into(),
        enable_sleep: false,
    });
    let wall = BodyBundle::static_body()
        .with_pose(Pose::from_xy_angle(2.0, 0.0, 0.0))
        .with_collider(
            ColliderBundle::new(SharedShape::rect(0.1, 6.0))
                .with_filter(CollisionLayerPreset::StaticGeometry)
                .with_material(MaterialPreset::Rough),
        );
    let bullet = BodyBundle::new(BodyDesc {
        body_type: BodyType::Dynamic,
        pose: Pose::from_xy_angle(-3.0, 0.0, 0.0),
        linear_velocity: (420.0, 0.0).into(),
        gravity_scale: 0.0,
        can_sleep: false,
        ..BodyDesc::default()
    })
    .with_collider(
        ColliderBundle::new(SharedShape::circle(0.25))
            .with_filter(CollisionLayerPreset::DynamicBody)
            .with_material(MaterialPreset::Bouncy),
    );
    world
        .commands()
        .create_bodies([wall, bullet])
        .expect("ccd bullet setup should create");
    world
}

fn ccd_dynamic_pair_world() -> World {
    let mut world = World::new(WorldDesc {
        gravity: (0.0, 0.0).into(),
        enable_sleep: false,
    });
    let left = BodyBundle::new(BodyDesc {
        body_type: BodyType::Dynamic,
        pose: Pose::from_xy_angle(-1.0, 0.0, 0.0),
        linear_velocity: (200.0, 0.0).into(),
        gravity_scale: 0.0,
        can_sleep: false,
        ..BodyDesc::default()
    })
    .with_collider(
        ColliderBundle::new(SharedShape::rect(0.1, 0.1))
            .with_filter(CollisionLayerPreset::DynamicBody)
            .with_material(MaterialPreset::Default),
    );
    let right = BodyBundle::new(BodyDesc {
        body_type: BodyType::Dynamic,
        pose: Pose::from_xy_angle(1.0, 0.0, 0.0),
        linear_velocity: (-200.0, 0.0).into(),
        gravity_scale: 0.0,
        can_sleep: false,
        ..BodyDesc::default()
    })
    .with_collider(
        ColliderBundle::new(SharedShape::rect(0.1, 0.1))
            .with_filter(CollisionLayerPreset::DynamicBody)
            .with_material(MaterialPreset::Default),
    );
    world
        .commands()
        .create_bodies([left, right])
        .expect("dynamic CCD pair setup should create");
    world
}

fn api_batch_bodies(count: usize) -> Vec<BodyBundle> {
    (0..count)
        .map(|index| {
            let x = (index % 16) as f32 * 1.25;
            let y = (index / 16) as f32 * 1.25;
            BodyBundle::dynamic()
                .with_pose(Pose::from_xy_angle(x, y, 0.0))
                .with_collider(
                    ColliderBundle::new(SharedShape::circle(0.35))
                        .with_filter(CollisionLayerPreset::DynamicBody)
                        .with_material(MaterialPreset::Default),
                )
        })
        .collect()
}

fn bench_api_batch_creation(c: &mut Criterion) {
    let mut baseline_world = World::new(WorldDesc::default());
    let baseline = baseline_world
        .commands()
        .create_bodies(api_batch_bodies(API_BATCH_SIZE))
        .expect("baseline API batch should create");
    let id = BenchmarkId::new(
        "create_bodies",
        format!(
            "bodies={}/colliders={}",
            baseline.body_handles.len(),
            baseline.collider_handles.len()
        ),
    );

    let mut group = c.benchmark_group("api_batch_creation");
    group.bench_function(id, |bench| {
        bench.iter_batched(
            || {
                (
                    World::new(WorldDesc::default()),
                    api_batch_bodies(API_BATCH_SIZE),
                )
            },
            |(mut world, bodies)| {
                let mut commands = WorldCommands::new(&mut world);
                black_box(
                    commands
                        .create_bodies(bodies)
                        .expect("API batch should create"),
                )
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

fn physics_scenarios(c: &mut Criterion) {
    bench_step_scenario(c, "sparse_broadphase", sparse_broadphase_world, 1);
    bench_step_scenario(c, "dense_broadphase", dense_broadphase_world, 1);
    bench_query_heavy(c);
    bench_step_scenario(c, "many_small_islands", many_small_islands_world, 10);
    bench_step_scenario(c, "one_large_island", one_large_island_world, 10);
    bench_step_scenario(c, "stack_stability", stack_stability_world, 60);
    bench_step_scenario(c, "ccd_bullet", ccd_bullet_world, 1);
    bench_step_scenario(c, "ccd_dynamic_pair", ccd_dynamic_pair_world, 1);
    bench_api_batch_creation(c);
}

criterion_group!(benches, physics_scenarios);
criterion_main!(benches);
