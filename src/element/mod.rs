pub mod alias;
pub(crate) mod store;

use crate::{
    algo::{
        collision::{Collider, Projector, SubCollider},
        constraint::ConstraintObject,
    },
    math::{
        axis::AxisDirection,
        edge::Edge,
        point::Point,
        vector::{Vector, Vector3},
        FloatNum,
    },
    meta::{Mass, Meta},
    shape::{CenterPoint, EdgeIterable, GeometryTransform, NearestPoint},
};

type ID = u32;

// TODO refactor element builder
pub struct ElementBuilder {
    shape: Box<dyn ShapeTraitUnion>,
    meta: Meta,
}

pub trait ComputeMomentOfInertia {
    fn compute_moment_of_inertia(&self, m: Mass) -> f32;
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

        shape.rotate(&shape.center_point(), meta.angular());

        // FIXME update moment_of_inertia when meta update
        let moment_of_inertia = shape.compute_moment_of_inertia(meta.mass());

        meta.set_moment_of_inertia(|_| moment_of_inertia);

        let id = 0;

        Self { id, shape, meta }
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
        self.shape.translate(vector)
    }

    #[inline]
    pub fn rotate(&mut self, deg: f32) {
        self.shape.rotate(&self.shape.center_point(), deg);
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

    pub fn integrate_velocity(&mut self, delta_time: FloatNum) {
        if self.meta().is_fixed() {
            return;
        }
        let path = self.meta().velocity() * delta_time;
        let angular = self.meta().angular_velocity() * delta_time;
        self.translate(&path);
        // NOTE this is important, all rotate is reverse
        self.rotate(-angular);
    }

    // TODO remove
    pub fn debug_shape(&self) {
        let edges: Vec<Edge> = self.shape().edge_iter().collect();
        dbg!(edges);
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
        let angular_velocity = meta.angular_velocity();
        let w: Vector3 = (0., 0., angular_velocity).into();
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

impl EdgeIterable for Element {
    fn edge_iter(&self) -> Box<dyn Iterator<Item = Edge<'_>> + '_> {
        self.shape.edge_iter()
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
