use common::ConfigBuilder;
use picea::{
    constraints::JoinConstraintConfigBuilder,
    element::ElementBuilder,
    math::{vector::Vector, FloatNum},
    meta::MetaBuilder,
    scene::Scene,
    shape::circle::Circle,
};

#[path = "../examples_common.rs"]
mod common;

fn init_elements(scene: &mut Scene) {
    scene
        .context_mut()
        .constraint_parameters
        .skip_friction_constraints = true;

    scene.context_mut().constraint_parameters.max_allow_permeate = 0.001;
    scene.context_mut().constraint_parameters.factor_elastic = 0.5;

    scene.set_gravity(|_| (0., 10.).into());

    let start_x = 45.;
    let start_y = 60.;

    const SIZE: FloatNum = 10.;

    let mut shape = Circle::new((start_x, start_y), SIZE);

    let mut element_ids = vec![];

    const BOX_COUNT: usize = 2;

    for i in 0..BOX_COUNT {
        let mut meta_builder = MetaBuilder::new(1.);
        if i == 0 {
            meta_builder = meta_builder.velocity((-30., 0.));
        }
        let element_id = scene.push_element(ElementBuilder::new(shape.clone(), meta_builder, ()));
        element_ids.push(element_id);
        shape.translate(&(SIZE * 2., 0.).into());
    }

    let elements: Vec<_> = scene
        .elements_iter_mut()
        .map(|element| (element.id(), element.center_point()))
        .collect();

    elements
        .into_iter()
        .for_each(|(element_id, element_center_point)| {
            let p = element_center_point + Vector::from((0., -40.));
            scene.create_point_constraint(
                element_id,
                element_center_point,
                p,
                JoinConstraintConfigBuilder::default()
                    .distance(40.)
                    .hard(true)
                    .build()
                    .unwrap(),
            );
        });
}

fn update(scene: &mut Scene, _selected_element_id: Option<u32>) {
    let duration = std::time::Duration::from_secs(10);
    scene.update_elements_by_duration(duration.as_secs_f32());
}

fn main() {
    let config = ConfigBuilder::default()
        .draw_center_point(true)
        .draw_join_constraints(true)
        .draw_point_constraints(true)
        .enable_mouse_constraint(true);

    common::run_window("point constraint - link", config, init_elements, update)
}
