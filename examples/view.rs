use nannou::prelude::*;

use picea::{
    element::{Element, ElementShape},
    math::{
        point::Point,
        vector::{Vector, Vector3},
    },
    meta::{
        force::{Force, ForceGroup},
        MetaBuilder,
    },
    scene::Scene,
    shape::rect::RectShape,
};

use rand::prelude::*;
use std::time;

const G: f32 = 9.8;

struct Model {
    scene: Scene,
    time: time::Instant,
    gravity_force: Force,
    data: Vec<Point<f32>>,
    stop: bool,
}

fn main() {
    nannou::app(create_model)
        .event(event)
        .simple_window(view)
        .run();
}

fn create_model(app: &App) -> Model {
    create_model3(app)
}

fn create_model1(_app: &App) -> Model {
    let mut rng = rand::thread_rng();
    let mut scene = Scene::new();

    let gravity_force = Force::new("gravity", (0., -G));

    for i in 0..70 {
        let gravity_force = gravity_force.clone();

        let speed_x: f32 = rng.gen_range(-20.0..20.0);
        let speed_y: f32 = rng.gen_range(-20.0..20.0);

        let mut force_group = ForceGroup::new();

        force_group.add_force(Force::new("air", (0., 0.)));

        let shape = ElementShape::Rect(
            (
                (
                    -350. + ((i % 20) as f32 * 40.),
                    100. - ((i / 20) * 50) as f32,
                ),
                (20., 20.),
            )
                .into(),
        );

        let meta = MetaBuilder::new(1.)
            .force("air", (0., 0.))
            .angular_velocity(std::f32::consts::PI / 10.)
            .velocity((speed_x, speed_y));

        let element = Element::new(shape, meta);

        scene.push_element(element);
    }

    Model {
        scene,
        time: time::Instant::now(),
        gravity_force,
        data: vec![],
        stop: false,
    }
}

fn create_model3(_app: &App) -> Model {
    let mut rng = rand::thread_rng();
    let mut scene = Scene::new();

    let gravity_force = Force::new("gravity", (0., -G * 10.));

    let shape = ElementShape::Rect(((-200., -300.), (150., 650.)).into());

    let meta = MetaBuilder::new(1.)
        .force("air", (0., 0.))
        // .angular(std::f32::consts::FRAC_PI_3)
        .velocity((20.0, 0.))
        .is_fixed(true);

    let element = Element::new(shape, meta);

    scene.push_element(element);

    let shape = ElementShape::Rect(((10., 100.), (-100., 100.)).into());

    let meta = MetaBuilder::new(10.)
        // .angular_velocity(std::f32::consts::PI / 10.)
        // FIXME;
        .angular(-std::f32::consts::FRAC_PI_8)
        .velocity((rng.gen_range(-50.0..-20.0), 0.))
        .force("gravity", (-3000., 0.));

    let element = Element::new(shape, meta);

    scene.push_element(element);

    Model {
        scene,
        time: time::Instant::now(),
        gravity_force,
        stop: false,
        data: vec![],
    }
}

fn create_model4(_app: &App) -> Model {
    let mut rng = rand::thread_rng();
    let mut scene = Scene::new();

    let gravity_force = Force::new("gravity", (0., -G));

    // force_group.add_force(Force::new("air", (0., 0.).into()));
    let p1 = (-31.720043, 79.35677);
    let p2 = (-26.027605, 98.52961);
    let p3 = (-45.200382, 104.22198);
    let p4 = (-50.892815, 85.04917);
    let shape = ElementShape::Rect(RectShape::new([p1, p2, p3, p4]));

    let meta = MetaBuilder::new(1.)
        .angular_velocity(-std::f32::consts::FRAC_PI_2)
        .velocity((20.0, 0.));

    let element = Element::new(shape, meta);
    scene.push_element(element);

    let p1 = (-37.031464, 56.207893);
    let p2 = (-17.36696, 59.855812);
    let p3 = (-21.014877, 79.520325);
    let p4 = (-40.679382, 75.8724);

    let shape = ElementShape::Rect(RectShape::new([p1, p2, p3, p4]));

    let meta = MetaBuilder::new(1.)
        .angular_velocity(std::f32::consts::PI / 10.)
        // FIXME;
        .angular(std::f32::consts::FRAC_PI_8)
        .velocity((rng.gen_range(-20.0..0.0), 0.));

    let element = Element::new(shape, meta);

    scene.push_element(element);

    Model {
        scene,
        time: time::Instant::now(),
        gravity_force,
        stop: false,
        data: vec![],
    }
}

fn event(app: &App, model: &mut Model, event: Event) {
    match event {
        Event::WindowEvent {
            simple: Some(WindowEvent::KeyPressed(Key::R)),
            ..
        } => {
            *model = create_model(app);
        }
        Event::WindowEvent {
            simple: Some(WindowEvent::KeyPressed(Key::Space)),
            ..
        } => model.stop = !model.stop,
        Event::Update(update) => {
            if !model.stop {
                let duration = update.since_last;

                model.data.clear();
                model
                    .scene
                    .update_elements_by_duration(duration.as_secs_f32(), |data| {
                        model.data.extend(data);
                    });
            }
        }
        _ => {}
    }
}

fn view(app: &App, model: &Model, frame: Frame) {
    frame.clear(WHITESMOKE);

    let draw = app.draw();

    // draw cross
    draw.line()
        .color(BLACK)
        .points((-1000., 0.).into(), (1000., 0.).into());

    draw.line()
        .color(BLACK)
        .points((0., -1000.).into(), (0., 1000.).into());

    fn p2pt2(p: impl Into<Point<f32>>) -> Point2 {
        Some(p)
            .map(|v| v.into())
            .map(|v| v.into())
            .map(|(x, y)| pt2(x, y))
            .unwrap()
    }
}
