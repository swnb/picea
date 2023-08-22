pub mod alias;
pub(crate) mod store;

use crate::{
    algo::{
        collision::{Collider, Projector, SubCollider},
        constraint::ConstraintObject,
    },
    math::{
        axis::AxisDirection,
        point::Point,
        vector::{Vector, Vector3},
        FloatNum, PI,
    },
    meta::{nail::Nail, Mass, Meta},
    shape::{CenterPoint, EdgeIterable, GeometryTransform, NearestPoint},
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
    nails: Vec<Nail>,
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
            nails: Default::default(),
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
        self.nails.iter_mut().for_each(|nail| *nail += vector);
    }

    #[inline]
    pub fn rotate(&mut self, rad: f32) {
        let center_point = &self.center_point();

        self.shape.rotate(center_point, rad);

        self.nails
            .iter_mut()
            .for_each(|nail| nail.rotate(center_point, rad));

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

    pub fn nails_iter(&self) -> impl Iterator<Item = &Nail> {
        self.nails.iter()
    }

    pub fn create_nail(&mut self, nail: impl Into<Nail>) {
        let nail = nail.into();
        self.nails.push(nail);
    }

    pub(crate) fn solve_nail_constraints(&mut self, delta_time: FloatNum) {
        let meta = self.meta();
        let mass = meta.mass();
        let inv_mass = meta.inv_mass();
        let inv_moment_of_inertia = meta.inv_moment_of_inertia();
        let center_point = self.center_point();

        let self_ptr = self as *mut Self;

        for nail in &self.nails {
            const F: f32 = 0.4;

            let normal_frequency_omega = F * PI() * 2.;

            // 胡克定律 f = kx
            let k = mass * normal_frequency_omega * normal_frequency_omega;
            // f = cv
            let c = 2. * mass * normal_frequency_omega; // * 0.1

            // (b * distance / delta_time) == position fix
            let b = k * delta_time * (c + k * delta_time).recip();
            // r is the coefficient for impulse lambda
            let r = (c + k * delta_time).recip();

            let stretch_length = nail.stretch_length();

            // NOTE  if stretch_length == 0
            let n = stretch_length.normalize();

            let r_t: Vector = (&center_point, nail.point_bind_with_element()).into();

            let inv_mass_efficiency =
                (inv_mass + (r_t ^ n).powf(2.) * inv_moment_of_inertia).recip();

            let v = self.compute_point_velocity(nail.point_bind_with_element());

            let lambda = -(v * n + b * stretch_length.abs() / delta_time)
                * (inv_mass_efficiency + r * delta_time.recip()).recip();

            let impulse = n * lambda;

            unsafe {
                (*self_ptr).meta_mut().apply_impulse(-impulse, r_t);
            }
        }
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
