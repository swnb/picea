use crate::{
    algo::collision::{Collider, Projector, SubCollider},
    element::ComputeMomentOfInertia,
    math::{edge::Edge, point::Point, vector::Vector, FloatNum},
    meta::Mass,
    shape::utils::rotate_polygon,
};

use super::{
    utils::{
        compute_area_of_convex, compute_area_of_triangle, compute_convex_center_point,
        compute_moment_of_inertia_of_triangle, projection_polygon_on_vector,
        split_convex_polygon_to_triangles, VertexesToEdgeIter,
    },
    CenterPoint, EdgeIterable, GeometryTransform,
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

    fn rotate(&mut self, origin_point: &Point, deg: f32) {
        rotate_polygon(*origin_point, self.vertexes.iter_mut(), deg);
    }
}

impl EdgeIterable for ConvexPolygon {
    fn edge_iter(&self) -> Box<dyn Iterator<Item = Edge<'_>> + '_> {
        Box::new(VertexesToEdgeIter::new(&self.vertexes))
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
