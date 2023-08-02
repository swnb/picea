use super::{
    utils::{
        compute_area_of_triangle, compute_moment_of_inertia_of_triangle,
        compute_polygon_approximate_center_point, find_nearest_point, projection_polygon_on_vector,
        resize_by_vector, rotate_point, rotate_polygon, translate_polygon,
    },
    CenterPoint, EdgeIterable, GeometryTransform, NearestPoint,
};
use crate::{
    algo::collision::{Collider, Projector},
    element::{ComputeMomentOfInertia, SelfClone, ShapeTraitUnion},
    math::{axis::AxisDirection, edge::Edge, point::Point, vector::Vector, FloatNum, PI, TAU},
    meta::Mass,
};
use std::{mem::MaybeUninit, slice};

// all polygon should impl trait CommonPolygon
trait CommonPolygon {
    type PointIter<'a>: Iterator<Item = &'a Point>
    where
        Self: 'a;

    type PointIterMut<'a>: Iterator<Item = &'a mut Point>
    where
        Self: 'a;

    fn get_center_point(&self) -> Point;

    fn center_point_mut(&mut self) -> &mut Point;

    fn edge_count(&self) -> usize;

    fn point_iter(&self) -> Self::PointIter<'_>;

    fn point_iter_mut(&mut self) -> Self::PointIterMut<'_>;
}

macro_rules! impl_shape_for_common_polygon {
    (@center_point,@inner_impl) => {
        #[inline]
        fn center_point(&self) -> Point {
            self.get_center_point()
        }
    };
    (@nearest_point,@inner_impl) => {
        #[inline]
        fn nearest_point(&self, reference_point: &Point, direction: &Vector) -> Point {
            find_nearest_point(self, reference_point, direction)
        }
    };
    (@transform,@inner_impl) => {
        #[inline]
        fn translate(&mut self, vector: &Vector) {
            translate_polygon(self.point_iter_mut(), vector);
            *self.center_point_mut() += vector;
        }

        #[inline]
        fn rotate(&mut self, &origin_point: &Point, rad: f32) {
            rotate_polygon(origin_point, self.point_iter_mut(), rad);

            if origin_point != self.center_point() {
                *self.center_point_mut() = rotate_point(&self.center_point(), &origin_point, rad);
            }
        }

        fn scale(&mut self, from:&Point,to:&Point) {
            let center_point = self.center_point();
            resize_by_vector(self.point_iter_mut(), &center_point, from,to);
        }
    };
    (@edge_iter,@inner_impl) => {
        #[inline]
        fn edge_iter(&self) -> Box<dyn Iterator<Item = Edge<'_>> + '_> {
            // TODO move to normal loop, performance issue
            let iter = self.point_iter()
                    .zip(self.point_iter().skip(1).chain(self.point_iter().take(1)))
                    .map(|v| v.into());
            Box::new(iter)
        }
    };
    (@projector,@inner_impl) => {
        #[inline]
        fn projection_on_vector(&self, &vector: &Vector) -> (Point, Point) {
            projection_polygon_on_vector(self.point_iter(), vector)
        }

        #[inline]
        fn projection_on_axis(&self, axis: AxisDirection) -> (f32, f32) {
            use AxisDirection::*;
            let point_iter = self.point_iter();
            type Reducer<T> = fn((T, T), &Point<T>) -> (T, T);
            let reducer: Reducer<f32> = match axis {
                X => |mut pre, v| {
                    pre.0 = v.x().min(pre.0);
                    pre.1 = v.x().max(pre.1);
                    pre
                },
                Y => |mut pre, v| {
                    pre.0 = v.y().min(pre.0);
                    pre.1 = v.y().max(pre.1);
                    pre
                },
            };
            point_iter.fold((f32::MAX, f32::MIN), reducer)
        }
    };
    (@self_clone,@inner_impl) => {
        fn self_clone(&self) -> Box<dyn ShapeTraitUnion>{
            self.clone().into()
        }
    };
    ($struct_name:ident) => {
        impl GeometryTransform for $struct_name {
            impl_shape_for_common_polygon!(@transform, @inner_impl);
        }
        impl CenterPoint for $struct_name {
            impl_shape_for_common_polygon!(@center_point,@inner_impl);
        }
        impl NearestPoint for $struct_name {
            impl_shape_for_common_polygon!(@nearest_point,@inner_impl);
        }
        impl EdgeIterable for $struct_name {
            impl_shape_for_common_polygon!(@edge_iter,@inner_impl);
        }
        impl Projector for $struct_name {
            impl_shape_for_common_polygon!(@projector,@inner_impl);
        }
        impl Collider for $struct_name {}
        impl SelfClone for $struct_name {
            impl_shape_for_common_polygon!(@self_clone,@inner_impl);
        }
    };
    (@const,$struct_name:ident) => {
        impl<const N:usize> GeometryTransform for $struct_name<N> {
            impl_shape_for_common_polygon!(@transform,@inner_impl);
        }
        impl<const N:usize> NearestPoint for $struct_name<N> {
            impl_shape_for_common_polygon!(@nearest_point,@inner_impl);
        }
        impl<const N:usize> CenterPoint for $struct_name<N> {
            impl_shape_for_common_polygon!(@center_point,@inner_impl);
        }
        impl<const N:usize> EdgeIterable for $struct_name<N> {
            impl_shape_for_common_polygon!(@edge_iter,@inner_impl);
        }
        impl<const N:usize> Projector for $struct_name<N> {
            impl_shape_for_common_polygon!(@projector,@inner_impl);
        }
        impl<const N:usize> Collider for $struct_name<N> {}
        impl<const N:usize> SelfClone for $struct_name<N> {
            impl_shape_for_common_polygon!(@self_clone,@inner_impl);
        }
    };
}

#[derive(Clone, Debug)]
pub(crate) struct ConstPolygon<const N: usize> {
    vertexes: [Point; N],
    center_point: Point,
}

impl<const N: usize> ConstPolygon<N> {
    const EDGE_COUNT: usize = N;

    #[inline]
    pub fn new(vertexes: [Point; N]) -> Self {
        let center_point =
            compute_polygon_approximate_center_point(vertexes.iter(), vertexes.len() as f32);

        Self {
            vertexes,
            center_point,
        }
    }
}

impl<const N: usize> CommonPolygon for ConstPolygon<N> {
    type PointIter<'a> = slice::Iter<'a, Point>;

    type PointIterMut<'a> = slice::IterMut<'a, Point>;

    #[inline]
    fn get_center_point(&self) -> Point {
        self.center_point
    }

    #[inline]
    fn center_point_mut(&mut self) -> &mut Point {
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

macro_rules! impl_common_polygon {
    (@inside_impl, $($field:tt),*) => {
        type PointIter<'a> = slice::Iter<'a, Point>;

        type PointIterMut<'a> = slice::IterMut<'a, Point>;

        #[inline]
        fn get_center_point(&self) -> Point {
            self.$(
                $field.
            )*get_center_point()
        }

        #[inline]
        fn center_point_mut(&mut self) -> &mut Point {
            self.$(
                $field.
            )*center_point_mut()
        }

        #[inline]
        fn edge_count(&self) -> usize {
            self.$(
                $field.
            )*edge_count()
        }

        #[inline]
        fn point_iter(&self) -> Self::PointIter<'_> {
            self.$(
                $field.
            )*point_iter()
        }

        #[inline]
        fn point_iter_mut(&mut self) -> Self::PointIterMut<'_> {
            self.$(
                $field.
            )*point_iter_mut()
        }
    };
    (@const,$struct_name:tt, $($field:tt),*) => {
        impl<const N: usize> CommonPolygon for $struct_name<N> {
            impl_common_polygon!(@inside_impl,$($field),*);
        }

        impl_shape_for_common_polygon!(@const,$struct_name);
    };
    ($struct_name:tt, $($field:tt),+) => {
        impl CommonPolygon for $struct_name {
            impl_common_polygon!(@inside_impl,$($field),*);
        }

        impl_shape_for_common_polygon!($struct_name);
    };
}

#[derive(Clone, Debug)]
pub struct ConstRegularPolygon<const N: usize> {
    radius: f32,
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
                this.inner.point_iter_mut(),
                -Self::HALF_EDGE_ANGLE,
            );
        }

        translate_polygon(this.inner.point_iter_mut(), &center);
        *this.inner.center_point_mut() += &center;

        this
    }
}

impl_common_polygon!(@const,ConstRegularPolygon, inner);

impl<const N: usize> ComputeMomentOfInertia for ConstRegularPolygon<N> {
    fn compute_moment_of_inertia(&self, m: Mass) -> FloatNum {
        let radius = self.radius;

        0.5 * m
            * radius.powf(2.)
            * (1. - (2. / 3. * (PI() * (Self::EDGE_COUNT as f32).recip()).sin().powf(2.)))
    }
}

// common shape  Rectangle
#[derive(Clone)]
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

impl ComputeMomentOfInertia for Rect {
    fn compute_moment_of_inertia(&self, m: Mass) -> f32 {
        m * (self.width().powf(2.) + self.height().powf(2.)) * 12f32.recip()
    }
}

impl_common_polygon!(Rect, inner);

// common shape triangle
#[derive(Clone)]
pub struct Triangle {
    inner: ConstPolygon<3>,
}

impl Triangle {
    pub fn new(points: [Point; 3]) -> Self {
        let inner = ConstPolygon::new(points);
        Self { inner }
    }

    pub fn compute_area(&self) -> FloatNum {
        compute_area_of_triangle(&self.inner.vertexes)
    }
}

impl ComputeMomentOfInertia for Triangle {
    // the inertia of triangle is (1/36) * m * (a^2 + b^2 + c^2)
    fn compute_moment_of_inertia(&self, m: Mass) -> f32 {
        compute_moment_of_inertia_of_triangle(&self.inner.vertexes, m)
    }
}

#[derive(Clone)]
pub struct RegularTriangle {
    inner: ConstRegularPolygon<3>,
}

impl RegularTriangle {
    #[inline]
    pub fn new(center_point: impl Into<Point>, radius: f32) -> Self {
        Self {
            inner: ConstRegularPolygon::new(center_point, radius),
        }
    }

    #[inline]
    pub fn radius(&self) -> f32 {
        self.inner.radius
    }

    #[inline]
    pub fn point_iter(&self) -> impl Iterator<Item = &Point> {
        self.inner.point_iter()
    }
}

impl_common_polygon!(RegularTriangle, inner);

impl ComputeMomentOfInertia for RegularTriangle {
    fn compute_moment_of_inertia(&self, m: Mass) -> f32 {
        0.25 * m * self.radius().powf(2.)
    }
}

#[derive(Clone)]
pub struct Square {
    inner_rect: Rect,
}

impl Square {
    #[inline]
    pub fn new(top_left_x: f32, top_left_y: f32, size: f32) -> Self {
        let inner_rect = Rect::new(top_left_x, top_left_y, size, size);
        Self { inner_rect }
    }
}

impl_common_polygon!(Square, inner_rect);

impl ComputeMomentOfInertia for Square {
    #[inline]
    fn compute_moment_of_inertia(&self, m: Mass) -> f32 {
        self.inner_rect.compute_moment_of_inertia(m)
    }
}

#[derive(Clone)]
pub(crate) struct NormalPolygon {
    vertexes: Vec<Point>,
    center_point: Point,
}

impl NormalPolygon {
    #[inline]
    pub fn new(center_point: impl Into<Point>, vertexes: Vec<Point>) -> Self {
        let center_point = center_point.into();
        Self {
            vertexes,
            center_point,
        }
    }
}

impl CommonPolygon for NormalPolygon {
    type PointIter<'a> = slice::Iter<'a, Point>;

    type PointIterMut<'a> = slice::IterMut<'a, Point>;

    #[inline]
    fn get_center_point(&self) -> Point {
        self.center_point
    }

    #[inline]
    fn center_point_mut(&mut self) -> &mut Point {
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

#[derive(Clone)]
pub struct RegularPolygon {
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
                this.inner_polygon.point_iter_mut(),
                -edge_angle * 0.5,
            );
        }

        translate_polygon(
            this.inner_polygon.point_iter_mut(),
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

impl_common_polygon!(RegularPolygon, inner_polygon);

impl ComputeMomentOfInertia for RegularPolygon {
    fn compute_moment_of_inertia(&self, m: Mass) -> FloatNum {
        let radius = self.radius;

        let edge_count = self.inner_polygon.edge_count() as f32;

        0.5 * m * radius.powf(2.) * (1. - (2. / 3. * (PI() * edge_count.recip()).sin().powf(2.)))
    }
}
