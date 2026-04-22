//! World ownership and lifecycle APIs for the stable v1 core surface.

use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

use crate::{
    body::{BodyDesc, BodyPatch, BodyRecord, BodyType, BodyView, Pose},
    collider::{
        ColliderDesc, ColliderPatch, ColliderRecord, ColliderView, CollisionFilter, ShapeAabb,
        SharedShape,
    },
    debug::{DebugSnapshot, DebugSnapshotOptions},
    events::{ContactEvent, SleepEvent, WorldEvent},
    handles::{BodyHandle, ColliderHandle, ContactId, JointHandle, ManifoldId, WorldRevision},
    joint::{JointDesc, JointPatch, JointRecord, JointView},
    math::{point::Point, vector::Vector, FloatNum},
    pipeline::{SimulationWorld, StepConfig, StepOutcome, StepStats},
};

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

/// Errors surfaced by stable world lifecycle APIs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorldError {
    /// The provided body descriptor contains a non-finite public input.
    InvalidBodyDesc { field: &'static str },
    /// The provided body patch contains a non-finite public input.
    InvalidBodyPatch { field: &'static str },
    /// The provided body handle never referred to a slot in this world.
    MissingBodyHandle { handle: BodyHandle },
    /// The provided body handle refers to a recycled slot generation.
    StaleBodyHandle { handle: BodyHandle },
    /// The provided collider handle never referred to a slot in this world.
    MissingColliderHandle { handle: ColliderHandle },
    /// The provided collider handle refers to a recycled slot generation.
    StaleColliderHandle { handle: ColliderHandle },
    /// The provided joint handle never referred to a slot in this world.
    MissingJointHandle { handle: JointHandle },
    /// The provided joint handle refers to a recycled slot generation.
    StaleJointHandle { handle: JointHandle },
    /// The provided joint patch does not match the stored joint kind.
    JointKindMismatch { handle: JointHandle },
}

impl fmt::Display for WorldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBodyDesc { .. } => {
                f.write_str("body descriptor contains a non-finite input")
            }
            Self::InvalidBodyPatch { .. } => f.write_str("body patch contains a non-finite input"),
            Self::MissingBodyHandle { .. } => {
                f.write_str("body handle does not belong to this world")
            }
            Self::StaleBodyHandle { .. } => {
                f.write_str("body handle refers to a recycled body slot")
            }
            Self::MissingColliderHandle { .. } => {
                f.write_str("collider handle does not belong to this world")
            }
            Self::StaleColliderHandle { .. } => {
                f.write_str("collider handle refers to a recycled collider slot")
            }
            Self::MissingJointHandle { .. } => {
                f.write_str("joint handle does not belong to this world")
            }
            Self::StaleJointHandle { .. } => {
                f.write_str("joint handle refers to a recycled joint slot")
            }
            Self::JointKindMismatch { .. } => {
                f.write_str("joint patch kind does not match the stored joint")
            }
        }
    }
}

impl Error for WorldError {}

#[derive(Clone, Debug)]
struct Slot<T> {
    generation: u32,
    value: Option<T>,
}

impl<T> Default for Slot<T> {
    fn default() -> Self {
        Self {
            generation: 0,
            value: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ContactKey {
    collider_a: ColliderHandle,
    collider_b: ColliderHandle,
}

#[derive(Clone, Debug)]
struct ContactRecord {
    contact: ContactEvent,
}

#[derive(Clone, Debug)]
struct ContactObservation {
    key: ContactKey,
    body_a: BodyHandle,
    body_b: BodyHandle,
    collider_a: ColliderHandle,
    collider_b: ColliderHandle,
    point: Point,
    normal: Vector,
    depth: FloatNum,
    is_sensor: bool,
}

#[derive(Clone, Debug)]
struct ColliderSnapshot {
    handle: ColliderHandle,
    body: BodyHandle,
    world_pose: Pose,
    shape: SharedShape,
    filter: CollisionFilter,
    is_sensor: bool,
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
    #[allow(dead_code)]
    last_step_events: Vec<WorldEvent>,
    active_contacts: BTreeMap<ContactKey, ContactRecord>,
    next_contact_raw: u32,
    next_manifold_raw: u32,
}

impl World {
    /// Creates a new world using the provided immutable configuration.
    pub fn new(desc: WorldDesc) -> Self {
        Self {
            desc,
            revision: WorldRevision::default(),
            bodies: Vec::new(),
            free_bodies: Vec::new(),
            colliders: Vec::new(),
            free_colliders: Vec::new(),
            joints: Vec::new(),
            free_joints: Vec::new(),
            last_step_stats: StepStats::default(),
            last_step_dt: 0.0,
            simulated_time: 0.0,
            pending_events: Vec::new(),
            last_step_events: Vec::new(),
            active_contacts: BTreeMap::new(),
            next_contact_raw: 0,
            next_manifold_raw: 0,
        }
    }

    /// Returns the immutable world description.
    pub fn desc(&self) -> &WorldDesc {
        &self.desc
    }

    /// Returns the current world revision.
    pub fn revision(&self) -> WorldRevision {
        self.revision
    }

    /// Creates a new body and returns its opaque handle.
    pub fn create_body(&mut self, desc: BodyDesc) -> Result<BodyHandle, WorldError> {
        if let Err(field) = desc.validate() {
            return Err(WorldError::InvalidBodyDesc { field });
        }
        let handle = allocate_slot(
            &mut self.bodies,
            &mut self.free_bodies,
            BodyHandle::from_raw_parts,
            BodyRecord::from_desc(desc),
        );
        self.pending_events
            .push(WorldEvent::BodyCreated { body: handle });
        self.bump_revision();
        Ok(handle)
    }

    /// Destroys a body and cascades removal to all attached colliders and joints.
    pub fn destroy_body(&mut self, handle: BodyHandle) -> Result<(), WorldError> {
        let (attached_colliders, attached_joints) = {
            let record = self.body_record(handle)?;
            (record.colliders.clone(), record.joints.clone())
        };
        for collider in attached_colliders {
            self.destroy_collider_internal(collider)?;
        }
        for joint in attached_joints {
            if self.joint_record(joint).is_ok() {
                self.destroy_joint_internal(joint)?;
            }
        }
        self.pending_events
            .push(WorldEvent::BodyRemoved { body: handle });
        remove_slot(
            &mut self.bodies,
            &mut self.free_bodies,
            handle,
            WorldError::MissingBodyHandle { handle },
            WorldError::StaleBodyHandle { handle },
        )?;
        self.bump_revision();
        Ok(())
    }

    /// Creates a collider attached to an existing body.
    pub fn create_collider(
        &mut self,
        body: BodyHandle,
        desc: ColliderDesc,
    ) -> Result<ColliderHandle, WorldError> {
        self.body_record(body)?;
        let handle = allocate_slot(
            &mut self.colliders,
            &mut self.free_colliders,
            ColliderHandle::from_raw_parts,
            ColliderRecord::from_desc(body, desc),
        );
        self.body_record_mut(body)?.attach_collider(handle);
        self.bump_revision();
        Ok(handle)
    }

    /// Destroys a collider and detaches it from its parent body.
    pub fn destroy_collider(&mut self, handle: ColliderHandle) -> Result<(), WorldError> {
        self.destroy_collider_internal(handle)?;
        self.bump_revision();
        Ok(())
    }

    /// Creates a joint after validating all referenced bodies.
    pub fn create_joint(&mut self, desc: JointDesc) -> Result<JointHandle, WorldError> {
        let body_handles = match &desc {
            JointDesc::Distance(desc) => vec![desc.body_a, desc.body_b],
            JointDesc::WorldAnchor(desc) => vec![desc.body],
        };
        for handle in body_handles.iter().copied() {
            self.body_record(handle)?;
        }
        let handle = allocate_slot(
            &mut self.joints,
            &mut self.free_joints,
            JointHandle::from_raw_parts,
            JointRecord::from_desc(desc),
        );
        let attached = self
            .joint_record(handle)
            .expect("newly created joint must exist")
            .body_handles();
        for body in attached {
            self.body_record_mut(body)?.attach_joint(handle);
        }
        self.pending_events
            .push(WorldEvent::JointCreated { joint: handle });
        self.bump_revision();
        Ok(handle)
    }

    /// Destroys a joint and detaches it from all referenced bodies.
    pub fn destroy_joint(&mut self, handle: JointHandle) -> Result<(), WorldError> {
        self.destroy_joint_internal(handle)?;
        self.bump_revision();
        Ok(())
    }

    /// Applies a partial update to an existing body.
    pub fn apply_body_patch(
        &mut self,
        handle: BodyHandle,
        patch: BodyPatch,
    ) -> Result<(), WorldError> {
        if let Err(field) = patch.validate() {
            return Err(WorldError::InvalidBodyPatch { field });
        }
        self.body_record_mut(handle)?.apply_patch(patch);
        self.bump_revision();
        Ok(())
    }

    /// Applies a partial update to an existing collider.
    pub fn apply_collider_patch(
        &mut self,
        handle: ColliderHandle,
        patch: ColliderPatch,
    ) -> Result<(), WorldError> {
        self.collider_record_mut(handle)?.apply_patch(patch);
        self.bump_revision();
        Ok(())
    }

    /// Applies a partial update to an existing joint.
    pub fn apply_joint_patch(
        &mut self,
        handle: JointHandle,
        patch: JointPatch,
    ) -> Result<(), WorldError> {
        let applied = self.joint_record_mut(handle)?.apply_patch(patch);
        if !applied {
            return Err(WorldError::JointKindMismatch { handle });
        }
        self.bump_revision();
        Ok(())
    }

    /// Resolves a body handle into a read-only body view.
    #[track_caller]
    pub fn body(&self, handle: BodyHandle) -> Result<BodyView, WorldError> {
        self.try_body(handle)
    }

    /// Resolves a collider handle into a read-only collider view.
    #[track_caller]
    pub fn collider(&self, handle: ColliderHandle) -> Result<ColliderView, WorldError> {
        self.try_collider(handle)
    }

    /// Resolves a joint handle into a read-only joint view.
    #[track_caller]
    pub fn joint(&self, handle: JointHandle) -> Result<JointView, WorldError> {
        self.try_joint(handle)
    }

    /// Iterates over all currently live body handles.
    pub fn bodies(&self) -> impl Iterator<Item = BodyHandle> + '_ {
        self.bodies.iter().enumerate().filter_map(|(index, slot)| {
            slot.value
                .as_ref()
                .map(|_| BodyHandle::from_raw_parts(index as u32, slot.generation))
        })
    }

    /// Iterates over all currently live collider handles attached to the provided body.
    #[track_caller]
    pub fn colliders_for_body(
        &self,
        body: BodyHandle,
    ) -> Result<std::vec::IntoIter<ColliderHandle>, WorldError> {
        self.try_colliders_for_body(body)
    }

    /// Iterates over all currently live joint handles.
    pub fn joints(&self) -> impl Iterator<Item = JointHandle> + '_ {
        self.joints.iter().enumerate().filter_map(|(index, slot)| {
            slot.value
                .as_ref()
                .map(|_| JointHandle::from_raw_parts(index as u32, slot.generation))
        })
    }

    /// Produces the stable debug snapshot for the current world state.
    pub fn debug_snapshot(&self, options: &DebugSnapshotOptions) -> DebugSnapshot {
        DebugSnapshot::from_world(self, options)
    }

    pub(crate) fn last_step_stats(&self) -> StepStats {
        self.last_step_stats
    }

    #[allow(dead_code)]
    pub(crate) fn last_step_dt(&self) -> FloatNum {
        self.last_step_dt
    }

    #[allow(dead_code)]
    pub(crate) fn simulated_time(&self) -> f64 {
        self.simulated_time
    }

    #[allow(dead_code)]
    pub(crate) fn last_step_events(&self) -> &[WorldEvent] {
        &self.last_step_events
    }

    /// Resolves a body handle while preserving stale/foreign-handle errors on the read path.
    pub fn try_body(&self, handle: BodyHandle) -> Result<BodyView, WorldError> {
        Ok(self.body_record(handle)?.view(handle))
    }

    /// Resolves a collider handle while preserving stale/foreign-handle errors on the read path.
    pub fn try_collider(&self, handle: ColliderHandle) -> Result<ColliderView, WorldError> {
        let record = self.collider_record(handle)?;
        let body_pose = self.body_record(record.body)?.pose;
        Ok(record.view(handle, body_pose))
    }

    /// Resolves a joint handle while preserving stale/foreign-handle errors on the read path.
    pub fn try_joint(&self, handle: JointHandle) -> Result<JointView, WorldError> {
        Ok(self.joint_record(handle)?.view(handle))
    }

    /// Iterates attached colliders while preserving stale/foreign-handle errors on the read path.
    pub fn try_colliders_for_body(
        &self,
        body: BodyHandle,
    ) -> Result<std::vec::IntoIter<ColliderHandle>, WorldError> {
        Ok(self.body_record(body)?.colliders.clone().into_iter())
    }

    pub(crate) fn body_record(&self, handle: BodyHandle) -> Result<&BodyRecord, WorldError> {
        self.body_slot_checked(handle)?
            .value
            .as_ref()
            .ok_or(WorldError::StaleBodyHandle { handle })
    }

    pub(crate) fn body_record_mut(
        &mut self,
        handle: BodyHandle,
    ) -> Result<&mut BodyRecord, WorldError> {
        self.body_slot_checked_mut(handle)?
            .value
            .as_mut()
            .ok_or(WorldError::StaleBodyHandle { handle })
    }

    pub(crate) fn collider_record(
        &self,
        handle: ColliderHandle,
    ) -> Result<&ColliderRecord, WorldError> {
        self.collider_slot_checked(handle)?
            .value
            .as_ref()
            .ok_or(WorldError::StaleColliderHandle { handle })
    }

    pub(crate) fn collider_record_mut(
        &mut self,
        handle: ColliderHandle,
    ) -> Result<&mut ColliderRecord, WorldError> {
        self.collider_slot_checked_mut(handle)?
            .value
            .as_mut()
            .ok_or(WorldError::StaleColliderHandle { handle })
    }

    pub(crate) fn joint_record(&self, handle: JointHandle) -> Result<&JointRecord, WorldError> {
        self.joint_slot_checked(handle)?
            .value
            .as_ref()
            .ok_or(WorldError::StaleJointHandle { handle })
    }

    pub(crate) fn joint_record_mut(
        &mut self,
        handle: JointHandle,
    ) -> Result<&mut JointRecord, WorldError> {
        self.joint_slot_checked_mut(handle)?
            .value
            .as_mut()
            .ok_or(WorldError::StaleJointHandle { handle })
    }

    pub(crate) fn collider_records(
        &self,
    ) -> impl Iterator<Item = (ColliderHandle, &ColliderRecord)> + '_ {
        self.colliders
            .iter()
            .enumerate()
            .filter_map(|(index, slot)| {
                slot.value.as_ref().map(|record| {
                    (
                        ColliderHandle::from_raw_parts(index as u32, slot.generation),
                        record,
                    )
                })
            })
    }

    #[allow(dead_code)]
    pub(crate) fn body_records(&self) -> impl Iterator<Item = (BodyHandle, &BodyRecord)> + '_ {
        self.bodies.iter().enumerate().filter_map(|(index, slot)| {
            slot.value.as_ref().map(|record| {
                (
                    BodyHandle::from_raw_parts(index as u32, slot.generation),
                    record,
                )
            })
        })
    }

    #[allow(dead_code)]
    pub(crate) fn joint_records(&self) -> impl Iterator<Item = (JointHandle, &JointRecord)> + '_ {
        self.joints.iter().enumerate().filter_map(|(index, slot)| {
            slot.value.as_ref().map(|record| {
                (
                    JointHandle::from_raw_parts(index as u32, slot.generation),
                    record,
                )
            })
        })
    }

    fn destroy_collider_internal(&mut self, handle: ColliderHandle) -> Result<(), WorldError> {
        let body = self.collider_record(handle)?.body;
        remove_slot(
            &mut self.colliders,
            &mut self.free_colliders,
            handle,
            WorldError::MissingColliderHandle { handle },
            WorldError::StaleColliderHandle { handle },
        )?;
        self.body_record_mut(body)?.detach_collider(handle);
        Ok(())
    }

    fn destroy_joint_internal(&mut self, handle: JointHandle) -> Result<(), WorldError> {
        let bodies = self.joint_record(handle)?.body_handles();
        self.pending_events
            .push(WorldEvent::JointRemoved { joint: handle });
        remove_slot(
            &mut self.joints,
            &mut self.free_joints,
            handle,
            WorldError::MissingJointHandle { handle },
            WorldError::StaleJointHandle { handle },
        )?;
        for body in bodies {
            if let Ok(record) = self.body_record_mut(body) {
                record.detach_joint(handle);
            }
        }
        Ok(())
    }

    fn body_slot_checked(&self, handle: BodyHandle) -> Result<&Slot<BodyRecord>, WorldError> {
        slot_checked(
            &self.bodies,
            handle,
            WorldError::MissingBodyHandle { handle },
            WorldError::StaleBodyHandle { handle },
        )
    }

    fn body_slot_checked_mut(
        &mut self,
        handle: BodyHandle,
    ) -> Result<&mut Slot<BodyRecord>, WorldError> {
        slot_checked_mut(
            &mut self.bodies,
            handle,
            WorldError::MissingBodyHandle { handle },
            WorldError::StaleBodyHandle { handle },
        )
    }

    fn collider_slot_checked(
        &self,
        handle: ColliderHandle,
    ) -> Result<&Slot<ColliderRecord>, WorldError> {
        slot_checked(
            &self.colliders,
            handle,
            WorldError::MissingColliderHandle { handle },
            WorldError::StaleColliderHandle { handle },
        )
    }

    fn collider_slot_checked_mut(
        &mut self,
        handle: ColliderHandle,
    ) -> Result<&mut Slot<ColliderRecord>, WorldError> {
        slot_checked_mut(
            &mut self.colliders,
            handle,
            WorldError::MissingColliderHandle { handle },
            WorldError::StaleColliderHandle { handle },
        )
    }

    fn joint_slot_checked(&self, handle: JointHandle) -> Result<&Slot<JointRecord>, WorldError> {
        slot_checked(
            &self.joints,
            handle,
            WorldError::MissingJointHandle { handle },
            WorldError::StaleJointHandle { handle },
        )
    }

    fn joint_slot_checked_mut(
        &mut self,
        handle: JointHandle,
    ) -> Result<&mut Slot<JointRecord>, WorldError> {
        slot_checked_mut(
            &mut self.joints,
            handle,
            WorldError::MissingJointHandle { handle },
            WorldError::StaleJointHandle { handle },
        )
    }

    fn bump_revision(&mut self) {
        self.revision = self.revision.next();
    }
}

impl SimulationWorld for World {
    fn simulate_step(&mut self, config: &StepConfig) -> StepOutcome {
        let previous_sleep_states = self
            .bodies()
            .map(|handle| {
                (
                    handle,
                    self.body_record(handle)
                        .map(|record| record.sleeping)
                        .unwrap_or(false),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let mut events = std::mem::take(&mut self.pending_events);
        let mut awake_bodies = BTreeSet::new();

        self.integrate_body_motion(config);
        self.apply_joint_constraints(config.dt, &mut awake_bodies);
        let contacts = self.collect_contact_observations();
        self.resolve_contacts(&contacts, &mut awake_bodies);
        let (contact_events, contact_count, manifold_count) = self.refresh_contact_events(contacts);
        events.extend(contact_events);
        let (sleep_events, sleep_transition_count, active_body_count) =
            self.refresh_sleep_states(config, &previous_sleep_states, &awake_bodies);
        events.extend(sleep_events);

        let stats = StepStats {
            step_index: self.last_step_stats.step_index.saturating_add(1),
            body_count: self.bodies().count(),
            collider_count: self.collider_records().count(),
            joint_count: self.joints().count(),
            active_body_count,
            contact_count,
            manifold_count,
            velocity_iterations: config.velocity_iterations,
            position_iterations: config.position_iterations,
            sleep_transition_count,
        };
        self.last_step_stats = stats;
        self.last_step_dt = config.dt;
        self.simulated_time += f64::from(config.dt);
        self.last_step_events = events.clone();
        self.bump_revision();

        StepOutcome {
            revision: self.revision(),
            stats,
            events,
        }
    }
}

impl World {
    fn integrate_body_motion(&mut self, config: &StepConfig) {
        let body_handles = self.bodies().collect::<Vec<_>>();
        let enable_sleep = self.desc.enable_sleep;
        let gravity = self.desc.gravity;
        for handle in body_handles {
            let record = self
                .body_record_mut(handle)
                .expect("live body handles must resolve during step");
            match record.body_type {
                BodyType::Static => {
                    record.sleeping = false;
                }
                BodyType::Dynamic => {
                    if !(config.enable_sleep && enable_sleep) {
                        record.sleeping = false;
                    }
                    if record.sleeping {
                        continue;
                    }
                    record.linear_velocity += gravity * config.dt * record.gravity_scale;
                    record.linear_velocity *= (1.0 - record.linear_damping * config.dt).max(0.0);
                    record.angular_velocity *= (1.0 - record.angular_damping * config.dt).max(0.0);
                    translate_pose(
                        &mut record.pose,
                        record.linear_velocity * config.dt,
                        record.angular_velocity * config.dt,
                    );
                }
                BodyType::Kinematic => {
                    record.sleeping = false;
                    translate_pose(
                        &mut record.pose,
                        record.linear_velocity * config.dt,
                        record.angular_velocity * config.dt,
                    );
                }
            }
        }
    }

    fn apply_joint_constraints(&mut self, dt: FloatNum, awake_bodies: &mut BTreeSet<BodyHandle>) {
        let joints = self
            .joint_records()
            .map(|(handle, record)| (handle, record.desc.clone()))
            .collect::<Vec<_>>();
        for (_, desc) in joints {
            match desc {
                JointDesc::Distance(desc) => {
                    let pose_a = self
                        .body_record(desc.body_a)
                        .expect("joint endpoints must stay live during step")
                        .pose;
                    let pose_b = self
                        .body_record(desc.body_b)
                        .expect("joint endpoints must stay live during step")
                        .pose;
                    let anchor_a = pose_a.transform_point(desc.local_anchor_a);
                    let anchor_b = pose_b.transform_point(desc.local_anchor_b);
                    let delta = anchor_b - anchor_a;
                    let distance = delta.abs();
                    let direction = normalized_or_x_axis(delta);
                    let error = distance - desc.rest_length;
                    if error.abs() <= f32::EPSILON {
                        continue;
                    }
                    let correction = direction * error * desc.stiffness.max(0.0) * dt;
                    self.apply_body_pair_correction(
                        desc.body_a,
                        desc.body_b,
                        correction,
                        awake_bodies,
                    );
                }
                JointDesc::WorldAnchor(desc) => {
                    let pose = self
                        .body_record(desc.body)
                        .expect("joint endpoint must stay live during step")
                        .pose;
                    let anchor = pose.transform_point(desc.local_anchor);
                    let correction = (desc.world_anchor - anchor) * desc.stiffness.max(0.0) * dt;
                    self.apply_single_body_correction(desc.body, correction, awake_bodies);
                }
            }
        }
    }

    fn collect_contact_observations(&self) -> Vec<ContactObservation> {
        let colliders = self.live_collider_snapshots();
        let mut observations = Vec::new();

        for index in 0..colliders.len() {
            for other_index in (index + 1)..colliders.len() {
                let collider_a = &colliders[index];
                let collider_b = &colliders[other_index];
                if collider_a.body == collider_b.body
                    || !collider_a.filter.allows(&collider_b.filter)
                {
                    continue;
                }
                let Some((point, normal, depth)) = overlap_from_aabbs(
                    collider_a.shape.aabb(collider_a.world_pose),
                    collider_b.shape.aabb(collider_b.world_pose),
                ) else {
                    continue;
                };

                let (ordered_a, ordered_b, ordered_body_a, ordered_body_b, ordered_normal) =
                    if collider_a.handle <= collider_b.handle {
                        (
                            collider_a.handle,
                            collider_b.handle,
                            collider_a.body,
                            collider_b.body,
                            normal,
                        )
                    } else {
                        (
                            collider_b.handle,
                            collider_a.handle,
                            collider_b.body,
                            collider_a.body,
                            -normal,
                        )
                    };

                observations.push(ContactObservation {
                    key: ContactKey {
                        collider_a: ordered_a,
                        collider_b: ordered_b,
                    },
                    body_a: ordered_body_a,
                    body_b: ordered_body_b,
                    collider_a: ordered_a,
                    collider_b: ordered_b,
                    point,
                    normal: ordered_normal,
                    depth,
                    is_sensor: collider_a.is_sensor || collider_b.is_sensor,
                });
            }
        }

        observations
    }

    fn resolve_contacts(
        &mut self,
        contacts: &[ContactObservation],
        awake_bodies: &mut BTreeSet<BodyHandle>,
    ) {
        for contact in contacts {
            if contact.is_sensor {
                continue;
            }
            self.apply_body_pair_correction(
                contact.body_a,
                contact.body_b,
                contact.normal * (contact.depth * 0.5),
                awake_bodies,
            );
        }
    }

    fn refresh_contact_events(
        &mut self,
        contacts: Vec<ContactObservation>,
    ) -> (Vec<WorldEvent>, usize, usize) {
        let mut previous = std::mem::take(&mut self.active_contacts);
        let mut next = BTreeMap::new();
        let mut events = Vec::new();

        for contact in contacts {
            let existing = previous.remove(&contact.key);
            let is_persisted = existing.is_some();
            let event = if let Some(existing) = existing {
                ContactEvent {
                    contact_id: existing.contact.contact_id,
                    manifold_id: existing.contact.manifold_id,
                    body_a: contact.body_a,
                    body_b: contact.body_b,
                    collider_a: contact.collider_a,
                    collider_b: contact.collider_b,
                    point: contact.point,
                    normal: contact.normal,
                    depth: contact.depth,
                }
            } else {
                ContactEvent {
                    contact_id: self.next_contact_id(),
                    manifold_id: self.next_manifold_id(),
                    body_a: contact.body_a,
                    body_b: contact.body_b,
                    collider_a: contact.collider_a,
                    collider_b: contact.collider_b,
                    point: contact.point,
                    normal: contact.normal,
                    depth: contact.depth,
                }
            };

            if is_persisted {
                events.push(WorldEvent::ContactPersisted(event));
            } else {
                events.push(WorldEvent::ContactStarted(event));
            }
            next.insert(contact.key, ContactRecord { contact: event });
        }

        for (_, record) in previous {
            events.push(WorldEvent::ContactEnded(record.contact));
        }

        let contact_count = next.len();
        let manifold_count = next
            .values()
            .map(|record| ordered_body_pair(record.contact.body_a, record.contact.body_b))
            .collect::<BTreeSet<_>>()
            .len();

        self.active_contacts = next;
        (events, contact_count, manifold_count)
    }

    fn refresh_sleep_states(
        &mut self,
        config: &StepConfig,
        previous_sleep_states: &BTreeMap<BodyHandle, bool>,
        awake_bodies: &BTreeSet<BodyHandle>,
    ) -> (Vec<WorldEvent>, usize, usize) {
        let body_handles = self.bodies().collect::<Vec<_>>();
        let enable_sleep = self.desc.enable_sleep;
        let mut events = Vec::new();
        let mut transitions = 0usize;
        let mut active_body_count = 0usize;

        for handle in body_handles {
            let record = self
                .body_record_mut(handle)
                .expect("live body handles must resolve during sleep refresh");
            let was_sleeping = previous_sleep_states.get(&handle).copied().unwrap_or(false);

            match record.body_type {
                BodyType::Static => {
                    record.sleeping = false;
                }
                BodyType::Dynamic => {
                    if awake_bodies.contains(&handle) {
                        record.sleeping = false;
                    } else if config.enable_sleep && enable_sleep && record.can_sleep {
                        record.sleeping = record.linear_velocity.abs() < 0.0001
                            && record.angular_velocity.abs() < 0.0001;
                    } else {
                        record.sleeping = false;
                    }

                    if !record.sleeping {
                        active_body_count += 1;
                    }
                    if record.sleeping != was_sleeping {
                        transitions += 1;
                        events.push(WorldEvent::SleepChanged(SleepEvent {
                            body: handle,
                            is_sleeping: record.sleeping,
                        }));
                    }
                }
                BodyType::Kinematic => {
                    record.sleeping = false;
                    active_body_count += 1;
                }
            }
        }

        (events, transitions, active_body_count)
    }

    fn apply_body_pair_correction(
        &mut self,
        body_a: BodyHandle,
        body_b: BodyHandle,
        correction_toward_a: Vector,
        awake_bodies: &mut BTreeSet<BodyHandle>,
    ) {
        let body_a_dynamic = self
            .body_record(body_a)
            .expect("live body handles must resolve")
            .body_type
            .is_dynamic();
        let body_b_dynamic = self
            .body_record(body_b)
            .expect("live body handles must resolve")
            .body_type
            .is_dynamic();

        match (body_a_dynamic, body_b_dynamic) {
            (true, true) => {
                self.apply_single_body_correction(body_a, correction_toward_a, awake_bodies);
                self.apply_single_body_correction(body_b, -correction_toward_a, awake_bodies);
            }
            (true, false) => {
                self.apply_single_body_correction(body_a, correction_toward_a * 2.0, awake_bodies);
            }
            (false, true) => {
                self.apply_single_body_correction(body_b, -correction_toward_a * 2.0, awake_bodies);
            }
            (false, false) => {}
        }
    }

    fn apply_single_body_correction(
        &mut self,
        body: BodyHandle,
        translation: Vector,
        awake_bodies: &mut BTreeSet<BodyHandle>,
    ) {
        if translation.abs() <= f32::EPSILON {
            return;
        }
        let record = self
            .body_record_mut(body)
            .expect("live body handles must resolve");
        if !record.body_type.is_dynamic() {
            return;
        }
        translate_pose(&mut record.pose, translation, 0.0);
        record.linear_velocity = Vector::default();
        record.angular_velocity = 0.0;
        record.sleeping = false;
        awake_bodies.insert(body);
    }

    fn live_collider_snapshots(&self) -> Vec<ColliderSnapshot> {
        self.collider_records()
            .filter_map(|(handle, record)| {
                let body = self.body_record(record.body).ok()?;
                Some(ColliderSnapshot {
                    handle,
                    body: record.body,
                    world_pose: body.pose.compose(record.local_pose),
                    shape: record.shape.clone(),
                    filter: record.filter,
                    is_sensor: record.is_sensor,
                })
            })
            .collect()
    }

    fn next_contact_id(&mut self) -> ContactId {
        let raw = self.next_contact_raw;
        self.next_contact_raw = self.next_contact_raw.wrapping_add(1);
        ContactId::from_raw_parts(raw, 0)
    }

    fn next_manifold_id(&mut self) -> ManifoldId {
        let raw = self.next_manifold_raw;
        self.next_manifold_raw = self.next_manifold_raw.wrapping_add(1);
        ManifoldId::from_raw_parts(raw, 0)
    }
}

fn allocate_slot<T, H: Copy>(
    slots: &mut Vec<Slot<T>>,
    free_list: &mut Vec<usize>,
    make_handle: fn(u32, u32) -> H,
    value: T,
) -> H {
    if let Some(index) = free_list.pop() {
        let slot = &mut slots[index];
        debug_assert!(
            slot.value.is_none(),
            "free-list slots must be empty before reuse"
        );
        slot.value = Some(value);
        make_handle(index as u32, slot.generation)
    } else {
        let index = slots.len();
        slots.push(Slot {
            generation: 0,
            value: Some(value),
        });
        make_handle(index as u32, 0)
    }
}

fn ordered_body_pair(a: BodyHandle, b: BodyHandle) -> (BodyHandle, BodyHandle) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

fn normalized_or_x_axis(vector: Vector) -> Vector {
    if vector.abs() <= f32::EPSILON {
        Vector::new(1.0, 0.0)
    } else {
        vector.normalize()
    }
}

fn translate_pose(pose: &mut Pose, translation: Vector, angle_delta: FloatNum) {
    *pose = Pose::from_xy_angle(
        pose.translation().x() + translation.x(),
        pose.translation().y() + translation.y(),
        pose.angle() + angle_delta,
    );
}

fn overlap_from_aabbs(a: ShapeAabb, b: ShapeAabb) -> Option<(Point, Vector, FloatNum)> {
    let overlap_x = a.max.x().min(b.max.x()) - a.min.x().max(b.min.x());
    let overlap_y = a.max.y().min(b.max.y()) - a.min.y().max(b.min.y());
    if overlap_x <= 0.0 || overlap_y <= 0.0 {
        return None;
    }

    let point = Point::new(
        (a.min.x().max(b.min.x()) + a.max.x().min(b.max.x())) * 0.5,
        (a.min.y().max(b.min.y()) + a.max.y().min(b.max.y())) * 0.5,
    );
    let center_a = Point::new((a.min.x() + a.max.x()) * 0.5, (a.min.y() + a.max.y()) * 0.5);
    let center_b = Point::new((b.min.x() + b.max.x()) * 0.5, (b.min.y() + b.max.y()) * 0.5);
    let delta = center_a - center_b;

    if overlap_x <= overlap_y {
        let normal = if delta.x() <= 0.0 {
            Vector::new(-1.0, 0.0)
        } else {
            Vector::new(1.0, 0.0)
        };
        Some((point, normal, overlap_x))
    } else {
        let normal = if delta.y() <= 0.0 {
            Vector::new(0.0, -1.0)
        } else {
            Vector::new(0.0, 1.0)
        };
        Some((point, normal, overlap_y))
    }
}

fn remove_slot<T, H>(
    slots: &mut [Slot<T>],
    free_list: &mut Vec<usize>,
    handle: H,
    missing_error: WorldError,
    stale_error: WorldError,
) -> Result<T, WorldError>
where
    H: HandleLike,
{
    let index = handle.index().ok_or_else(|| missing_error.clone())?;
    let generation = handle.generation().ok_or(missing_error)?;
    let slot = slots.get_mut(index).ok_or_else(|| stale_error.clone())?;
    if slot.generation != generation {
        return Err(stale_error);
    }
    let value = slot.value.take().ok_or_else(|| stale_error.clone())?;
    // Generation increments on remove so stale handles cannot silently hit a reused slot.
    slot.generation = slot.generation.wrapping_add(1);
    free_list.push(index);
    Ok(value)
}

fn slot_checked<T, H: HandleLike>(
    slots: &[Slot<T>],
    handle: H,
    missing_error: WorldError,
    stale_error: WorldError,
) -> Result<&Slot<T>, WorldError> {
    let index = handle.index().ok_or_else(|| missing_error.clone())?;
    let generation = handle.generation().ok_or(missing_error)?;
    let slot = slots.get(index).ok_or_else(|| stale_error.clone())?;
    if slot.generation == generation {
        Ok(slot)
    } else {
        Err(stale_error)
    }
}

fn slot_checked_mut<T, H: HandleLike>(
    slots: &mut [Slot<T>],
    handle: H,
    missing_error: WorldError,
    stale_error: WorldError,
) -> Result<&mut Slot<T>, WorldError> {
    let index = handle.index().ok_or_else(|| missing_error.clone())?;
    let generation = handle.generation().ok_or(missing_error)?;
    let slot = slots.get_mut(index).ok_or_else(|| stale_error.clone())?;
    if slot.generation == generation {
        Ok(slot)
    } else {
        Err(stale_error)
    }
}

trait HandleLike: Copy {
    fn index(self) -> Option<usize>;
    fn generation(self) -> Option<u32>;
}

impl HandleLike for BodyHandle {
    fn index(self) -> Option<usize> {
        BodyHandle::index(self)
    }

    fn generation(self) -> Option<u32> {
        BodyHandle::generation(self)
    }
}

impl HandleLike for ColliderHandle {
    fn index(self) -> Option<usize> {
        ColliderHandle::index(self)
    }

    fn generation(self) -> Option<u32> {
        ColliderHandle::generation(self)
    }
}

impl HandleLike for JointHandle {
    fn index(self) -> Option<usize> {
        JointHandle::index(self)
    }

    fn generation(self) -> Option<u32> {
        JointHandle::generation(self)
    }
}
