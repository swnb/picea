use crate::{
    element::ShapeTraitUnion,
    math::point::Point,
    shape::{
        circle::Circle,
        polygon::{Rect, RegularPolygon, Square},
    },
};

impl<T: ShapeTraitUnion + 'static> From<T> for Box<dyn ShapeTraitUnion> {
    fn from(value: T) -> Self {
        Box::new(value)
    }
}

// create rect
impl From<(f32, f32, f32, f32)> for Box<dyn ShapeTraitUnion> {
    fn from((x, y, width, height): (f32, f32, f32, f32)) -> Self {
        Rect::new(x, y, width, height).into()
    }
}

// create square
impl From<(f32, f32, f32)> for Box<dyn ShapeTraitUnion> {
    fn from((x, y, size): (f32, f32, f32)) -> Self {
        Square::new(x, y, size).into()
    }
}

// create circle
impl<C> From<(C, f32)> for Box<dyn ShapeTraitUnion>
where
    C: Into<Point>,
{
    fn from((center_point, radius): (C, f32)) -> Self {
        Circle::new(center_point, radius).into()
    }
}

impl<C> From<(usize, C, f32)> for Box<dyn ShapeTraitUnion>
where
    C: Into<Point>,
{
    fn from((n, center, radius): (usize, C, f32)) -> Self {
        RegularPolygon::new(center, n, radius).into()
    }
}
