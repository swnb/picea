use crate::math::{point::Point, vector::Vector, FloatNum};

use std::{ops::AddAssign, rc::Rc};

pub struct Nail {
    origin_point: Point,
    // follow with element
    current_point: Point,
    total_lambda: FloatNum,
}

impl From<Point> for Nail {
    fn from(value: Point) -> Self {
        Self::new(value)
    }
}

impl AddAssign<&Vector> for Nail {
    fn add_assign(&mut self, rhs: &Vector) {
        self.current_point += rhs
    }
}

impl AsRef<Point> for Nail {
    fn as_ref(&self) -> &Point {
        &self.origin_point
    }
}

impl Nail {
    pub fn new(point: Point) -> Self {
        Self {
            origin_point: point,
            current_point: point,
            total_lambda: 0.,
        }
    }

    pub fn restrict_lambda(&mut self, lambda: FloatNum) -> FloatNum {
        let previous_lambda = self.total_lambda;
        self.total_lambda += lambda;
        self.total_lambda = self.total_lambda.max(5.0);
        self.total_lambda - previous_lambda
    }

    pub(crate) fn reset_total_lambda(&mut self) {
        self.total_lambda = 0.;
    }

    pub fn rotate(&mut self, center_point: &Point, rad: FloatNum) {
        let mut v: Vector = (center_point, &self.current_point).into();
        v.affine_transformation_rotate_self(rad);
        self.current_point = center_point + &v;
    }

    pub fn stretch_length(&self) -> Vector {
        (self.current_point, self.origin_point).into()
    }

    pub fn point_bind_with_element(&self) -> &Point {
        &self.current_point
    }
}

struct JoinNailInner {
    point_a: Point,
    point_b: Point,
}

pub struct JoinNail(Rc<JoinNailInner>);

impl JoinNail {}
