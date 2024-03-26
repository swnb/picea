use std::ops::{Deref, DerefMut};

use macro_support::Shape;

use crate::{element::ComputeMomentOfInertia, impl_shape_traits_use_deref, meta::Mass, prelude::*};

use super::{polygon::ConstPolygon, utils::VertexesIter};

// common shape  Rectangle
#[derive(Clone, Shape)]
pub struct Rect {
    width: f32,
    height: f32,
    inner: ConstPolygon<4>,
}

impl Rect {
    pub fn new(top_left_x: f32, top_left_y: f32, width: f32, height: f32) -> Self {
        let point = (top_left_x, top_left_y).into();
        let vf = Vector::<_>::from;
        Self {
            width,
            height,
            inner: ConstPolygon::<4>::new([
                point,
                point + vf((width, 0.)),
                point + vf((width, height)),
                point + vf((0., height)),
            ]),
        }
    }

    pub fn width(&self) -> f32 {
        self.width
    }

    pub fn height(&self) -> f32 {
        self.height
    }
}

impl Deref for Rect {
    type Target = ConstPolygon<4>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Rect {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl_shape_traits_use_deref!(Rect,);

impl ComputeMomentOfInertia for Rect {
    fn compute_moment_of_inertia(&self, m: Mass) -> f32 {
        m * (self.width().powf(2.) + self.height().powf(2.)) * 12f32.recip()
    }
}
