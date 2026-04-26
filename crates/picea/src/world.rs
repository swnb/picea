//! World ownership and lifecycle APIs for the stable v1 core surface.

use std::collections::BTreeMap;

mod api;
pub(crate) mod contact_state;
pub mod error;
pub(crate) mod runtime;
pub mod store;

use crate::{
    body::BodyRecord,
    collider::ColliderRecord,
    events::{SleepTransitionReason, WorldEvent},
    handles::{BodyHandle, ColliderHandle, JointHandle, WorldRevision},
    joint::JointRecord,
    math::{vector::Vector, FloatNum},
    pipeline::{broadphase::Broadphase, SimulationWorld, StepConfig, StepOutcome, StepStats},
};

use contact_state::{ContactKey, ContactRecord};
pub use error::{HandleError, TopologyError, ValidationError, WorldError};
use store::Slot;

/// World-level immutable configuration.
#[derive(Clone, Debug, PartialEq)]
pub struct WorldDesc {
    /// Default gravity vector consumed by the simulation pipeline.
    pub gravity: Vector,
    /// Enables body sleeping when the simulation pipeline supports it.
    pub enable_sleep: bool,
}

impl Default for WorldDesc {
    fn default() -> Self {
        Self {
            gravity: (0.0, 9.8).into(),
            enable_sleep: true,
        }
    }
}

/// Stable world container that owns bodies, colliders, joints, and revision state.
#[derive(Clone, Debug, Default)]
pub struct World {
    desc: WorldDesc,
    revision: WorldRevision,
    bodies: Vec<Slot<BodyRecord>>,
    free_bodies: Vec<usize>,
    colliders: Vec<Slot<ColliderRecord>>,
    free_colliders: Vec<usize>,
    joints: Vec<Slot<JointRecord>>,
    free_joints: Vec<usize>,
    last_step_stats: StepStats,
    #[allow(dead_code)]
    last_step_dt: FloatNum,
    #[allow(dead_code)]
    simulated_time: f64,
    pending_events: Vec<WorldEvent>,
    pending_wake_reasons: BTreeMap<BodyHandle, SleepTransitionReason>,
    #[allow(dead_code)]
    last_step_events: Vec<WorldEvent>,
    broadphase: Broadphase,
    active_contacts: BTreeMap<ContactKey, ContactRecord>,
    next_contact_raw: u32,
    next_manifold_raw: u32,
}

impl SimulationWorld for World {
    fn simulate_step(&mut self, config: &StepConfig) -> StepOutcome {
        crate::pipeline::step::simulate_world_step(self, config)
    }
}

fn missing_body_error(handle: BodyHandle) -> WorldError {
    WorldError::Handle(HandleError::MissingBody { handle })
}

fn stale_body_error(handle: BodyHandle) -> WorldError {
    WorldError::Handle(HandleError::StaleBody { handle })
}

fn missing_collider_error(handle: ColliderHandle) -> WorldError {
    WorldError::Handle(HandleError::MissingCollider { handle })
}

fn stale_collider_error(handle: ColliderHandle) -> WorldError {
    WorldError::Handle(HandleError::StaleCollider { handle })
}

fn missing_joint_error(handle: JointHandle) -> WorldError {
    WorldError::Handle(HandleError::MissingJoint { handle })
}

fn stale_joint_error(handle: JointHandle) -> WorldError {
    WorldError::Handle(HandleError::StaleJoint { handle })
}
