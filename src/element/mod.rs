pub mod alias;
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
    shape::{ComputeMomentOfInertia, Shape},
};

type ID = u32;

// TODO refactor element builder
pub struct ElementBuilder {
    shape: Box<dyn ElementShape>,
    meta: Meta,
}

pub trait ElementShape: Shape + ComputeMomentOfInertia {}
impl<T> ElementShape for T where T: Shape + ComputeMomentOfInertia {}

impl ElementBuilder {
    pub fn new(shape: impl Into<Box<dyn ElementShape>>, meta: impl Into<Meta>) -> Self {
        let shape = shape.into();
        let meta = meta.into();
        Self { shape, meta }
    }

    pub fn shape(mut self, shape: impl Into<Box<dyn ElementShape>>) -> Self {
        self.shape = shape.into();
        self
    }

    pub fn meta(mut self, meta: Meta) -> Self {
        self.meta = meta;
        self
    }
}

// #[derive(Clone)]
pub struct Element {
    id: ID,
    meta: Meta,
    shape: Box<dyn ElementShape>,
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
    pub fn new(mut shape: Box<dyn ElementShape>, meta: impl Into<Meta>) -> Self {
        let mut meta = meta.into();

        shape.rotate(&shape.center_point(), meta.angular());

        let moment_of_inertia = shape.compute_moment_of_inertia(meta.mass());

        meta.set_moment_of_inertia(|_| moment_of_inertia);

        let id = 0;

        Self { id, shape, meta }
    }

    #[inline]
    pub fn center_point(&self) -> Point<f32> {
        self.shape.center_point()
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
        self.shape.translate(vector)
    }

    #[inline]
    pub fn rotate(&mut self, deg: f32) {
        self.shape.rotate(&self.shape.center_point(), deg);
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
        let center_point = self.shape.center_point();
        let w = self.meta().angular_velocity();
        let w: Vector3<_> = (0., 0., w).into();
        let r: Vector<_> = (center_point, point).into();
        let angular_velocity = w ^ r.into();
        let velocity = self.meta().velocity();
        velocity + Vector::from(angular_velocity)
    }

    pub fn shape(&self) -> &dyn ElementShape {
        &*self.shape
    }
}

impl From<ElementBuilder> for Element {
    fn from(builder: ElementBuilder) -> Self {
        Self::new(builder.shape, builder.meta)
    }
}

impl CollisionElement for Element {
    #[inline]
    fn center_point(&self) -> Point<f32> {
        self.shape.center_point()
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
    fn projection_on_vector(&self, vector: &Vector<f32>) -> (Point<f32>, Point<f32>) {
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
        self.shape.center_point()
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
