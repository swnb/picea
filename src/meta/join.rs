use crate::math::{point::Point, vector::Vector, FloatNum};

pub struct JoinPoint {
    id: u32,
    point: Point,
}

impl JoinPoint {
    pub fn new(id: u32, point: impl Into<Point>) -> Self {
        Self {
            id,
            point: point.into(),
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn point_mut(&mut self) -> &mut Point {
        &mut self.point
    }

    pub fn point(&self) -> &Point {
        &self.point
    }

    pub fn rotate(&mut self, center_point: &Point, rad: FloatNum) {
        let mut v: Vector = (center_point, &self.point).into();
        v.affine_transformation_rotate_self(rad);
        self.point = center_point + &v;
    }
}
