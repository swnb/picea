use common::ConfigBuilder;

use picea::{
    element::{self, ElementBuilder},
    meta::MetaBuilder,
    scene::Scene,
    shape::{circle::Circle, line::Line, polygon::Square, GeometryTransform},
};

#[path = "../examples_common.rs"]
mod common;

fn init_elements(scene: &mut Scene) {
    let start_x = 10.;
    let start_y = 10.;

    let mut map = [[0; 10]; 32];

    const GAP: f32 = 5.;

    for row in 0..map.len() {
        for col in 0..map[row].len() {
            let shape = Circle::new(
                (start_x + row as f32 * GAP, start_y + col as f32 * GAP),
                0.3,
            );
            let element = ElementBuilder::new(shape, MetaBuilder::new(1.));
            map[row][col] = scene.push_element(element);
        }
    }

    (0..map.len()).for_each(|row| {
        let center_point = scene
            .get_element(map[row][0])
            .map(|element| element.center_point());
        if let Some(center_point) = center_point {
            scene.create_point_constraint(map[row][0], center_point, center_point, 0.);
        }
    });

    for row in 0..map.len() {
        for col in 0..map[row].len() {
            let element = scene.get_element(map[row][col]).unwrap();
            let element_id = element.id();
            let element_center_point = element.center_point();

            if row != 0 {
                let left_element = scene.get_element(map[row - 1][col]).unwrap();
                scene.create_join_constraint(
                    element_id,
                    element_center_point,
                    left_element.id(),
                    left_element.center_point(),
                    GAP,
                );
            }

            if col != 0 {
                let top_element = scene.get_element(map[row][col - 1]).unwrap();
                scene.create_join_constraint(
                    element_id,
                    element_center_point,
                    top_element.id(),
                    top_element.center_point(),
                    GAP,
                );
            }

            if col != map[row].len() - 1 {
                let bottom_element = scene.get_element(map[row][col + 1]).unwrap();
                scene.create_join_constraint(
                    element_id,
                    element_center_point,
                    bottom_element.id(),
                    bottom_element.center_point(),
                    GAP,
                );
            }

            if row != map.len() - 1 {
                let right_element = scene.get_element(map[row + 1][col]).unwrap();
                scene.create_join_constraint(
                    element_id,
                    element_center_point,
                    right_element.id(),
                    right_element.center_point(),
                    GAP,
                );
            }
        }
    }

    scene.push_element(ElementBuilder::new(
        Circle::new((60., 60.), 10.),
        MetaBuilder::new(100.)
            .velocity((0., -30.))
            .is_ignore_gravity(true),
    ));

    scene.push_element(ElementBuilder::new(
        Line::new((10., 100.), (100., 100.)),
        MetaBuilder::new(100.).is_fixed(true),
    ));
}

fn update(scene: &mut Scene, _selected_element_id: Option<u32>) {
    let duration = std::time::Duration::from_secs(10);
    scene.update_elements_by_duration(duration.as_secs_f32());
}

fn main() {
    let config = ConfigBuilder::default()
        .draw_center_point(false)
        .draw_join_constraints(true)
        .draw_point_constraints(true)
        .enable_mouse_constraint(true);
    common::run_window("point constraint - link", config, init_elements, update)
}
