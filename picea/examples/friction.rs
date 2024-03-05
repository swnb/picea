use common::ConfigBuilder;
use picea::{
    element::ElementBuilder, math::FloatNum, meta::MetaBuilder, scene::Scene, shape::line::Line,
};

#[path = "../examples_common.rs"]
mod common;

#[derive(Clone, Default)]
struct Data {
    previous_created_time: FloatNum,
}

fn init(scene: &mut Scene<Data>, handler: &mut common::Handler<Data>) {
    scene.set_gravity(|_| (0., 3.).into());

    let incline = Line::new((10., 70.), (1000., 200.));

    scene.push_element(ElementBuilder::new(
        incline,
        MetaBuilder::new(1.).is_fixed(true).friction(0.1),
        Default::default(),
    ));
}

fn update(
    scene: &mut Scene<Data>,
    _selected_element_id: Option<u32>,
    _: &mut common::Handler<Data>,
) {
    let duration = std::time::Duration::from_secs(10);
    scene.update_elements_by_duration(duration.as_secs_f32());

    let previous_created_time = scene.data.previous_created_time;

    if scene.total_duration() - previous_created_time > 2. {
        let element = ElementBuilder::new(
            (4, (30., 10.), 5.),
            MetaBuilder::new(10.).friction((((scene.total_duration()) as FloatNum) / 1.) * 0.001),
            Default::default(),
        );

        scene.push_element(element);

        dbg!((((scene.total_duration()) as FloatNum) / 1.) * 0.001);
        scene.data.previous_created_time += duration.as_secs_f32();
    }
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
