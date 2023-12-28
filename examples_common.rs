use derive_builder::Builder;
use picea::constraints::JoinConstraintConfig;
use picea::math::edge::Edge;
use picea::math::point::Point;
use picea::math::{FloatNum, PI};
use picea::scene::Scene;
use picea::shape::utils::is_point_inside_shape;
use speedy2d::color::Color;
use speedy2d::dimen::Vector2;
use speedy2d::window::{VirtualKeyCode, WindowHandler, WindowHelper};
use speedy2d::Graphics2D;

#[derive(Builder)]
#[builder(pattern = "immutable")]
pub struct Config {
    #[builder(default = "10.0")]
    scale: FloatNum,
    #[builder(default = "false")]
    draw_velocity: bool,
    #[builder(default = "false")]
    is_default_paused: bool,
    #[builder(default = "true")]
    draw_center_point: bool,
    #[builder(default = "false")]
    draw_join_constraints: bool,
    #[builder(default = "false")]
    draw_point_constraints: bool,
    #[builder(default = "false")]
    enable_mouse_constraint: bool,
}

type UpdateFn = dyn FnMut(&mut Scene, Option<u32>);

struct Handler {
    scene: Scene,
    init: Box<dyn FnMut(&mut Scene)>,
    update: Box<UpdateFn>,
    is_paused: bool,
    is_mouse_down: bool,
    current_mouse_pos: Option<Point>,
    selected_element_id: Option<u32>,
    mouse_constraint_id: Option<u32>,
    config: Config,
}

fn into_vector2(p: Point) -> Vector2<FloatNum> {
    Vector2::new(p.x(), p.y())
}

struct DrawHelper<'a> {
    graphics: &'a mut Graphics2D,
    scale: FloatNum,
}

impl<'a> DrawHelper<'a> {
    fn draw_line(&mut self, start_point: Point, end_point: Point, color: Color) {
        self.graphics.draw_line(
            into_vector2((start_point.to_vector() * self.scale).to_point()),
            into_vector2((end_point.to_vector() * self.scale).to_point()),
            3.0,
            color,
        )
    }

    fn draw_circle(&mut self, center_point: Point, radius: FloatNum, color: Color) {
        self.graphics.draw_circle(
            into_vector2((center_point.to_vector() * self.scale).to_point()),
            radius * self.scale,
            color,
        );
    }
}

impl Handler {
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
                    JoinConstraintConfig::default(),
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

impl WindowHandler for Handler {
    fn on_start(
        &mut self,
        helper: &mut WindowHelper<()>,
        info: speedy2d::window::WindowStartupInfo,
    ) {
        (self.init)(&mut self.scene);
    }

    fn on_key_down(
        &mut self,
        helper: &mut WindowHelper<()>,
        virtual_key_code: Option<speedy2d::window::VirtualKeyCode>,
        scancode: speedy2d::window::KeyScancode,
    ) {
        if let Some(key) = virtual_key_code {
            match key {
                VirtualKeyCode::R => {
                    self.scene.clear();
                    (self.init)(&mut self.scene);
                }
                VirtualKeyCode::Space => {
                    self.is_paused = !self.is_paused;
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

    fn on_mouse_move(&mut self, helper: &mut WindowHelper<()>, position: speedy2d::dimen::Vec2) {
        if !self.is_mouse_down {
            return;
        }

        let current_mouse_pos = Point::new(
            position.x / self.config.scale,
            position.y / self.config.scale,
        );

        if self.current_mouse_pos.is_none() {
            let element_id = self
                .scene
                .elements_iter()
                .find(|element| {
                    is_point_inside_shape(current_mouse_pos, &mut element.shape().edge_iter())
                })
                .map(|element| element.id());

            self.selected_element_id = element_id;
        }

        self.current_mouse_pos = Some(current_mouse_pos);

        self.solve_mouse_constraint()
    }

    fn on_user_event(&mut self, helper: &mut WindowHelper<()>, user_event: ()) {}

    fn on_draw(&mut self, helper: &mut WindowHelper, graphics: &mut Graphics2D) {
        if !self.is_paused {
            (self.update)(&mut self.scene, self.selected_element_id);
        }

        graphics.clear_screen(Color::from_gray(0.8));

        let mut draw_helper = DrawHelper {
            graphics,
            scale: self.config.scale,
        };

        self.scene.elements_iter().for_each(|element| {
            element.shape().edge_iter().for_each(|edge| match edge {
                Edge::Line {
                    start_point,
                    end_point,
                } => draw_helper.draw_line(*start_point, *end_point, Color::WHITE),
                Edge::Circle {
                    center_point,
                    radius,
                } => {
                    draw_helper.draw_circle(center_point, radius, Color::WHITE);
                }
                _ => unimplemented!(),
            });
        });

        if self.config.draw_center_point {
            self.scene.elements_iter().for_each(|element| {
                draw_helper.draw_circle(element.center_point(), 1., Color::BLUE)
            });
        }
        // draw velocity
        if self.config.draw_velocity {
            self.scene.elements_iter().for_each(|element| {
                draw_helper.draw_line(
                    element.center_point(),
                    element.center_point() + element.meta().velocity() * 3.,
                    Color::YELLOW,
                );
            });
        }

        if self.config.draw_point_constraints {
            self.scene.point_constraints().for_each(|point_constraint| {
                let move_point = *point_constraint.move_point();
                let fixed_point = *point_constraint.fixed_point();
                draw_helper.draw_line(move_point, fixed_point, Color::RED);
                draw_helper.draw_circle(move_point, 0.5, Color::RED);
                draw_helper.draw_circle(fixed_point, 0.5, Color::RED);
            })
        }

        if self.config.draw_join_constraints {
            self.scene.join_constraints().for_each(|join_constraint| {
                let (&move_point1, &move_point2) = join_constraint.move_point_pair();
                draw_helper.draw_line(move_point1, move_point2, Color::RED);
                draw_helper.draw_circle(move_point1, 0.5, Color::RED);
                draw_helper.draw_circle(move_point2, 0.5, Color::RED);
            });
        }

        helper.request_redraw();
    }
}

pub fn run_window(
    title: &str,
    config: ConfigBuilder,
    init: impl FnMut(&mut Scene) + 'static,
    // update receive mut scene reference, second argument is mouse selected element id
    update: impl FnMut(&mut Scene, Option<u32>) + 'static,
) {
    use speedy2d::Window;

    let window = Window::new_centered(title, (1920, 1080)).unwrap();

    let config = config.build().unwrap();

    window.run_loop(Handler {
        scene: Default::default(),
        init: Box::new(init),
        update: Box::new(update),
        is_paused: config.is_default_paused,
        is_mouse_down: false,
        selected_element_id: None,
        mouse_constraint_id: None,
        current_mouse_pos: None,
        config,
    });
}
