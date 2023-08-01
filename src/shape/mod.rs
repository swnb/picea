use crate::{
    element::ShapeTraitUnion,
    math::{edge::Edge, point::Point, vector::Vector},
};

pub mod circle;
pub mod concave;
pub mod convex;
pub mod line;
pub mod polygon;
pub mod utils;

pub trait GeometryTransform {
    fn translate(&mut self, vector: &Vector);

    fn rotate(&mut self, origin_point: &Point, rad: f32);
}

pub trait EdgeIterable {
    fn edge_iter(&self) -> Box<dyn Iterator<Item = Edge<'_>> + '_>;
}

pub trait CenterPoint {
    fn center_point(&self) -> Point;
}

pub trait NearestPoint {
    fn nearest_point(&self, reference_point: &Point, direction: &Vector) -> Point;
}
