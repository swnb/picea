use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use picea::{
    element::ElementBuilder, math::FloatNum, meta::MetaBuilder, scene::Scene, shape::Circle,
};

const STEP_DT: FloatNum = 1. / 60.;

fn continuing_contact_scene() -> Scene<()> {
    let mut scene = Scene::width_capacity(2);
    scene.set_gravity(|_| (0., 0.).into());

    let element_a_id = scene.push_element(ElementBuilder::new(
        Circle::new((0., 0.), 1.),
        MetaBuilder::new().mass(1.),
        (),
    ));
    let element_b_id = scene.push_element(ElementBuilder::new(
        Circle::new((1.5, 0.), 1.),
        MetaBuilder::new().mass(1.),
        (),
    ));

    *scene
        .get_element_mut(element_a_id)
        .expect("element a exists")
        .meta_mut()
        .velocity_mut() = (8., 0.).into();
    *scene
        .get_element_mut(element_b_id)
        .expect("element b exists")
        .meta_mut()
        .velocity_mut() = (-8., 0.).into();

    scene
}

fn circle_sweep_scene(count: usize, spacing: FloatNum) -> Scene<()> {
    let mut scene = Scene::width_capacity(count);
    scene.set_gravity(|_| (0., 0.).into());

    for index in 0..count {
        let element_id = scene.push_element(ElementBuilder::new(
            Circle::new((index as FloatNum * spacing, 0.), 1.),
            MetaBuilder::new().mass(1.),
            (),
        ));
        let direction = if index % 2 == 0 { 1. } else { -1. };
        *scene
            .get_element_mut(element_id)
            .expect("element exists")
            .meta_mut()
            .velocity_mut() = (direction * 3., 0.).into();
    }

    scene
}

fn step_scene(mut scene: Scene<()>, steps: usize) -> Scene<()> {
    for _ in 0..steps {
        scene.tick(STEP_DT);
    }
    scene
}

fn contact_refresh_transfer(c: &mut Criterion) {
    let mut group = c.benchmark_group("manifold");
    group.bench_function("contact_refresh_transfer", |b| {
        b.iter_batched(
            || {
                let mut scene = continuing_contact_scene();
                scene.tick(STEP_DT);
                scene
            },
            |mut scene| {
                scene.tick(black_box(STEP_DT));
                black_box(scene.get_position_fix_map().len());
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

fn step_circles(c: &mut Criterion) {
    let mut group = c.benchmark_group("step");
    for count in [16_usize, 64] {
        group.bench_function(format!("circles_{count}"), |b| {
            b.iter_batched(
                || circle_sweep_scene(count, 2.4),
                |scene| {
                    let scene = step_scene(scene, black_box(4));
                    black_box(scene.frame_count());
                    black_box(scene.element_size());
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn collision_broadphase(c: &mut Criterion) {
    let mut group = c.benchmark_group("collision");
    for (name, spacing) in [("broadphase_sparse_64", 3.2), ("broadphase_dense_64", 1.6)] {
        group.bench_function(name, |b| {
            b.iter_batched(
                || circle_sweep_scene(64, spacing),
                |mut scene| {
                    scene.tick(black_box(STEP_DT));
                    black_box(scene.get_position_fix_map().len());
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

criterion_group!(
    physics_scenarios,
    contact_refresh_transfer,
    step_circles,
    collision_broadphase
);
criterion_main!(physics_scenarios);
