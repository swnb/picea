use crate::element::Element;

pub trait Renderer {
    fn before_render(&mut self);

    fn render(&mut self, element: &Element);

    fn after_render(&mut self);
}
