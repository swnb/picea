use derive_builder::Builder;
use picea::math::edge::Edge;
use picea::math::point::Point;
use picea::math::FloatNum;
use picea::scene::Scene;
use speedy2d::color::Color;
use speedy2d::dimen::Vector2;
use speedy2d::window::{VirtualKeyCode, WindowHandler, WindowHelper};
use speedy2d::Graphics2D;

#[derive(Builder)]
pub struct Config {
    scale: FloatNum,
    show_velocity: FloatNum,
}

struct Handler {
    scene: Scene,
    init: Box<dyn FnMut(&mut Scene)>,
    update: Box<dyn FnMut(&mut Scene)>,
    scale: FloatNum,
    is_paused: bool,
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
    }

    fn on_mouse_button_up(
        &mut self,
        helper: &mut WindowHelper<()>,
        button: speedy2d::window::MouseButton,
    ) {
    }

    fn on_mouse_move(&mut self, helper: &mut WindowHelper<()>, position: speedy2d::dimen::Vec2) {}

    fn on_user_event(&mut self, helper: &mut WindowHelper<()>, user_event: ()) {}

    fn on_draw(&mut self, helper: &mut WindowHelper, graphics: &mut Graphics2D) {
        if !self.is_paused {
            self.scene.update_elements_by_duration(0.5);
        }

        graphics.clear_screen(Color::from_gray(0.8));

        let mut draw_helper = DrawHelper {
            graphics,
            scale: self.scale,
        };
        // draw velocity
        self.scene.elements_iter().for_each(|element| {
            draw_helper.draw_line(
                element.center_point(),
                element.center_point() + element.meta().velocity() * 3.,
                Color::YELLOW,
            );

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

        helper.request_redraw();
    }
}

pub fn entry_main(
    title: &str,
    init: impl FnMut(&mut Scene) + 'static,
    update: impl FnMut(&mut Scene) + 'static,
) {
    use speedy2d::Window;

    let window = Window::new_centered(title, (1920, 1080)).unwrap();

    window.run_loop(Handler {
        scene: Default::default(),
        init: Box::new(init),
        update: Box::new(update),
        scale: 10.,
        is_paused: false,
    });
}
