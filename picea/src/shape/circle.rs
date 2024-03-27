use macro_tools::{Fields, Shape};

use super::{CenterPoint, EdgeIterable, GeometryTransformer, NearestPoint, Transform};
use crate::{
    collision::Projector,
    element::ComputeMomentOfInertia,
    math::{axis::AxisDirection, edge::Edge, point::Point, vector::Vector},
    meta::Mass,
};

#[derive(Clone, Debug, Shape, Fields)]
pub struct Circle {
    origin_center_point: Point,
    transform: Transform,
    center_point: Point,
    #[field(r)]
    radius: f32,
    rad: f32,
}

impl Circle {
    #[inline]
    pub fn new(center_point: impl Into<Point>, radius: f32) -> Self {
        let center_point = center_point.into();
        Self {
            origin_center_point: center_point,
            transform: Default::default(),
            center_point,
            radius,
            rad: 0.,
        }
    }
}

impl GeometryTransformer for Circle {
    fn transform_mut(&mut self) -> &mut Transform {
        &mut self.transform
    }

    fn apply_transform(&mut self) {
        self.center_point = self.origin_center_point + self.transform.translation;
        self.transform.is_changed = false;
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
