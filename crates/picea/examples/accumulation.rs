use common::ConfigBuilder;
use picea::{
    element::{ElementBuilder, ShapeTraitUnion},
    math::{point::Point, FloatNum},
    meta::MetaBuilder,
    scene::Scene,
    shape::{circle::Circle, concave::ConcavePolygon, polygon::RegularPolygon},
};
use rand::Rng;
use std::collections::VecDeque;

#[path = "../examples_common.rs"]
mod common;

fn init(scene: &mut Scene, _: &mut common::Handler<()>) {
    let top = 10;
    let down = 85;

    let mut x = 15;
    let mut y: i32 = down;

    let mut vertices = Vec::new();
    for i in 0..17 {
        vertices.push((x, y));
        x += 5;
        if i % 2 == 0 {
            y += 5;
        } else {
            y -= 2;
        }
    }

    for i in 0..17 {
        vertices.push((x, y));
        x += 5;
        if i % 2 == 0 {
            y -= 5;
        } else {
            y += 2;
        }
    }

    const WALL_HEIGHT: i32 = 120;

    vertices.push((185, 50));
    vertices.push((190, WALL_HEIGHT));
    vertices.push((1, WALL_HEIGHT));
    vertices.push((10, 50));

    let vertices = vertices
        .iter()
        .map(|&(x, y)| (x as FloatNum, y as FloatNum - 30.))
        .map(|v| v.into())
        .collect::<VecDeque<Point>>();

    let concave_polygon = ConcavePolygon::new(&Vec::from(vertices)[..]);

    let element = ElementBuilder::new(
        concave_polygon,
        MetaBuilder::default().mass(200.).is_fixed(true),
        (),
    );

    scene.push_element(element);

    let mut gen = rand::thread_rng();

    for i in 0..70 {
        for j in 0..17 {
            let x = 40. + j as FloatNum * 5.;
            let y = -300. + i as FloatNum * 5.;

            let value: u8 = gen.gen();

            let edge = value % 10;
            let shape: Box<dyn ShapeTraitUnion> = if edge < 3 {
                Box::new(Circle::new((x, y), 2.))
            } else {
                Box::new(RegularPolygon::new((x, y), edge as usize, 2.))
            };

            scene.push_element(ElementBuilder::new(shape, MetaBuilder::new().mass(10.), ()));
        }
    }
}

fn update(scene: &mut Scene, _selected_element_id: Option<u32>, _: &mut common::Handler<()>) {
    scene.tick(1. / 60.);
}

fn main() {
    let config = ConfigBuilder::default().draw_center_point(false);
    common::run_window("concave", config, init, update)
}
