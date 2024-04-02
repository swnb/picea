use common::ConfigBuilder;
use picea::{
    constraints::JoinConstraintConfigBuilder,
    element::ElementBuilder,
    math::{vector::Vector, FloatNum},
    meta::MetaBuilder,
    prelude::*,
    scene::Scene,
    shape::circle::Circle,
};

#[path = "../examples_common.rs"]
mod common;

fn init_elements(scene: &mut Scene, _: &mut common::Handler<()>) {
    // scene
    //     .context_mut()
    //     .constraint_parameters
    //     .skip_friction_constraints = true;

    scene.context_mut().constraint_parameters.max_allow_permeate = 0.01;
    scene.context_mut().constraint_parameters.factor_restitution = 1.;

    scene.set_gravity(|_| (0., 30.).into());

    let start_x = 45.;
    let start_y = 60.;

    const SIZE: FloatNum = 10.;

    let mut shape = Circle::new((start_x, start_y), SIZE);
    // let mut shape = ConstRegularPolygon::<4>::new((start_x, start_y), SIZE);

    let mut element_ids = vec![];

    const BOX_COUNT: usize = 6;

    for i in 0..BOX_COUNT {
        let mut meta_builder = MetaBuilder::new();
        if i == 0 {
            meta_builder = meta_builder.angle_velocity(1.);
        } else if i == (BOX_COUNT - 1) {
            meta_builder = meta_builder.velocity((10., 0.));
        }
        let element_id = scene.push_element(ElementBuilder::new(shape.clone(), meta_builder, ()));
        element_ids.push(element_id);
        shape.translate(&(SIZE * 2.0, 0.).into());
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
                    .hard(true),
            );
        });
}

fn update(scene: &mut Scene, _selected_element_id: Option<u32>, _: &mut common::Handler<()>) {
    scene.tick(1. / 60.);
}

fn main() {
    let config = ConfigBuilder::default()
        .draw_center_point(true)
        .draw_join_constraints(true)
        .draw_point_constraints(true)
        .enable_mouse_constraint(true)
        .draw_contact_point_pair(true);
    // .draw_velocity(true);

    common::run_window("point constraint - link", config, init_elements, update)
}
