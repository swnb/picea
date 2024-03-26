use std::ops::{Deref, DerefMut};

use macro_support::Deref;

use crate::{element::ComputeMomentOfInertia, impl_shape_traits_use_deref, meta::Mass, prelude::*};

use super::{
    polygon::ConstPolygon,
    utils::{compute_area_of_triangle, compute_moment_of_inertia_of_triangle},
};

// common shape triangle
#[derive(Clone, Deref)]
pub struct Triangle {
    #[deref]
    inner: ConstPolygon<3>,
}

impl_shape_traits_use_deref!(Triangle,);

impl Triangle {
    pub fn new(points: [Point; 3]) -> Self {
        let inner = ConstPolygon::new(points);
        Self { inner }
    }

    pub fn compute_area(&self) -> FloatNum {
        compute_area_of_triangle(self.inner.vertexes())
    }
}

impl ComputeMomentOfInertia for Triangle {
    // the inertia of triangle is (1/36) * m * (a^2 + b^2 + c^2)
    fn compute_moment_of_inertia(&self, m: Mass) -> f32 {
        compute_moment_of_inertia_of_triangle(self.inner.vertexes(), m)
    }
}
