use crate::math::{edge::Edge, point::Point, vector::Vector};

use self::utils::find_nearest_point;

pub mod circle;
pub mod concave;
pub mod convex;
pub mod line;
pub mod polygon;
pub mod utils;

pub trait GeometryTransform {
    fn translate(&mut self, vector: &Vector);

    fn rotate(&mut self, origin_point: &Point, deg: f32);
}

pub trait EdgeIterable {
    fn edge_iter(&self) -> Box<dyn Iterator<Item = Edge<'_>> + '_>;
}

pub trait CenterPoint {
    fn center_point(&self) -> Point;
}

pub trait NearestPoint: EdgeIterable {
    fn nearest_point(&self, reference_point: &Point, direction: &Vector) -> Point {
        find_nearest_point(self, reference_point, direction)
    }
}
