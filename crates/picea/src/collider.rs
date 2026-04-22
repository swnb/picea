//! Stable collider descriptors and shape wrappers for the v1 world API.

use serde::{Deserialize, Serialize};

use crate::{
    body::Pose,
    handles::{BodyHandle, ColliderHandle},
    math::{point::Point, vector::Vector, FloatNum},
};

/// Basic material parameters consumed by simulation, queries, and debug output.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Material {
    /// Surface friction coefficient.
    pub friction: FloatNum,
    /// Surface restitution coefficient.
    pub restitution: FloatNum,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            friction: 0.2,
            restitution: 0.0,
        }
    }
}

/// Bitmask-based collision filter used by broadphase and queries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollisionFilter {
    /// Membership bits contributed by this collider.
    pub memberships: u64,
    /// Mask bits that this collider is willing to interact with.
    pub collides_with: u64,
}

impl CollisionFilter {
    /// Returns `true` when two filters may interact.
    pub fn allows(&self, other: &Self) -> bool {
        (self.memberships & other.collides_with) != 0
            && (other.memberships & self.collides_with) != 0
    }
}

impl Default for CollisionFilter {
    fn default() -> Self {
        Self {
            memberships: u64::MAX,
            collides_with: u64::MAX,
        }
    }
}

/// Stable owned geometry descriptor used by colliders.
#[derive(Clone, Debug, PartialEq)]
pub enum SharedShape {
    /// Circle centered on the collider's local origin.
    Circle { radius: FloatNum },
    /// Axis-aligned rectangle centered on the collider's local origin.
    Rect { half_extents: Vector },
    /// Regular polygon centered on the collider's local origin.
    RegularPolygon { sides: usize, radius: FloatNum },
    /// Convex polygon expressed in local-space vertices.
    ConvexPolygon { vertices: Vec<Point> },
    /// Concave polygon expressed in local-space vertices.
    ConcavePolygon { vertices: Vec<Point> },
    /// Segment expressed in local space.
    Segment { start: Point, end: Point },
}

impl SharedShape {
    /// Creates a circle shape centered at the local origin.
    pub fn circle(radius: FloatNum) -> Self {
        Self::Circle { radius }
    }

    /// Creates a rectangle centered at the local origin.
    pub fn rect(width: FloatNum, height: FloatNum) -> Self {
        Self::Rect {
            half_extents: (width.abs() * 0.5, height.abs() * 0.5).into(),
        }
    }

    /// Creates a regular polygon centered at the local origin.
    pub fn regular_polygon(sides: usize, radius: FloatNum) -> Self {
        Self::RegularPolygon { sides, radius }
    }

    /// Creates a convex polygon from local-space vertices.
    pub fn convex_polygon(vertices: impl Into<Vec<Point>>) -> Self {
        Self::ConvexPolygon {
            vertices: vertices.into(),
        }
    }

    /// Creates a concave polygon from local-space vertices.
    pub fn concave_polygon(vertices: impl Into<Vec<Point>>) -> Self {
        Self::ConcavePolygon {
            vertices: vertices.into(),
        }
    }

    /// Creates a segment from local-space endpoints.
    pub fn segment(start: impl Into<Point>, end: impl Into<Point>) -> Self {
        Self::Segment {
            start: start.into(),
            end: end.into(),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn contains_point(&self, pose: Pose, point: Point) -> bool {
        let local_point = pose.inverse_transform_point(point);
        match self {
            Self::Circle { radius } => local_point.to_vector().abs() <= *radius,
            Self::Rect { half_extents } => {
                local_point.x().abs() <= half_extents.x()
                    && local_point.y().abs() <= half_extents.y()
            }
            Self::RegularPolygon { .. }
            | Self::ConvexPolygon { .. }
            | Self::ConcavePolygon { .. } => point_in_polygon(local_point, &self.local_vertices()),
            Self::Segment { start, end } => point_on_segment(local_point, *start, *end),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn aabb(&self, pose: Pose) -> ShapeAabb {
        match self {
            Self::Circle { radius } => ShapeAabb {
                min: (pose.point().x() - radius, pose.point().y() - radius).into(),
                max: (pose.point().x() + radius, pose.point().y() + radius).into(),
            },
            _ => ShapeAabb::from_points(self.world_vertices(pose)),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn world_vertices(&self, pose: Pose) -> Vec<Point> {
        self.local_vertices()
            .into_iter()
            .map(|point| pose.transform_point(point))
            .collect()
    }

    #[allow(dead_code)]
    fn local_vertices(&self) -> Vec<Point> {
        match self {
            Self::Circle { .. } => Vec::new(),
            Self::Rect { half_extents } => vec![
                (-half_extents.x(), -half_extents.y()).into(),
                (half_extents.x(), -half_extents.y()).into(),
                (half_extents.x(), half_extents.y()).into(),
                (-half_extents.x(), half_extents.y()).into(),
            ],
            Self::RegularPolygon { sides, radius } => {
                let sides = (*sides).max(3);
                (0..sides)
                    .map(|index| {
                        let angle = (index as FloatNum) * crate::math::tau() / sides as FloatNum;
                        let vector =
                            Vector::new(0.0, *radius).affine_transformation_rotate(angle);
                        vector.to_point()
                    })
                    .collect()
            }
            Self::ConvexPolygon { vertices } | Self::ConcavePolygon { vertices } => {
                vertices.clone()
            }
            Self::Segment { start, end } => vec![*start, *end],
        }
    }
}

/// Descriptor used to create a collider attached to a body.
#[derive(Clone, Debug, PartialEq)]
pub struct ColliderDesc {
    /// Local-space geometry definition.
    pub shape: SharedShape,
    /// Local-space transform relative to the parent body.
    pub local_pose: Pose,
    /// Density used by future mass-property adapters.
    pub density: FloatNum,
    /// Surface material parameters.
    pub material: Material,
    /// Collision filtering bits.
    pub filter: CollisionFilter,
    /// Whether this collider should skip impulse generation.
    pub is_sensor: bool,
    /// User-owned opaque payload preserved by the core API.
    pub user_data: u64,
}

impl Default for ColliderDesc {
    fn default() -> Self {
        Self {
            shape: SharedShape::circle(0.5),
            local_pose: Pose::default(),
            density: 1.0,
            material: Material::default(),
            filter: CollisionFilter::default(),
            is_sensor: false,
            user_data: 0,
        }
    }
}

/// Partial update applied to an existing collider.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ColliderPatch {
    /// Replaces the shape when present.
    pub shape: Option<SharedShape>,
    /// Replaces the local pose when present.
    pub local_pose: Option<Pose>,
    /// Replaces the density when present.
    pub density: Option<FloatNum>,
    /// Replaces the material when present.
    pub material: Option<Material>,
    /// Replaces the collision filter when present.
    pub filter: Option<CollisionFilter>,
    /// Replaces the sensor flag when present.
    pub is_sensor: Option<bool>,
    /// Replaces the user payload when present.
    pub user_data: Option<u64>,
}

/// Read-only collider snapshot resolved from a world handle.
#[derive(Clone, Debug, PartialEq)]
pub struct ColliderView {
    handle: ColliderHandle,
    body: BodyHandle,
    shape: SharedShape,
    local_pose: Pose,
    world_pose: Pose,
    density: FloatNum,
    material: Material,
    filter: CollisionFilter,
    is_sensor: bool,
    user_data: u64,
}

impl ColliderView {
    /// Returns the collider handle.
    pub fn handle(&self) -> ColliderHandle {
        self.handle
    }

    /// Returns the parent body handle.
    pub fn body(&self) -> BodyHandle {
        self.body
    }

    /// Returns the owned shape description.
    pub fn shape(&self) -> &SharedShape {
        &self.shape
    }

    /// Returns the local pose relative to the parent body.
    pub fn local_pose(&self) -> Pose {
        self.local_pose
    }

    /// Returns the resolved world pose.
    pub fn world_pose(&self) -> Pose {
        self.world_pose
    }

    /// Returns the density value.
    pub fn density(&self) -> FloatNum {
        self.density
    }

    /// Returns the material parameters.
    pub fn material(&self) -> Material {
        self.material
    }

    /// Returns the collision filter.
    pub fn filter(&self) -> CollisionFilter {
        self.filter
    }

    /// Returns whether the collider is a sensor.
    pub fn is_sensor(&self) -> bool {
        self.is_sensor
    }

    /// Returns the opaque user payload.
    pub fn user_data(&self) -> u64 {
        self.user_data
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code)]
pub(crate) struct ShapeAabb {
    pub(crate) min: Point,
    pub(crate) max: Point,
}

impl ShapeAabb {
    #[allow(dead_code)]
    pub(crate) fn from_points(points: Vec<Point>) -> Self {
        if points.is_empty() {
            return Self {
                min: Point::default(),
                max: Point::default(),
            };
        }

        let mut min_x = FloatNum::INFINITY;
        let mut min_y = FloatNum::INFINITY;
        let mut max_x = FloatNum::NEG_INFINITY;
        let mut max_y = FloatNum::NEG_INFINITY;
        for point in points {
            min_x = min_x.min(point.x());
            min_y = min_y.min(point.y());
            max_x = max_x.max(point.x());
            max_y = max_y.max(point.y());
        }
        Self {
            min: (min_x, min_y).into(),
            max: (max_x, max_y).into(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ColliderRecord {
    pub(crate) body: BodyHandle,
    pub(crate) shape: SharedShape,
    pub(crate) local_pose: Pose,
    pub(crate) density: FloatNum,
    pub(crate) material: Material,
    pub(crate) filter: CollisionFilter,
    pub(crate) is_sensor: bool,
    pub(crate) user_data: u64,
}

impl ColliderRecord {
    pub(crate) fn from_desc(body: BodyHandle, desc: ColliderDesc) -> Self {
        Self {
            body,
            shape: desc.shape,
            local_pose: desc.local_pose,
            density: desc.density,
            material: desc.material,
            filter: desc.filter,
            is_sensor: desc.is_sensor,
            user_data: desc.user_data,
        }
    }

    pub(crate) fn apply_patch(&mut self, patch: ColliderPatch) {
        if let Some(shape) = patch.shape {
            self.shape = shape;
        }
        if let Some(local_pose) = patch.local_pose {
            self.local_pose = local_pose;
        }
        if let Some(density) = patch.density {
            self.density = density;
        }
        if let Some(material) = patch.material {
            self.material = material;
        }
        if let Some(filter) = patch.filter {
            self.filter = filter;
        }
        if let Some(is_sensor) = patch.is_sensor {
            self.is_sensor = is_sensor;
        }
        if let Some(user_data) = patch.user_data {
            self.user_data = user_data;
        }
    }

    pub(crate) fn world_pose(&self, body_pose: Pose) -> Pose {
        body_pose.compose(self.local_pose)
    }

    pub(crate) fn view(&self, handle: ColliderHandle, body_pose: Pose) -> ColliderView {
        ColliderView {
            handle,
            body: self.body,
            shape: self.shape.clone(),
            local_pose: self.local_pose,
            world_pose: self.world_pose(body_pose),
            density: self.density,
            material: self.material,
            filter: self.filter,
            is_sensor: self.is_sensor,
            user_data: self.user_data,
        }
    }
}

#[allow(dead_code)]
fn point_in_polygon(point: Point, polygon: &[Point]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut previous = *polygon.last().expect("polygon has at least one vertex");
    for current in polygon.iter().copied() {
        let y_crosses = (current.y() > point.y()) != (previous.y() > point.y());
        if y_crosses {
            let denominator = previous.y() - current.y();
            if denominator.abs() > FloatNum::EPSILON {
                let x_intersection = (previous.x() - current.x()) * (point.y() - current.y())
                    / denominator
                    + current.x();
                if point.x() < x_intersection {
                    inside = !inside;
                }
            }
        }
        previous = current;
    }
    inside
}

#[allow(dead_code)]
fn point_on_segment(point: Point, start: Point, end: Point) -> bool {
    let ab: Vector = (start, end).into();
    let ap: Vector = (start, point).into();
    let cross = (ab ^ ap).abs();
    if cross > 1e-4 {
        return false;
    }
    let dot = (point.x() - start.x()) * (point.x() - end.x())
        + (point.y() - start.y()) * (point.y() - end.y());
    dot <= 0.0
}
