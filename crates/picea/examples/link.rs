use common::ConfigBuilder;
use picea::{
    constraints::JoinConstraintConfig, element::ElementBuilder, meta::MetaBuilder, scene::Scene,
    shape::Square,
};

#[path = "../examples_common.rs"]
mod common;

fn init_elements(scene: &mut Scene, _: &mut common::Handler<()>) {
    let start_x = 60.;
    let start_y = 20.;

    let mut shape = Square::new(start_x, start_y, 10.);

    let mut element_ids = vec![];

    const BOX_COUNT: usize = 6;

    for _ in 0..BOX_COUNT {
        let element_id =
            scene.push_element(ElementBuilder::new(shape.clone(), MetaBuilder::new(), ()));
        element_ids.push(element_id);
        shape.translate(&(20., 0.).into());
    }

    let mut x = start_x + 10f32;
    let mut y = start_y + 10f32;

    for i in 0..(BOX_COUNT - 1) {
        scene.create_join_constraint(
            element_ids[i],
            (x, y),
            element_ids[i + 1],
            (x + 10., y),
            JoinConstraintConfig::default(),
        );
        x += 20.;
        if i % 2 == 0 {
            y -= 10.;
        } else {
            y += 10.;
        }
    }

    scene.create_point_constraint(
        element_ids[0],
        (start_x, start_y),
        (start_x, start_y),
        JoinConstraintConfig::default(),
    );
}

fn update(scene: &mut Scene, _selected_element_id: Option<u32>, _: &mut common::Handler<()>) {
    scene.tick(1. / 60.);
}

fn main() {
    let config = ConfigBuilder::default()
        .is_default_paused(true)
        .draw_center_point(true)
        .draw_join_constraints(true)
        .draw_point_constraints(true)
        .enable_mouse_constraint(true);
    common::run_window("point constraint - link", config, init_elements, update)
}
