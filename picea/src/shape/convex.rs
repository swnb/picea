use macro_tools::Shape;

use crate::{
    collision::Projector,
    element::ComputeMomentOfInertia,
    impl_shape_traits_use_deref,
    math::{edge::Edge, point::Point, vector::Vector, FloatNum},
    meta::Mass,
    shape::utils::rotate_polygon,
};

use super::{
    utils::{
        compute_area_of_convex, compute_area_of_triangle, compute_convex_center_point,
        compute_moment_of_inertia_of_triangle, find_nearest_point, projection_polygon_on_vector,
        resize_by_vector, split_convex_polygon_to_triangles, VertexesIter, VertexesToEdgeIter,
    },
    CenterPoint, EdgeIterable, GeometryTransformer, NearestPoint, Transform,
};

#[derive(Clone, Shape)]
pub struct ConvexPolygon {
    origin_vertexes: Vec<Point>,
    vertexes: Vec<Point>,
    origin_center_point: Point,
    center_point: Point,
    area: FloatNum,
    transform: Transform,
}

impl ConvexPolygon {
    pub fn new(points: impl Into<Vec<Point>>) -> Self {
        let vertexes: Vec<_> = points.into();
        let center_point = compute_convex_center_point(&vertexes);
        let area = compute_area_of_convex(&vertexes);

        Self {
            origin_vertexes: vertexes.clone(),
            vertexes,
            origin_center_point: center_point,
            center_point,
            area,
            transform: Default::default(),
        }
    }

    pub fn area(&self) -> FloatNum {
        self.area
    }

    pub fn scale_with_center_point(&mut self, center_point: &Point, from: &Point, to: &Point) {
        resize_by_vector(self.vertexes.iter_mut(), center_point, from, to);
    }
}

impl VertexesIter for ConvexPolygon {
    fn vertexes_iter(&self) -> impl Iterator<Item = &Point> {
        self.vertexes.iter()
    }

    fn vertexes_iter_mut(&mut self) -> impl Iterator<Item = &mut Point> {
        self.vertexes.iter_mut()
    }
}

impl GeometryTransformer for ConvexPolygon {
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
    }
}

impl CenterPoint for ConvexPolygon {
    fn center_point(&self) -> Point {
        self.center_point
    }
}

impl EdgeIterable for ConvexPolygon {
    fn edge_iter(&self) -> Box<dyn Iterator<Item = Edge<'_>> + '_> {
        Box::new(VertexesToEdgeIter::new(&self.vertexes))
    }
}

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
