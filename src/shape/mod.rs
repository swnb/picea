use crate::{
    math::{axis::AxisDirection, edge::Edge, point::Point, vector::Vector},
    meta::Mass,
};

pub mod circle;
pub mod concave;
pub mod convex;
pub mod line;
pub mod polygon;
pub mod utils;

pub trait Shape {
    fn center_point(&self) -> Point;

    fn projection_on_vector(&self, vector: &Vector) -> (Point, Point);

    fn projection_on_axis(&self, axis: AxisDirection) -> (f32, f32) {
        use AxisDirection::*;
        let (p1, p2) = self.projection_on_vector(&axis.into());
        match axis {
            X => (p1.x(), p2.x()),
            Y => (p1.y(), p2.y()),
        }
    }

    fn translate(&mut self, vector: &Vector);

    fn rotate(&mut self, origin_point: &Point, deg: f32);

    fn edge_iter(&self) -> Box<dyn Iterator<Item = Edge<'_>> + '_>;
}

pub trait ComputeMomentOfInertia {
    // TODO cache compute result
    fn compute_moment_of_inertia(&self, m: Mass) -> f32;
}
