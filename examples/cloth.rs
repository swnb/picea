use common::ConfigBuilder;

use picea::{
    constraints::{JoinConstraintConfig, JoinConstraintConfigBuilder},
    element::ElementBuilder,
    math::PI,
    meta::MetaBuilder,
    scene::Scene,
    shape::{circle::Circle, line::Line},
};

#[path = "../examples_common.rs"]
mod common;

fn init_elements(scene: &mut Scene) {
    let start_x = 10.;
    let start_y = 10.;

    let mut map = [[0; 10]; 32];

    const GAP: f32 = 5.;

    (0..map.len()).for_each(|row| {
        for col in 0..map[row].len() {
            let shape = Circle::new(
                (start_x + row as f32 * GAP, start_y + col as f32 * GAP),
                0.3,
            );
            let element = ElementBuilder::new(shape, MetaBuilder::new(1.), ());
            map[row][col] = scene.push_element(element);
        }
    });

    (0..map.len()).for_each(|row| {
        let center_point = scene
            .get_element(map[row][0])
            .map(|element| element.center_point());
        if let Some(center_point) = center_point {
            scene.create_point_constraint(
                map[row][0],
                center_point,
                center_point,
                JoinConstraintConfigBuilder::default()
                    .damping_ratio(0.5)
                    .frequency(PI())
                    .distance(GAP)
                    .build()
                    .unwrap(),
            );
        }
    });

    let mut create_join_constraint =
        |(row_a, col_a): (usize, usize), (row_b, col_b): (usize, usize)| {
            let element_a_id = map[row_a][col_a];
            let element_b_id = map[row_b][col_b];
            let center_point = scene
                .get_element(element_a_id)
                .map(|element| element.center_point())
                .zip(
                    scene
                        .get_element(element_b_id)
                        .map(|element| element.center_point()),
                );

            if let Some((center_point_a, center_point_b)) = center_point {
                scene.create_join_constraint(
                    element_a_id,
                    center_point_a,
                    element_b_id,
                    center_point_b,
                    JoinConstraintConfigBuilder::default()
                        .damping_ratio(0.5)
                        .frequency(PI())
                        .distance(GAP)
                        .build()
                        .unwrap(),
                );
            }
        };

    (0..(map.len() - 1)).for_each(|row| {
        for col in 0..map[row].len() {
            create_join_constraint((row, col), (row + 1, col));
        }
    });

    (0..(map.len())).for_each(|row| {
        for col in 0..(map[row].len() - 1) {
            create_join_constraint((row, col), (row, col + 1));
        }
    });

    scene.push_element(ElementBuilder::new(
        Circle::new((60., 60.), 10.),
        MetaBuilder::new(100.)
            .velocity((0., -30.))
            .is_ignore_gravity(true),
        (),
    ));

    scene.push_element(ElementBuilder::new(
        Line::new((10., 100.), (100., 100.)),
        MetaBuilder::new(100.).is_fixed(true),
        (),
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
