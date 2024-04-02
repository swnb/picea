pub mod algo;
pub mod collision;
pub mod constraints;
pub mod element;
pub mod math;
pub mod meta;
pub mod scene;
pub mod shape;
pub mod tools;

pub mod prelude {

    use crate::scene;

    pub use super::element::{
        ComputeMomentOfInertia, ElementBuilder, SelfClone, ShapeTraitUnion, ID,
    };

    pub use super::collision::Projector;
    pub use super::math::{edge::Edge, point::Point, segment::Segment, vector::Vector, FloatNum};
    pub use super::meta::{Mass, Meta, MetaBuilder};
    pub use super::shape::{CenterPoint, EdgeIterable, GeometryTransformer, NearestPoint};

    pub use super::constraints::{JoinConstraintConfig, JoinConstraintConfigBuilder};

    pub use scene::Scene;

    // pub use super::shape::{
    // CenterPoint, Circle, ConcavePolygon, ConstRegularPolygon, ConvexPolygon,
    // GeometryTransformer, Line, Rect, RegularPolygon, Square, Triangle,
    // };
}
