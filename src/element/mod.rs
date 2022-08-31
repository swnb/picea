pub mod element;
pub(crate) mod store;

use crate::{
    algo::{
        collision::Element as CollisionElement,
        constraint::{update_elements_by_duration, Element as ConstraintElement},
    },
    math::{
        axis::AxisDirection,
        point::Point,
        vector::{Vector, Vector3},
    },
    meta::Meta,
    shape::{shapes::ShapeUnion, ComputeMomentOfInertia, ProjectionOnAxis, Shape},
};

type ID = u32;

// TODO refactor element builder
pub struct ElementBuilder {
    shape: ShapeUnion,
    meta: Meta,
}

impl ElementBuilder {
    pub fn new(shape: impl Into<ShapeUnion>, meta: Meta) -> Self {
        Self {
            shape: shape.into(),
            meta,
        }
    }

    pub fn shape(mut self, shape: impl Into<ShapeUnion>) -> Self {
        self.shape = shape.into();
        self
    }

    pub fn meta(mut self, meta: Meta) -> Self {
        self.meta = meta;
        self
    }
}

#[derive(Clone)]
pub struct Element {
    id: ID,
    meta: Meta,
    shape: ShapeUnion,
    center_point_cache: Option<Point<f32>>, // if shape is translate , recompute center point
}

impl Element {
    #[inline]
    pub fn id(&self) -> ID {
        self.id
    }

    pub(crate) fn inject_id(&mut self, id: ID) {
        self.id = id
    }

    #[inline]
    pub fn new(shape: impl Into<ShapeUnion>, meta: impl Into<Meta>) -> Self {
        let mut shape = shape.into();
        let mut meta = meta.into();

        shape.rotate(meta.angular());

        let moment_of_inertia = shape.compute_moment_of_inertia(meta.mass());

        meta.set_moment_of_inertia(|_| moment_of_inertia);

        let id = 0;

        Self {
            id,
            shape,
            meta,
            center_point_cache: None,
        }
    }

    #[inline]
    fn get_center_point(&self) -> Point<f32> {
        if let Some(center_point) = self.center_point_cache {
            center_point
        } else {
            let center_point = self.shape().compute_center_point();
            // center_point is pure function , but i want to do some opt for it, it is not safe
            // FIXME
            let this = self as *const _ as *mut Self;
            unsafe { (*this).center_point_cache = Some(center_point) };
            center_point
        }
    }

    #[inline]
    pub fn meta(&self) -> &Meta {
        &self.meta
    }

    #[inline]
    pub fn meta_mut(&mut self) -> &mut Meta {
        &mut self.meta
    }

    #[inline]
    pub fn translate(&mut self, vector: &Vector<f32>) {
        // translate will change center point
        self.center_point_cache = None;
        self.shape.translate(vector)
    }

    #[inline]
    pub fn rotate(&mut self, deg: f32) {
        self.shape.rotate(deg)
    }

    #[inline]
    pub fn tick(&mut self, secs: f32) {
        if !self.meta.is_fixed() {
            update_elements_by_duration(self, secs)
        }
    }

    /**
     * assume point is inside element
     */
    pub(crate) fn compute_point_velocity(&self, point: Point<f32>) -> Vector<f32> {
        let center_point = self.get_center_point();
        let w = self.meta().angular_velocity();
        let w: Vector3<_> = (0., 0., w).into();
        let r: Vector<_> = (center_point, point).into();
        let angular_velocity = w ^ r.into();
        let velocity = self.meta().velocity();
        velocity + Vector::from(angular_velocity)
    }

    fn shape(&self) -> &(impl Shape + ProjectionOnAxis) {
        &self.shape
    }
}

impl CollisionElement for Element {
    #[inline]
    fn center_point(&self) -> Point<f32> {
        self.get_center_point()
    }

    #[inline]
    fn id(&self) -> u32 {
        self.id()
    }

    #[inline]
    fn projection_on_axis(&self, axis: AxisDirection) -> (f32, f32) {
        self.shape().projection_on_axis(axis)
    }

    #[inline]
    fn projection_on_vector(&self, vector: Vector<f32>) -> (Point<f32>, Point<f32>) {
        self.shape().projection_on_vector(vector)
    }
}

impl ConstraintElement for Element {
    #[inline]
    fn translate(&mut self, vector: &Vector<f32>) {
        self.translate(vector);
    }

    #[inline]
    fn rotate(&mut self, deg: f32) {
        self.rotate(deg)
    }

    #[inline]
    fn center_point(&self) -> Point<f32> {
        self.get_center_point()
    }

    fn meta(&self) -> &Meta {
        &self.meta
    }

    fn meta_mut(&mut self) -> &mut Meta {
        &mut self.meta
    }

    fn compute_point_velocity(&self, concat_point: Point<f32>) -> Vector<f32> {
        self.compute_point_velocity(concat_point)
    }
}
