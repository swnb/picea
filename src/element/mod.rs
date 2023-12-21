pub mod alias;
pub(crate) mod store;

use std::collections::BTreeMap;

use crate::{
    algo::collision::{Collider, Projector, SubCollider},
    constraints::ConstraintObject,
    math::{
        axis::AxisDirection,
        point::Point,
        vector::{Vector, Vector3},
        FloatNum,
    },
    meta::{Mass, Meta},
    shape::{utils::rotate_point, CenterPoint, EdgeIterable, GeometryTransform, NearestPoint},
};

pub(crate) type ID = u32;

// TODO refactor element builder
pub struct ElementBuilder {
    shape: Box<dyn ShapeTraitUnion>,
    meta: Meta,
}

pub trait ComputeMomentOfInertia {
    fn compute_moment_of_inertia(&self, m: Mass) -> f32;
}

pub trait SelfClone {
    fn self_clone(&self) -> Box<dyn ShapeTraitUnion>;
}

// TODO rename
pub trait ShapeTraitUnion:
    GeometryTransform
    + CenterPoint
    + NearestPoint
    + EdgeIterable
    + ComputeMomentOfInertia
    + Projector
    + Collider
    + SelfClone
{
}

impl<T> ShapeTraitUnion for T where
    T: GeometryTransform
        + CenterPoint
        + EdgeIterable
        + ComputeMomentOfInertia
        + Projector
        + Collider
        + NearestPoint
        + SelfClone
{
}

impl ElementBuilder {
    pub fn new(shape: impl Into<Box<dyn ShapeTraitUnion>>, meta: impl Into<Meta>) -> Self {
        let shape = shape.into();
        let meta = meta.into();
        Self { shape, meta }
    }

    pub fn shape(mut self, shape: impl Into<Box<dyn ShapeTraitUnion>>) -> Self {
        self.shape = shape.into();
        self
    }

    pub fn meta(mut self, meta: Meta) -> Self {
        self.meta = meta;
        self
    }
}

pub struct Element {
    id: ID,
    meta: Meta,
    shape: Box<dyn ShapeTraitUnion>,
    bind_points: BTreeMap<u32, Point>, // move with element
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
    pub fn new(mut shape: Box<dyn ShapeTraitUnion>, meta: impl Into<Meta>) -> Self {
        let mut meta = meta.into();

        shape.rotate(&shape.center_point(), meta.angle());

        // FIXME update moment_of_inertia when meta update
        let moment_of_inertia = shape.compute_moment_of_inertia(meta.mass());

        meta.set_moment_of_inertia(|_| moment_of_inertia);

        let id = 0;

        Self {
            id,
            shape,
            meta,
            bind_points: Default::default(),
        }
    }

    #[inline]
    pub fn center_point(&self) -> Point {
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
    pub fn translate(&mut self, vector: &Vector) {
        self.shape.translate(vector);

        self.bind_points
            .values_mut()
            .for_each(|point| *point += vector)
    }

    #[inline]
    pub fn rotate(&mut self, rad: f32) {
        let center_point = &self.center_point();

        self.shape.rotate(center_point, rad);

        self.bind_points.values_mut().for_each(|point| {
            *point = rotate_point(point, center_point, rad);
        });

        self.meta_mut().set_angle(|pre| pre - rad);
    }

    pub fn scale(&mut self, from: &Point, to: &Point) {
        self.shape.scale(from, to);
    }

    #[inline]
    pub fn tick(&mut self, secs: f32) {
        if !self.meta.is_fixed() {
            todo!();
        }
    }

    pub fn shape(&self) -> &dyn ShapeTraitUnion {
        &*self.shape
    }

    pub fn integrate_position(&mut self, delta_time: FloatNum) -> Option<(Vector, FloatNum)> {
        if self.meta().is_fixed() {
            return None;
        }
        let path = self.meta().velocity() * delta_time;
        let angle = self.meta().angle_velocity() * delta_time;

        self.translate(&path);
        // NOTE this is important, all rotate is reverse
        self.rotate(-angle);

        (path, angle).into()
    }

    pub(crate) fn create_bind_point(&mut self, id: u32, point: Point) {
        self.bind_points.insert(id, point);
    }

    pub(crate) fn get_bind_point(&self, id: u32) -> Option<&Point> {
        self.bind_points.get(&id)
    }

    pub(crate) fn remove_bind_point(&mut self, id: u32) {
        self.bind_points.remove(&id);
    }
}

impl From<ElementBuilder> for Element {
    fn from(builder: ElementBuilder) -> Self {
        Self::new(builder.shape, builder.meta)
    }
}

impl ConstraintObject for Element {
    fn center_point(&self) -> Point {
        self.shape.center_point()
    }

    fn meta(&self) -> &Meta {
        &self.meta
    }

    fn meta_mut(&mut self) -> &mut Meta {
        &mut self.meta
    }

    /// assume point is inside shape
    fn compute_point_velocity(&self, point: &Point) -> Vector {
        let meta = self.meta();
        if meta.is_fixed() {
            return (0., 0.).into();
        }

        let center_point = self.center_point();
        let r: Vector = (center_point, *point).into();
        let angle_velocity = meta.angle_velocity();
        let w: Vector3 = (0., 0., angle_velocity).into();
        let mut v: Vector = (w ^ r.into()).into();
        v += meta.velocity();

        v
    }
}

impl CenterPoint for Element {
    fn center_point(&self) -> Point {
        self.shape().center_point()
    }
}

impl NearestPoint for Element {
    fn nearest_point(&self, reference_point: &Point, direction: &Vector) -> Point {
        self.shape.nearest_point(reference_point, direction)
    }
}

impl Projector for Element {
    fn projection_on_axis(&self, axis: AxisDirection) -> (f32, f32) {
        self.shape().projection_on_axis(axis)
    }

    fn projection_on_vector(&self, vector: &Vector) -> (Point, Point) {
        self.shape().projection_on_vector(vector)
    }
}

impl Collider for Element {
    fn sub_colliders(&self) -> Option<Box<dyn Iterator<Item = &dyn SubCollider> + '_>> {
        self.shape().sub_colliders()
    }
}
