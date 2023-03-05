use nannou::{
    prelude::*,
    winit::event::{self},
};
use picea::{
    algo::is_point_inside_shape,
    element::{Element, ElementBuilder},
    math::{edge::Edge, point::Point, vector::Vector},
    meta::MetaBuilder,
    scene::Scene,
};
use std::time::SystemTime;

struct Model {
    scene: Scene,
    timer: SystemTime,
    collision_info: Option<Vec<[Point<f32>; 2]>>,
    addition_render_line: Vec<[Point<f32>; 2]>,
    addition_render_dot: Vec<Point<f32>>,
    cursor_point: Option<Point<f32>>,
    is_paused: bool,
    is_mouse_enter: bool,
}

fn create_model(_app: &App) -> Model {
    let mut scene = Scene::new();

    let ground: Element = ElementBuilder::new(
        (-500., -300., 1000., 100.),
        MetaBuilder::new(1.).is_fixed(true),
    )
    .into();

    scene.push_element(ground);

    let ball: Element = ElementBuilder::new(
        ((-400., -100.), 60.),
        MetaBuilder::new(1.).is_transparent(true), // .force("gravity", (0., -10.)),
    )
    .into();

    dbg!(ball.id());

    scene.push_element(ball);

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
            .is_transparent(true)
            .angular(-std::f32::consts::FRAC_PI_8)
            .force("gravity", (0., -10.)), // .is_fixed(true),
    );

    let element: Element = element.into();

    let center_point = element.shape().center_point();

    // scene.push_element(element);

    let element = ElementBuilder::new(
        (7, (50., 200.), 100.),
        MetaBuilder::new(1.).angular(-std::f32::consts::FRAC_PI_8),
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
        is_paused: false,
        cursor_point: None,
        is_mouse_enter: false,
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
        Event::WindowEvent {
            id: _,
            simple: Some(ev),
        } => match ev {
            WindowEvent::MousePressed(_) => {
                // mouse down
                model.is_mouse_enter = true;
            }
            WindowEvent::MouseReleased(_) => {
                model.is_mouse_enter = false;
                model.cursor_point = None;
            }
            WindowEvent::MouseMoved(p) if model.is_mouse_enter => {
                let x = p.x;
                let y = p.y;

                model.cursor_point = Some((x, y).into());

                for element in model.scene.elements_iter_mut() {
                    if is_point_inside_shape((x, y), &mut element.shape().edge_iter()) {
                        let center_point = element.shape().center_point();
                        element.translate(&(center_point, model.cursor_point.unwrap()).into());
                        break;
                    }
                }
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

            let ground = model.scene.get_element(1).unwrap();

            let ball = model.scene.get_element(2).unwrap();

            let a = ground;
            let b = ball;

            {
                #[derive(Clone, Debug)]
                struct MinkowskiDifferencePoint {
                    start_point: Point<f32>,
                    end_point: Point<f32>,
                    vector: Vector<f32>,
                }

                impl PartialEq for MinkowskiDifferencePoint {
                    fn eq(&self, other: &Self) -> bool {
                        self.vector == other.vector
                    }
                }

                impl From<(Point<f32>, Point<f32>)> for MinkowskiDifferencePoint {
                    fn from((s, e): (Point<f32>, Point<f32>)) -> Self {
                        Self {
                            start_point: s,
                            end_point: e,
                            vector: (s, e).into(),
                        }
                    }
                }

                let compute_support_point =
                    |reference_vector: Vector<f32>| -> MinkowskiDifferencePoint {
                        let (_, max_point_a) = a.shape().projection_on_vector(&reference_vector);
                        let (_, max_point_b) = b.shape().projection_on_vector(&-reference_vector);
                        (max_point_b, max_point_a).into()
                    };

                let center_point_a = a.center_point();
                let center_point_b = b.center_point();

                let first_approximation_vector: Vector<f32> =
                    (center_point_a, center_point_b).into();

                let gjk_point = compute_support_point(first_approximation_vector);
                model.addition_render_line = vec![];

                model
                    .addition_render_line
                    .push([gjk_point.start_point, gjk_point.end_point]);

                model
                    .addition_render_line
                    .push([a.center_point(), b.center_point()]);
            }

            if model.is_paused {
                return;
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

        let element_color = model
            .cursor_point
            .and_then(|p| {
                if is_point_inside_shape(p, &mut element.shape().edge_iter()) {
                    Some(RED)
                } else {
                    None
                }
            })
            .unwrap_or(WHITE);

        if let Some(edge) = element.shape().edge_iter().next() {
            match edge {
                Edge::Line { start_point, .. } => {
                    let center_point = element.center_point();
                    draw.line()
                        .color(element_color)
                        .start(vec2(center_point.x(), center_point.y()))
                        .end(vec2(start_point.x(), start_point.y()));
                }
                Edge::Circle { .. } => {}
                _ => unimplemented!(),
            }
        }

        element.shape().edge_iter().for_each(|edge| match edge {
            Edge::Line {
                start_point,
                end_point,
            } => {
                draw.line()
                    .color(element_color)
                    .start(vec2(start_point.x(), start_point.y()))
                    .end(vec2(end_point.x(), end_point.y()));
            }
            Edge::Circle {
                center_point,
                radius,
            } => {
                draw.ellipse()
                    .color(element_color)
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

    if let Some(p) = model.cursor_point {
        draw.ellipse().x_y(p.x(), p.y()).radius(2.).color(RED);
        draw.to_frame(app, &frame).unwrap()
    }

    draw.to_frame(app, &frame).unwrap();
}

fn main() {
    nannou::app(create_model)
        .event(event)
        .simple_window(view)
        .run();
}
