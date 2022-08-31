use crate::{
    math::{axis::AxisDirection, point::Point, vector::Vector},
    meta::Mass,
};

pub mod circle;
pub mod polygon;
pub mod rect;
pub mod shapes;

pub trait Shape {
    fn compute_center_point(&self) -> Point<f32>;

    fn projection_on_vector(&self, vector: Vector<f32>) -> (Point<f32>, Point<f32>);

    fn translate(&mut self, vector: &Vector<f32>);

    fn rotate(&mut self, deg: f32);
}

pub trait ProjectionOnAxis {
    fn projection_on_axis(&self, axis_direction: AxisDirection) -> (f32, f32);
}

pub trait ComputeMomentOfInertia {
    fn compute_moment_of_inertia(&self, m: Mass) -> f32;
}
