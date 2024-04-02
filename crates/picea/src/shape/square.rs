use core::ops::{Deref, DerefMut};

use picea_macro_tools::{Deref, Shape};

use crate::impl_shape_traits_use_deref;
use crate::math::point::Point;
use crate::meta::Mass;
use crate::prelude::ComputeMomentOfInertia;

use super::Rect;

#[derive(Debug, Clone, Shape, Deref)]
pub struct Square {
    #[deref]
    rect: Rect,
}

impl Square {
    pub fn new(top_left_x: f32, top_left_y: f32, size: f32) -> Self {
        let rect = Rect::new(top_left_x, top_left_y, size, size);
        Self { rect }
    }
}

impl ComputeMomentOfInertia for Square {
    #[inline]
    fn compute_moment_of_inertia(&self, m: Mass) -> f32 {
        self.rect.compute_moment_of_inertia(m)
    }
}

impl_shape_traits_use_deref!(Square,);
