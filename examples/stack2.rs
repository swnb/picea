use common::ConfigBuilder;
use picea::{
    element::ElementBuilder,
    meta::MetaBuilder,
    scene::Scene,
    shape::{line::Line, polygon::Rect},
};

#[path = "../examples_common.rs"]
mod common;

fn init(scene: &mut Scene, _: &mut common::Handler<()>) {
    scene.set_gravity(|_| (0., 30.).into());

    scene.context_mut().constraint_parameters.max_allow_permeate = 0.7;
    scene.context_mut().constraint_parameters.split_position_fix = true;
    // scene
    //     .context_mut()
    //     .constraint_parameters
    //     .skip_friction_constraints = true;

    let ground_bottom = Line::new((50., 100.), (150., 100.));

    scene.push_element(ElementBuilder::new(
        ground_bottom,
        MetaBuilder::new(100.).is_fixed(true).friction(0.8),
        (),
    ));

    const MAX_LEVEL: usize = 3;

    let mut start_y = 30.;

    let mut start_x = 100.;

    for level in 0..MAX_LEVEL {
        let mut meta = MetaBuilder::new(10.).friction(1.0).factor_restitution(1.);
        if level == (MAX_LEVEL - 1) {
            meta = meta.is_fixed(true);
        }
        let element = ElementBuilder::new(
            Rect::new(start_x, start_y + (level as f32 * 11.), 10., 10.),
            meta,
            (),
        );

        scene.push_element(element);
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
