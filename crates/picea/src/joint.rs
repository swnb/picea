//! Stable joint descriptors and views for the v1 world API.

use serde::{Deserialize, Serialize};

use crate::{
    handles::{BodyHandle, JointHandle},
    math::{point::Point, FloatNum},
};

/// Descriptor for a distance-preserving joint between two bodies.
#[derive(Clone, Debug, PartialEq)]
pub struct DistanceJointDesc {
    /// First joint body endpoint.
    pub body_a: BodyHandle,
    /// Second joint body endpoint.
    pub body_b: BodyHandle,
    /// Local anchor on `body_a`.
    pub local_anchor_a: Point,
    /// Local anchor on `body_b`.
    pub local_anchor_b: Point,
    /// Target rest length for the joint.
    pub rest_length: FloatNum,
    /// Spring stiffness parameter.
    pub stiffness: FloatNum,
    /// Spring damping parameter.
    pub damping: FloatNum,
    /// User-owned opaque payload preserved by the core API.
    pub user_data: u64,
}

impl Default for DistanceJointDesc {
    fn default() -> Self {
        Self {
            body_a: BodyHandle::default(),
            body_b: BodyHandle::default(),
            local_anchor_a: Point::default(),
            local_anchor_b: Point::default(),
            rest_length: 0.0,
            stiffness: 1.0,
            damping: 0.0,
            user_data: 0,
        }
    }
}

/// Descriptor for a body-to-world anchor joint.
#[derive(Clone, Debug, PartialEq)]
pub struct WorldAnchorJointDesc {
    /// Joint body endpoint.
    pub body: BodyHandle,
    /// Local anchor on the body.
    pub local_anchor: Point,
    /// Fixed world-space anchor.
    pub world_anchor: Point,
    /// Spring stiffness parameter.
    pub stiffness: FloatNum,
    /// Spring damping parameter.
    pub damping: FloatNum,
    /// User-owned opaque payload preserved by the core API.
    pub user_data: u64,
}

impl Default for WorldAnchorJointDesc {
    fn default() -> Self {
        Self {
            body: BodyHandle::default(),
            local_anchor: Point::default(),
            world_anchor: Point::default(),
            stiffness: 1.0,
            damping: 0.0,
            user_data: 0,
        }
    }
}

/// Stable joint kind used by read-only views and debug outputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JointKind {
    /// Distance-preserving body pair joint.
    Distance,
    /// Body-to-world anchor joint.
    WorldAnchor,
}

/// Stable owned joint descriptor.
#[derive(Clone, Debug, PartialEq)]
pub enum JointDesc {
    /// Distance-preserving body pair joint.
    Distance(DistanceJointDesc),
    /// Body-to-world anchor joint.
    WorldAnchor(WorldAnchorJointDesc),
}

impl JointDesc {
    /// Returns the stable joint kind.
    pub fn kind(&self) -> JointKind {
        match self {
            Self::Distance(_) => JointKind::Distance,
            Self::WorldAnchor(_) => JointKind::WorldAnchor,
        }
    }
}

/// Partial update for a distance joint.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DistanceJointPatch {
    /// Replaces the local anchor on `body_a` when present.
    pub local_anchor_a: Option<Point>,
    /// Replaces the local anchor on `body_b` when present.
    pub local_anchor_b: Option<Point>,
    /// Replaces the rest length when present.
    pub rest_length: Option<FloatNum>,
    /// Replaces the stiffness when present.
    pub stiffness: Option<FloatNum>,
    /// Replaces the damping when present.
    pub damping: Option<FloatNum>,
    /// Replaces the user payload when present.
    pub user_data: Option<u64>,
}

/// Partial update for a world-anchor joint.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WorldAnchorJointPatch {
    /// Replaces the local anchor when present.
    pub local_anchor: Option<Point>,
    /// Replaces the world anchor when present.
    pub world_anchor: Option<Point>,
    /// Replaces the stiffness when present.
    pub stiffness: Option<FloatNum>,
    /// Replaces the damping when present.
    pub damping: Option<FloatNum>,
    /// Replaces the user payload when present.
    pub user_data: Option<u64>,
}

/// Stable joint patch enum matching the descriptor kind.
#[derive(Clone, Debug, PartialEq)]
pub enum JointPatch {
    /// Partial update for a distance joint.
    Distance(DistanceJointPatch),
    /// Partial update for a world-anchor joint.
    WorldAnchor(WorldAnchorJointPatch),
}

/// Read-only joint snapshot resolved from a world handle.
#[derive(Clone, Debug, PartialEq)]
pub struct JointView {
    handle: JointHandle,
    desc: JointDesc,
}

impl JointView {
    /// Returns the joint handle.
    pub fn handle(&self) -> JointHandle {
        self.handle
    }

    /// Returns the stable joint kind.
    pub fn kind(&self) -> JointKind {
        self.desc.kind()
    }

    /// Returns the owned joint descriptor snapshot.
    pub fn desc(&self) -> &JointDesc {
        &self.desc
    }
}

#[derive(Clone, Debug)]
pub(crate) struct JointRecord {
    pub(crate) desc: JointDesc,
}

impl JointRecord {
    pub(crate) fn from_desc(desc: JointDesc) -> Self {
        Self { desc }
    }

    pub(crate) fn body_handles(&self) -> Vec<BodyHandle> {
        match &self.desc {
            JointDesc::Distance(desc) => {
                if desc.body_a == desc.body_b {
                    vec![desc.body_a]
                } else {
                    vec![desc.body_a, desc.body_b]
                }
            }
            JointDesc::WorldAnchor(desc) => vec![desc.body],
        }
    }

    pub(crate) fn apply_patch(&mut self, patch: JointPatch) -> bool {
        match (&mut self.desc, patch) {
            (JointDesc::Distance(desc), JointPatch::Distance(patch)) => {
                if let Some(value) = patch.local_anchor_a {
                    desc.local_anchor_a = value;
                }
                if let Some(value) = patch.local_anchor_b {
                    desc.local_anchor_b = value;
                }
                if let Some(value) = patch.rest_length {
                    desc.rest_length = value;
                }
                if let Some(value) = patch.stiffness {
                    desc.stiffness = value;
                }
                if let Some(value) = patch.damping {
                    desc.damping = value;
                }
                if let Some(value) = patch.user_data {
                    desc.user_data = value;
                }
                true
            }
            (JointDesc::WorldAnchor(desc), JointPatch::WorldAnchor(patch)) => {
                if let Some(value) = patch.local_anchor {
                    desc.local_anchor = value;
                }
                if let Some(value) = patch.world_anchor {
                    desc.world_anchor = value;
                }
                if let Some(value) = patch.stiffness {
                    desc.stiffness = value;
                }
                if let Some(value) = patch.damping {
                    desc.damping = value;
                }
                if let Some(value) = patch.user_data {
                    desc.user_data = value;
                }
                true
            }
            _ => false,
        }
    }

    pub(crate) fn view(&self, handle: JointHandle) -> JointView {
        JointView {
            handle,
            desc: self.desc.clone(),
        }
    }
}
