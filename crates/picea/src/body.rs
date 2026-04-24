//! Stable body types and read/write descriptors for the v1 world API.

use serde::{Deserialize, Serialize};

use crate::{
    handles::{BodyHandle, ColliderHandle, JointHandle},
    math::{point::Point, vector::Vector, FloatNum},
    world::ValidationError,
};

/// World-space position and rotation for bodies and colliders.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pose {
    translation: Vector,
    angle: FloatNum,
}

impl Pose {
    /// Creates a pose from explicit translation and rotation values.
    pub fn from_xy_angle(x: FloatNum, y: FloatNum, angle: FloatNum) -> Self {
        Self {
            translation: (x, y).into(),
            angle,
        }
    }

    /// Returns the pose translation.
    pub fn translation(&self) -> Vector {
        self.translation
    }

    /// Returns the pose translation as a point.
    pub fn point(&self) -> Point {
        self.translation.into()
    }

    /// Returns the pose rotation in radians.
    pub fn angle(&self) -> FloatNum {
        self.angle
    }

    /// Composes `other` onto this pose, producing a world-space transform.
    pub fn compose(&self, other: Pose) -> Pose {
        let rotated = other.translation.rotated(self.angle);
        Self {
            translation: self.translation + rotated,
            angle: self.angle + other.angle,
        }
    }

    /// Transforms a local-space point into world space.
    pub fn transform_point(&self, point: Point) -> Point {
        let rotated = Vector::from(point).rotated(self.angle);
        self.point() + rotated
    }

    /// Transforms a world-space point into this pose's local space.
    pub fn inverse_transform_point(&self, point: Point) -> Point {
        let local = point - self.point();
        Point::from(local.rotated(-self.angle))
    }
}

/// Stable body simulation category.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BodyType {
    /// Bodies that never integrate velocity and never receive solver write-back.
    Static,
    /// Bodies that fully participate in force integration and constraint solving.
    #[default]
    Dynamic,
    /// Bodies that are moved by the user but still act as collidable world geometry.
    Kinematic,
}

impl BodyType {
    /// Returns `true` for static bodies.
    pub const fn is_static(self) -> bool {
        matches!(self, Self::Static)
    }

    /// Returns `true` for dynamic bodies.
    pub const fn is_dynamic(self) -> bool {
        matches!(self, Self::Dynamic)
    }

    /// Returns `true` for kinematic bodies.
    pub const fn is_kinematic(self) -> bool {
        matches!(self, Self::Kinematic)
    }
}

/// Descriptor used to create a body in a [`World`](crate::world::World).
#[derive(Clone, Debug, PartialEq)]
pub struct BodyDesc {
    /// Stable body simulation category.
    pub body_type: BodyType,
    /// Initial world-space pose.
    pub pose: Pose,
    /// Initial world-space linear velocity.
    pub linear_velocity: Vector,
    /// Initial angular velocity in radians per second.
    pub angular_velocity: FloatNum,
    /// Linear damping factor consumed by the simulation pipeline.
    pub linear_damping: FloatNum,
    /// Angular damping factor consumed by the simulation pipeline.
    pub angular_damping: FloatNum,
    /// Gravity scale multiplier applied by the simulation pipeline.
    pub gravity_scale: FloatNum,
    /// Whether the body may enter a sleeping state.
    pub can_sleep: bool,
    /// Whether the body starts asleep.
    pub sleeping: bool,
    /// User-owned opaque payload preserved by the core API.
    pub user_data: u64,
}

impl Default for BodyDesc {
    fn default() -> Self {
        Self {
            body_type: BodyType::Dynamic,
            pose: Pose::default(),
            linear_velocity: Vector::default(),
            angular_velocity: 0.0,
            linear_damping: 0.0,
            angular_damping: 0.0,
            gravity_scale: 1.0,
            can_sleep: true,
            sleeping: false,
            user_data: 0,
        }
    }
}

impl BodyDesc {
    pub(crate) fn validate(&self) -> Result<(), ValidationError> {
        if !self.pose.translation().x().is_finite() {
            return Err(ValidationError::BodyDesc {
                field: "pose.translation.x",
            });
        }
        if !self.pose.translation().y().is_finite() {
            return Err(ValidationError::BodyDesc {
                field: "pose.translation.y",
            });
        }
        if !self.pose.angle().is_finite() {
            return Err(ValidationError::BodyDesc {
                field: "pose.angle",
            });
        }
        if !self.linear_velocity.x().is_finite() {
            return Err(ValidationError::BodyDesc {
                field: "linear_velocity.x",
            });
        }
        if !self.linear_velocity.y().is_finite() {
            return Err(ValidationError::BodyDesc {
                field: "linear_velocity.y",
            });
        }
        if !self.angular_velocity.is_finite() {
            return Err(ValidationError::BodyDesc {
                field: "angular_velocity",
            });
        }
        if !self.linear_damping.is_finite() || self.linear_damping < 0.0 {
            return Err(ValidationError::BodyDesc {
                field: "linear_damping",
            });
        }
        if !self.angular_damping.is_finite() || self.angular_damping < 0.0 {
            return Err(ValidationError::BodyDesc {
                field: "angular_damping",
            });
        }
        if !self.gravity_scale.is_finite() {
            return Err(ValidationError::BodyDesc {
                field: "gravity_scale",
            });
        }
        Ok(())
    }
}

/// Partial update applied to an existing body.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BodyPatch {
    /// Replaces the body type when present.
    pub body_type: Option<BodyType>,
    /// Replaces the pose when present.
    pub pose: Option<Pose>,
    /// Replaces the linear velocity when present.
    pub linear_velocity: Option<Vector>,
    /// Replaces the angular velocity when present.
    pub angular_velocity: Option<FloatNum>,
    /// Replaces the linear damping when present.
    pub linear_damping: Option<FloatNum>,
    /// Replaces the angular damping when present.
    pub angular_damping: Option<FloatNum>,
    /// Replaces the gravity scale when present.
    pub gravity_scale: Option<FloatNum>,
    /// Replaces the sleep eligibility when present.
    pub can_sleep: Option<bool>,
    /// Explicitly sets the sleeping state when present.
    pub sleeping: Option<bool>,
    /// Replaces the user payload when present.
    pub user_data: Option<u64>,
    /// Clears sleeping even if the patch does not touch `sleeping`.
    pub wake: bool,
}

impl BodyPatch {
    pub(crate) fn validate(&self) -> Result<(), ValidationError> {
        if let Some(pose) = self.pose {
            if !pose.translation().x().is_finite() {
                return Err(ValidationError::BodyPatch {
                    field: "pose.translation.x",
                });
            }
            if !pose.translation().y().is_finite() {
                return Err(ValidationError::BodyPatch {
                    field: "pose.translation.y",
                });
            }
            if !pose.angle().is_finite() {
                return Err(ValidationError::BodyPatch {
                    field: "pose.angle",
                });
            }
        }
        if let Some(linear_velocity) = self.linear_velocity {
            if !linear_velocity.x().is_finite() {
                return Err(ValidationError::BodyPatch {
                    field: "linear_velocity.x",
                });
            }
            if !linear_velocity.y().is_finite() {
                return Err(ValidationError::BodyPatch {
                    field: "linear_velocity.y",
                });
            }
        }
        if self
            .angular_velocity
            .is_some_and(|angular_velocity| !angular_velocity.is_finite())
        {
            return Err(ValidationError::BodyPatch {
                field: "angular_velocity",
            });
        }
        if self
            .linear_damping
            .is_some_and(|linear_damping| !linear_damping.is_finite() || linear_damping < 0.0)
        {
            return Err(ValidationError::BodyPatch {
                field: "linear_damping",
            });
        }
        if self
            .angular_damping
            .is_some_and(|angular_damping| !angular_damping.is_finite() || angular_damping < 0.0)
        {
            return Err(ValidationError::BodyPatch {
                field: "angular_damping",
            });
        }
        if self
            .gravity_scale
            .is_some_and(|gravity_scale| !gravity_scale.is_finite())
        {
            return Err(ValidationError::BodyPatch {
                field: "gravity_scale",
            });
        }
        Ok(())
    }
}

/// Read-only dynamic body status exposed by the stable API.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyStatusView {
    linear_velocity: Vector,
    angular_velocity: FloatNum,
    sleeping: bool,
}

impl BodyStatusView {
    /// Returns the linear velocity.
    pub fn linear_velocity(&self) -> Vector {
        self.linear_velocity
    }

    /// Returns the angular velocity.
    pub fn angular_velocity(&self) -> FloatNum {
        self.angular_velocity
    }

    /// Returns whether the body is currently sleeping.
    pub fn sleeping(&self) -> bool {
        self.sleeping
    }
}

/// Read-only body snapshot resolved from a world handle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyView {
    handle: BodyHandle,
    body_type: BodyType,
    pose: Pose,
    linear_velocity: Vector,
    angular_velocity: FloatNum,
    linear_damping: FloatNum,
    angular_damping: FloatNum,
    gravity_scale: FloatNum,
    can_sleep: bool,
    sleeping: bool,
    user_data: u64,
}

impl BodyView {
    /// Returns the body handle.
    pub fn handle(&self) -> BodyHandle {
        self.handle
    }

    /// Returns the body type.
    pub fn body_type(&self) -> BodyType {
        self.body_type
    }

    /// Returns the current pose.
    pub fn pose(&self) -> Pose {
        self.pose
    }

    /// Returns the current linear velocity.
    pub fn linear_velocity(&self) -> Vector {
        self.linear_velocity
    }

    /// Returns the current angular velocity.
    pub fn angular_velocity(&self) -> FloatNum {
        self.angular_velocity
    }

    /// Returns the linear damping factor.
    pub fn linear_damping(&self) -> FloatNum {
        self.linear_damping
    }

    /// Returns the angular damping factor.
    pub fn angular_damping(&self) -> FloatNum {
        self.angular_damping
    }

    /// Returns the gravity scale multiplier.
    pub fn gravity_scale(&self) -> FloatNum {
        self.gravity_scale
    }

    /// Returns whether the body may sleep.
    pub fn can_sleep(&self) -> bool {
        self.can_sleep
    }

    /// Returns whether the body is currently sleeping.
    pub fn sleeping(&self) -> bool {
        self.sleeping
    }

    /// Returns the opaque user payload.
    pub fn user_data(&self) -> u64 {
        self.user_data
    }

    /// Returns a focused status view for frequently changing simulation state.
    pub fn status(&self) -> BodyStatusView {
        BodyStatusView {
            linear_velocity: self.linear_velocity,
            angular_velocity: self.angular_velocity,
            sleeping: self.sleeping,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct BodyRecord {
    pub(crate) body_type: BodyType,
    pub(crate) pose: Pose,
    pub(crate) linear_velocity: Vector,
    pub(crate) angular_velocity: FloatNum,
    pub(crate) linear_damping: FloatNum,
    pub(crate) angular_damping: FloatNum,
    pub(crate) gravity_scale: FloatNum,
    pub(crate) can_sleep: bool,
    pub(crate) sleeping: bool,
    pub(crate) sleep_idle_time: FloatNum,
    pub(crate) user_data: u64,
    pub(crate) colliders: Vec<ColliderHandle>,
    pub(crate) joints: Vec<JointHandle>,
}

impl BodyRecord {
    pub(crate) fn from_desc(desc: BodyDesc) -> Self {
        let mut record = Self {
            body_type: desc.body_type,
            pose: desc.pose,
            linear_velocity: desc.linear_velocity,
            angular_velocity: desc.angular_velocity,
            linear_damping: desc.linear_damping,
            angular_damping: desc.angular_damping,
            gravity_scale: desc.gravity_scale,
            can_sleep: desc.can_sleep,
            sleeping: desc.sleeping,
            sleep_idle_time: 0.0,
            user_data: desc.user_data,
            colliders: Vec::new(),
            joints: Vec::new(),
        };
        record.normalize_motion_for_body_type();
        record
    }

    pub(crate) fn apply_patch(&mut self, patch: BodyPatch) {
        if let Some(body_type) = patch.body_type {
            self.body_type = body_type;
        }
        if let Some(pose) = patch.pose {
            self.pose = pose;
        }
        if let Some(linear_velocity) = patch.linear_velocity {
            self.linear_velocity = linear_velocity;
        }
        if let Some(angular_velocity) = patch.angular_velocity {
            self.angular_velocity = angular_velocity;
        }
        if let Some(linear_damping) = patch.linear_damping {
            self.linear_damping = linear_damping;
        }
        if let Some(angular_damping) = patch.angular_damping {
            self.angular_damping = angular_damping;
        }
        if let Some(gravity_scale) = patch.gravity_scale {
            self.gravity_scale = gravity_scale;
        }
        if let Some(can_sleep) = patch.can_sleep {
            self.can_sleep = can_sleep;
        }
        if let Some(sleeping) = patch.sleeping {
            self.sleeping = sleeping;
        }
        if let Some(user_data) = patch.user_data {
            self.user_data = user_data;
        }
        if patch.body_type.is_some()
            || patch.pose.is_some()
            || patch.linear_velocity.is_some()
            || patch.angular_velocity.is_some()
            || patch.can_sleep == Some(false)
            || patch.sleeping == Some(false)
            || patch.wake
        {
            self.sleep_idle_time = 0.0;
        }
        if patch.wake {
            self.sleeping = false;
        }
        self.normalize_motion_for_body_type();
    }

    pub(crate) fn view(&self, handle: BodyHandle) -> BodyView {
        BodyView {
            handle,
            body_type: self.body_type,
            pose: self.pose,
            linear_velocity: self.linear_velocity,
            angular_velocity: self.angular_velocity,
            linear_damping: self.linear_damping,
            angular_damping: self.angular_damping,
            gravity_scale: self.gravity_scale,
            can_sleep: self.can_sleep,
            sleeping: self.sleeping,
            user_data: self.user_data,
        }
    }

    pub(crate) fn attach_collider(&mut self, handle: ColliderHandle) {
        self.colliders.push(handle);
    }

    pub(crate) fn detach_collider(&mut self, handle: ColliderHandle) {
        self.colliders.retain(|candidate| *candidate != handle);
    }

    pub(crate) fn attach_joint(&mut self, handle: JointHandle) {
        if !self.joints.contains(&handle) {
            self.joints.push(handle);
        }
    }

    pub(crate) fn detach_joint(&mut self, handle: JointHandle) {
        self.joints.retain(|candidate| *candidate != handle);
    }

    fn normalize_motion_for_body_type(&mut self) {
        // Static bodies are the only ones modeled as truly frozen. Kinematic bodies intentionally
        // keep authored motion so the pipeline can approximate "scripted dynamic" behavior later.
        if self.body_type.is_static() {
            self.linear_velocity = Vector::default();
            self.angular_velocity = 0.0;
            self.sleeping = false;
            self.sleep_idle_time = 0.0;
        }
    }
}
