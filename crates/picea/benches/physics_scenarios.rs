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

criterion_group!(physics_scenarios, contact_refresh_transfer);
criterion_main!(physics_scenarios);
