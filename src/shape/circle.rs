use crate::math::{point::Point, vector::Vector};

#[derive(Clone, Debug)]
pub struct CircleShape {
    center: Point<f32>,
    r: f32,
}

impl CircleShape {
    pub fn radius(&self) -> f32 {
        self.r
    }

    pub fn get_center_point(&self) -> Point<f32> {
        self.center
    }

    pub fn translate(&mut self, vector: &Vector<f32>) {
        self.center += vector;
    }
}
