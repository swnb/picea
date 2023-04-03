use crate::{
    math::{edge::Edge, point::Point, vector::Vector},
    meta::Mass,
    shape::utils::rotate_polygon,
};

use super::{
    utils::{
        compute_area_of_triangle, compute_convex_center_point,
        compute_moment_of_inertia_of_triangle, projection_polygon_on_vector,
        split_convex_polygon_to_triangles,
    },
    ComputeMomentOfInertia, Shape,
};

#[derive(Clone)]
pub struct ConvexPolygon {
    vertexes: Vec<Point>,
    center_point: Point,
}

impl ConvexPolygon {
    pub fn new(points: impl Into<Vec<Point>>) -> Self {
        let vertexes: Vec<_> = points.into();
        let center_point = compute_convex_center_point(&vertexes);
        Self {
            vertexes,
            center_point,
        }
    }
}

struct EdgeIter<'a> {
    index: usize,
    points: &'a [Point],
}

impl<'a> Iterator for EdgeIter<'a> {
    type Item = Edge<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let len = self.points.len();
        if self.index >= len {
            return None;
        }

        let edge = Edge::Line {
            start_point: &self.points[self.index],
            end_point: &self.points[(self.index + 1) % len],
        };

        self.index += 1;

        edge.into()
    }
}

impl Shape for ConvexPolygon {
    fn center_point(&self) -> Point {
        self.center_point
    }

    fn edge_iter(&self) -> Box<dyn Iterator<Item = Edge<'_>> + '_> {
        Box::new(EdgeIter {
            index: 0,
            points: &self.vertexes,
        })
    }

    fn projection_on_vector(&self, vector: &Vector) -> (Point, Point) {
        projection_polygon_on_vector(self.vertexes.iter(), *vector)
    }

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

impl ComputeMomentOfInertia for ConvexPolygon {
    // split into multi triangles ,compute each triangle's moment_of_inertia , sum them all
    fn compute_moment_of_inertia(&self, m: Mass) -> f32 {
        let triangles = split_convex_polygon_to_triangles(&self.vertexes);

        let total_area = triangles
            .iter()
            .fold(0., |acc, triangle| acc + compute_area_of_triangle(triangle));

        triangles.into_iter().fold(0., |acc, triangle| {
            let mass = m * compute_area_of_triangle(&triangle) / total_area;
            compute_moment_of_inertia_of_triangle(&triangle, mass) + acc
        })
    }
}
