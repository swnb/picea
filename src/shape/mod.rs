use crate::math::{point::Point, vector::Vector};

pub mod circle;
pub mod rect;

pub trait Shape {
    fn compute_center_point(&self) -> Point<f32>;

    fn projection_on_vector(&self, vector: Vector<f32>) -> (Point<f32>, Point<f32>);

    fn translate(&mut self, vector: &Vector<f32>);

    fn rotate(&mut self, deg: f32);
}
