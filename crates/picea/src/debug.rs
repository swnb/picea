//! Stable debug facts for Picea v1.
//!
//! This module owns the consumer-neutral read model that future lab, wasm,
//! and remote tooling can share. Structured facts describe the simulation
//! state; draw primitives are optional hints for viewers and must not be
//! treated as authoritative physics state.

use serde::{Deserialize, Serialize};

use crate::{
    body::{BodyType, Pose},
    collider::{CollisionFilter, Material, SharedShape},
    events::{ContactEvent, ContactReductionReason, WorldEvent},
    handles::{
        BodyHandle, ColliderHandle, ContactFeatureId, ContactId, JointHandle, ManifoldId,
        WorldRevision,
    },
    joint::JointDesc,
    math::{point::Point, vector::Vector, FloatNum},
    pipeline::StepReport,
    world::World,
};

/// World-space bounds that consumers can use for camera framing or coarse culling.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DebugAabb {
    /// Inclusive lower corner in world space.
    pub min: Point,
    /// Inclusive upper corner in world space.
    pub max: Point,
}

impl DebugAabb {
    /// Creates an AABB from two corners, normalizing the range and sanitizing
    /// non-finite inputs to zero.
    pub fn new(min: Point, max: Point) -> Self {
        let min_x = sanitize_scalar(min.x()).min(sanitize_scalar(max.x()));
        let max_x = sanitize_scalar(min.x()).max(sanitize_scalar(max.x()));
        let min_y = sanitize_scalar(min.y()).min(sanitize_scalar(max.y()));
        let max_y = sanitize_scalar(min.y()).max(sanitize_scalar(max.y()));

        Self {
            min: Point::new(min_x, min_y),
            max: Point::new(max_x, max_y),
        }
    }

    /// Builds an AABB around a circle.
    pub fn from_circle(center: Point, radius: FloatNum) -> Self {
        let radius = sanitize_scalar(radius).max(0.0);
        Self::new(
            Point::new(center.x() - radius, center.y() - radius),
            Point::new(center.x() + radius, center.y() + radius),
        )
    }

    /// Builds an AABB around a point cloud.
    pub fn from_points(points: &[Point]) -> Option<Self> {
        let mut points = points.iter().copied();
        let first = sanitize_point(points.next()?);
        let mut min_x = first.x();
        let mut max_x = first.x();
        let mut min_y = first.y();
        let mut max_y = first.y();

        for point in points.map(sanitize_point) {
            min_x = min_x.min(point.x());
            max_x = max_x.max(point.x());
            min_y = min_y.min(point.y());
            max_y = max_y.max(point.y());
        }

        Some(Self::new(
            Point::new(min_x, min_y),
            Point::new(max_x, max_y),
        ))
    }

    /// Returns whether a point lies inside the box.
    pub fn contains_point(&self, point: Point) -> bool {
        let point = sanitize_point(point);
        point.x() >= self.min.x()
            && point.x() <= self.max.x()
            && point.y() >= self.min.y()
            && point.y() <= self.max.y()
    }

    /// Returns whether two AABBs overlap.
    pub fn overlaps(&self, other: &Self) -> bool {
        !(self.max.x() < other.min.x()
            || other.max.x() < self.min.x()
            || self.max.y() < other.min.y()
            || other.max.y() < self.min.y())
    }
}

/// Shape facts exported in world space for query/debug consumers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DebugShape {
    /// World-space circle geometry.
    Circle { center: Point, radius: FloatNum },
    /// World-space filled polygon geometry.
    Polygon { vertices: Vec<Point> },
    /// World-space segment or capsule geometry.
    Segment {
        start: Point,
        end: Point,
        radius: FloatNum,
    },
}

impl DebugShape {
    /// Computes coarse bounds for this shape.
    pub fn aabb(&self) -> Option<DebugAabb> {
        match self {
            Self::Circle { center, radius } => Some(DebugAabb::from_circle(*center, *radius)),
            Self::Polygon { vertices } => DebugAabb::from_points(vertices),
            Self::Segment { start, end, radius } => {
                let radius = sanitize_scalar(*radius).max(0.0);
                let mut aabb = DebugAabb::from_points(&[*start, *end])?;
                aabb.min = Point::new(aabb.min.x() - radius, aabb.min.y() - radius);
                aabb.max = Point::new(aabb.max.x() + radius, aabb.max.y() + radius);
                Some(aabb)
            }
        }
    }

    pub(crate) fn sanitized(&self) -> Self {
        match self {
            Self::Circle { center, radius } => Self::Circle {
                center: sanitize_point(*center),
                radius: sanitize_scalar(*radius).max(0.0),
            },
            Self::Polygon { vertices } => Self::Polygon {
                vertices: vertices.iter().copied().map(sanitize_point).collect(),
            },
            Self::Segment { start, end, radius } => Self::Segment {
                start: sanitize_point(*start),
                end: sanitize_point(*end),
                radius: sanitize_scalar(*radius).max(0.0),
            },
        }
    }
}

/// Structured world-level metadata included with a debug snapshot.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DebugMeta {
    /// The revision that produced this snapshot, when known.
    pub revision: Option<WorldRevision>,
    /// Fixed simulation delta that produced the snapshot.
    pub dt: FloatNum,
    /// Total simulated time at the snapshot boundary.
    pub simulated_time: f64,
    /// Gravity reported by the world.
    pub gravity: Vector,
}

/// Stable statistics exported with a debug snapshot.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DebugStats {
    /// Monotonic step number seen by the world.
    pub step_index: u64,
    /// Number of bodies known to the world.
    pub active_body_count: usize,
    /// Number of colliders known to the world.
    pub active_collider_count: usize,
    /// Number of joints known to the world.
    pub active_joint_count: usize,
    /// Number of broadphase candidates considered in the last step.
    pub broadphase_candidate_count: usize,
    /// Number of broadphase proxy insert/remove/reinsert updates in the last step.
    pub broadphase_update_count: usize,
    /// Number of stale broadphase proxies dropped in the last step.
    pub broadphase_stale_proxy_drop_count: usize,
    /// Number of broadphase candidates dropped because both colliders belong to one body.
    pub broadphase_same_body_drop_count: usize,
    /// Number of broadphase candidates dropped by collision filters.
    pub broadphase_filter_drop_count: usize,
    /// Number of broadphase candidates rejected by narrowphase geometry.
    pub broadphase_narrowphase_drop_count: usize,
    /// Number of broadphase tree rebuilds in the last step.
    pub broadphase_rebuild_count: usize,
    /// Broadphase tree depth after the last step.
    pub broadphase_tree_depth: usize,
    /// Number of contacts in the last step.
    pub contact_count: usize,
    /// Number of active manifolds in the last step.
    pub manifold_count: usize,
}

/// Translation/rotation facts exported without exposing engine internals.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DebugTransform {
    /// World or local translation.
    pub translation: Vector,
    /// Rotation in radians.
    pub rotation: FloatNum,
}

/// Read-only body facts for external consumers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DebugBody {
    /// Stable body handle.
    pub handle: BodyHandle,
    /// Public body type.
    pub body_type: BodyType,
    /// World transform.
    pub transform: DebugTransform,
    /// Linear velocity in world space.
    pub linear_velocity: Vector,
    /// Angular velocity in radians per second.
    pub angular_velocity: FloatNum,
    /// Whether the body is currently sleeping.
    pub sleeping: bool,
    /// Consumer-owned opaque data.
    pub user_data: u64,
}

impl DebugBody {
    pub(crate) fn sanitized(&self) -> Self {
        Self {
            handle: self.handle,
            body_type: self.body_type,
            transform: DebugTransform {
                translation: sanitize_vector(self.transform.translation),
                rotation: sanitize_scalar(self.transform.rotation),
            },
            linear_velocity: sanitize_vector(self.linear_velocity),
            angular_velocity: sanitize_scalar(self.angular_velocity),
            sleeping: self.sleeping,
            user_data: self.user_data,
        }
    }
}

/// Read-only collider facts for external consumers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DebugCollider {
    /// Stable collider handle.
    pub handle: ColliderHandle,
    /// Owning body handle.
    pub body: BodyHandle,
    /// Local transform relative to the body.
    pub local_transform: DebugTransform,
    /// World transform at snapshot time.
    pub world_transform: DebugTransform,
    /// Coarse bounds for culling and broadphase-style queries.
    pub aabb: Option<DebugAabb>,
    /// World-space geometry facts.
    pub shape: DebugShape,
    /// Density carried by the stable collider API.
    pub density: FloatNum,
    /// Surface material parameters carried by the stable collider API.
    pub material: Material,
    /// Collision group semantics carried by the stable collider API.
    pub filter: CollisionFilter,
    /// Whether the collider is a sensor.
    pub is_sensor: bool,
    /// Consumer-owned opaque data.
    pub user_data: u64,
}

impl DebugCollider {
    pub(crate) fn sanitized(&self) -> Self {
        Self {
            handle: self.handle,
            body: self.body,
            local_transform: DebugTransform {
                translation: sanitize_vector(self.local_transform.translation),
                rotation: sanitize_scalar(self.local_transform.rotation),
            },
            world_transform: DebugTransform {
                translation: sanitize_vector(self.world_transform.translation),
                rotation: sanitize_scalar(self.world_transform.rotation),
            },
            aabb: self.aabb.map(|aabb| DebugAabb::new(aabb.min, aabb.max)),
            shape: self.shape.sanitized(),
            density: sanitize_scalar(self.density).max(0.0),
            material: sanitize_material(self.material),
            filter: self.filter,
            is_sensor: self.is_sensor,
            user_data: self.user_data,
        }
    }
}

/// Stable joint kinds reflected in debug snapshots.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DebugJointKind {
    /// A distance-preserving joint between two bodies.
    Distance,
    /// A body tied to a world-space anchor.
    WorldAnchor,
}

/// Read-only joint facts for external consumers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DebugJoint {
    /// Stable joint handle.
    pub handle: JointHandle,
    /// Stable joint kind.
    pub kind: DebugJointKind,
    /// Bodies referenced by the joint.
    pub bodies: Vec<BodyHandle>,
    /// World-space anchors used by the joint.
    pub anchors: Vec<Point>,
}

impl DebugJoint {
    pub(crate) fn sanitized(&self) -> Self {
        Self {
            handle: self.handle,
            kind: self.kind,
            bodies: self.bodies.clone(),
            anchors: self.anchors.iter().copied().map(sanitize_point).collect(),
        }
    }
}

/// Read-only contact facts for external consumers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DebugContact {
    /// Stable contact identifier.
    pub id: ContactId,
    /// Bodies touched by this contact.
    pub bodies: [BodyHandle; 2],
    /// Colliders touched by this contact.
    pub colliders: [ColliderHandle; 2],
    /// Stable geometric feature identity for this contact point.
    pub feature_id: ContactFeatureId,
    /// World-space contact point.
    pub point: Point,
    /// Contact normal pointing from collider B toward collider A.
    pub normal: Vector,
    /// Penetration depth.
    pub depth: FloatNum,
    /// Why this contact's manifold was reduced to the exported points.
    pub reduction_reason: ContactReductionReason,
    /// Cached normal impulse when available.
    pub normal_impulse: FloatNum,
    /// Cached tangent impulse when available.
    pub tangent_impulse: FloatNum,
}

impl DebugContact {
    pub(crate) fn sanitized(&self) -> Self {
        Self {
            id: self.id,
            bodies: self.bodies,
            colliders: self.colliders,
            feature_id: self.feature_id,
            point: sanitize_point(self.point),
            normal: sanitize_vector(self.normal),
            depth: sanitize_scalar(self.depth),
            reduction_reason: self.reduction_reason,
            normal_impulse: sanitize_scalar(self.normal_impulse),
            tangent_impulse: sanitize_scalar(self.tangent_impulse),
        }
    }
}

/// One point inside a debug manifold.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DebugManifoldPoint {
    /// Contact id associated with this point.
    pub contact_id: ContactId,
    /// Stable geometric feature identity for this point.
    pub feature_id: ContactFeatureId,
    /// World-space contact point.
    pub point: Point,
    /// Penetration depth for this point's manifold.
    pub depth: FloatNum,
}

impl DebugManifoldPoint {
    fn sanitized(&self) -> Self {
        Self {
            contact_id: self.contact_id,
            feature_id: self.feature_id,
            point: sanitize_point(self.point),
            depth: sanitize_scalar(self.depth),
        }
    }
}

/// Read-only manifold facts for external consumers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DebugManifold {
    /// Stable manifold identifier.
    pub id: ManifoldId,
    /// Bodies participating in this manifold.
    pub bodies: [BodyHandle; 2],
    /// Colliders participating in this manifold.
    pub colliders: [ColliderHandle; 2],
    /// Contacts currently attached to the manifold.
    pub contact_ids: Vec<ContactId>,
    /// Contact points currently attached to the manifold.
    pub points: Vec<DebugManifoldPoint>,
    /// Manifold normal pointing from collider B toward collider A.
    pub normal: Vector,
    /// Maximum penetration depth among the manifold points.
    pub depth: FloatNum,
    /// Why this manifold was reduced to the exported points.
    pub reduction_reason: ContactReductionReason,
    /// Whether the manifold is active this step.
    pub active: bool,
}

/// Stable primitive color.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugColor {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
    /// Alpha channel.
    pub a: u8,
}

impl DebugColor {
    /// Constructs an RGBA color.
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

impl Default for DebugColor {
    fn default() -> Self {
        Self::rgba(255, 255, 255, 255)
    }
}

/// Optional drawing hints for consumers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DebugPrimitive {
    /// Simple line primitive.
    Line {
        start: Point,
        end: Point,
        color: DebugColor,
    },
    /// Polyline primitive.
    Polyline {
        points: Vec<Point>,
        closed: bool,
        color: DebugColor,
    },
    /// Filled or stroked polygon primitive.
    Polygon {
        points: Vec<Point>,
        stroke: DebugColor,
        fill: Option<DebugColor>,
    },
    /// Circle primitive.
    Circle {
        center: Point,
        radius: FloatNum,
        color: DebugColor,
    },
    /// Directional arrow primitive.
    Arrow {
        origin: Point,
        direction: Vector,
        color: DebugColor,
    },
    /// Text label primitive.
    Label {
        position: Point,
        text: String,
        color: DebugColor,
    },
}

impl DebugPrimitive {
    pub(crate) fn sanitized(&self) -> Self {
        match self {
            Self::Line { start, end, color } => Self::Line {
                start: sanitize_point(*start),
                end: sanitize_point(*end),
                color: *color,
            },
            Self::Polyline {
                points,
                closed,
                color,
            } => Self::Polyline {
                points: points.iter().copied().map(sanitize_point).collect(),
                closed: *closed,
                color: *color,
            },
            Self::Polygon {
                points,
                stroke,
                fill,
            } => Self::Polygon {
                points: points.iter().copied().map(sanitize_point).collect(),
                stroke: *stroke,
                fill: *fill,
            },
            Self::Circle {
                center,
                radius,
                color,
            } => Self::Circle {
                center: sanitize_point(*center),
                radius: sanitize_scalar(*radius).max(0.0),
                color: *color,
            },
            Self::Arrow {
                origin,
                direction,
                color,
            } => Self::Arrow {
                origin: sanitize_point(*origin),
                direction: sanitize_vector(*direction),
                color: *color,
            },
            Self::Label {
                position,
                text,
                color,
            } => Self::Label {
                position: sanitize_point(*position),
                text: text.clone(),
                color: *color,
            },
        }
    }

    fn aabb(&self) -> Option<DebugAabb> {
        match self {
            Self::Line { start, end, .. } => DebugAabb::from_points(&[*start, *end]),
            Self::Polyline { points, .. } | Self::Polygon { points, .. } => {
                DebugAabb::from_points(points)
            }
            Self::Circle { center, radius, .. } => Some(DebugAabb::from_circle(*center, *radius)),
            Self::Arrow {
                origin, direction, ..
            } => DebugAabb::from_points(&[*origin, *origin + *direction]),
            Self::Label { position, .. } => Some(DebugAabb::new(*position, *position)),
        }
    }
}

/// Options that control which layers a debug snapshot should include.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugSnapshotOptions {
    /// Include contact facts.
    pub include_contacts: bool,
    /// Include manifold facts.
    pub include_manifolds: bool,
    /// Include draw primitives.
    pub include_primitives: bool,
    /// Sanitize non-finite numeric values before returning the snapshot.
    pub sanitize_non_finite: bool,
}

impl DebugSnapshotOptions {
    /// Options tuned for query cache construction.
    pub const fn for_query() -> Self {
        Self {
            include_contacts: false,
            include_manifolds: false,
            include_primitives: false,
            sanitize_non_finite: true,
        }
    }
}

impl Default for DebugSnapshotOptions {
    fn default() -> Self {
        Self {
            include_contacts: true,
            include_manifolds: true,
            include_primitives: true,
            sanitize_non_finite: true,
        }
    }
}

/// Full read-only snapshot exported by the core.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DebugSnapshot {
    /// World-level metadata.
    pub meta: DebugMeta,
    /// Body facts.
    pub bodies: Vec<DebugBody>,
    /// Collider facts.
    pub colliders: Vec<DebugCollider>,
    /// Joint facts.
    pub joints: Vec<DebugJoint>,
    /// Contact facts.
    pub contacts: Vec<DebugContact>,
    /// Manifold facts.
    pub manifolds: Vec<DebugManifold>,
    /// Viewer-oriented draw hints.
    ///
    /// The authoritative simulation state lives in the structured facts above;
    /// primitives are an optional convenience layer for renderers.
    pub primitives: Vec<DebugPrimitive>,
    /// Snapshot-level statistics.
    pub stats: DebugStats,
}

impl DebugSnapshot {
    /// Builds a stable snapshot from the authoritative world.
    ///
    /// This path can recover durable world facts such as bodies, colliders,
    /// joints, and derived draw primitives. Callers that need exact step
    /// timing or contact/manifold facts should prefer
    /// [`Self::from_world_with_step_report`].
    pub fn from_world(world: &World, options: &DebugSnapshotOptions) -> Self {
        Self::from_world_internal(world, None, options)
    }

    /// Builds a stable snapshot from the world plus an exact step report.
    ///
    /// The report carries transient step facts that the world does not retain
    /// as durable read-only state today, such as step timing and ordered
    /// contact lifecycle events.
    pub fn from_world_with_step_report(
        world: &World,
        report: &StepReport,
        options: &DebugSnapshotOptions,
    ) -> Self {
        Self::from_world_internal(world, Some(report), options)
    }

    fn from_world_internal(
        world: &World,
        report: Option<&StepReport>,
        options: &DebugSnapshotOptions,
    ) -> Self {
        let bodies = world
            .bodies()
            .filter_map(|handle| world.body(handle).ok())
            .map(|body| DebugBody {
                handle: body.handle(),
                body_type: body.body_type(),
                transform: DebugTransform {
                    translation: body.pose().translation(),
                    rotation: body.pose().angle(),
                },
                linear_velocity: body.linear_velocity(),
                angular_velocity: body.angular_velocity(),
                sleeping: body.sleeping(),
                user_data: body.user_data(),
            })
            .collect::<Vec<_>>();

        let colliders = world
            .bodies()
            .filter_map(|body| world.colliders_for_body(body).ok())
            .flatten()
            .filter_map(|handle| world.collider(handle).ok())
            .map(|collider| {
                let shape = debug_shape_from_shared_shape(collider.shape(), collider.world_pose());
                let aabb = shape.aabb();
                DebugCollider {
                    handle: collider.handle(),
                    body: collider.body(),
                    local_transform: DebugTransform {
                        translation: collider.local_pose().translation(),
                        rotation: collider.local_pose().angle(),
                    },
                    world_transform: DebugTransform {
                        translation: collider.world_pose().translation(),
                        rotation: collider.world_pose().angle(),
                    },
                    aabb,
                    shape,
                    density: collider.density(),
                    material: collider.material(),
                    filter: collider.filter(),
                    is_sensor: collider.is_sensor(),
                    user_data: collider.user_data(),
                }
            })
            .collect::<Vec<_>>();

        let joints = world
            .joints()
            .filter_map(|handle| world.joint(handle).ok())
            .map(|joint| debug_joint_from_view(world, joint.handle(), joint.desc()))
            .collect::<Vec<_>>();

        let last_step = world.last_step_stats();
        let step_events = report
            .map(|report| report.events.as_slice())
            .unwrap_or_else(|| world.last_step_events());
        let (contacts, manifolds) = debug_contacts_and_manifolds(step_events);
        let stats = report.map(|report| report.stats).unwrap_or(last_step);
        let contacts = if options.include_contacts {
            contacts
        } else {
            Vec::new()
        };
        let manifolds = if options.include_manifolds {
            manifolds
        } else {
            Vec::new()
        };
        let primitives = if options.include_primitives {
            build_debug_primitives(&colliders, &joints, &contacts)
        } else {
            Vec::new()
        };
        let snapshot = Self {
            meta: DebugMeta {
                revision: Some(report.map_or_else(|| world.revision(), |report| report.revision)),
                dt: report.map_or_else(|| world.last_step_dt(), |report| report.dt),
                simulated_time: report
                    .map_or_else(|| world.simulated_time(), |report| report.simulated_time),
                gravity: world.desc().gravity,
            },
            stats: DebugStats {
                step_index: report.map_or(last_step.step_index, |report| report.step_index),
                active_body_count: stats.body_count.max(bodies.len()),
                active_collider_count: stats.collider_count.max(colliders.len()),
                active_joint_count: stats.joint_count.max(joints.len()),
                broadphase_candidate_count: stats.broadphase_candidate_count,
                broadphase_update_count: stats.broadphase_update_count,
                broadphase_stale_proxy_drop_count: stats.broadphase_stale_proxy_drop_count,
                broadphase_same_body_drop_count: stats.broadphase_same_body_drop_count,
                broadphase_filter_drop_count: stats.broadphase_filter_drop_count,
                broadphase_narrowphase_drop_count: stats.broadphase_narrowphase_drop_count,
                broadphase_rebuild_count: stats.broadphase_rebuild_count,
                broadphase_tree_depth: stats.broadphase_tree_depth,
                contact_count: stats.contact_count.max(contacts.len()),
                manifold_count: stats.manifold_count.max(manifolds.len()),
                ..DebugStats::default()
            },
            bodies,
            colliders,
            joints,
            contacts,
            manifolds,
            primitives,
        };

        if options.sanitize_non_finite {
            snapshot.sanitized()
        } else {
            snapshot
        }
    }

    /// Returns a sanitized copy suitable for stable cross-process consumption.
    pub fn sanitized(&self) -> Self {
        Self {
            meta: DebugMeta {
                revision: self.meta.revision,
                dt: sanitize_scalar(self.meta.dt),
                simulated_time: sanitize_f64(self.meta.simulated_time),
                gravity: sanitize_vector(self.meta.gravity),
            },
            bodies: self.bodies.iter().map(DebugBody::sanitized).collect(),
            colliders: self
                .colliders
                .iter()
                .map(DebugCollider::sanitized)
                .collect(),
            joints: self.joints.iter().map(DebugJoint::sanitized).collect(),
            contacts: self.contacts.iter().map(DebugContact::sanitized).collect(),
            manifolds: self.manifolds.iter().map(sanitize_manifold).collect(),
            primitives: self
                .primitives
                .iter()
                .map(DebugPrimitive::sanitized)
                .collect(),
            stats: self.stats.clone(),
        }
    }

    /// Computes world-space bounds from structured facts and optional draw primitives.
    pub fn world_bounds(&self) -> Option<DebugAabb> {
        let mut bounds = self
            .colliders
            .iter()
            .filter_map(|collider| collider.aabb.or_else(|| collider.shape.aabb()));
        let mut primitive_bounds = self.primitives.iter().filter_map(DebugPrimitive::aabb);

        let mut aggregate = bounds.next().or_else(|| primitive_bounds.next())?;
        for aabb in bounds.chain(primitive_bounds) {
            aggregate = extend_aabb(aggregate, aabb);
        }
        Some(aggregate)
    }
}

impl Default for DebugSnapshot {
    fn default() -> Self {
        Self {
            meta: DebugMeta::default(),
            bodies: Vec::new(),
            colliders: Vec::new(),
            joints: Vec::new(),
            contacts: Vec::new(),
            manifolds: Vec::new(),
            primitives: Vec::new(),
            stats: DebugStats::default(),
        }
    }
}

fn extend_aabb(lhs: DebugAabb, rhs: DebugAabb) -> DebugAabb {
    DebugAabb::new(
        Point::new(lhs.min.x().min(rhs.min.x()), lhs.min.y().min(rhs.min.y())),
        Point::new(lhs.max.x().max(rhs.max.x()), lhs.max.y().max(rhs.max.y())),
    )
}

fn debug_contacts_and_manifolds(events: &[WorldEvent]) -> (Vec<DebugContact>, Vec<DebugManifold>) {
    let mut contacts = Vec::new();
    let mut manifolds: Vec<DebugManifold> = Vec::new();

    for event in events.iter().filter_map(active_contact_event) {
        let contact = DebugContact {
            id: event.contact_id,
            bodies: [event.body_a, event.body_b],
            colliders: [event.collider_a, event.collider_b],
            feature_id: event.feature_id,
            point: event.point,
            normal: event.normal,
            depth: event.depth,
            reduction_reason: event.reduction_reason,
            normal_impulse: 0.0,
            tangent_impulse: 0.0,
        };
        contacts.push(contact);

        if let Some(manifold) = manifolds
            .iter_mut()
            .find(|entry| entry.id == event.manifold_id)
        {
            manifold.contact_ids.push(event.contact_id);
            manifold.points.push(DebugManifoldPoint {
                contact_id: event.contact_id,
                feature_id: event.feature_id,
                point: event.point,
                depth: event.depth,
            });
            if event.depth > manifold.depth {
                manifold.depth = event.depth;
                manifold.normal = event.normal;
            }
            if event.reduction_reason == ContactReductionReason::DuplicateReduced {
                manifold.reduction_reason = event.reduction_reason;
            }
        } else {
            manifolds.push(DebugManifold {
                id: event.manifold_id,
                bodies: [event.body_a, event.body_b],
                colliders: [event.collider_a, event.collider_b],
                contact_ids: vec![event.contact_id],
                points: vec![DebugManifoldPoint {
                    contact_id: event.contact_id,
                    feature_id: event.feature_id,
                    point: event.point,
                    depth: event.depth,
                }],
                normal: event.normal,
                depth: event.depth,
                reduction_reason: event.reduction_reason,
                active: true,
            });
        }
    }

    (contacts, manifolds)
}

fn sanitize_manifold(manifold: &DebugManifold) -> DebugManifold {
    DebugManifold {
        id: manifold.id,
        bodies: manifold.bodies,
        colliders: manifold.colliders,
        contact_ids: manifold.contact_ids.clone(),
        points: manifold
            .points
            .iter()
            .map(DebugManifoldPoint::sanitized)
            .collect(),
        normal: sanitize_vector(manifold.normal),
        depth: sanitize_scalar(manifold.depth),
        reduction_reason: manifold.reduction_reason,
        active: manifold.active,
    }
}

fn active_contact_event(event: &WorldEvent) -> Option<ContactEvent> {
    match event {
        WorldEvent::ContactStarted(event) | WorldEvent::ContactPersisted(event) => Some(*event),
        _ => None,
    }
}

fn build_debug_primitives(
    colliders: &[DebugCollider],
    joints: &[DebugJoint],
    contacts: &[DebugContact],
) -> Vec<DebugPrimitive> {
    let mut primitives = Vec::new();

    for collider in colliders {
        primitives.push(debug_primitive_from_shape(&collider.shape));
    }

    for joint in joints {
        if joint.anchors.len() >= 2 {
            primitives.push(DebugPrimitive::Line {
                start: joint.anchors[0],
                end: joint.anchors[1],
                color: DebugColor::rgba(255, 196, 64, 255),
            });
        }
    }

    for contact in contacts {
        let direction = if contact.normal.length() > FloatNum::EPSILON {
            contact.normal.normalized() * contact.depth.abs().max(0.25)
        } else {
            Vector::new(0.0, 0.0)
        };
        primitives.push(DebugPrimitive::Arrow {
            origin: contact.point,
            direction,
            color: DebugColor::rgba(255, 96, 96, 255),
        });
    }

    primitives
}

fn debug_primitive_from_shape(shape: &DebugShape) -> DebugPrimitive {
    match shape {
        DebugShape::Circle { center, radius } => DebugPrimitive::Circle {
            center: *center,
            radius: *radius,
            color: DebugColor::rgba(64, 200, 255, 255),
        },
        DebugShape::Polygon { vertices } => DebugPrimitive::Polygon {
            points: vertices.clone(),
            stroke: DebugColor::rgba(64, 200, 255, 255),
            fill: None,
        },
        DebugShape::Segment { start, end, .. } => DebugPrimitive::Line {
            start: *start,
            end: *end,
            color: DebugColor::rgba(64, 200, 255, 255),
        },
    }
}

fn debug_shape_from_shared_shape(shape: &SharedShape, world_pose: Pose) -> DebugShape {
    match shape {
        SharedShape::Circle { radius } => DebugShape::Circle {
            center: world_pose.point(),
            radius: *radius,
        },
        SharedShape::Rect { half_extents } => {
            let half_width = half_extents.x();
            let half_height = half_extents.y();
            DebugShape::Polygon {
                vertices: vec![
                    transform_point(Point::new(-half_width, -half_height), world_pose),
                    transform_point(Point::new(half_width, -half_height), world_pose),
                    transform_point(Point::new(half_width, half_height), world_pose),
                    transform_point(Point::new(-half_width, half_height), world_pose),
                ],
            }
        }
        SharedShape::RegularPolygon { sides, radius } => DebugShape::Polygon {
            vertices: (0..(*sides).max(3))
                .map(|index| {
                    let angle =
                        (index as FloatNum) * crate::math::tau() / (*sides).max(3) as FloatNum;
                    let vector = Vector::new(0.0, *radius).rotated(angle);
                    transform_point(vector.into(), world_pose)
                })
                .collect(),
        },
        SharedShape::ConvexPolygon { vertices } | SharedShape::ConcavePolygon { vertices } => {
            DebugShape::Polygon {
                vertices: vertices
                    .iter()
                    .copied()
                    .map(|point| transform_point(point, world_pose))
                    .collect(),
            }
        }
        SharedShape::Segment { start, end } => DebugShape::Segment {
            start: transform_point(*start, world_pose),
            end: transform_point(*end, world_pose),
            radius: 0.0,
        },
    }
}

fn debug_joint_from_view(world: &World, handle: JointHandle, desc: &JointDesc) -> DebugJoint {
    match desc {
        JointDesc::Distance(desc) => DebugJoint {
            handle,
            kind: DebugJointKind::Distance,
            bodies: vec![desc.body_a, desc.body_b],
            anchors: vec![
                world
                    .body(desc.body_a)
                    .ok()
                    .map(|body| transform_point(desc.local_anchor_a, body.pose()))
                    .unwrap_or(desc.local_anchor_a),
                world
                    .body(desc.body_b)
                    .ok()
                    .map(|body| transform_point(desc.local_anchor_b, body.pose()))
                    .unwrap_or(desc.local_anchor_b),
            ],
        },
        JointDesc::WorldAnchor(desc) => DebugJoint {
            handle,
            kind: DebugJointKind::WorldAnchor,
            bodies: vec![desc.body],
            anchors: vec![
                world
                    .body(desc.body)
                    .ok()
                    .map(|body| transform_point(desc.local_anchor, body.pose()))
                    .unwrap_or(desc.local_anchor),
                desc.world_anchor,
            ],
        },
    }
}

fn transform_point(point: Point, pose: Pose) -> Point {
    pose.transform_point(point)
}

pub(crate) fn sanitize_point(point: Point) -> Point {
    Point::new(sanitize_scalar(point.x()), sanitize_scalar(point.y()))
}

pub(crate) fn sanitize_vector(vector: Vector) -> Vector {
    Vector::new(sanitize_scalar(vector.x()), sanitize_scalar(vector.y()))
}

pub(crate) fn sanitize_scalar(value: FloatNum) -> FloatNum {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}

fn sanitize_material(material: Material) -> Material {
    Material {
        friction: sanitize_scalar(material.friction).max(0.0),
        restitution: sanitize_scalar(material.restitution),
    }
}

fn sanitize_f64(value: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::{sanitize_scalar, DebugAabb, DebugPrimitive, DebugShape, DebugSnapshot};
    use crate::math::{point::Point, vector::Vector};

    #[test]
    fn debug_aabb_normalizes_non_finite_ranges() {
        let aabb = DebugAabb::new(Point::new(f32::NAN, 2.0), Point::new(1.0, f32::INFINITY));

        assert_eq!(aabb.min, Point::new(0.0, 0.0));
        assert_eq!(aabb.max, Point::new(1.0, 2.0));
    }

    #[test]
    fn debug_snapshot_world_bounds_merge_structured_and_draw_layers() {
        let mut snapshot = DebugSnapshot::default();
        snapshot.primitives.push(DebugPrimitive::Arrow {
            origin: Point::new(1.0, 2.0),
            direction: Vector::new(3.0, -1.0),
            color: Default::default(),
        });
        snapshot.primitives.push(DebugPrimitive::Circle {
            center: Point::new(-2.0, -1.0),
            radius: 0.5,
            color: Default::default(),
        });

        let bounds = snapshot.world_bounds().expect("bounds should exist");
        assert_eq!(bounds.min, Point::new(-2.5, -1.5));
        assert_eq!(bounds.max, Point::new(4.0, 2.0));
    }

    #[test]
    fn debug_shape_aabb_tracks_polygon_points() {
        let shape = DebugShape::Polygon {
            vertices: vec![
                Point::new(-2.0, 1.0),
                Point::new(0.0, 4.0),
                Point::new(5.0, -1.0),
            ],
        };

        let aabb = shape.aabb().expect("polygon bounds should exist");
        assert_eq!(aabb.min, Point::new(-2.0, -1.0));
        assert_eq!(aabb.max, Point::new(5.0, 4.0));
    }

    #[test]
    fn sanitize_scalar_collapses_non_finite_numbers() {
        assert_eq!(sanitize_scalar(f32::NAN), 0.0);
        assert_eq!(sanitize_scalar(f32::INFINITY), 0.0);
        assert_eq!(sanitize_scalar(-1.5), -1.5);
    }
}
