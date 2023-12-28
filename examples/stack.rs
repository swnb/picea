use common::ConfigBuilder;
use picea::{element::ElementBuilder, meta::MetaBuilder, scene::Scene, shape::line::Line};

#[path = "../examples_common.rs"]
mod common;

fn init(scene: &mut Scene) {
    let ground_bottom = Line::new((-10., 100.), (1000., 100.));

    scene.push_element(ElementBuilder::new(
        ground_bottom,
        MetaBuilder::new(1.).is_fixed(true),
    ));

    const MAX_LEVEL: usize = 11;
    for level in 0..MAX_LEVEL {
        let max_col = level;
        for col in 0..max_col {
            let element = ElementBuilder::new(
                (
                    4,
                    (30. + (col as f32 * 20.), (level as f32 * 10.) - 10.),
                    5.,
                ),
                MetaBuilder::new(10.),
            );
            scene.push_element(element);
        }
    }
}

fn update(scene: &mut Scene, _selected_element_id: Option<u32>) {
    let duration = std::time::Duration::from_secs(10);
    scene.update_elements_by_duration(duration.as_secs_f32());
}

fn main() {
    common::run_window("stack", ConfigBuilder::default(), init, update);
}
