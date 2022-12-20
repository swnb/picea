use std::time::SystemTime;

use nannou::{prelude::*, winit::event};
use picea::{
    element::{Element, ElementBuilder},
    math::{edge::Edge, point::Point},
    meta::MetaBuilder,
    scene::Scene,
};

struct Model {
    scene: Scene,
    timer: SystemTime,
    collision_info: Option<Vec<[Point<f32>; 2]>>,
    addition_render_line: Vec<[Point<f32>; 2]>,
    addition_render_dot: Vec<Point<f32>>,
}

fn create_model(_app: &App) -> Model {
    let mut scene = Scene::new();

    let ground = ElementBuilder::new(
        (-100., -200., 400., 200.),
        MetaBuilder::new(1.).is_fixed(true),
    );

    scene.push_element(ground);

    // let element = ElementBuilder::new(
    //     ((200., 200.), 100.),
    //     MetaBuilder::new(1.)
    //         .angular(std::f32::consts::FRAC_PI_6)
    //         .force("gravity", (0., -10.)), // .is_fixed(true),
    // );

    // let element: Element = element.into();

    // scene.push_element(element);

    let element = ElementBuilder::new(
        (7, (50., 200.), 100.),
        MetaBuilder::new(1.)
            .angular(-std::f32::consts::FRAC_PI_8)
            .force("gravity", (0., -10.)), // .is_fixed(true),
    );

    // let element = ElementBuilder::new(
    //     (7, (50., 200.), 100.),
    //     MetaBuilder::new(1.)
    //         .angular(-std::f32::consts::FRAC_PI_8)
    //         .force("gravity", (0., -10.)), // .is_fixed(true),
    // );

    let element: Element = element.into();

    let center_point = element.shape().center_point();

    scene.push_element(element);

    let element = ElementBuilder::new(
        (7, (50., 400.), 100.),
        MetaBuilder::new(1.)
            .angular(-std::f32::consts::FRAC_PI_8)
            .force("gravity", (0., -10.)), // .is_fixed(true),
    );

    let element: Element = element.into();

    let center_point = element.shape().center_point();

    scene.push_element(element);

    Model {
        scene,
        timer: SystemTime::now(),
        collision_info: None,
        addition_render_line: vec![],
        addition_render_dot: vec![center_point],
    }
}

fn event(app: &App, model: &mut Model, event: Event) {
    match event {
        Event::WindowEvent {
            id: _,
            simple: Some(WindowEvent::KeyPressed(t)),
        } => match t {
            event::VirtualKeyCode::R => *model = create_model(app),
            event::VirtualKeyCode::C => {
                model.collision_info = None;
            }
            _ => {}
        },
        Event::Update(_) => {
            let now = SystemTime::now();

            let duration = now.duration_since(model.timer).unwrap();

            model.timer = now;

            if model.collision_info.is_none() {
                // return;
            }

            model.addition_render_dot = vec![];
            model.scene.elements_iter().for_each(|element| {
                model
                    .addition_render_dot
                    .push(element.shape().center_point())
            });

            model
                .scene
                .update_elements_by_duration(duration.as_secs_f32(), |collision_info| {
                    model.collision_info = Some(collision_info);
                })
        }
        _ => {}
    }
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
    });

    if let Some(collision_info) = &model.collision_info {
        // frame.clear(BLACK);
        collision_info.iter().for_each(|point| {
            let draw = app.draw();

            // dbg!(point[0].x(), point[0].y());
            draw.line()
                .weight(2.)
                .color(YELLOW)
                .start(vec2(point[0].x(), point[0].y()))
                .end(vec2(point[1].x(), point[1].y()));
            draw.to_frame(app, &frame).unwrap();
        });
    }

    model.addition_render_line.iter().for_each(|point| {
        let draw = app.draw();

        dbg!(point[1].x(), point[1].y());

        draw.line()
            .weight(2.)
            .color(YELLOW)
            .start(vec2(point[0].x(), point[0].y()))
            .end(vec2(point[1].x(), point[1].y()));
        draw.to_frame(app, &frame).unwrap();
    });

    model.addition_render_dot.iter().for_each(|point| {
        let draw = app.draw();

        draw.ellipse()
            .x_y(point.x(), point.y())
            .radius(2.)
            .color(YELLOW);
        draw.to_frame(app, &frame).unwrap();
    });
}

fn main() {
    nannou::app(create_model)
        .event(event)
        .simple_window(view)
        .run();
}