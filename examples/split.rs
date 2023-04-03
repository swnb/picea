use std::collections::VecDeque;

use picea::{
    math::{point::Point, vector::Vector, FloatNum},
    shape::utils::split_concave_polygon_to_convex_polygons,
};

use nannou::{prelude::*, winit::event};

fn rotate(vertexes: &[(i32, i32)], i: usize) -> VecDeque<Point> {
    let mut vertexes = vertexes
        .iter()
        .map(|&(x, y)| (x as FloatNum, y as FloatNum))
        .map(|v| v.into())
        .collect::<VecDeque<Point>>();

    for _ in 0..i {
        let last = vertexes.pop_back().unwrap();
        vertexes.push_front(last);
    }

    vertexes
}

struct Model {
    count: usize,
}

fn create_model(_app: &App) -> Model {
    Model { count: 0 }
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
                model.count += 1;
            }
            _ => {}
        },
        _ => {}
    }
}

fn view(app: &App, model: &Model, frame: Frame) {
    frame.clear(BLACK);

    let draw = app.draw();

    let vertexes = vec![(-1, 1), (0, 0), (1, 1), (1, -1), (-1, -1)];

    dbg!(model.count);

    let i = model.count;
    let vertexes = rotate(&vertexes, i);

    let v = Vector::from((0., 0.));

    for p in &vertexes {
        let mut p = (p.to_vector() * 10.).to_point();
        p += v * i as FloatNum;
        draw.ellipse()
            .x_y(p.x() * 10., p.y() * 10.)
            .color(WHITE)
            .radius(10.);
    }

    let [a, b] = split_clockwise_concave_polygon_once(&Vec::from(vertexes)[..]).unwrap();

    for mut p in a {
        p = (p.to_vector() * 10.).to_point();
        p += v * i as FloatNum;

        draw.ellipse()
            .x_y(p.x() * 10., p.y() * 10.)
            .color(BLUE)
            .radius(10.);
    }

    for mut p in b {
        p = (p.to_vector() * 10.).to_point();
        p += v * i as FloatNum;
        draw.ellipse()
            .x_y(p.x() * 10., p.y() * 10.)
            .color(YELLOWGREEN)
            .radius(10.);
    }

    draw.to_frame(app, &frame).unwrap();
}

fn view2(app: &App, model: &Model, frame: Frame) {
    frame.clear(BLACK);

    let draw = app.draw();

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

    dbg!(model.count);

    // let i = model.count;
    let vertexes = rotate(&vertexes, 0);

    let v = Vector::from((0., 0.));

    for p in &vertexes {
        let p = (p.to_vector() * 10.).to_point();

        draw.ellipse().x_y(p.x(), p.y()).color(WHITE).radius(3.);
    }

    let vertexes_len = vertexes.len();
    for i in 0..vertexes_len {
        let a = vertexes[i];
        let b = vertexes[(i + 1) % vertexes_len];

        draw.line()
            .start(vec2(a.x() * 10., a.y() * 10.))
            .end(vec2(b.x() * 10., b.y() * 10.))
            .color(WHITE);
        // draw.ellipse().x_y(p.x(), p.y()).color(WHITE).radius(3.);
    }

    let polygons = split_concave_polygon_to_convex_polygons(&Vec::from(vertexes)[..]);
    let polygons_len = polygons.len();

    // dbg!(polygons);
    // std::process::exit(-1);
    let polygon = &polygons[model.count % polygons_len];

    for p in polygon {
        draw.ellipse()
            .x_y(p.x() * 10., p.y() * 10.)
            .color(BLUE)
            .radius(10.);
    }

    // let [a, b] = split_clockwise_concave_polygon_once(&Vec::from(vertexes)[..]).unwrap();

    // for mut p in b {
    //     p = (p.to_vector() * 10.).to_point();
    //     p += v * i as FloatNum;
    //     draw.ellipse()
    //         .x_y(p.x() * 10., p.y() * 10.)
    //         .color(YELLOWGREEN)
    //         .radius(10.);
    // }

    draw.to_frame(app, &frame).unwrap();
}

fn main() {
    // let vertexes = vec![(-1, 1), (0, 0), (1, 1), (1, -1), (-1, -1)];

    // dbg!(model.count);

    // let vertexes = rotate(&vertexes, 3);

    nannou::app(create_model)
        .event(event)
        .simple_window(view2)
        .run();
}
