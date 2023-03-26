use std::{
    mem,
    ops::{Neg, Sub},
};

use super::{point::Point, vector::Vector, FloatNum};

pub type Axis<T> = Segment<T>;

#[derive(Clone)]
pub struct Segment<T: Clone + Copy = FloatNum> {
    start_point: Point<T>,
    end_point: Point<T>,
}

impl<T: Clone + Copy> Segment<T> {
    pub fn new(start_point: Point<T>, end_point: Point<T>) -> Self {
        Self {
            start_point,
            end_point,
        }
    }

    pub fn start_point(&self) -> &Point<T> {
        &self.start_point
    }

    pub fn start_point_mut(&mut self) -> &mut Point<T> {
        &mut self.start_point
    }

    pub fn end_point(&self) -> &Point<T> {
        &self.end_point
    }

    pub fn end_point_mut(&mut self) -> &mut Point<T> {
        &mut self.end_point
    }

    pub fn flip(&self) -> Segment<T> {
        (self.end_point, self.start_point).into()
    }

    pub fn flip_mut(&mut self) {
        mem::swap(&mut self.end_point, &mut self.start_point);
    }
}

impl<T: Clone + Copy> Segment<T>
where
    T: Neg<Output = T> + Sub<Output = T>,
{
    pub fn to_vector(&self) -> Vector<T> {
        (self.start_point, self.end_point).into()
    }
}

impl<T: Clone + Copy> From<(Point<T>, Point<T>)> for Segment<T> {
    fn from((start_point, end_point): (Point<T>, Point<T>)) -> Self {
        Segment {
            start_point,
            end_point,
        }
    }
}
