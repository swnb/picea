use std::{
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
};

use macro_tools::{Deref, Shape};

use crate::{
    element::ComputeMomentOfInertia,
    impl_shape_traits_use_deref,
    math::{PI, TAU},
    meta::Mass,
    prelude::*,
};

use super::{
    utils::{
        compute_polygon_approximate_center_point, rotate_polygon, translate_polygon,
        CenterPointHelper, VertexesIter,
    },
    CenterPoint, GeometryTransformer, Transform,
};

macro_rules! impl_shape_trait_for {
    ($struct_name:ty, $($variants:tt)*) => {
        impl<$($variants)*> CenterPoint for $struct_name {
            fn center_point(&self) -> Point {
                self.center_point
            }
        }

        impl<$($variants)*> CenterPointHelper for $struct_name {
            fn center_point_mut(&mut self) -> &mut Point {
                &mut self.center_point
            }
        }

        impl<$($variants)*>  VertexesIter for $struct_name {
            fn vertexes_iter(&self) -> impl Iterator<Item = &Point> {
                self.vertexes.iter()
            }

            fn vertexes_iter_mut(&mut self) -> impl Iterator<Item = &mut Point> {
                self.vertexes.iter_mut()
            }
        }

        impl<$($variants)*> GeometryTransformer for $struct_name {
            fn transform_mut(&mut self) -> &mut Transform {
                &mut self.transform
            }

            fn apply_transform(&mut self) {
                let translation = &self.transform.translation;


                for (i, p) in self.origin_vertexes.iter().enumerate() {
                    self.vertexes[i] = p + translation;
                }
                self.center_point = self.origin_center_point + translation;


                let rotation = self.transform.rotation;

                rotate_polygon(self.center_point, self.vertexes_iter_mut(), rotation)
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct ConstPolygon<const N: usize> {
    origin_vertexes: [Point; N],
    vertexes: [Point; N],
    origin_center_point: Point,
    center_point: Point,
    transform: Transform,
}

impl<const N: usize> ConstPolygon<N> {
    const EDGE_COUNT: usize = N;

    #[inline]
    pub fn new(vertexes: [Point; N]) -> Self {
        let center_point =
            compute_polygon_approximate_center_point(vertexes.iter(), vertexes.len() as f32);

        Self {
            origin_vertexes: vertexes,
            vertexes,
            origin_center_point: center_point,
            center_point,
            transform: Default::default(),
        }
    }

    pub fn vertexes(&self) -> &[Point; N] {
        &self.vertexes
    }
}

impl_shape_trait_for!(NormalPolygon,);

#[derive(Clone)]
pub struct NormalPolygon {
    origin_vertexes: Vec<Point>,
    vertexes: Vec<Point>,
    origin_center_point: Point,
    center_point: Point,
    transform: Transform,
}

impl_shape_trait_for!(ConstPolygon<N>, const N:usize);

impl NormalPolygon {
    pub fn new(center_point: impl Into<Point>, vertexes: Vec<Point>) -> Self {
        let center_point = center_point.into();
        Self {
            origin_vertexes: vertexes.clone(),
            vertexes,
            origin_center_point: center_point,
            center_point,
            transform: Default::default(),
        }
    }

    pub fn edge_count(&self) -> usize {
        self.vertexes.len()
    }
}

#[derive(Clone, Debug, Deref, Shape)]
pub struct ConstRegularPolygon<const N: usize> {
    radius: f32,
    #[deref]
    inner: ConstPolygon<N>,
}

impl<const N: usize> ConstRegularPolygon<N> {
    const EDGE_COUNT: usize = N;

    const EDGE_ANGLE: FloatNum = TAU() / (N as FloatNum);

    const IS_EVENT: bool = Self::EDGE_COUNT & 1 == 0;

    const HALF_EDGE_ANGLE: f32 = Self::EDGE_ANGLE * 0.5;

    #[inline]
    pub fn new(center: impl Into<Point>, radius: f32) -> Self {
        #[allow(clippy::uninit_assumed_init)]
        let mut vertexes: [Point; N] = unsafe { MaybeUninit::uninit().assume_init() };

        let center = center.into().to_vector();

        let mut point: Vector<_> = (0., radius).into();
        vertexes[0] = point.to_point();

        (1..N).for_each(|i| {
            point.affine_transformation_rotate_self(Self::EDGE_ANGLE);
            vertexes[i] = point.to_point();
        });

        let mut this = Self {
            radius,
            inner: ConstPolygon::new(vertexes),
        };

        if Self::IS_EVENT {
            rotate_polygon(
                (0., 0.).into(),
                this.inner.vertexes_iter_mut(),
                -Self::HALF_EDGE_ANGLE,
            );
        }

        translate_polygon(this.inner.vertexes_iter_mut(), &center);
        *this.inner.center_point_mut() += &center;

        this
    }
}

impl<const N: usize> ComputeMomentOfInertia for ConstRegularPolygon<N> {
    fn compute_moment_of_inertia(&self, m: Mass) -> FloatNum {
        let radius = self.radius;

        0.5 * m
            * radius.powf(2.)
            * (1. - (2. / 3. * (PI() * (Self::EDGE_COUNT as f32).recip()).sin().powf(2.)))
    }
}

impl_shape_traits_use_deref!(ConstRegularPolygon<N>,  const N:usize);

#[derive(Clone, Shape, Deref)]
pub struct RegularPolygon {
    #[deref]
    inner_polygon: NormalPolygon,
    edge_count: usize,
    edge_angle: f32,
    radius: f32,
}

impl RegularPolygon {
    pub fn new(center_point: impl Into<Point>, edge_count: usize, radius: f32) -> Self {
        let mut vertexes: Vec<Point> = Vec::with_capacity(edge_count);

        let edge_angle = TAU() * (edge_count as f32).recip();

        let mut point: Vector<_> = (0., radius).into();
        vertexes.push(point.to_point());

        (1..edge_count).for_each(|_| {
            point.affine_transformation_rotate_self(edge_angle);
            vertexes.push(point.to_point());
        });

        let center_point = center_point.into();

        let mut this = Self {
            radius,
            edge_count,
            edge_angle,
            inner_polygon: NormalPolygon::new((0., 0.), vertexes),
        };

        if edge_count & 1 == 0 {
            rotate_polygon(
                (0., 0.).into(),
                this.inner_polygon.vertexes_iter_mut(),
                -edge_angle * 0.5,
            );
        }

        translate_polygon(
            this.inner_polygon.vertexes_iter_mut(),
            &center_point.to_vector(),
        );

        *this.inner_polygon.center_point_mut() += &center_point.to_vector();

        this
    }

    #[inline]
    pub fn edge_count(&self) -> usize {
        self.edge_count
    }

    #[inline]
    pub fn edge_angle(&self) -> f32 {
        self.edge_angle
    }
}

impl_shape_traits_use_deref!(RegularPolygon,);

impl ComputeMomentOfInertia for RegularPolygon {
    fn compute_moment_of_inertia(&self, m: Mass) -> FloatNum {
        let radius = self.radius;

        let edge_count = self.inner_polygon.edge_count() as f32;

        0.5 * m * radius.powf(2.) * (1. - (2. / 3. * (PI() * edge_count.recip()).sin().powf(2.)))
    }
}
