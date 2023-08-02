use crate::{
    algo::collision::{Collider, Projector},
    element::{ComputeMomentOfInertia, SelfClone, ShapeTraitUnion},
    math::{edge::Edge, point::Point, segment::Segment, vector::Vector},
    meta::Mass,
};

use super::{
    utils::{find_nearest_point, resize_by_vector, rotate_point},
    CenterPoint, EdgeIterable, GeometryTransform, NearestPoint,
};

#[derive(Clone)]
pub struct Line {
    segment: Segment,
    center_point: Point,
}

impl From<(Point, Point)> for Line {
    fn from(value: (Point, Point)) -> Self {
        let segment: Segment = value.into();
        let center_point = (segment.start_point().to_vector() * 0.5
            + segment.end_point().to_vector() * 0.5)
            .to_point();

        Self {
            segment,
            center_point,
        }
    }
}

impl Line {
    pub fn new<T: Into<Point>>(v1: T, v2: T) -> Self {
        (v1.into(), v2.into()).into()
    }

    pub fn start_point(&self) -> &Point {
        self.segment.start_point()
    }

    pub fn start_point_mut(&mut self) -> &mut Point {
        self.segment.start_point_mut()
    }

    pub fn end_point(&self) -> &Point {
        self.segment.end_point()
    }

    pub fn end_point_mut(&mut self) -> &mut Point {
        self.segment.end_point_mut()
    }
}

impl CenterPoint for Line {
    fn center_point(&self) -> Point {
        self.center_point
    }
}

impl NearestPoint for Line {
    fn nearest_point(&self, reference_point: &Point, direction: &Vector) -> Point {
        if reference_point == self.start_point() {
            return *self.end_point();
        } else if reference_point == self.end_point() {
            return *self.start_point();
        }

        find_nearest_point(self, reference_point, direction)
    }
}

impl EdgeIterable for Line {
    fn edge_iter(&self) -> Box<dyn Iterator<Item = Edge<'_>> + '_> {
        let edges = [
            Edge::Line {
                start_point: self.start_point(),
                end_point: self.end_point(),
            },
            Edge::Line {
                end_point: self.start_point(),
                start_point: self.end_point(),
            },
        ];

        Box::new(edges.into_iter())
    }
}

impl Collider for Line {}

impl GeometryTransform for Line {
    fn translate(&mut self, vector: &Vector) {
        *self.start_point_mut() += vector;
        *self.end_point_mut() += vector;
        self.center_point += vector;
    }

    fn rotate(&mut self, origin_point: &Point, rad: f32) {
        let update_point = |point: &mut Point| {
            let mut tmp_vector: Vector = (*origin_point, *point).into();
            tmp_vector.affine_transformation_rotate_self(rad);
            *point = *origin_point + tmp_vector;
        };
        update_point(self.start_point_mut());
        update_point(self.end_point_mut());

        if origin_point != &self.center_point {
            self.center_point = rotate_point(&self.center_point, origin_point, rad);
        }
    }

    fn scale(&mut self, from: &Point, to: &Point) {
        let (start_point, end_point) = self.segment.ends_mut();
        resize_by_vector(
            [start_point, end_point].into_iter(),
            &self.center_point,
            from,
            to,
        )
    }
}

impl Projector for Line {
    fn projection_on_vector(&self, vector: &Vector) -> (Point, Point) {
        let vector = vector.normalize();
        let &start_point = self.start_point();
        let &end_point = self.end_point();
        let start_point_projection_size = start_point.to_vector() * vector;
        let end_point_projection_size = end_point.to_vector() * vector;
        if start_point_projection_size < end_point_projection_size {
            (start_point, end_point)
        } else {
            (end_point, start_point)
        }
    }
}

impl ComputeMomentOfInertia for Line {
    // Line can't compute inertia
    fn compute_moment_of_inertia(&self, m: Mass) -> f32 {
        let l: Vector = (self.end_point(), self.start_point()).into();

        let l = l.x().powf(2.) + l.y().powf(2.);

        m * 12f32.recip() * l
    }
}

impl SelfClone for Line {
    fn self_clone(&self) -> Box<dyn ShapeTraitUnion> {
        self.clone().into()
    }
}
