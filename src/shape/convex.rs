use crate::{
    algo::collision::{Collider, Projector},
    element::{ComputeMomentOfInertia, SelfClone, ShapeTraitUnion},
    math::{edge::Edge, point::Point, vector::Vector, FloatNum},
    meta::Mass,
    shape::utils::rotate_polygon,
};

use super::{
    utils::{
        compute_area_of_convex, compute_area_of_triangle, compute_convex_center_point,
        compute_moment_of_inertia_of_triangle, find_nearest_point, projection_polygon_on_vector,
        resize_by_vector, rotate_point, split_convex_polygon_to_triangles, VertexesToEdgeIter,
    },
    CenterPoint, EdgeIterable, GeometryTransform, NearestPoint,
};

#[derive(Clone)]
pub struct ConvexPolygon {
    vertexes: Vec<Point>,
    center_point: Point,
    area: FloatNum,
}

impl ConvexPolygon {
    pub fn new(points: impl Into<Vec<Point>>) -> Self {
        let vertexes: Vec<_> = points.into();
        let center_point = compute_convex_center_point(&vertexes);
        let area = compute_area_of_convex(&vertexes);

        Self {
            vertexes,
            center_point,
            area,
        }
    }

    pub fn area(&self) -> FloatNum {
        self.area
    }

    pub fn scale_with_center_point(&mut self, center_point: &Point, from: &Point, to: &Point) {
        resize_by_vector(self.vertexes.iter_mut(), center_point, from, to);
    }
}

impl CenterPoint for ConvexPolygon {
    fn center_point(&self) -> Point {
        self.center_point
    }
}

impl GeometryTransform for ConvexPolygon {
    fn translate(&mut self, vector: &Vector) {
        for point in self.vertexes.iter_mut() {
            *point += vector;
        }
        self.center_point += vector;
    }

    fn rotate(&mut self, origin_point: &Point, rad: f32) {
        rotate_polygon(*origin_point, self.vertexes.iter_mut(), rad);

        if origin_point != &self.center_point {
            self.center_point = rotate_point(&self.center_point, origin_point, rad);
        }
    }

    fn scale(&mut self, from: &Point, to: &Point) {
        self.scale_with_center_point(&self.center_point.clone(), from, to)
    }
}

impl EdgeIterable for ConvexPolygon {
    fn edge_iter(&self) -> Box<dyn Iterator<Item = Edge<'_>> + '_> {
        Box::new(VertexesToEdgeIter::new(&self.vertexes))
    }
}

impl NearestPoint for ConvexPolygon {
    fn nearest_point(&self, reference_point: &Point, direction: &Vector) -> Point {
        find_nearest_point(self, reference_point, direction)
    }
}

impl Projector for ConvexPolygon {
    fn projection_on_vector(&self, vector: &Vector) -> (Point, Point) {
        projection_polygon_on_vector(self.vertexes.iter(), *vector)
    }
}

impl Collider for ConvexPolygon {}

impl ComputeMomentOfInertia for ConvexPolygon {
    // split into multi triangles ,compute each triangle's moment_of_inertia , sum them all
    fn compute_moment_of_inertia(&self, m: Mass) -> f32 {
        let triangles = split_convex_polygon_to_triangles(&self.vertexes);

        let total_area = triangles
            .iter()
            .fold(0., |acc, triangle| acc + compute_area_of_triangle(triangle));
        let total_area_inv = total_area.recip();
        triangles.into_iter().fold(0., |acc, triangle| {
            let mass = m * compute_area_of_triangle(&triangle) * total_area_inv;
            compute_moment_of_inertia_of_triangle(&triangle, mass) + acc
        })
    }
}

impl SelfClone for ConvexPolygon {
    fn self_clone(&self) -> Box<dyn ShapeTraitUnion> {
        self.clone().into()
    }
}
