use crate::math::{point::Point, vector::Vector};

use super::Shape;

#[derive(Clone, Debug)]
pub struct CircleShape {
    center: Point<f32>,
    r: f32,
    deg: f32,
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

impl Shape for CircleShape {
    fn compute_center_point(&self) -> Point<f32> {
        self.center
    }

    fn projection_on_vector(&self, vector: Vector<f32>) -> (Point<f32>, Point<f32>) {
        // TOD
        unimplemented!()
    }

    fn translate(&mut self, vector: &Vector<f32>) {
        self.center += vector
    }

    fn rotate(&mut self, deg: f32) {
        use std::f32::consts::TAU;

        self.deg += deg;
        if self.deg > TAU {
            self.deg %= TAU
        }
    }
}
