use common::ConfigBuilder;
use picea::{
    element::ElementBuilder,
    math::FloatNum,
    meta::MetaBuilder,
    scene::Scene,
    shape::{circle::Circle, line::Line},
};

#[path = "../examples_common.rs"]
mod common;

fn init(scene: &mut Scene) {
    scene.set_gravity(|_| (0., 3.).into());

    let ground_bottom = Line::new((-10., 100.), (1000., 100.));

    scene.push_element(ElementBuilder::new(
        ground_bottom,
        MetaBuilder::new(1.).is_fixed(true),
        (),
    ));

    let mut start_x = 30.;
    let start_y = 10.;

    const BALL_COUNT: usize = 1;
    const BALL_SIZE: FloatNum = 10.;
    const BALL_GAP: FloatNum = 20.;

    for _ in 0..BALL_COUNT {
        let ball = Circle::new((start_x, start_y), BALL_SIZE);
        scene.push_element(ElementBuilder::new(ball, MetaBuilder::new(1.), ()));
        start_x += BALL_GAP;
    }
}

fn update(scene: &mut Scene, _selected_element_id: Option<u32>) {
    let duration = std::time::Duration::from_secs(10);
    scene.update_elements_by_duration(duration.as_secs_f32());
}

fn main() {
    common::run_window("stack", ConfigBuilder::default(), init, update);
}
