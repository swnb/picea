use picea_macro_tools::Shape;

use crate::{
    element::ComputeMomentOfInertia,
    math::{edge::Edge, point::Point, FloatNum},
    meta::Mass,
    shape::utils::rotate_polygon,
};

use super::{
    utils::{
        compute_area_of_convex, compute_area_of_triangle, compute_convex_center_point,
        compute_moment_of_inertia_of_triangle, resize_by_vector, split_convex_polygon_to_triangles,
        VerticesIter, VerticesToEdgeIter,
    },
    CenterPoint, EdgeIterable, GeometryTransformer, Transform,
};

#[derive(Clone, Shape)]
pub struct ConvexPolygon {
    origin_vertices: Vec<Point>,
    vertices: Vec<Point>,
    origin_center_point: Point,
    center_point: Point,
    area: FloatNum,
}

impl ConvexPolygon {
    pub fn new(points: impl Into<Vec<Point>>) -> Self {
        let vertices: Vec<_> = points.into();
        let center_point = compute_convex_center_point(&vertices);
        let area = compute_area_of_convex(&vertices);

        Self {
            origin_vertices: vertices.clone(),
            vertices,
            origin_center_point: center_point,
            center_point,
            area,
        }
    }

    pub fn area(&self) -> FloatNum {
        self.area
    }

    pub fn scale_with_center_point(&mut self, center_point: &Point, from: &Point, to: &Point) {
        resize_by_vector(self.vertices.iter_mut(), center_point, from, to);
    }
}

impl VerticesIter for ConvexPolygon {
    fn vertices_iter(&self) -> impl Iterator<Item = &Point> {
        self.vertices.iter()
    }

    fn vertices_iter_mut(&mut self) -> impl Iterator<Item = &mut Point> {
        self.vertices.iter_mut()
    }
}

impl GeometryTransformer for ConvexPolygon {
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
    }
}

impl CenterPoint for ConvexPolygon {
    fn center_point(&self) -> Point {
        self.center_point
    }
}

impl EdgeIterable for ConvexPolygon {
    fn edge_iter(&self) -> Box<dyn Iterator<Item = Edge<'_>> + '_> {
        Box::new(VerticesToEdgeIter::new(&self.vertices))
    }
}

impl ComputeMomentOfInertia for ConvexPolygon {
    // split into multi triangles ,compute each triangle's moment_of_inertia , sum them all
    fn compute_moment_of_inertia(&self, m: Mass) -> f32 {
        let triangles = split_convex_polygon_to_triangles(&self.vertices);

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
