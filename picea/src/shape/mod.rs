use std::ops::Deref;

use crate::{
    math::{edge::Edge, point::Point, vector::Vector},
    prelude::FloatNum,
    scene::context::global_context,
};

pub mod alias;
pub mod circle;
pub mod concave;
pub mod convex;
pub mod line;
pub mod polygon;
pub mod rect;
pub mod triangle;
pub mod utils;

#[derive(Clone, Default, Debug)]
pub struct Transform {
    rotation: FloatNum,
    translation: Vector,
    pub(crate) is_changed: bool,
}

impl Transform {
    pub fn rotation(&self) -> FloatNum {
        self.rotation
    }

    pub fn set_rotation(&mut self, reducer: impl FnOnce(FloatNum) -> FloatNum) {
        let rotation = reducer(self.rotation);
        if rotation != self.rotation {
            self.rotation = rotation;
            self.is_changed = true;
        }
    }

    pub fn translation(&self) -> &Vector {
        &self.translation
    }

    pub fn set_translation(&mut self, reducer: impl Fn(Vector) -> Vector) {
        self.translation = reducer(self.translation);
        self.is_changed = true;
    }
}

pub trait GeometryTransformer {
    // write transform, won't update shape directly
    fn translate(&mut self, translation: &Vector) {
        self.transform_mut()
            .set_translation(|pre| pre + translation);
        self.merge_transform();
    }

    fn rotation(&mut self, rad: FloatNum) {
        self.transform_mut().set_rotation(|pre| pre + rad);
        self.merge_transform();
    }

    fn transform_mut(&mut self) -> &mut Transform;

    // update shape use current transform information
    fn apply_transform(&mut self);

    fn merge_transform(&mut self) {
        if !global_context().merge_shape_transform {
            self.apply_transform();
        }
    }
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
pub use triangle::Triangle;
