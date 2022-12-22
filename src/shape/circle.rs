use super::{ComputeMomentOfInertia, Shape};
use crate::{
    math::{axis::AxisDirection, edge::Edge, point::Point, vector::Vector},
    meta::Mass,
};

#[derive(Clone, Debug)]
pub struct CircleShape {
    center: Point<f32>,
    r: f32,
    deg: f32,
}

impl<P: Into<Point<f32>>> From<(P, f32)> for CircleShape {
    fn from((p, radius): (P, f32)) -> Self {
        let center_point = p.into();
        Self::new(center_point, radius)
    }
}

impl CircleShape {
    #[inline]
    pub fn new(center_point: impl Into<Point<f32>>, radius: f32) -> Self {
        Self {
            center: center_point.into(),
            r: radius,
            deg: 0.,
        }
    }

    #[inline]
    pub fn radius(&self) -> f32 {
        self.r
    }

    #[inline]
    pub fn get_center_point(&self) -> Point<f32> {
        self.center
    }

    #[inline]
    pub fn translate(&mut self, vector: &Vector<f32>) {
        self.center += vector;
    }
}

impl Shape for CircleShape {
    fn center_point(&self) -> Point<f32> {
        self.center
    }

    fn projection_on_vector(&self, vector: &Vector<f32>) -> (Point<f32>, Point<f32>) {
        let vector = vector.normalize();

        let center_point = self.center_point();
        (
            center_point - vector * self.radius(),
            center_point + vector * self.radius(),
        )
    }

    fn projection_on_axis(&self, axis: AxisDirection) -> (f32, f32) {
        let center_point = self.center_point();
        let (center_x, center_y): (f32, f32) = center_point.into();
        let radius = self.radius();
        use AxisDirection::*;
        match axis {
            X => (center_x - radius, center_x + radius),
            Y => (center_y - radius, center_y + radius),
        }
    }

    fn translate(&mut self, vector: &Vector<f32>) {
        self.center += vector
    }

    fn rotate(&mut self, &origin: &Point<f32>, deg: f32) {
        use std::f32::consts::TAU;

        if origin != self.center {
            let center_vector: Vector<f32> = (origin, self.center).into();
            let new_center = origin + center_vector.affine_transformation_rotate(deg);
            self.center = new_center;
        }

        self.deg += deg;
        if self.deg > TAU {
            self.deg %= TAU
        }
    }

    fn edge_iter(&self) -> Box<dyn Iterator<Item = Edge<'_>> + '_> {
        struct EdgeIter<'a> {
            is_consumed: bool,
            circle_ref: &'a CircleShape,
        }

        impl<'a> Iterator for EdgeIter<'a> {
            type Item = Edge<'a>;
            fn next(&mut self) -> Option<Self::Item> {
                if self.is_consumed {
                    None
                } else {
                    self.is_consumed = true;
                    Some(Edge::Circle {
                        center_point: self.circle_ref.center_point(),
                        radius: self.circle_ref.radius(),
                    })
                }
            }
        }

        Box::new(EdgeIter {
            is_consumed: false,
            circle_ref: self,
        })
    }
}

impl ComputeMomentOfInertia for CircleShape {
    // compute moment of inertia;
    fn compute_moment_of_inertia(&self, m: Mass) -> f32 {
        m * self.radius().powf(2.) * 0.5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let circle_shape = CircleShape::new((0., 0.), 25.);

        let p = circle_shape.projection_on_vector(&(1., 0.).into());
        dbg!(p);
        let p = circle_shape.projection_on_vector(&(1., 1.).into());
        dbg!(p);
        let p = circle_shape.projection_on_vector(&(0., 1.).into());
        dbg!(p);
        let p = circle_shape.projection_on_vector(&(-1., 1.).into());
        dbg!(p);
        let p = circle_shape.projection_on_vector(&(-1., 0.).into());
        dbg!(p);
        let p = circle_shape.projection_on_vector(&(-1., -1.).into());
        dbg!(p);
        let p = circle_shape.projection_on_vector(&(0., -1.).into());
        dbg!(p);
        let p = circle_shape.projection_on_vector(&(1., -1.).into());
        dbg!(p);
    }
}
