use nannou::{
    prelude::*,
    winit::event::{self},
};
use picea::{
    algo::is_point_inside_shape,
    element::{Element, ElementBuilder},
    math::{edge::Edge, vector::Vector},
    meta::MetaBuilder,
    scene::Scene,
    tools::{collision_view::CollisionStatusViewer, drag::Draggable},
};
use std::time::SystemTime;

struct Model {
    scene: Scene,
    draggable: Draggable,
    timer: SystemTime,
    collision_viewer: CollisionStatusViewer,
    is_paused: bool,
}

fn create_model(_app: &App) -> Model {
    let mut scene = Scene::new();

    // let ground: Element = ElementBuilder::new(
    //     (-500., -300., 1000., 100.),
    //     MetaBuilder::new(1.).is_fixed(true),
    // )
    // .into();

    // scene.push_element(ground);

    let ball = ElementBuilder::new(
        ((-400., -100.), 60.),
        MetaBuilder::new(1.).is_transparent(true),
    );

    let ball: Element = ball.into();

    let v: Vector<f32> = (0., 350.).into();
    dbg!(v.abs());

    let result = ball.shape().projection_on_vector(&(0., 350.).into());
    dbg!(result);

    scene.push_element(ball);

    let meta = MetaBuilder::new(1.)
        .is_transparent(true)
        .angular(f32::PI() / 3.);

    // scene.push_element(ElementBuilder::new(
    //     (3, (-1000. + 3. * 200., 250.), 100.),
    //     meta.clone().angular(f32::PI() / 6.),
    // ));

    for edge_size in 3..=6 {
        scene.push_element(ElementBuilder::new(
            (edge_size, (-1000. + edge_size as f32 * 200., 250.), 100.),
            meta.clone(), // .angular(f32::PI() / 6.),
        ));
        scene.push_element(ElementBuilder::new(
            (edge_size, (-1000. + edge_size as f32 * 200., -250.), 100.),
            meta.clone(),
        ));
    }

    Model {
        scene,
        draggable: Default::default(),
        collision_viewer: Default::default(),
        timer: SystemTime::now(),
        // collision_info: None,
        // addition_render_line: vec![],
        // addition_render_dot: vec![],
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
                // model.collision_info = None;
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
                model.draggable.on_mouse_down(&mut model.scene);
            }
            WindowEvent::MouseReleased(_) => {
                model.draggable.on_mouse_up();
            }
            WindowEvent::MouseMoved(p) => {
                model.draggable.on_mouse_move(&mut model.scene, p.x, p.y);
            }
            _ => {}
        },
        Event::Update(_) => {
            model.collision_viewer.on_update(&mut model.scene);

            let now = SystemTime::now();

            let duration = now.duration_since(model.timer).unwrap();

            model.timer = now;

            // model
            //     .scene
            //     .update_elements_by_duration(duration.as_secs_f32(), |collision_info| {
            //         // model.collision_info = Some(collision_info);
            //     })
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
            .draggable
            .mouse_point()
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

    model
        .collision_viewer
        .get_minkowski_different_points()
        .iter()
        .for_each(|points| {
            let draw = app.draw();

            for i in 0..points.len() {
                let p1 = points[i];
                let p2 = if i + 1 >= points.len() {
                    points[0]
                } else {
                    points[i + 1]
                };

                draw.line()
                    .weight(2.)
                    .color(YELLOW)
                    .start(vec2(p1.x(), p1.y()))
                    .end(vec2(p2.x(), p2.y()));
            }

            draw.to_frame(app, &frame).unwrap();
        });

    for info in model.collision_viewer.get_collision_infos() {
        info.points.iter().take(2).for_each(|point| {
            draw.ellipse()
                .x_y(point.x(), point.y())
                .radius(2.)
                .color(RED);
        });

        info.points.iter().skip(2).for_each(|point| {
            draw.ellipse()
                .x_y(point.x(), point.y())
                .radius(2.)
                .color(BLUE);
        });

        draw.line()
            .weight(2.)
            .color(RED)
            .start(vec2(0., 0.))
            .end(vec2(info.vector.x() * 100., info.vector.y() * 100.));
    }

    if let Some(p) = model.draggable.mouse_point() {
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
