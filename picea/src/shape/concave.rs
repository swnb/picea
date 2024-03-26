use crate::{
    collision::{Collider, Projector, SubCollider},
    element::{ComputeMomentOfInertia, SelfClone},
    math::{edge::Edge, point::Point, vector::Vector, FloatNum},
};

use super::{
    convex::ConvexPolygon,
    utils::{
        find_nearest_point, projection_polygon_on_vector, rotate_polygon,
        split_concave_polygon_to_convex_polygons, VertexesToEdgeIter,
    },
    CenterPoint, EdgeIterable, GeometryTransformer, NearestPoint, Transform,
};

#[derive(Clone)]
pub struct ConcavePolygon {
    origin_vertexes: Vec<Point>,
    vertexes: Vec<Point>,
    sub_convex_polygons: Vec<ConvexPolygon>,
    origin_center_point: Point,
    center_point: Point,
    area: FloatNum,
    transform: Transform,
}

impl ConcavePolygon {
    pub fn new(vertexes: impl Into<Vec<Point>>) -> Self {
        let vertexes: Vec<Point> = vertexes.into();
        let sub_convex_polygons: Vec<_> = split_concave_polygon_to_convex_polygons(&vertexes)
            .into_iter()
            .map(ConvexPolygon::new)
            .collect();

        let total_area = sub_convex_polygons
            .iter()
            .fold(0., |acc, convex_polygon| acc + convex_polygon.area());
        let total_area_inv = total_area.recip();

        let center_point = sub_convex_polygons
            .iter()
            .fold(Default::default(), |acc: Vector, convex_polygon| {
                let rate = convex_polygon.area() * total_area_inv;
                acc + convex_polygon.center_point().to_vector() * rate
            })
            .to_point();

        Self {
            origin_vertexes: vertexes.clone(),
            vertexes,
            sub_convex_polygons,
            origin_center_point: center_point,
            center_point,
            area: total_area,
            transform: Default::default(),
        }
    }

    pub fn to_convex_polygons(mut self) -> impl Iterator<Item = ConvexPolygon> {
        self.apply_transform();
        self.sub_convex_polygons.into_iter()
    }
}

impl GeometryTransformer for ConcavePolygon {
    fn transform_mut(&mut self) -> &mut Transform {
        &mut self.transform
    }

    fn apply_transform(&mut self) {
        for (i, p) in self.origin_vertexes.iter().enumerate() {
            self.vertexes[i] = p + &self.transform.translation;
        }

        self.center_point = self.origin_center_point + self.transform.translation;

        rotate_polygon(
            self.center_point,
            self.vertexes.iter_mut(),
            self.transform.rotation,
        );

        // TODO cache this method
        self.sub_convex_polygons = split_concave_polygon_to_convex_polygons(&self.vertexes)
            .into_iter()
            .map(ConvexPolygon::new)
            .collect();
    }
}

impl CenterPoint for ConcavePolygon {
    fn center_point(&self) -> Point {
        self.center_point
    }
}

impl Projector for ConcavePolygon {
    fn projection_on_vector(&self, vector: &Vector) -> (Point, Point) {
        projection_polygon_on_vector(self.vertexes.iter(), *vector)
    }
}

impl Collider for ConcavePolygon {
    fn sub_colliders(&'_ self) -> Option<Box<dyn Iterator<Item = &'_ dyn SubCollider> + '_>> {
        Some(Box::new(
            self.sub_convex_polygons
                .iter()
                .map(|p| p as &dyn SubCollider),
        ))
    }
}

impl SelfClone for ConcavePolygon {
    fn self_clone(&self) -> Box<dyn crate::prelude::ShapeTraitUnion> {
        self.clone().into()
    }
}

impl EdgeIterable for ConcavePolygon {
    fn edge_iter(&self) -> Box<dyn Iterator<Item = Edge<'_>> + '_> {
        Box::new(VertexesToEdgeIter::new(&self.vertexes))
    }
}

impl NearestPoint for ConcavePolygon {
    fn nearest_point(&self, reference_point: &Point, direction: &Vector) -> Point {
        find_nearest_point(self, reference_point, direction)
    }
}

impl ComputeMomentOfInertia for ConcavePolygon {
    fn compute_moment_of_inertia(&self, m: crate::meta::Mass) -> f32 {
        let area_inv = self.area.recip();
        self.sub_convex_polygons
            .iter()
            .fold(0., |acc, convex_polygon| {
                let convex_area = convex_polygon.area();
                let rate = convex_area * area_inv;
                acc + convex_polygon.compute_moment_of_inertia(m * rate) * rate
            })
    }
}
