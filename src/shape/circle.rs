use super::{
    CenterPoint, EdgeIterable, GeometryTransform, GeometryTransformFromOrigin, NearestPoint,
};
use crate::{
    collision::{Collider, Projector},
    element::{ComputeMomentOfInertia, SelfClone, ShapeTraitUnion},
    math::{axis::AxisDirection, edge::Edge, point::Point, vector::Vector, TAU},
    meta::Mass,
    shape::utils::rotate_point,
};

#[derive(Clone, Debug)]
pub struct Circle {
    center_point: Point,
    r: f32,
}

impl<P: Into<Point>> From<(P, f32)> for Circle {
    fn from((p, radius): (P, f32)) -> Self {
        let center_point = p.into();
        Self::new(center_point, radius)
    }
}

impl Circle {
    #[inline]
    pub fn new(center_point: impl Into<Point>, radius: f32) -> Self {
        Self {
            center_point: center_point.into(),
            r: radius,
        }
    }

    #[inline]
    pub fn radius(&self) -> f32 {
        self.r
    }

    #[inline]
    pub fn get_center_point(&self) -> Point {
        self.center_point
    }

    #[inline]
    pub fn translate(&mut self, vector: &Vector) {
        self.center_point += vector;
    }
}

impl Projector for Circle {
    fn projection_on_vector(&self, vector: &Vector) -> (Point, Point) {
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
}

impl CenterPoint for Circle {
    fn center_point(&self) -> Point {
        self.center_point
    }
}

impl NearestPoint for Circle {
    fn support_find_nearest_point(&self) -> bool {
        false
    }

    // FIXME use direction
    fn nearest_point(&self, reference_point: &Point, _: &Vector) -> Point {
        let vector = *reference_point - self.center_point;
        let vector = vector.normalize();
        let vector = vector * self.radius();
        self.center_point + vector
    }
}

impl GeometryTransformFromOrigin for Circle {
    fn transform_from_origin<'a>(&mut self, transform: super::Transform<'a>) {
        self.center_point += transform.vector;
    }
}

// impl GeometryTransform for Circle {
//     fn translate(&mut self, vector: &Vector) {
//         self.center_point += vector
//     }

//     fn rotate(&mut self, &origin_point: &Point, rad: f32) {
//         if origin_point != self.center_point {
//             let center_vector: Vector = (origin_point, self.center_point).into();
//             let new_center = origin_point + center_vector.affine_transformation_rotate(rad);
//             self.center_point = new_center;
//         }

//         self.rad += rad;
//         if self.rad > TAU() {
//             self.rad %= TAU()
//         }

//         if origin_point != self.center_point {
//             self.center_point = rotate_point(&self.center_point, &origin_point, rad);
//         }
//     }

//     fn scale(&mut self, from: &Point, to: &Point) {
//         let resize_vector: Vector = (from, to).into();
//         // TODO resize to ellipse
//         self.r += resize_vector.abs();
//     }
// }

impl EdgeIterable for Circle {
    fn edge_iter(&self) -> Box<dyn Iterator<Item = Edge<'_>> + '_> {
        struct EdgeIter<'a> {
            is_consumed: bool,
            circle_ref: &'a Circle,
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

impl ComputeMomentOfInertia for Circle {
    // compute moment of inertia;
    fn compute_moment_of_inertia(&self, m: Mass) -> f32 {
        m * self.radius().powf(2.) * 0.5
    }
}

impl Collider for Circle {}

impl SelfClone for Circle {
    fn self_clone(&self) -> Box<dyn ShapeTraitUnion> {
        self.clone().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let circle_shape = Circle::new((0., 0.), 25.);

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
