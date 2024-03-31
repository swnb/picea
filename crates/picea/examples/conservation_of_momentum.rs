use common::{ConfigBuilder, Handler};
use picea::prelude::*;
use picea::{element::ElementBuilder, meta::MetaBuilder, scene::Scene, shape::circle::Circle};

#[path = "../examples_common.rs"]
mod common;

fn init_elements(scene: &mut Scene, _: &mut Handler<()>) {
    scene
        .context_mut()
        .constraint_parameters
        .skip_friction_constraints = true;

    scene.context_mut().constraint_parameters.split_position_fix = true;

    scene.context_mut().constraint_parameters.max_allow_permeate = 0.1;

    scene.set_gravity(|_| (0., 0.).into());

    let start_x = 45.;
    let start_y = 60.;

    let mut shape = Circle::new((start_x, start_y), 10.);
    // let mut shape = RegularPolygon::new((start_x, start_y), 4, 10.);

    let mut element_ids = vec![];

    const BOX_COUNT: usize = 2;

    for i in 0..BOX_COUNT {
        let mut meta_builder = MetaBuilder::default();
        if i == 0 {
            meta_builder = meta_builder.velocity((10., 0.));
        } else {
            meta_builder = meta_builder.velocity((0., 0.));
        }
        let element_id = scene.push_element(ElementBuilder::new(shape.clone(), meta_builder, ()));
        element_ids.push(element_id);
        shape.translate(&(40., 0.).into());
    }
}

fn update(scene: &mut Scene, _selected_element_id: Option<u32>, _: &mut Handler<()>) {
    let duration = std::time::Duration::from_secs(10);
    scene.update_elements_by_duration(duration.as_secs_f32());
}

fn main() {
    let config = ConfigBuilder::default()
        .draw_center_point(true)
        .draw_join_constraints(true)
        .draw_point_constraints(true)
        .enable_mouse_constraint(true)
        .draw_contact_point_pair(true)
        .draw_velocity(true);

    common::run_window("point constraint - link", config, init_elements, update)
}
