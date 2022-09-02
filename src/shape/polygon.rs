use super::{
    utils::{
        compute_polygon_center_point, projection_polygon_on_vector, rotate_polygon,
        translate_polygon,
    },
    ComputeMomentOfInertia, ProjectionOnAxis, Shape,
};
use crate::math::{axis::AxisDirection, point::Point, vector::Vector};
use std::{mem::MaybeUninit, slice};

trait CommonPolygon {
    type PointIter<'a>: Iterator<Item = &'a Point<f32>>
    where
        Self: 'a;

    type PointIterMut<'a>: Iterator<Item = &'a mut Point<f32>>
    where
        Self: 'a;

    fn center_point(&self) -> Point<f32>;

    fn center_point_mut(&mut self) -> &mut Point<f32>;

    fn edge_count(&self) -> usize;

    fn point_iter(&self) -> Self::PointIter<'_>;

    fn point_iter_mut(&mut self) -> Self::PointIterMut<'_>;
}

impl<T> Shape for T
where
    T: CommonPolygon,
{
    #[inline]
    fn center_point(&self) -> Point<f32> {
        self.center_point()
    }

    #[inline]
    fn projection_on_vector(&self, &vector: &Vector<f32>) -> (Point<f32>, Point<f32>) {
        projection_polygon_on_vector(self.point_iter(), vector)
    }

    #[inline]
    fn translate(&mut self, vector: &Vector<f32>) {
        translate_polygon(self.point_iter_mut(), vector);
        *self.center_point_mut() += vector;
    }

    #[inline]
    fn rotate(&mut self, &origin_point: &Point<f32>, deg: f32) {
        rotate_polygon(origin_point, self.point_iter_mut(), deg);
    }
}

impl<'a, T: 'a> ProjectionOnAxis for T
where
    T: CommonPolygon,
{
    fn projection_on_axis(&self, axis: AxisDirection) -> (f32, f32) {
        use AxisDirection::*;
        match axis {
            X => self.point_iter().fold((f32::MAX, f32::MIN), |mut pre, v| {
                pre.0 = v.x().min(pre.0);
                pre.1 = v.x().max(pre.1);
                pre
            }),
            Y => self.point_iter().fold((f32::MAX, f32::MIN), |mut pre, v| {
                pre.0 = v.y().min(pre.0);
                pre.1 = v.y().max(pre.1);
                pre
            }),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ConstPolygon<const N: usize> {
    vertexes: [Point<f32>; N],
    center_point: Point<f32>,
}

impl<const N: usize> ConstPolygon<N> {
    const EDGE_COUNT: usize = N;

    #[inline]
    pub fn new(vertexes: [Point<f32>; N]) -> Self {
        let center_point = compute_polygon_center_point(vertexes.iter(), vertexes.len() as f32);

        Self {
            vertexes,
            center_point,
        }
    }
}

impl<const N: usize> CommonPolygon for ConstPolygon<N> {
    type PointIter<'a> = slice::Iter<'a, Point<f32>>;

    type PointIterMut<'a> = slice::IterMut<'a, Point<f32>>;

    #[inline]
    fn center_point(&self) -> Point<f32> {
        self.center_point
    }

    #[inline]
    fn center_point_mut(&mut self) -> &mut Point<f32> {
        &mut self.center_point
    }

    #[inline]
    fn edge_count(&self) -> usize {
        Self::EDGE_COUNT
    }

    #[inline]
    fn point_iter(&self) -> Self::PointIter<'_> {
        self.vertexes.iter()
    }

    #[inline]
    fn point_iter_mut(&mut self) -> Self::PointIterMut<'_> {
        self.vertexes.iter_mut()
    }
}

#[derive(Clone, Debug)]
pub struct ConstRegularPolygon<const N: usize> {
    radius: f32,
    inner: ConstPolygon<N>,
}

impl<const N: usize> ConstRegularPolygon<N> {
    const EDGE_COUNT: usize = N;

    const EDGE_ANGLE: f32 = std::f32::consts::TAU / (N as f32);

    const IS_EVENT: bool = Self::EDGE_COUNT & 1 == 0;

    const HALF_EDGE_ANGLE: f32 = Self::EDGE_ANGLE * 0.5;

    #[inline]
    pub fn new(center: impl Into<Point<f32>>, radius: f32) -> Self {
        #[allow(clippy::uninit_assumed_init)]
        let mut vertexes: [Point<f32>; N] = unsafe { MaybeUninit::uninit().assume_init() };

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
            this.rotate(&(0., 0.).into(), -Self::HALF_EDGE_ANGLE);
        }

        this.translate(&center.into().to_vector());

        this
    }
}

impl<const N: usize> CommonPolygon for ConstRegularPolygon<N> {
    type PointIter<'a> = slice::Iter<'a, Point<f32>>;

    type PointIterMut<'a> = slice::IterMut<'a, Point<f32>>;

    #[inline]
    fn center_point(&self) -> Point<f32> {
        self.inner.center_point
    }

    #[inline]
    fn center_point_mut(&mut self) -> &mut Point<f32> {
        &mut self.inner.center_point
    }

    #[inline]
    fn edge_count(&self) -> usize {
        Self::EDGE_COUNT
    }

    #[inline]
    fn point_iter(&self) -> Self::PointIter<'_> {
        self.inner.vertexes.iter()
    }

    #[inline]
    fn point_iter_mut(&mut self) -> Self::PointIterMut<'_> {
        self.inner.vertexes.iter_mut()
    }
}

impl<const N: usize> ComputeMomentOfInertia for ConstRegularPolygon<N> {
    fn compute_moment_of_inertia(&self, m: crate::meta::Mass) -> f32 {
        use std::f32::consts::PI;

        let radius = self.radius;

        0.5 * m * radius.powf(2.) * (1. - (2. / 3. * (PI / Self::EDGE_COUNT as f32).sin().powf(2.)))
    }
}

pub type Triangle = ConstPolygon<3>;

pub type RegularTriangle = ConstRegularPolygon<3>;

pub type Rect = ConstPolygon<4>;

pub type Square = ConstRegularPolygon<4>;

pub struct NormalPolygon {
    vertexes: Vec<Point<f32>>,
    center_point: Point<f32>,
}

impl CommonPolygon for NormalPolygon {
    type PointIter<'a> = slice::Iter<'a, Point<f32>>;

    type PointIterMut<'a> = slice::IterMut<'a, Point<f32>>;

    #[inline]
    fn center_point(&self) -> Point<f32> {
        self.center_point
    }

    #[inline]
    fn center_point_mut(&mut self) -> &mut Point<f32> {
        &mut self.center_point
    }

    #[inline]
    fn edge_count(&self) -> usize {
        self.vertexes.len()
    }

    #[inline]
    fn point_iter(&self) -> Self::PointIter<'_> {
        self.vertexes.iter()
    }

    #[inline]
    fn point_iter_mut(&mut self) -> Self::PointIterMut<'_> {
        self.vertexes.iter_mut()
    }
}

pub struct RegularPolygon {
    inner_polygon: NormalPolygon,
    radius: f32,
}

impl CommonPolygon for RegularPolygon {
    type PointIter<'a> = slice::Iter<'a, Point<f32>>;

    type PointIterMut<'a> = slice::IterMut<'a, Point<f32>>;

    #[inline]
    fn center_point(&self) -> Point<f32> {
        self.inner_polygon.center_point
    }

    #[inline]
    fn center_point_mut(&mut self) -> &mut Point<f32> {
        &mut self.inner_polygon.center_point
    }

    #[inline]
    fn edge_count(&self) -> usize {
        self.inner_polygon.edge_count()
    }

    #[inline]
    fn point_iter(&self) -> Self::PointIter<'_> {
        self.inner_polygon.point_iter()
    }

    #[inline]
    fn point_iter_mut(&mut self) -> Self::PointIterMut<'_> {
        self.inner_polygon.point_iter_mut()
    }
}

impl ComputeMomentOfInertia for RegularPolygon {
    fn compute_moment_of_inertia(&self, m: crate::meta::Mass) -> f32 {
        use std::f32::consts::PI;

        let radius = self.radius;

        let edge_count = self.inner_polygon.edge_count() as f32;

        0.5 * m * radius.powf(2.) * (1. - (2. / 3. * (PI / edge_count).sin().powf(2.)))
    }
}
