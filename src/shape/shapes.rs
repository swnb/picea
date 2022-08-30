use super::{circle::CircleShape, rect::RectShape, ProjectionOnAxis};
use crate::math::axis::AxisDirection;

#[derive(Clone, Debug)]
pub enum ShapeUnion {
    Rect(RectShape),
    Circle(CircleShape),
}
