use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};
use std::time::{self, UNIX_EPOCH};

use macro_tools::Builder;
use picea::constraints::JoinConstraintConfigBuilder;
use picea::math::edge::Edge;
use picea::math::point::Point;
use picea::math::vector::Vector;
use picea::math::FloatNum;
use picea::scene::Scene;
use picea::shape::utils::{is_point_inside_shape, rotate_point};
use picea::tools::collision_view::CollisionStatusViewer;
use picea::tools::snapshot;
use serde::Serialize;
use speedy2d::color::Color;
use speedy2d::dimen::Vector2;
use speedy2d::window::{MouseScrollDistance, VirtualKeyCode, WindowHandler, WindowHelper};
use speedy2d::Graphics2D;

#[derive(Builder)]
pub struct Config {
    #[default = 10.0]
    scale: FloatNum,
    draw_velocity: bool,
    is_default_paused: bool,
    #[default = true]
    draw_center_point: bool,
    draw_join_constraints: bool,
    draw_point_constraints: bool,
    enable_mouse_constraint: bool,
    draw_contact_point_pair: bool,
    frame_by_frame: bool,
}

type UpdateFn<T> = dyn FnMut(&mut Scene<T>, Option<u32>, &mut Handler<T>);

type InitFn<T> = dyn FnMut(&mut Scene<T>, &mut Handler<T>);

type GetRecordFn<T, D> = dyn Fn(&Scene<T>) -> D;

#[derive(Serialize)]
struct Record<T> {
    value: T,
    timestamp: u128,
}

#[derive(Serialize, Default)]
struct Records(BTreeMap<String, Vec<Record<FloatNum>>>);

impl Deref for Records {
    type Target = BTreeMap<String, Vec<Record<FloatNum>>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Records {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub struct Handler<T = ()>
where
    T: Clone + Default,
{
    scene: Scene<T>,
    init: Box<InitFn<T>>,
    update: Box<UpdateFn<T>>,
    is_paused: bool,
    is_mouse_down: bool,
    current_mouse_pos: Option<Point>,
    selected_element_id: Option<u32>,
    mouse_constraint_id: Option<u32>,
    config: Config,
    contact_viewer: CollisionStatusViewer,
    render_offset: Vector,
    record_handler: Vec<(String, Box<GetRecordFn<T, FloatNum>>)>,
    records: Records,
    pub is_debug: bool,
    pub iter_count: usize,
}

fn into_vector2(p: Point) -> Vector2<FloatNum> {
    Vector2::new(p.x(), p.y())
}

struct DrawHelper<'a> {
    graphics: &'a mut Graphics2D,
    scale: FloatNum,
    render_offset: Vector,
}

impl<'a> DrawHelper<'a> {
    fn draw_line(&mut self, start_point: &Point, end_point: &Point, color: Color) {
        self.graphics.draw_line(
            into_vector2(((start_point.to_vector() + self.render_offset) * self.scale).to_point()),
            into_vector2(((end_point.to_vector() + self.render_offset) * self.scale).to_point()),
            3.0,
            color,
        )
    }

    fn draw_circle(&mut self, center_point: &Point, radius: FloatNum, color: Color) {
        self.graphics.draw_circle(
            into_vector2(((center_point.to_vector() + self.render_offset) * self.scale).to_point()),
            radius * self.scale,
            color,
        );
    }
}

impl<T> Handler<T>
where
    T: Default + Clone,
{
    fn solve_mouse_constraint(&mut self) {
        if !self.config.enable_mouse_constraint {
            return;
        }

        if let Some((element_id, current_mouse_pos)) =
            self.selected_element_id.zip(self.current_mouse_pos)
        {
            self.mouse_constraint_id = self.mouse_constraint_id.or_else(|| {
                self.scene.create_point_constraint(
                    element_id,
                    current_mouse_pos,
                    current_mouse_pos,
                    JoinConstraintConfigBuilder::default()
                        .frequency(1.0)
                        .damping_ratio(1.0)
                        .hard(false),
                )
            });

            let point_constraint = self
                .mouse_constraint_id
                .and_then(|constraint_id| self.scene.get_point_constraint_mut(constraint_id));

            if let Some(point_constraint) = point_constraint {
                *point_constraint.fixed_point_mut() = current_mouse_pos
            };
        }
    }
}

impl<T> WindowHandler for Handler<T>
where
    T: Default + Clone,
{
    fn on_start(
        &mut self,
        helper: &mut WindowHelper<()>,
        info: speedy2d::window::WindowStartupInfo,
    ) {
        let self_ptr = self as *mut _;

        unsafe {
            (self.init)(&mut self.scene, &mut *self_ptr);
        }
    }

    fn on_key_down(
        &mut self,
        helper: &mut WindowHelper<()>,
        virtual_key_code: Option<speedy2d::window::VirtualKeyCode>,
        scancode: speedy2d::window::KeyScancode,
    ) {
        let self_ptr = self as *mut _;

        if let Some(key) = virtual_key_code {
            match key {
                VirtualKeyCode::R => {
                    self.scene.clear();
                    unsafe {
                        (self.init)(&mut self.scene, &mut *self_ptr);
                    }
                }
                VirtualKeyCode::Space => {
                    self.is_paused = !self.is_paused;
                }
                VirtualKeyCode::F => {
                    self.config.frame_by_frame = !self.config.frame_by_frame;
                }
                VirtualKeyCode::S => {
                    if let Some(code) = self
                        .scene
                        .get_element(5)
                        .map(|element| snapshot::create_element_construct_code_snapshot(element))
                    {
                        println!("{}", code);
                    }
                }
                VirtualKeyCode::C => {
                    self.scene.silent();
                }
                VirtualKeyCode::D => {
                    self.is_debug = !self.is_debug;
                    self.iter_count = 0;
                }

                VirtualKeyCode::E => {
                    self.scene
                        .context_mut()
                        .constraint_parameters
                        .skip_friction_constraints = !self
                        .scene
                        .context_mut()
                        .constraint_parameters
                        .skip_friction_constraints
                }
                _ => {}
            }
        }
    }

    fn on_mouse_button_down(
        &mut self,
        helper: &mut WindowHelper<()>,
        button: speedy2d::window::MouseButton,
    ) {
        self.is_mouse_down = true;
    }

    fn on_mouse_button_up(
        &mut self,
        helper: &mut WindowHelper<()>,
        button: speedy2d::window::MouseButton,
    ) {
        self.is_mouse_down = false;
        self.current_mouse_pos = None;
        self.selected_element_id = None;
        self.mouse_constraint_id
            .take()
            .map(|constraint_id| self.scene.remove_point_constraint(constraint_id));
    }

    fn on_mouse_wheel_scroll(
        &mut self,
        helper: &mut WindowHelper<()>,
        distance: speedy2d::window::MouseScrollDistance,
    ) {
        if let MouseScrollDistance::Pixels { y, .. } = distance {
            self.config.scale += y as FloatNum * 0.1;
        }
    }

    fn on_mouse_move(&mut self, helper: &mut WindowHelper<()>, position: speedy2d::dimen::Vec2) {
        if !self.is_mouse_down {
            return;
        }

        let new_mouse_pos = Point::new(
            position.x / self.config.scale,
            position.y / self.config.scale,
        );

        if let Some(current_mouse_pos) = self.current_mouse_pos {
            if self.selected_element_id.is_none() {
                self.render_offset += new_mouse_pos - current_mouse_pos;
            }
        } else {
            let element_id = self
                .scene
                .elements_iter()
                .find(|element| {
                    is_point_inside_shape(
                        new_mouse_pos - self.render_offset,
                        &mut element.shape().edge_iter(),
                    )
                })
                .map(|element| element.id());

            self.selected_element_id = element_id;

            if let Some(element_id) = element_id {
                println!("selected id {}", element_id);
            }
        }

        self.current_mouse_pos = Some(new_mouse_pos);

        self.solve_mouse_constraint()
    }

    fn on_user_event(&mut self, helper: &mut WindowHelper<()>, user_event: ()) {}

    fn on_draw(&mut self, helper: &mut WindowHelper, graphics: &mut Graphics2D) {
        let self_ptr = self as *mut _;

        if !self.is_paused {
            unsafe {
                (self.update)(&mut self.scene, self.selected_element_id, &mut *self_ptr);
            }

            let timestamp = time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis();

            self.record_handler
                .iter()
                .for_each(|(name, get_record_data)| {
                    let value = get_record_data(&self.scene);
                    if let Some(record) = self.records.get_mut(name) {
                        record.push(Record { value, timestamp });
                    }
                });
        }

        if self.config.frame_by_frame {
            self.is_paused = true;
        }

        self.contact_viewer.on_update(&self.scene);

        graphics.clear_screen(Color::from_gray(0.8));

        let mut draw_helper = DrawHelper {
            graphics,
            scale: self.config.scale,
            render_offset: self.render_offset,
        };

        for i in 0..100 {
            draw_helper.draw_line(
                &((i as f32) * 10., 0.).into(),
                &((i as f32) * 10., 1000.).into(),
                Color::GRAY,
            )
        }

        for i in 0..100 {
            draw_helper.draw_line(
                &(0., ((i as f32) * 10.)).into(),
                &(1000., (i as f32) * 10.).into(),
                Color::GRAY,
            )
        }

        self.scene.elements_iter().for_each(|element| {
            element.shape().edge_iter().for_each(|edge| match edge {
                Edge::Line {
                    start_point,
                    end_point,
                } => draw_helper.draw_line(start_point, end_point, Color::WHITE),
                Edge::Circle {
                    center_point,
                    radius,
                } => {
                    draw_helper.draw_circle(&center_point, radius, Color::WHITE);
                }
                _ => unimplemented!(),
            });
        });

        if self.config.draw_center_point {
            self.scene.elements_iter().for_each(|element| {
                draw_helper.draw_circle(&element.center_point(), 1., Color::BLUE)
            });
        }

        self.scene.elements_iter().for_each(|element| {
            let angle = element.meta().total_transform().rotation();
            for edge in element.shape().edge_iter() {
                match edge {
                    Edge::Arc {
                        start_point,
                        support_point,
                        end_point,
                    } => {}
                    Edge::Circle {
                        center_point,
                        radius,
                    } => {
                        let p: Point = (0., -1.).into();
                        let p = (p.to_vector() * radius).to_point();
                        let p = rotate_point(&p, &(0., 0.).into(), angle).to_vector();
                        draw_helper.draw_line(&center_point, &(center_point + p), Color::BLUE)
                    }
                    Edge::Line {
                        start_point,
                        end_point,
                    } => {}
                }
            }
        });

        // draw velocity
        if self.config.draw_velocity {
            self.scene.elements_iter().for_each(|element| {
                draw_helper.draw_line(
                    &element.center_point(),
                    &(element.center_point() + *element.meta().velocity() * 100.),
                    Color::RED,
                );
            });

            self.scene.elements_iter().for_each(|element| {
                draw_helper.draw_line(
                    &element.center_point(),
                    &(element.center_point()
                        + Vector::from((element.meta().angle_velocity() * 10000., 0.))),
                    Color::BLACK,
                );
            });
        }

        if self.config.draw_point_constraints {
            self.scene.point_constraints().for_each(|point_constraint| {
                let move_point = point_constraint.move_point();
                let fixed_point = point_constraint.fixed_point();
                draw_helper.draw_line(move_point, fixed_point, Color::RED);
                draw_helper.draw_circle(move_point, 0.5, Color::RED);
                draw_helper.draw_circle(fixed_point, 0.5, Color::RED);
            })
        }

        if self.config.draw_join_constraints {
            self.scene.join_constraints().for_each(|join_constraint| {
                let (move_point1, move_point2) = join_constraint.move_point_pair();
                draw_helper.draw_line(move_point1, move_point2, Color::RED);
                draw_helper.draw_circle(move_point1, 0.5, Color::RED);
                draw_helper.draw_circle(move_point2, 0.5, Color::RED);
            });
        }

        if self.config.draw_contact_point_pair {
            self.contact_viewer
                .get_collision_infos()
                .iter()
                .for_each(|contact_info| {
                    let (object_id_a, object_id_b) = contact_info.object_id_pair();

                    draw_helper.draw_circle(contact_info.point_a(), 0.3, Color::MAGENTA);
                    // draw_helper.draw_line(
                    //     contact_info.point_a(),
                    //     &(contact_info.point_a() + &(contact_info.normal_toward_a() * 2.)),
                    //     Color::BLACK,
                    // );

                    // if let Some(element) = self.scene.get_element(object_id_a) {
                    //     let v = element.compute_point_velocity(contact_info.point_a());

                    //     draw_helper.draw_line(
                    //         contact_info.point_a(),
                    //         &(contact_info.point_a() + &(v * 100.)),
                    //         Color::RED,
                    //     );
                    // };

                    draw_helper.draw_circle(contact_info.point_b(), 0.3, Color::MAGENTA);
                    // draw_helper.draw_line(
                    //     contact_info.point_b(),
                    //     &(contact_info.point_b() + &(contact_info.normal_toward_a() * -2.)),
                    //     Color::BLACK,
                    // );

                    // if let Some(element) = self.scene.get_element(object_id_b) {
                    //     let v = element.compute_point_velocity(contact_info.point_b());
                    //     draw_helper.draw_line(
                    //         contact_info.point_b(),
                    //         &(contact_info.point_b() + &(v * 100.)),
                    //         Color::BLUE,
                    //     );
                    // };
                });
        }

        helper.request_redraw();
    }
}

pub fn run_window<T: Default + Clone + 'static>(
    title: &str,
    config: ConfigBuilder,
    init: impl FnMut(&mut Scene<T>, &mut Handler<T>) + 'static,
    // update receive mut scene reference, second argument is mouse selected element id
    update: impl FnMut(&mut Scene<T>, Option<u32>, &mut Handler<T>) + 'static,
) {
    use speedy2d::Window;

    let window = Window::new_centered(title, (1920, 1080)).unwrap();

    let config: Config = config.into();

    window.run_loop(Handler {
        scene: Default::default(),
        init: Box::new(init),
        update: Box::new(update),
        is_paused: config.is_default_paused,
        is_mouse_down: false,
        selected_element_id: None,
        mouse_constraint_id: None,
        current_mouse_pos: None,
        contact_viewer: CollisionStatusViewer::default(),
        config,
        render_offset: Default::default(),
        records: Default::default(),
        record_handler: vec![],
        is_debug: false,
        iter_count: 0,
    });
}

pub fn run_simple(init: fn(&mut Scene, &mut Handler<()>)) {
    fn update(scene: &mut Scene, _selected_element_id: Option<u32>, _: &mut Handler<()>) {
        let duration = std::time::Duration::from_secs(10);
        scene.tick(duration.as_secs_f32());
    }

    let config = ConfigBuilder::default()
        .draw_center_point(false)
        .draw_join_constraints(true)
        .draw_point_constraints(true)
        .enable_mouse_constraint(true);
    run_window("point constraint - link", config, init, update)
}
