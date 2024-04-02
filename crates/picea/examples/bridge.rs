use picea::{prelude::*, shape::Rect};

#[path = "../examples_common.rs"]
mod common;

fn init(scene: &mut Scene, _: &mut common::Handler<()>) {
    let bridge_count = 10;
    let start_x = 20f32;
    let start_y = 70f32;

    let bridge_width = 10f32;
    let bridge_height = 5f32;

    let mut elements = vec![];
    for i in 0..bridge_count {
        let id = scene.push_element(ElementBuilder::new(
            Rect::new(
                start_x + i as f32 * bridge_width,
                start_y,
                bridge_width,
                bridge_height,
            ),
            MetaBuilder::new(),
            (),
        ));
        elements.push(id);
    }

    for i in 0..(elements.len() - 1) {
        scene.create_join_constraint(
            elements[i],
            (start_x + (i + 1) as f32 * bridge_width, start_y),
            elements[i + 1],
            (start_x + (i + 1) as f32 * bridge_width, start_y),
            JoinConstraintConfigBuilder::new()
                .hard(false)
                .damping_ratio(2.)
                .frequency(0.9),
        );
    }

    scene.create_point_constraint(
        elements[0],
        (start_x, start_y),
        (start_x - 2., start_y),
        JoinConstraintConfigBuilder::new().hard(true),
    );

    scene.create_point_constraint(
        *elements.last().unwrap(),
        (start_x + elements.len() as f32 * bridge_width, start_y),
        (start_x + 2. + elements.len() as f32 * bridge_width, start_y),
        JoinConstraintConfigBuilder::new().hard(true),
    );

    // scene.push_element()
}

fn main() {
    common::run_simple(init)
}
