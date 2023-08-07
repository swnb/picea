use nannou::{prelude::*, winit::event};
use picea::{
    element::ElementBuilder,
    math::{edge::Edge, point::Point, FloatNum},
    meta::MetaBuilder,
    scene::Scene,
    shape::{concave::ConcavePolygon, polygon::RegularPolygon},
    tools::{
        collision_view::CollisionStatusViewer, snapshot::create_element_construct_code_snapshot,
    },
};
use rand::Rng;
use std::{collections::VecDeque, time::SystemTime};

struct Model {
    scene: Scene,
    timer: SystemTime,
    collision_viewer: CollisionStatusViewer,
    is_paused: bool,
}

fn create_model(_app: &App) -> Model {
    let mut scene = Scene::new();

    // let ground_bottom = Line::new((-200., -25.), (200., -25.));

    // &mut scene << ElementBuilder::new(ground_bottom, MetaBuilder::new(1.).is_fixed(true));

    let top = 1;
    let down = -16;

    let mut x = -75;
    let mut y: i32 = down;

    let mut vertexes = Vec::new();
    for i in 0..17 {
        vertexes.push((x, y));
        x += 5;
        if i % 2 == 0 {
            y += 5;
        } else {
            y -= 2;
        }
    }

    for i in 0..17 {
        vertexes.push((x, y));
        x += 5;
        if i % 2 == 0 {
            y -= 5;
        } else {
            y += 2;
        }
    }

    vertexes.push((155, -20));
    vertexes.push((-155, -20));

    // let vertexes = vec![
    //     (-55, top),
    //     (-50, down),
    //     (-45, top + 1),
    //     (-40, down),
    //     (-35, top + 2),
    //     (-30, down),
    //     (-25, top + 3),
    //     (-20, down),
    //     (-15, top + 4),
    //     (-10, down),
    //     (0, top + 5),
    //     (10, down),
    //     (15, top + 4),
    //     (20, down),
    //     (25, top + 3),
    //     (30, down),
    //     (35, top + 2),
    //     (40, down),
    //     (45, top + 1),
    //     (50, down),
    //     (55, top),
    //     (55, -20),
    //     (-55, -20),
    // ];

    let vertexes = vertexes
        .iter()
        .map(|&(x, y)| (x as FloatNum, y as FloatNum - 30.))
        .map(|v| v.into())
        .collect::<VecDeque<Point>>();

    let concave_polygon = ConcavePolygon::new(&Vec::from(vertexes)[..]);

    let element = ElementBuilder::new(concave_polygon, MetaBuilder::new(200.).is_fixed(true));

    &mut scene << element;

    let mut gen = rand::thread_rng();

    for i in 0..70 {
        for j in 0..17 {
            let x = -40. + j as FloatNum * 5.;
            let y = 10. + i as FloatNum * 5.;

            let value: u8 = gen.gen();

            let mut edge = value % 10;
            if edge < 3 {
                edge = 3;
            }

            let shape = RegularPolygon::new((x, y), edge as usize, 2.);

            &mut scene << ElementBuilder::new(shape, MetaBuilder::new(10.));
        }
    }

    Model {
        scene,
        timer: SystemTime::now(),
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
    frame.clear(WHITESMOKE);

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
        // element
        //     .shape()
        //     .edge_iter()
        //     .take(1)
        //     .for_each(|edge| match edge {
        //         Edge::Line {
        //             start_point,
        //             end_point,
        //         } => {
        //             make_line(WHITE, element.center_point(), *start_point);
        //         }
        //         Edge::Circle {
        //             center_point,
        //             radius,
        //         } => {}
        //         _ => unimplemented!(),
        //     });

        // make_line(
        // YELLOWGREEN,
        // element.center_point(),
        // element.center_point() + element.meta().velocity() * 10.,
        // );

        // make_ellipse(BLUE, element.center_point(), 0.5);

        let color = match element.id() % 3 {
            0 => ORANGE,
            1 => GREEN,
            2 => SKYBLUE,
            _ => unreachable!(),
        };

        element.shape().edge_iter().for_each(|edge| match edge {
            Edge::Line {
                start_point,
                end_point,
            } => make_line(color, *start_point, *end_point),
            Edge::Circle {
                center_point,
                radius,
            } => {
                make_ellipse(color, center_point, radius);
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
