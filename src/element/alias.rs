use crate::{
    math::point::Point,
    shape::polygon::{Rect, RegularTriangle, Square},
};

use super::ElementShape;

impl<T: ElementShape + 'static> From<T> for Box<dyn ElementShape> {
    fn from(value: T) -> Self {
        Box::new(value)
    }
}

// create react
impl From<(f32, f32, f32, f32)> for Box<dyn ElementShape> {
    fn from((x, y, width, height): (f32, f32, f32, f32)) -> Self {
        Rect::new(x, y, width, height).into()
    }
}

// create square
impl From<(f32, f32, f32)> for Box<dyn ElementShape> {
    fn from((x, y, size): (f32, f32, f32)) -> Self {
        Square::new(x, y, size).into()
    }
}

// create triangle
impl<C> From<(C, f32)> for Box<dyn ElementShape>
where
    C: Into<Point<f32>>,
{
    fn from((center_point, radius): (C, f32)) -> Self {
        RegularTriangle::new(center_point, radius).into()
    }
}
