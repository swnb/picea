use crate::math::point::Point;

use super::{convex::ConvexPolygon, utils::check_is_concave};

#[derive(Default)]
pub struct ConcavePolygon {
    sub_convex_polygons: Vec<ConvexPolygon>,
}

fn split_concave_polygon(vertexes: &[Point]) -> [ConvexPolygon; 2] {
    todo!()
}

impl ConcavePolygon {
    pub fn new(vertexes: impl Into<Vec<Point>>) -> Self {
        let vertexes: Vec<_> = vertexes.into();
        let sub_convex_polygons = if check_is_concave(&vertexes) {
            vec![ConvexPolygon::new(vertexes)]
        } else {
            vec![]
        };
        Self {
            sub_convex_polygons,
        }
    }
}
