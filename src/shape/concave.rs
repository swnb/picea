use crate::math::point::Point;

use super::{convex::ConvexPolygon, utils::split_concave_polygon_to_convex_polygons};

#[derive(Default)]
pub struct ConcavePolygon {
    sub_convex_polygons: Vec<ConvexPolygon>,
}

impl ConcavePolygon {
    pub fn new(vertexes: &[Point]) -> Self {
        let sub_convex_polygons = split_concave_polygon_to_convex_polygons(vertexes)
            .into_iter()
            .map(ConvexPolygon::new)
            .collect();
        Self {
            sub_convex_polygons,
        }
    }
}
