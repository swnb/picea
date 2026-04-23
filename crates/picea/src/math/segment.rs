use std::mem;

use super::{point::Point, vector::Vector};

pub type Axis = Segment;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Segment {
    start_point: Point,
    end_point: Point,
}

impl Segment {
    pub const fn new(start_point: Point, end_point: Point) -> Self {
        Self {
            start_point,
            end_point,
        }
    }

    pub fn start_point(&self) -> &Point {
        &self.start_point
    }

    pub fn start_point_mut(&mut self) -> &mut Point {
        &mut self.start_point
    }

    pub fn end_point(&self) -> &Point {
        &self.end_point
    }

    pub fn end_point_mut(&mut self) -> &mut Point {
        &mut self.end_point
    }

    pub fn flip(&self) -> Self {
        Self::new(self.end_point, self.start_point)
    }

    pub fn flip_mut(&mut self) {
        mem::swap(&mut self.end_point, &mut self.start_point);
    }

    pub fn ends(&self) -> (&Point, &Point) {
        (&self.start_point, &self.end_point)
    }

    pub fn ends_mut(&mut self) -> (&mut Point, &mut Point) {
        (&mut self.start_point, &mut self.end_point)
    }

    pub fn direction(&self) -> Vector {
        Vector::from((self.start_point, self.end_point))
    }
}

impl From<(Point, Point)> for Segment {
    fn from((start_point, end_point): (Point, Point)) -> Self {
        Self::new(start_point, end_point)
    }
}
