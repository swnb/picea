pub mod algo;
pub mod collision;
pub mod element;
pub mod math;
pub mod meta;
pub mod scene;
pub mod shape;
pub mod tools;

pub mod constraints;

pub mod prelude {
    pub use super::element::{
        ComputeMomentOfInertia, ElementBuilder, SelfClone, ShapeTraitUnion, ID,
    };
    pub use super::math::{point::Point, segment::Segment, vector::Vector, FloatNum};
    pub use super::meta::{Meta, MetaBuilder};
    pub use super::shape::{CenterPoint, GeometryTransformer};

    // pub use super::shape::{
    // CenterPoint, Circle, ConcavePolygon, ConstRegularPolygon, ConvexPolygon,
    // GeometryTransformer, Line, Rect, RegularPolygon, Square, Triangle,
    // };
}
