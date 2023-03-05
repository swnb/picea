use nannou::prelude::*;
use picea::math::{point::Point, vector::Vector};

struct Model {}

fn main() {
    nannou::app(create_model)
        .event(event)
        .simple_window(view)
        .run();
}

fn create_model(app: &App) -> Model {
    Model {}
}
fn event(app: &App, model: &mut Model, event: Event) {
    // match event {
    //     Event::WindowEvent {
    //         simple: Some(WindowEvent::KeyPressed(Key::R)),
    //         ..
    //     } => {
    //         *model = create_model(app);
    //     }
    //     Event::WindowEvent {
    //         simple: Some(WindowEvent::KeyPressed(Key::Space)),
    //         ..
    //     } => model.stop = !model.stop,
    //     Event::Update(update) => {
    //         if !model.stop {
    //             let duration = update.since_last;

    //             model.scene.update_elements_by_duration(duration * 5);
    //         }
    //     }
    //     _ => {}
    // }
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

    let drawABC = |a: (f32, f32), b: (f32, f32), c: (f32, f32)| {
        // draw cross
        draw.arrow()
            .color(BLACK)
            .points((a.0, a.1).into(), (b.0, b.1).into());

        draw.arrow()
            .color(BLUE)
            .points((b.0, b.1).into(), (c.0, c.1).into());

        draw.arrow()
            .color(RED)
            .points((c.0, c.1).into(), (a.0, a.1).into());
    };

    let drawRect = |list: Vec<((f32, f32), (f32, f32))>| {
        for v in list {
            draw.arrow().color(YELLOWGREEN).points(
                (v.0 .0 * 10., v.0 .1 * 10.).into(),
                (v.1 .0 * 10., v.1 .1 * 10.).into(),
            );
        }
    };

    let draw_poly = |list: Vec<(f32, f32)>| {
        for i in 0..list.len() {
            let j = (i + 1) % list.len();
            draw.arrow()
                .color(YELLOWGREEN)
                .points((list[i].0, list[i].1).into(), (list[j].0, list[j].1).into());
        }
    };

    // let simplex = [
    //     (0.3048706, 0.9535732),
    //     (-26.077576, -49.08609),
    //     (12.133606, -37.257442),
    // ];

    // let a = [
    //     ((0.3048706, 0.9535732), (-26.077576, -49.08609)),
    //     ((-26.077576, -49.08609), (12.133606, -37.257442)),
    //     ((12.133606, -37.257442), (0.3048706, 0.9535732)),
    // ];

    // let b = [
    //     ((0.3048706, 0.9535732), (-37.90631, -10.875082)),
    //     ((-37.90631, -10.875082), (-26.077576, -49.08609)),
    //     ((-26.077576, -49.08609), (12.133606, -37.257442)),
    //     ((12.133606, -37.257442), (0.3048706, 0.9535732)),
    // ];

    let simplex = [
        (53.457916, -99.3826),
        (105.221756, 267.00763),
        (-67.25275, 109.69443),
    ];

    let c = [
        (53.457916, -99.38263),
        (105.221756, 267.00763),
        (-67.25275, 109.69443),
        (3.4579163, -12.78009),
        (-17.252747, 23.091888),
        (3.4579163, -12.78009),
        (-67.25275, 109.69443),
        (3.4579163, -12.78009),
    ];

    draw_poly(c.to_vec());

    // drawABC(simplex[0], simplex[1], simplex[2]);

    // drawRect(b.to_vec());
    // drawRect(c.to_vec());
    // drawRect(d.to_vec());

    // drawABC(
    //     (-10.093773, 12.805595),
    //     (-44.831787, -5.198963),
    //     (-7.5455246, -34.62496),
    // );

    // drawABC(
    //     (-10.093773 * 10., 12.805595 * 10.),
    //     (7.5455246 * 10., -34.62496 * 10.),
    //     (7.910759 * 10., -21.93235 * 10.),
    // );

    // drawABC(
    //     (7.910759 * 10., -21.93235 * 10.),
    //     (-10.093773 * 10., 12.805595 * 10.),
    //     (2.598816 * 10., -2.6506805 * 10.),
    // );

    fn p2pt2(p: impl Into<Point<f32>>) -> Point2 {
        Some(p)
            .map(|v| v.into())
            .map(|v| v.into())
            .map(|(x, y)| pt2(x, y))
            .unwrap()
    }

    draw.to_frame(app, &frame).unwrap();
}
