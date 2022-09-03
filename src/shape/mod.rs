use crate::{
    math::{axis::AxisDirection, point::Point, vector::Vector},
    meta::Mass,
};

pub mod circle;
pub mod polygon;
mod utils;

pub trait Shape {
    fn center_point(&self) -> Point<f32>;

    fn projection_on_vector(&self, vector: &Vector<f32>) -> (Point<f32>, Point<f32>);

    fn projection_on_axis(&self, axis: AxisDirection) -> (f32, f32) {
        use AxisDirection::*;
        let result = self.projection_on_vector(&axis.into());
        match axis {
            X => (result.0.x(), result.1.x()),
            Y => (result.0.y(), result.1.y()),
        }
    }

    fn translate(&mut self, vector: &Vector<f32>);

    fn rotate(&mut self, origin_point: &Point<f32>, deg: f32);
}

pub trait ComputeMomentOfInertia {
    fn compute_moment_of_inertia(&self, m: Mass) -> f32;
}
