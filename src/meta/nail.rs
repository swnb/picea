use crate::math::{point::Point, vector::Vector, FloatNum};

use std::{ops::AddAssign, rc::Rc};

pub struct Nail {
    origin_point: Point,
    // follow with element
    current_point: Point,
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

impl Nail {
    pub fn new(point: Point) -> Self {
        Self {
            origin_point: point,
            current_point: point,
        }
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
