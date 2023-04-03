use nannou::{prelude::*, winit::event};
use picea::{
    element::{Element, ElementBuilder},
    math::{edge::Edge, point::Point, FloatNum},
    meta::MetaBuilder,
    scene::Scene,
    shape::{convex::ConvexPolygon, line::Line, utils::split_concave_polygon_to_convex_polygons},
    tools::{
        collision_view::CollisionStatusViewer, snapshot::create_element_construct_code_snapshot,
    },
};
use std::{collections::VecDeque, time::SystemTime};

struct Model {
    scene: Scene,
    timer: SystemTime,
    collision_viewer: CollisionStatusViewer,
    is_paused: bool,
}

fn create_model(_app: &App) -> Model {
    let mut scene = Scene::new();

    let ground_bottom = Line::new((-200., -14.), (200., -14.));

    scene.push_element(ElementBuilder::new(
        ground_bottom,
        MetaBuilder::new(1.).is_fixed(true),
    ));

    let vertexes = vec![
        (-1, 5),
        (0, 0),
        (1, 0),
        (1, 10),
        (-10, 10),
        (-10, 17),
        (5, 17),
        (5, -10),
        (-1, -11),
    ];

    let vertexes = vertexes
        .iter()
        .map(|&(x, y)| (x as FloatNum, y as FloatNum))
        .map(|v| v.into())
        .collect::<VecDeque<Point>>();

    let polygons = split_concave_polygon_to_convex_polygons(&Vec::from(vertexes)[..]);

    let elements: Vec<Element> = polygons
        .into_iter()
        .map(ConvexPolygon::new)
        .map(|shape| {
            ElementBuilder::new(shape, MetaBuilder::new(10.).force("gravity", (0., -100.))).into()
        })
        .collect();

    elements.into_iter().for_each(|e| {
        scene.push_element(e);
    });

    Model {
        scene,
        timer: SystemTime::now(),
        is_paused: true,
        collision_viewer: Default::default(),
    }
}

fn event(app: &App, model: &mut Model, event: Event) {
    match event {
        Event::WindowEvent {
            id: _,
            simple: Some(WindowEvent::KeyPressed(t)),
        } => match t {
            event::VirtualKeyCode::R => *model = create_model(app),
            event::VirtualKeyCode::D => model.scene.elements_iter().for_each(|element| {
                dbg!(element.id());
                let code = create_element_construct_code_snapshot(element);
                dbg!(code);
            }),
            event::VirtualKeyCode::Space => {
                model.is_paused = !model.is_paused;
            }
            _ => {}
        },
        Event::Update(_) => {
            // model.collision_viewer.on_update(&mut model.scene);

            let now = SystemTime::now();

            let duration = now.duration_since(model.timer).unwrap();

            model.timer = now;

            if model.is_paused {
                return;
            }

            model
                .scene
                .update_elements_by_duration(duration.as_secs_f32());
            // model.is_paused = true;
        }
        _ => {}
    }
}

fn view(app: &App, model: &Model, frame: Frame) {
    frame.clear(BLACK);

    let draw = app.draw();

    let scale = 10.;

    let make_line = |color: rgb::Srgb<u8>, start_point: Point, end_point: Point| {
        draw.line()
            .color(color)
            .start(vec2(start_point.x() * scale, start_point.y() * scale))
            .end(vec2(end_point.x() * scale, end_point.y() * scale));
    };

    let make_ellipse = |color: rgb::Srgb<u8>, center_point: Point, radius: f32| {
        draw.ellipse()
            .color(color)
            .x_y(center_point.x() * scale, center_point.y() * scale)
            .width(radius * 2. * scale)
            .height(radius * 2. * scale);
    };

    model.scene.elements_iter().for_each(|element| {
        element
            .shape()
            .edge_iter()
            .take(1)
            .for_each(|edge| match edge {
                Edge::Line {
                    start_point,
                    end_point,
                } => {
                    make_line(WHITE, element.center_point(), *start_point);
                }
                Edge::Circle {
                    center_point,
                    radius,
                } => {}
                _ => unimplemented!(),
            });

        make_ellipse(BLUE, element.center_point(), 0.5);

        element.shape().edge_iter().for_each(|edge| match edge {
            Edge::Line {
                start_point,
                end_point,
            } => make_line(WHITE, *start_point, *end_point),
            Edge::Circle {
                center_point,
                radius,
            } => {
                make_ellipse(WHITE, center_point, radius);
            }
            _ => unimplemented!(),
        });
    });

    for info in model.collision_viewer.get_collision_infos() {
        let point = info.point_a();

        make_ellipse(RED, point, 6. / scale);

        let point = info.point_b();

        make_ellipse(ORANGE, point, 6. / scale);

        let v = info.normal_toward_a();

        make_line(RED, (0., 0.).into(), (v * 10f32).to_point());
    }

    draw.to_frame(app, &frame).unwrap();
}

fn main() {
    nannou::app(create_model)
        .event(event)
        .simple_window(view)
        .run();
}
