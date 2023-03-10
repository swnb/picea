use nannou::{prelude::*, winit::event};
use picea::{
    element::{Element, ElementBuilder},
    math::{edge::Edge, point::Point, vector::Vector},
    meta::MetaBuilder,
    scene::Scene,
};
use std::time::SystemTime;

struct Model {
    scene: Scene,
    timer: SystemTime,
    collision_info: Option<Vec<[Point; 2]>>,
    addition_render_line: Vec<[Point; 2]>,
    addition_render_dot: Vec<Point>,
    is_paused: bool,
}

fn create_model(_app: &App) -> Model {
    let mut scene = Scene::new();

    let ground: Element = ElementBuilder::new(
        (-500., -300., 1000., 100.),
        MetaBuilder::new(1.).is_fixed(true),
    )
    .into();

    scene.push_element(ground);

    // let ball: Element = ElementBuilder::new(
    //     ((-400., -100.), 60.),
    //     MetaBuilder::new(1.).force("gravity", (0., -10.)),
    // )
    // .into();

    // scene.push_element(ball);

    // let element = ElementBuilder::new(
    //     ((200., 200.), 100.),
    //     MetaBuilder::new(1.)
    //         .angular(std::f32::consts::FRAC_PI_6)
    //         .force("gravity", (0., -10.)), // .is_fixed(true),
    // );

    // let element: Element = element.into();

    // scene.push_element(element);

    // let element = ElementBuilder::new(
    //     (7, (50., 200.), 100.),
    //     MetaBuilder::new(1.)
    //         .angular(-std::f32::consts::FRAC_PI_8)
    //         .force("gravity", (0., -10.)), // .is_fixed(true),
    // );

    // let element = ElementBuilder::new(
    //     (7, (50., 200.), 100.),
    //     MetaBuilder::new(1.)
    //         .angular(-std::f32::consts::FRAC_PI_8)
    //         .force("gravity", (0., -10.)), // .is_fixed(true),
    // );

    // let element: Element = element.into();

    let element = ElementBuilder::new(
        (7, (50., 40.), 100.),
        MetaBuilder::new(1.)
            .angular(-std::f32::consts::FRAC_PI_8)
            .force("gravity", (0., -10.)), // .is_fixed(true),
    );

    let element: Element = element.into();

    scene.push_element(element);

    Model {
        scene,
        timer: SystemTime::now(),
        collision_info: None,
        addition_render_line: vec![],
        addition_render_dot: vec![],
        is_paused: false,
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
            event::VirtualKeyCode::Space => {
                model.is_paused = !model.is_paused;
            }
            _ => {}
        },
        Event::Update(_) => {
            let now = SystemTime::now();

            let duration = now.duration_since(model.timer).unwrap();

            model.timer = now;

            if model.is_paused {
                return;
            }

            model
                .scene
                .update_elements_by_duration(duration.as_secs_f32())
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
