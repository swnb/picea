use nannou::prelude::*;

use picea::{
    element::{Element, ElementShape},
    math::{
        point::Point,
        vector::{Vector, Vector3},
    },
    meta::{
        force::{Force, ForceGroup},
        MetaBuilder,
    },
    scene::Scene,
    shape::rect::RectShape,
};

use rand::prelude::*;
use std::time;

const G: f32 = 9.8;

struct Model {
    scene: Scene,
    time: time::Instant,
    gravity_force: Force,
    data: Vec<Point<f32>>,
    stop: bool,
}

fn main() {
    nannou::app(create_model)
        .event(event)
        .simple_window(view)
        .run();
}

fn create_model(app: &App) -> Model {
    create_model3(app)
}

fn create_model1(_app: &App) -> Model {
    let mut rng = rand::thread_rng();
    let mut scene = Scene::new();

    let gravity_force = Force::new("gravity", (0., -G));

    for i in 0..70 {
        let gravity_force = gravity_force.clone();

        let speed_x: f32 = rng.gen_range(-20.0..20.0);
        let speed_y: f32 = rng.gen_range(-20.0..20.0);

        let mut force_group = ForceGroup::new();

        force_group.add_force(Force::new("air", (0., 0.)));

        let shape = ElementShape::Rect(
            (
                (
                    -350. + ((i % 20) as f32 * 40.),
                    100. - ((i / 20) * 50) as f32,
                ),
                (20., 20.),
            )
                .into(),
        );

        let meta = MetaBuilder::new(1.)
            .force("air", (0., 0.))
            .angular_velocity(std::f32::consts::PI / 10.)
            .velocity((speed_x, speed_y));

        let element = Element::new(shape, meta);

        scene.push_element(element);
    }

    Model {
        scene,
        time: time::Instant::now(),
        gravity_force,
        data: vec![],
        stop: false,
    }
}

fn create_model3(_app: &App) -> Model {
    let mut rng = rand::thread_rng();
    let mut scene = Scene::new();

    let shape = ElementShape::Rect(((-200., -300.), (850., 150.)).into());

    let meta = MetaBuilder::new(1.)
        .angular(-std::f32::consts::FRAC_PI_8 / 2.)
        .is_fixed(true);

    let element = Element::new(shape, meta);

    scene.push_element(element);

    let shape = ElementShape::Rect(((200., -100.), (-100., 100.)).into());

    let meta = MetaBuilder::new(10.)
        // .angular_velocity(std::f32::consts::PI / 10.)
        // FIXME;
        .angular(-std::f32::consts::FRAC_PI_8)
        // .velocity((rng.gen_range(-50.0..-20.0), 0.))
        .force("gravity", (0.0, -G * 300.));

    let element = Element::new(shape, meta);

    scene.push_element(element);

    Model {
        scene,
        time: time::Instant::now(),
        gravity_force: Force::new("", (0., 0.)),
        stop: false,
        data: vec![],
    }
}

fn create_model4(_app: &App) -> Model {
    let mut rng = rand::thread_rng();
    let mut scene = Scene::new();

    let gravity_force = Force::new("gravity", (0., -G));

    // force_group.add_force(Force::new("air", (0., 0.).into()));
    let p1 = (-31.720043, 79.35677);
    let p2 = (-26.027605, 98.52961);
    let p3 = (-45.200382, 104.22198);
    let p4 = (-50.892815, 85.04917);
    let shape = ElementShape::Rect(RectShape::new([p1, p2, p3, p4]));

    let meta = MetaBuilder::new(1.)
        .angular_velocity(-std::f32::consts::FRAC_PI_2)
        .velocity((20.0, 0.));

    let element = Element::new(shape, meta);
    scene.push_element(element);

    let p1 = (-37.031464, 56.207893);
    let p2 = (-17.36696, 59.855812);
    let p3 = (-21.014877, 79.520325);
    let p4 = (-40.679382, 75.8724);

    let shape = ElementShape::Rect(RectShape::new([p1, p2, p3, p4]));

    let meta = MetaBuilder::new(1.)
        .angular_velocity(std::f32::consts::PI / 10.)
        // FIXME;
        .angular(std::f32::consts::FRAC_PI_8)
        .velocity((rng.gen_range(-20.0..0.0), 0.));

    let element = Element::new(shape, meta);

    scene.push_element(element);

    Model {
        scene,
        time: time::Instant::now(),
        gravity_force,
        stop: false,
        data: vec![],
    }
}

fn event(app: &App, model: &mut Model, event: Event) {
    match event {
        Event::WindowEvent {
            simple: Some(WindowEvent::KeyPressed(Key::R)),
            ..
        } => {
            *model = create_model(app);
        }
        Event::WindowEvent {
            simple: Some(WindowEvent::KeyPressed(Key::Space)),
            ..
        } => model.stop = !model.stop,
        Event::Update(update) => {
            if !model.stop {
                let duration = update.since_last;

                model.data.clear();
                model
                    .scene
                    .update_elements_by_duration(duration.as_secs_f32(), |data| {
                        model.data.extend(data);
                    });
            }
        }
        _ => {}
    }
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

    fn p2pt2(p: impl Into<Point<f32>>) -> Point2 {
        Some(p)
            .map(|v| v.into())
            .map(|v| v.into())
            .map(|(x, y)| pt2(x, y))
            .unwrap()
    }

    model.scene.elements_iter().for_each(|e| render1(e, &draw));

    model.data.iter().for_each(|p| {
        draw.ellipse().color(RED).radius(3.).x_y(p.x(), p.y());
    });

    // return;
    // let a = &elements[0];
    // let b = &elements[1];

    // let shape_a = a.shape();
    // let shape_b = b.shape();
    // let center_point_a = shape_a.compute_center_point();
    // let center_point_b = shape_b.compute_center_point();

    // let compute_support_point = move |reference_vector: Vector<f32>| {
    //     let (_, max_point_a) = shape_a.projection(reference_vector);
    //     let (_, max_point_b) = shape_b.projection(-reference_vector);
    //     Vector::<f32>::from((max_point_b, max_point_a))
    // };

    // let approximation_vector: Vector<f32> = (center_point_a, center_point_b).into();

    // let mut a: Vector<f32> = compute_support_point(approximation_vector);

    // let approximation_vector = -a;
    // let mut b = compute_support_point(approximation_vector);

    // fn is_wrong_support_point(point: Vector<f32>, reference_vector: Vector<f32>) -> bool {
    //     // FIXME this is wrong? <= 0
    //     (point * reference_vector) < 0.
    // }

    // if is_wrong_support_point(b, approximation_vector) {
    //     return;
    // }

    // fn compute_third_reference_vector(a: Vector<f32>, b: Vector<f32>) -> Vector<f32> {
    //     let base_vector: Vector<f32> = a - b;
    //     let base_vector: Vector3<f32> = base_vector.into();
    //     let b = (-b).into();
    //     (base_vector ^ b ^ base_vector).into()
    // }

    // let approximation_vector = compute_third_reference_vector(a, b);

    // let mut c = compute_support_point(approximation_vector);

    // if is_wrong_support_point(c, approximation_vector) || c == a || c == b {
    //     return;
    // }

    // let mut approximation_vector = Vector::new(0., 0.);
    // let mut is_break = false;

    // loop {
    //     let mut is_origin_inside_triangle = || -> bool {
    //         let ca: Vector3<_> = (a - c).into();
    //         let cb: Vector3<_> = (b - c).into();
    //         let cb_normal = (cb ^ (cb ^ ca)).into();

    //         if -c * cb_normal > 0. {
    //             // refactor
    //             approximation_vector = cb_normal;

    //             let tmp = compute_support_point(approximation_vector);

    //             draw.arrow()
    //                 .color(RED)
    //                 .points(p2pt2((0., 0.)), p2pt2((cb_normal.x(), cb_normal.y())));

    //             draw.arrow()
    //                 .color(RED)
    //                 .points(p2pt2((0., 0.)), p2pt2((tmp.x(), tmp.y())));

    //             if is_wrong_support_point(tmp, approximation_vector) || tmp == c || tmp == b {
    //                 is_break = true;
    //                 return false;
    //             }

    //             dbg!(tmp * approximation_vector);

    //             a = c;
    //             c = tmp;
    //             return false;
    //         }

    //         let ca_normal: Vector<f32> = (cb ^ ca ^ ca).into();

    //         if -c * ca_normal > 0. {
    //             approximation_vector = ca_normal;

    //             let tmp = compute_support_point(approximation_vector);

    //             if is_wrong_support_point(tmp, approximation_vector) || tmp == c || tmp == a {
    //                 // TODO refactor this code
    //                 is_break = true;
    //                 return false;
    //             }

    //             b = c;
    //             c = tmp;

    //             return false;
    //         }

    //         draw.arrow()
    //             .color(BLUE)
    //             .points(p2pt2((0., 0.)), p2pt2((ca_normal.x(), ca_normal.y())));

    //         true
    //     };

    //     if is_origin_inside_triangle() {
    //         break;
    //     }
    //     if is_break {
    //         return;
    //     }
    // }

    // draw.arrow()
    //     .color(BLACK)
    //     .points(p2pt2((c.x(), c.y())), p2pt2((a.x(), a.y())));

    // draw.arrow()
    //     .color(ORANGE)
    //     .points(p2pt2((a.x(), a.y())), p2pt2((b.x(), b.y())));

    // draw.arrow()
    //     .color(RED)
    //     .points(p2pt2((b.x(), b.y())), p2pt2((c.x(), c.y())));

    // let triangle = [a, b, c];

    // fn compute_edge_info_award_from_edge(a: Vector<f32>, b: Vector<f32>) -> (f32, Vector<f32>) {
    //     let ab = (b - a).into();
    //     let ao: Vector3<_> = (-a).into();
    //     let mut normal: Vector<_> = (ao ^ ab ^ ab).into();
    //     normal = normal.normalize();
    //     let depth = a * normal;
    //     (depth, normal)
    // }

    // let (d, v) = compute_edge_info_award_from_edge(a, b);
    // let p = compute_support_point(v);

    // draw.arrow()
    //     .color(YELLOWGREEN)
    //     .points(p2pt2((0., 0.)), p2pt2((v.x() * 100., v.y() * 100.)));

    // draw.arrow()
    //     .color(YELLOWGREEN)
    //     .points(p2pt2((0., 0.)), p2pt2((p.x(), p.y())));

    // put everything on the frame
    draw.to_frame(app, &frame).unwrap();
}

fn render1(element: &Element, draw: &Draw) {
    fn p2pt2(p: impl Into<Point<f32>>) -> Point2 {
        Some(p)
            .map(|v| v.into())
            .map(|v| v.into())
            .map(|(x, y)| pt2(x, y))
            .unwrap()
    }

    use ElementShape::*;
    match element.shape() {
        Rect(shape) => {
            let points: Vec<_> = shape.corner_iter().map(|&v| p2pt2(v)).collect();

            draw.quad()
                .color(GREY)
                .points(points[0], points[1], points[2], points[3]);

            shape.segment_iter().for_each(|segment| {
                let start_point = *segment.get_start_point();
                let end_point = *segment.get_end_point();
                draw.line()
                    .color(ORANGE)
                    .points(p2pt2(start_point), p2pt2(end_point));
            });

            shape.edge_iter().for_each(|edge| {
                let start_point = Point::from((0., 0.));
                let end_point = start_point + (-!edge * 100.);

                // draw.arrow()
                // .color(RED)
                // .points(p2pt2(start_point), p2pt2(end_point));

                let end_point = start_point + (!edge * 100.);

                // draw.arrow()
                // .color(RED)
                // .points(p2pt2(start_point), p2pt2(end_point));

                let reference_v: Vector<_> = !edge;

                // shape.corner_iter().for_each(|&corner| {
                //     let size = corner >> reference_v;
                //     let rate = size / reference_v.abs();
                //     draw.arrow().color(RED).points(
                //         p2pt2(corner),
                //         p2pt2(Point::<f32>::from((
                //             (reference_v * rate).x(),
                //             (reference_v * rate).y(),
                //         ))),
                //     );
                // });
            });
        }
        Circle(shape) => {}
    }
}
