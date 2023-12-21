use nannou::{prelude::*, winit::event};
use picea::{
    element::ElementBuilder,
    math::{edge::Edge, point::Point, vector::Vector},
    meta::MetaBuilder,
    scene::Scene,
    shape::{polygon::Square, CenterPoint, GeometryTransform},
    tools::{
        collision_view::CollisionStatusViewer, snapshot::create_element_construct_code_snapshot,
    },
};
use std::time::SystemTime;

struct Model {
    scene: Scene,
    timer: SystemTime,
    collision_viewer: CollisionStatusViewer,
    is_paused: bool,
}

fn create_model(_app: &App) -> Model {
    let mut scene = Scene::new();

    scene.set_gravity(|_| (0., -40.).into());

    // scene.set_gravity(|_| Vector::new(0., 0.));

    let mut shape = Square::new(20., -10., 10.);

    let shape_a = shape.clone();

    shape.translate(&(15., 0.).into());

    let shape_b = shape.clone();
    // shape.rotate(&(20., -10.).into(), 3. * PI() / 4.);

    let element_a_id = (&mut scene) << ElementBuilder::new(shape_a, MetaBuilder::new(1.));

    scene.create_point_constraint(element_a_id, (20., 0.).into(), (20., 0.).into());

    let element_b_id = (&mut scene) << ElementBuilder::new(shape_b, MetaBuilder::new(1.));

    scene.create_join(element_a_id, (30., -10.), element_b_id, (30. + 5., -10.));

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

    make_ellipse(RED, (20., 0.).into(), 0.5);

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
        //     YELLOWGREEN,
        //     element.center_point(),
        //     element.center_point() + element.meta().velocity() * 10.,
        // );

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

    model
        .scene
        .join_points()
        .into_iter()
        .for_each(|(point_a, point_b)| make_line(RED, point_a, point_b));

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
