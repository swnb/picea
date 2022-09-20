use std::time::SystemTime;

use nannou::prelude::*;
use picea::{
    element::{Element, ElementBuilder},
    math::edge::Edge,
    meta::MetaBuilder,
    scene::Scene,
};

struct Model {
    scene: Scene,
    timer: SystemTime,
}

fn model(_app: &App) -> Model {
    let mut scene = Scene::new();

    let ground = ElementBuilder::new(
        (-100., -200., 400., 200.),
        MetaBuilder::new(1.).is_fixed(true),
    );

    scene.push_element(ground);

    let element = ElementBuilder::new(
        ((200., 200.), 100.),
        MetaBuilder::new(1.)
            .angular(std::f32::consts::FRAC_PI_6)
            .force("gravity", (0., -10.)), // .is_fixed(true),
    );

    let element: Element = element.into();

    scene.push_element(element);

    // let element = ElementBuilder::new(
    //     (7, (50., 200.), 100.),
    //     MetaBuilder::new(1.)
    //         .angular(std::f32::consts::FRAC_PI_6)
    //         .force("gravity", (0., -10.)), // .is_fixed(true),
    // );

    // let element: Element = element.into();

    // scene.push_element(element);

    Model {
        scene,
        timer: SystemTime::now(),
    }
}

fn event(_app: &App, model: &mut Model, event: Event) {
    let now = SystemTime::now();
    let duration = now.duration_since(model.timer).unwrap();
    model.timer = now;
    model
        .scene
        .update_elements_by_duration(duration.as_secs_f32(), |_| {})
}

fn view(app: &App, model: &Model, frame: Frame) {
    frame.clear(BLACK);

    let draw = app.draw();

    draw.line()
        .color(WHITE)
        .start(vec2(-1000., 0.))
        .end(vec2(1000., 0.));

    draw.line()
        .color(WHITE)
        .start(vec2(0., -1000.))
        .end(vec2(0., 1000.));
    draw.to_frame(app, &frame).unwrap();

    model.scene.elements_iter().for_each(|element| {
        let draw = app.draw();

        element.shape().edge_iter().for_each(|edge| match edge {
            Edge::Line {
                start_point,
                end_point,
            } => {
                draw.line()
                    .color(WHITE)
                    .start(vec2(start_point.x(), start_point.y()))
                    .end(vec2(end_point.x(), end_point.y()));
            }
            Edge::Circle {
                center_point,
                radius,
            } => {
                draw.ellipse()
                    .color(WHITE)
                    .x_y(center_point.x(), center_point.y())
                    .width(radius * 2.)
                    .height(radius * 2.);
            }
            _ => unimplemented!(),
        });

        draw.to_frame(app, &frame).unwrap();
    })
}

fn main() {
    nannou::app(model).event(event).simple_window(view).run();
}
