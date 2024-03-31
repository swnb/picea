use common::ConfigBuilder;
use picea::{
    element::{Element, ElementBuilder},
    math::FloatNum,
    meta::MetaBuilder,
    scene::Scene,
    shape::{line::Line, Square},
};

#[path = "../examples_common.rs"]
mod common;

fn init(scene: &mut Scene, handler: &mut common::Handler<()>) {
    scene.set_gravity(|_| (0., 3.).into());

    scene.context_mut().constraint_parameters.max_allow_permeate = 0.7;
    scene.context_mut().constraint_parameters.split_position_fix = true;

    let mut start_x = 30.;
    let start_y = 60.;

    const BALL_COUNT: usize = 1;
    const BALL_SIZE: FloatNum = 10.;
    const BALL_GAP: FloatNum = 26.;

    let end_x = start_x * 2. + BALL_SIZE as FloatNum - (-10.);

    let ground_bottom = Line::new((-10., 100.), (end_x, 100.));

    scene.push_element(ElementBuilder::new(
        ground_bottom,
        MetaBuilder::new()
            .mass(2.)
            .is_fixed(true)
            .factor_restitution(0.),
        (),
    ));

    for _ in 0..BALL_COUNT {
        let mut ball = Square::new(start_x, start_y, BALL_SIZE);
        ball.rotate(-0.1);

        let mut element: Element<()> =
            ElementBuilder::new(ball, MetaBuilder::new().factor_restitution(1.), ()).into();

        let id = scene.push_element(element);
        dbg!(id);
        start_x += BALL_GAP;
    }
}

fn update(scene: &mut Scene, _selected_element_id: Option<u32>, _: &mut common::Handler<()>) {
    let duration = std::time::Duration::from_secs(10);
    scene.update_elements_by_duration(duration.as_secs_f32());
}

fn main() {
    common::run_window(
        "stack",
        ConfigBuilder::default()
            .draw_velocity(true)
            .draw_contact_point_pair(true),
        init,
        update,
    );
}
