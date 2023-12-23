use nannou::{prelude::*, winit::event};
use picea::{
    element::ElementBuilder,
    math::{edge::Edge, point::Point, vector::Vector},
    meta::MetaBuilder,
    scene::{self, Scene},
    shape::{circle::Circle, polygon::Square, CenterPoint, GeometryTransform},
    tools::snapshot::create_element_construct_code_snapshot,
};
use std::time::SystemTime;

struct Model {
    scene: Scene,
    timer: SystemTime,
    is_paused: bool,
    mouse_enable: bool,
    mouse_pos: Point,
    mouse_constraint_id: Option<u32>,
}

fn create_model(_app: &App) -> Model {
    let mut scene = Scene::new();

    scene.set_gravity(|_| (0., 0.).into());

    scene
        .get_context_mut()
        .constraint_parameters
        .skip_friction_constraints = true;

    scene.get_context_mut().constraint_parameters.factor_elastic = 1.0;

    let boxes: Vec<Square> = vec![];

    let start_x = -30.;
    for i in 0..10 {
        let shape = Circle::new((start_x + (i as f32 * 10.) + 5., -20. + 5.), 5.);
        let shape_clone = shape.clone();
        let element_id = (&mut scene) << ElementBuilder::new(shape, MetaBuilder::new(2.));
        let p = shape_clone.center_point() + Vector::from((0., 40.));
        scene.create_point_constraint(element_id, p, p);
    }

    let shape = Circle::new((-50. + 5., -20. + 5.), 5.);

    (&mut scene)
        << ElementBuilder::new(
            shape,
            MetaBuilder::new(2.)
                .is_transparent(false)
                .velocity((20., 0.)),
        );

    Model {
        scene,
        timer: SystemTime::now(),
        is_paused: false,
        mouse_enable: false,
        mouse_pos: Default::default(),
        mouse_constraint_id: None,
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
                let code = create_element_construct_code_snapshot(element);
            }),
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
                model.mouse_enable = true;
            }
            WindowEvent::MouseReleased(_) => {
                model.mouse_enable = false;
                if let Some(mouse_constraint_id) = model.mouse_constraint_id {
                    model.scene.remove_point_constraint(mouse_constraint_id);
                }

                model.mouse_constraint_id = None;
            }
            WindowEvent::MouseMoved(p) => {
                if !model.mouse_enable {
                    return;
                }
                let x = p.x / 10.;
                let y = p.y / 10.;
                model.mouse_pos = (x, y).into();

                if let Some(mouse_constraint_id) = model.mouse_constraint_id {
                    if let Some(mouse_constraint) =
                        model.scene.get_point_constraint_mut(mouse_constraint_id)
                    {
                        *mouse_constraint.fixed_point_mut() = model.mouse_pos;
                    }
                } else {
                    model.mouse_constraint_id = model.scene.create_point_constraint(
                        1,
                        model.scene.get_element(1).unwrap().center_point(),
                        model.mouse_pos,
                    );
                }
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

    model.scene.point_constraint().for_each(|constraint| {
        if let Some(element) = model.scene.get_element(constraint.element_id()) {
            make_line(RED, element.center_point(), *constraint.fixed_point())
        }
    });

    model.scene.elements_iter().for_each(|element| {
        make_ellipse(BLUE, element.center_point(), 0.5);

        make_line(
            ORANGERED,
            element.center_point(),
            element.center_point() + element.meta().velocity() * 10.,
        );

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

    draw.to_frame(app, &frame).unwrap();
}

fn main() {
    nannou::app(create_model)
        .event(event)
        .simple_window(view)
        .run();
}
