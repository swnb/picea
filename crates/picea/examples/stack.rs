use common::ConfigBuilder;
use picea::{element::ElementBuilder, meta::MetaBuilder, scene::Scene, shape::line::Line};

#[path = "../examples_common.rs"]
mod common;

fn init(scene: &mut Scene, _: &mut common::Handler<()>) {
    scene.set_gravity(|_| (0., 3.).into());

    scene.context_mut().constraint_parameters.max_allow_permeate = 0.5;
    scene.context_mut().constraint_parameters.split_position_fix = true;

    let ground_bottom = Line::new((0., 100.), (100., 100.));

    scene.push_element(ElementBuilder::new(
        ground_bottom,
        MetaBuilder::new()
            .mass(100.)
            .is_fixed(true)
            .factor_friction(0.8),
        (),
    ));

    const MAX_LEVEL: usize = 10;
    for level in 0..MAX_LEVEL {
        let max_col = level;
        for col in 0..max_col {
            let mut meta = MetaBuilder::new()
                .mass(10.)
                .factor_friction(1.0)
                .factor_restitution(0.);
            if level == 9 {
                meta = meta.is_fixed(true);
            }
            let element = ElementBuilder::new(
                (
                    4,
                    (30. + (col as f32 * 30.), (level as f32 * 30.) - 200.),
                    5.,
                ),
                meta,
                (),
            );
            scene.push_element(element);
        }
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
