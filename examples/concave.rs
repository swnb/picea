use std::collections::VecDeque;

use picea::{
    element::ElementBuilder,
    math::{point::Point, FloatNum},
    meta::MetaBuilder,
    shape::{circle::Circle, concave::ConcavePolygon, line::Line},
};

#[path = "../examples_common.rs"]
mod common;

fn main() {
    common::entry_main(
        "concave",
        |scene| {
            let ground_bottom = Line::new((10., 90.), (110., 90.));

            scene.push_element(ElementBuilder::new(
                ground_bottom,
                MetaBuilder::new(1.).is_fixed(true),
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

            let element = ElementBuilder::new(concave_polygon, MetaBuilder::new(100.));

            scene.push_element(element);

            for i in 0..6 {
                for j in 0..6 {
                    scene.push_element(ElementBuilder::new(
                        Circle::new(
                            (50. + j as FloatNum * 5. + 2., 10. + i as FloatNum * 5. + 2.),
                            2.,
                        ),
                        MetaBuilder::new(10.),
                    ));
                }
            }
        },
        |scene| {
            let duration = std::time::Duration::from_secs(10);
            scene.update_elements_by_duration(duration.as_secs_f32());
        },
    )
}
