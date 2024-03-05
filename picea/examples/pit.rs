use std::collections::VecDeque;

use common::ConfigBuilder;
use picea::{
    element::ElementBuilder,
    math::{point::Point, FloatNum},
    meta::MetaBuilder,
    scene::Scene,
    shape::{circle::Circle, concave::ConcavePolygon, line::Line},
};

#[path = "../examples_common.rs"]
mod common;

fn init(scene: &mut Scene, _: &mut common::Handler<()>) {
    let ground_bottom: Line = Line::new((10., 90.), (210., 90.));

    scene.context_mut().constraint_parameters.max_allow_permeate = 0.01;

    scene.push_element(ElementBuilder::new(
        ground_bottom,
        MetaBuilder::new(10.).is_fixed(true),
        (),
    ));

    let vertexes = [
        (30, 70),
        (80, 70),
        (100, 50),
        (90, 30),
        (110, 30),
        (110, 80),
        (20, 80),
        (20, 30),
        (40, 30),
    ];

    let vertexes = vertexes
        .iter()
        .map(|&(x, y)| (x as FloatNum, y as FloatNum))
        .map(|v| v.into())
        .collect::<VecDeque<Point>>();

    let concave_polygon = ConcavePolygon::new(&Vec::from(vertexes)[..]);

    let element = ElementBuilder::new(concave_polygon, MetaBuilder::new(10.), ());

    scene.push_element(element);

    for i in 0..6 {
        for j in 0..6 {
            scene.push_element(ElementBuilder::new(
                Circle::new(
                    (50. + j as FloatNum * 5. + 2., 10. + i as FloatNum * 5. + 2.),
                    2.,
                ),
                MetaBuilder::new(10.),
                (),
            ));
        }
    }
}

fn update(scene: &mut Scene, _selected_element_id: Option<u32>, _: &mut common::Handler<()>) {
    let duration = std::time::Duration::from_secs(10);
    scene.update_elements_by_duration(duration.as_secs_f32());
}

fn main() {
    let config = ConfigBuilder::default();

    common::run_window("concave", config, init, update)
}
