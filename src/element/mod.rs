pub(crate) mod store;

use std::collections::BTreeMap;

use crate::{
    collision::{Collider, Projector, SubCollider},
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
pub struct ElementBuilder<T: Clone = ()> {
    shape: Box<dyn ShapeTraitUnion>,
    meta: Meta,
    data: T,
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

impl<T: Clone> ElementBuilder<T> {
    pub fn new(shape: impl Into<Box<dyn ShapeTraitUnion>>, meta: impl Into<Meta>, data: T) -> Self {
        let shape = shape.into();
        let meta = meta.into();
        Self { shape, meta, data }
    }

    pub fn shape(mut self, shape: impl Into<Box<dyn ShapeTraitUnion>>) -> Self {
        self.shape = shape.into();
        self
    }

    pub fn meta(mut self, meta: Meta) -> Self {
        self.meta = meta;
        self
    }

    pub fn addition_data(mut self, data: T) -> Self {
        self.data = data;
        self
    }
}

pub struct Element<Data: Clone> {
    id: ID,
    meta: Meta,
    origin_shape: Box<dyn ShapeTraitUnion>,
    shape: Box<dyn ShapeTraitUnion>,
    bind_points: BTreeMap<u32, Point>, // move with element
    data: Data,
}

impl<T: Clone> Clone for Element<T> {
    fn clone(&self) -> Self {
        // clone element will return element with id unset
        Self {
            id: 0,
            meta: self.meta.clone(),
            origin_shape: self.origin_shape.self_clone(),
            shape: self.shape.self_clone(),
            bind_points: Default::default(),
            data: self.data.clone(),
        }
    }
}

impl<T: Clone> Element<T> {
    #[inline]
    pub fn id(&self) -> ID {
        self.id
    }

    pub(crate) fn inject_id(&mut self, id: ID) {
        self.id = id
    }

    #[inline]
    pub fn new(mut shape: Box<dyn ShapeTraitUnion>, meta: impl Into<Meta>, data: T) -> Self {
        let mut meta = meta.into();

        let origin_shape = shape.self_clone();

        shape.rotate(&shape.center_point(), -meta.angle());

        // FIXME update moment_of_inertia when meta update
        let moment_of_inertia = shape.compute_moment_of_inertia(meta.mass());

        meta.set_moment_of_inertia(|_| moment_of_inertia);

        let id = 0;

        Self {
            id,
            shape,
            origin_shape,
            meta,
            bind_points: Default::default(),
            data,
        }
    }

    pub(crate) fn refresh_shape(&mut self) {
        let mut origin_shape = self.origin_shape.self_clone();
        origin_shape.rotate(&origin_shape.center_point(), -self.meta().angle());
        origin_shape.translate(self.meta().position_translate());
        self.shape = origin_shape;
    }

    pub fn origin_shape(&self) -> &dyn ShapeTraitUnion {
        self.origin_shape.as_ref()
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

        self.meta_mut().translate_position(vector);

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

    // simple integrate position by velocity and angle_velocity;
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

impl<T: Clone> From<ElementBuilder<T>> for Element<T> {
    fn from(builder: ElementBuilder<T>) -> Self {
        Self::new(builder.shape, builder.meta, builder.data)
    }
}

impl<T: Clone> ConstraintObject for Element<T> {
    fn id(&self) -> ID {
        self.id
    }

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

    // apply position fix for obj , sum total position fix
    // in order to separate object from contact
    fn apply_position_fix(&mut self, fix: Vector, r: Vector) {
        if self.meta().is_fixed() {
            return;
        }

        let inv_mass = self.meta().inv_mass();

        let mut translate_fix = fix * inv_mass;
        // translate_fix.set_x(|_| 0.);

        self.translate(&translate_fix);

        let rad = (r ^ fix) * self.meta().inv_moment_of_inertia();

        // if self.id == 2 {
        //     dbg!(rad);
        //     dbg!(fix * inv_mass);
        // }

        self.rotate(-rad);

        self.refresh_shape();
    }
}

impl<T: Clone> CenterPoint for Element<T> {
    fn center_point(&self) -> Point {
        self.shape().center_point()
    }
}

impl<T: Clone> NearestPoint for Element<T> {
    fn support_find_nearest_point(&self) -> bool {
        self.shape().support_find_nearest_point()
    }

    fn nearest_point(&self, reference_point: &Point, direction: &Vector) -> Point {
        self.shape.nearest_point(reference_point, direction)
    }
}

impl<T: Clone> Projector for Element<T> {
    fn projection_on_axis(&self, axis: AxisDirection) -> (f32, f32) {
        self.shape().projection_on_axis(axis)
    }

    fn projection_on_vector(&self, vector: &Vector) -> (Point, Point) {
        self.shape().projection_on_vector(vector)
    }
}

impl<T: Clone> Collider for Element<T> {
    fn sub_colliders(&self) -> Option<Box<dyn Iterator<Item = &dyn SubCollider> + '_>> {
        self.shape().sub_colliders()
    }
}
