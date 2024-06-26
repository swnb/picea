use crate::{
    collision::{Collider, Projector, SubCollider},
    element::{ComputeMomentOfInertia, SelfClone},
    math::{edge::Edge, point::Point, vector::Vector, FloatNum},
};

use super::{
    convex::ConvexPolygon,
    utils::{
        find_nearest_point, projection_polygon_on_vector, rotate_polygon,
        split_concave_polygon_to_convex_polygons, VerticesToEdgeIter,
    },
    CenterPoint, EdgeIterable, GeometryTransformer, NearestPoint, Transform,
};

#[derive(Clone)]
pub struct ConcavePolygon {
    origin_vertices: Vec<Point>,
    vertices: Vec<Point>,
    sub_convex_polygons: Vec<ConvexPolygon>,
    origin_center_point: Point,
    center_point: Point,
    area: FloatNum,
}

impl ConcavePolygon {
    pub fn new(vertices: impl Into<Vec<Point>>) -> Self {
        let vertices: Vec<Point> = vertices.into();
        let sub_convex_polygons: Vec<_> = split_concave_polygon_to_convex_polygons(&vertices)
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
            origin_vertices: vertices.clone(),
            vertices,
            sub_convex_polygons,
            origin_center_point: center_point,
            center_point,
            area: total_area,
        }
    }

    pub fn to_convex_polygons(self) -> impl Iterator<Item = ConvexPolygon> {
        self.sub_convex_polygons.into_iter()
    }
}

impl GeometryTransformer for ConcavePolygon {
    fn sync_transform(&mut self, transform: &Transform) {
        for (i, p) in self.origin_vertices.iter().enumerate() {
            self.vertices[i] = p + transform.translation();
        }

        self.center_point = self.origin_center_point + transform.translation();

        rotate_polygon(
            self.center_point,
            self.vertices.iter_mut(),
            transform.rotation(),
        );

        // TODO cache this method
        self.sub_convex_polygons = split_concave_polygon_to_convex_polygons(&self.vertices)
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
        projection_polygon_on_vector(self.vertices.iter(), *vector)
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

    fn measure_sub_collider_concat_point(&self, contact_point: &Point) -> bool {
        self.vertices.iter().any(|p| p == contact_point)
    }
}

impl SelfClone for ConcavePolygon {
    fn self_clone(&self) -> Box<dyn crate::prelude::ShapeTraitUnion> {
        self.clone().into()
    }
}

impl EdgeIterable for ConcavePolygon {
    fn edge_iter(&self) -> Box<dyn Iterator<Item = Edge<'_>> + '_> {
        Box::new(VerticesToEdgeIter::new(&self.vertices))
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
