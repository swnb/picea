use nannou::{prelude::*, winit::event};
use picea::{
    element::ElementBuilder,
    math::{edge::Edge, point::Point},
    meta::MetaBuilder,
    scene::Scene,
    shape::{convex::ConvexPolygon, line::Line},
    tools::collision_view::CollisionStatusViewer,
};
use std::time::SystemTime;

struct Model {
    scene: Scene,
    timer: SystemTime,
    collision_info: Option<Vec<[Point; 2]>>,
    collision_viewer: CollisionStatusViewer,
    is_paused: bool,
}

fn create_model(_app: &App) -> Model {
    let mut scene = Scene::new();

    const height: f32 = 30.;

    let wall_bottom = Line::new((-50., -30.), (50., -30.));
    // let wall_bottom = (-50., -30., 100., 4.);
    let meta = MetaBuilder::new(100.).is_fixed(true);
    let id = scene.push_element(ElementBuilder::new(wall_bottom, meta.clone()));
    println!("{id} wall_bottom");

    let wall_right = Line::new((50., -30.), (50., height));
    // let wall_right = (50., -30., 4., 100.);
    let id = scene.push_element(ElementBuilder::new(wall_right, meta.clone()));
    println!("{id} wall_right");

    let wall_left = Line::new((-50., -30.), (-50., height));
    let id = scene.push_element(ElementBuilder::new(wall_left, meta.clone()));
    println!("{id} wall_left ");

    // let ball: Element = ElementBuilder::new(
    //     ((-40., -10.), 6.),
    //     MetaBuilder::new(10.).force("gravity", (0., -10.)),
    // )
    // .into();

    // scene.push_element(ball);

    // let element = ElementBuilder::new(
    //     ((200., 200.), 100.),
    //     MetaBuilder::new(1.)
    //         .angle(std::f32::consts::FRAC_PI_6)
    //         .force("gravity", (0., -10.)), // .is_fixed(true),
    // );

    // scene.push_element(element);

    let element = ElementBuilder::new(
        (7, (-30., 20.), 20.),
        MetaBuilder::new(10.)
            .angle(-f32::FRAC_PI_8())
            // .angle_velocity(-std::f32::consts::FRAC_PI_8)
            .force("gravity", (10., -10. * 10.)), // .is_fixed(true),
    );

    let id = scene.push_element(element);
    println!("{id} element left");

    let element = ElementBuilder::new(
        (6, (10., 20.), 20.),
        MetaBuilder::new(10.)
            .angle(-f32::FRAC_PI_8())
            // .angle_velocity(-std::f32::consts::FRAC_PI_8)
            .force("gravity", (10., -10. * 10.)), // .is_fixed(true),
    );

    let id = scene.push_element(element);
    println!("{id} element right");

    Model {
        scene,
        timer: SystemTime::now(),
        collision_info: None,
        is_paused: false,
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
            event::VirtualKeyCode::C => {
                model.collision_info = None;
            }
            event::VirtualKeyCode::Space => {
                model.is_paused = !model.is_paused;
            }
            _ => {}
        },
        Event::Update(_) => {
            model.collision_viewer.on_update(&mut model.scene);

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

    let scale = 5.;

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

    make_line(WHITE, (-1000., 0.).into(), (1000., 0.).into());
    make_line(WHITE, (0., -1000.).into(), (0., 1000.).into());

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
                } => {
                    // draw.ellipse()
                    //     .color(WHITE)
                    //     .x_y(center_point.x(), center_point.y())
                    //     .width(radius * 2.)
                    //     .height(radius * 2.);
                }
                _ => unimplemented!(),
            });

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

    if let Some(collision_info) = &model.collision_info {
        collision_info.iter().for_each(|point| {
            make_line(YELLOW, point[0], point[1]);
        });
    }

    // model
    //     .collision_viewer
    //     .get_minkowski_different_points()
    //     .iter()
    //     .for_each(|points| {
    //         for i in 0..points.len() {
    //             let p1 = points[i];
    //             let p2 = if i + 1 >= points.len() {
    //                 points[0]
    //             } else {
    //                 points[i + 1]
    //             };

    //             make_line(YELLOW, p1, p2);
    //         }
    //     });

    // let points = model.collision_viewer.get_all_minkowski_different_points();

    // for i in 0..points.len() {
    //     let p1 = points[i];
    //     let p2 = points[(i + 1) % points.len()];
    //     make_line(BLUE, p1, p2);
    // }

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
