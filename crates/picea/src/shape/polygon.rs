use std::{
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
};

use picea_macro_tools::{Deref, Shape};

use crate::{
    element::ComputeMomentOfInertia,
    impl_shape_traits_use_deref,
    math::{pi, tau},
    meta::Mass,
    prelude::*,
};

use super::{
    utils::{
        compute_polygon_approximate_center_point, rotate_polygon, translate_polygon,
        CenterPointHelper, VerticesIter, VerticesToEdgeIter,
    },
    CenterPoint, Edge, EdgeIterable, GeometryTransformer,
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

        impl<$($variants)*>  VerticesIter for $struct_name {
            fn vertices_iter(&self) -> impl Iterator<Item = &Point> {
                self.vertices.iter()
            }

            fn vertices_iter_mut(&mut self) -> impl Iterator<Item = &mut Point> {
                self.vertices.iter_mut()
            }
        }

        impl<$($variants)*> EdgeIterable for $struct_name {
            fn edge_iter(&self) -> Box<dyn Iterator<Item = Edge<'_>> + '_> {
                Box::new(VerticesToEdgeIter::new(&self.vertices))
            }
        }

        impl<$($variants)*> GeometryTransformer for $struct_name {
            fn sync_transform(&mut self,transform: &crate::meta::Transform) {
                let translation = transform.translation();

                for (i, p) in self.origin_vertices.iter().enumerate() {
                    self.vertices[i] = p + translation;
                }
                self.center_point = self.origin_center_point + translation;


                let rotation = transform.rotation();

                rotate_polygon(self.center_point, self.vertices_iter_mut(), rotation)
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct ConstPolygon<const N: usize> {
    origin_vertices: [Point; N],
    vertices: [Point; N],
    origin_center_point: Point,
    center_point: Point,
}

impl<const N: usize> ConstPolygon<N> {
    const EDGE_COUNT: usize = N;

    #[inline]
    pub fn new(vertices: [Point; N]) -> Self {
        let center_point =
            compute_polygon_approximate_center_point(vertices.iter(), vertices.len() as f32);

        Self {
            origin_vertices: vertices,
            vertices,
            origin_center_point: center_point,
            center_point,
        }
    }

    pub fn vertices(&self) -> &[Point; N] {
        &self.vertices
    }

    pub fn translate(&mut self, translation: &Vector) {
        self.center_point += translation;
        self.origin_center_point += translation;
        translate_polygon(self.origin_vertices.iter_mut(), translation);
        translate_polygon(self.vertices.iter_mut(), translation);
    }

    pub fn rotate(&mut self, rotation: FloatNum) {
        rotate_polygon(self.center_point, self.origin_vertices.iter_mut(), rotation);
        rotate_polygon(self.center_point, self.vertices.iter_mut(), rotation);
    }
}

impl_shape_trait_for!(NormalPolygon,);

#[derive(Clone)]
pub struct NormalPolygon {
    origin_vertices: Vec<Point>,
    vertices: Vec<Point>,
    origin_center_point: Point,
    center_point: Point,
}

impl_shape_trait_for!(ConstPolygon<N>, const N:usize);

impl NormalPolygon {
    pub fn new(center_point: impl Into<Point>, vertices: Vec<Point>) -> Self {
        let center_point = center_point.into();
        Self {
            origin_vertices: vertices.clone(),
            vertices,
            origin_center_point: center_point,
            center_point,
        }
    }

    pub fn edge_count(&self) -> usize {
        self.vertices.len()
    }

    pub fn translation(&mut self, translation: &Vector) {
        self.center_point += translation;
        self.origin_center_point += translation;
        translate_polygon(self.origin_vertices.iter_mut(), translation);
        translate_polygon(self.vertices.iter_mut(), translation);
    }

    pub fn rotate(&mut self, rotation: FloatNum) {
        rotate_polygon(self.center_point, self.origin_vertices.iter_mut(), rotation);
        rotate_polygon(self.center_point, self.vertices.iter_mut(), rotation);
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

    const EDGE_ANGLE: FloatNum = tau() / (N as FloatNum);

    const IS_EVENT: bool = Self::EDGE_COUNT & 1 == 0;

    const HALF_EDGE_ANGLE: f32 = Self::EDGE_ANGLE * 0.5;

    #[inline]
    pub fn new(center: impl Into<Point>, radius: f32) -> Self {
        #[allow(clippy::uninit_assumed_init)]
        let mut vertices: [Point; N] = unsafe { MaybeUninit::uninit().assume_init() };

        let center = center.into().to_vector();

        let mut point: Vector<_> = (0., radius).into();
        vertices[0] = point.to_point();

        (1..N).for_each(|i| {
            point.affine_transformation_rotate_self(Self::EDGE_ANGLE);
            vertices[i] = point.to_point();
        });

        let mut this = Self {
            radius,
            inner: ConstPolygon::new(vertices),
        };

        if Self::IS_EVENT {
            this.inner.rotate(-Self::HALF_EDGE_ANGLE)
        }

        this.inner.translate(&center);

        this
    }
}

impl<const N: usize> ComputeMomentOfInertia for ConstRegularPolygon<N> {
    fn compute_moment_of_inertia(&self, m: Mass) -> FloatNum {
        let radius = self.radius;

        0.5 * m
            * radius.powf(2.)
            * (1. - (2. / 3. * (pi() * (Self::EDGE_COUNT as f32).recip()).sin().powf(2.)))
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
        let mut vertices: Vec<Point> = Vec::with_capacity(edge_count);

        let edge_angle = tau() * (edge_count as f32).recip();

        let mut point: Vector<_> = (0., radius).into();
        vertices.push(point.to_point());

        (1..edge_count).for_each(|_| {
            point.affine_transformation_rotate_self(edge_angle);
            vertices.push(point.to_point());
        });

        let center_point = center_point.into();

        let mut this = Self {
            radius,
            edge_count,
            edge_angle,
            inner_polygon: NormalPolygon::new((0., 0.), vertices),
        };

        if edge_count & 1 == 0 {
            this.inner_polygon.rotate(-edge_angle * 0.5);
        }

        this.inner_polygon.translation(&center_point.to_vector());

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

        0.5 * m * radius.powf(2.) * (1. - (2. / 3. * (pi() * edge_count.recip()).sin().powf(2.)))
    }
}
