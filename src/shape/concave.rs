use crate::{
    algo::collision::{Collider, Projector, SubCollider},
    element::ComputeMomentOfInertia,
    math::{edge::Edge, point::Point, vector::Vector, FloatNum},
};

use super::{
    convex::ConvexPolygon,
    utils::{
        find_nearest_point, projection_polygon_on_vector, rotate_point, rotate_polygon,
        split_concave_polygon_to_convex_polygons, translate_polygon, VertexesToEdgeIter,
    },
    CenterPoint, EdgeIterable, GeometryTransform, NearestPoint,
};

#[derive(Clone)]
pub struct ConcavePolygon {
    origin_vertexes: Vec<Point>,
    sub_convex_polygons: Vec<ConvexPolygon>,
    center_point: Point,
    area: FloatNum,
}

impl ConcavePolygon {
    pub fn new(vertexes: &[Point]) -> Self {
        let origin_vertexes = vertexes.to_owned();
        let sub_convex_polygons: Vec<_> =
            split_concave_polygon_to_convex_polygons(&origin_vertexes)
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
            origin_vertexes,
            sub_convex_polygons,
            center_point,
            area: total_area,
        }
    }

    pub fn to_convex_polygons(self) -> impl Iterator<Item = ConvexPolygon> {
        self.sub_convex_polygons.into_iter()
    }
}

impl GeometryTransform for ConcavePolygon {
    fn translate(&mut self, vector: &Vector) {
        self.sub_convex_polygons
            .iter_mut()
            .for_each(|convex_polygon| convex_polygon.translate(vector));
        translate_polygon(self.origin_vertexes.iter_mut(), vector);
        self.center_point += vector;
    }

    fn rotate(&mut self, origin_point: &Point, deg: f32) {
        // TODO update center point ?
        self.sub_convex_polygons
            .iter_mut()
            .for_each(|convex_polygon| convex_polygon.rotate(origin_point, deg));

        rotate_polygon(*origin_point, self.origin_vertexes.iter_mut(), deg);

        if origin_point != &self.center_point {
            self.center_point = rotate_point(&self.center_point, origin_point, deg);
        }
    }
}

impl CenterPoint for ConcavePolygon {
    fn center_point(&self) -> Point {
        self.center_point
    }
}

impl NearestPoint for ConcavePolygon {
    fn nearest_point(&self, reference_point: &Point, direction: &Vector) -> Point {
        find_nearest_point(self.origin_vertexes.iter(), reference_point, direction)
    }
}

impl Projector for ConcavePolygon {
    fn projection_on_vector(&self, vector: &Vector) -> (Point, Point) {
        projection_polygon_on_vector(self.origin_vertexes.iter(), *vector)
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

impl EdgeIterable for ConcavePolygon {
    fn edge_iter(&self) -> Box<dyn Iterator<Item = Edge<'_>> + '_> {
        Box::new(VertexesToEdgeIter::new(&self.origin_vertexes))
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
