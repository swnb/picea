use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use picea::prelude::{
    BodyBundle, BodyDesc, BodyType, ColliderBundle, CollisionLayerPreset, MaterialPreset, Pose,
    SharedShape, SimulationPipeline, StepConfig, StepReport, World, WorldCommands, WorldDesc,
};

const BROADPHASE_GRID: usize = 12;
const DENSE_GRID: usize = 9;
const STACK_HEIGHT: usize = 10;
const API_BATCH_SIZE: usize = 128;

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
            "n={steps}/bodies={}/colliders={}/candidates={}/contacts={}/ccd_hits={}",
            stats.body_count,
            stats.collider_count,
            stats.broadphase_candidate_count,
            stats.contact_count,
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
    bench_step_scenario(c, "stack_stability", stack_stability_world, 60);
    bench_step_scenario(c, "ccd_bullet", ccd_bullet_world, 1);
    bench_api_batch_creation(c);
}

criterion_group!(benches, physics_scenarios);
criterion_main!(benches);
