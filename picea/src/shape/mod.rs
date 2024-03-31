use std::ops::Deref;

use crate::{
    math::{edge::Edge, point::Point, vector::Vector},
    meta::Transform,
};

pub mod alias;
pub mod circle;
pub mod concave;
pub mod convex;
pub mod line;
pub mod polygon;
pub mod rect;
pub mod square;
pub mod triangle;
pub mod utils;

pub trait GeometryTransformer {
    fn sync_transform(&mut self, transform: &Transform);
}

pub trait EdgeIterable {
    fn edge_iter(&self) -> Box<dyn Iterator<Item = Edge<'_>> + '_>;
}

pub trait CenterPoint {
    fn center_point(&self) -> Point;
}

impl<T, Z> CenterPoint for T
where
    T: Deref<Target = Z>,
    Z: CenterPoint,
{
    fn center_point(&self) -> Point {
        self.deref().center_point()
    }
}

pub trait NearestPoint {
    fn support_find_nearest_point(&self) -> bool {
        true
    }

    fn nearest_point(&self, reference_point: &Point, direction: &Vector) -> Point;
}

pub trait MeasureContactPoint {
    fn measure(&self, contact_points: Vec<Point>) -> Vec<Point> {
        contact_points
    }
}

pub use circle::Circle;
pub use concave::ConcavePolygon;
pub use convex::ConvexPolygon;
pub use line::Line;
pub use polygon::ConstRegularPolygon;
pub use polygon::RegularPolygon;
pub use rect::Rect;
pub use square::Square;
pub use triangle::Triangle;
