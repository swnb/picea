use crate::{
    body::{BodyDesc, BodyPatch, BodyRecord, BodyView},
    collider::{ColliderDesc, ColliderPatch, ColliderRecord, ColliderView},
    debug::{DebugSnapshot, DebugSnapshotOptions},
    events::WorldEvent,
    handles::{BodyHandle, ColliderHandle, JointHandle, WorldRevision},
    joint::{JointDesc, JointKind, JointPatch, JointRecord, JointView},
    world::World,
};

use super::{
    store::{allocate_slot, remove_slot},
    HandleError, TopologyError, WorldDesc, WorldError,
};

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
            last_step_stats: Default::default(),
            last_step_dt: 0.0,
            simulated_time: 0.0,
            pending_events: Vec::new(),
            last_step_events: Vec::new(),
            active_contacts: Default::default(),
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
        desc.validate().map_err(WorldError::Validation)?;
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
            super::missing_body_error(handle),
            super::stale_body_error(handle),
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
        desc.validate().map_err(WorldError::Validation)?;
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
        desc.validate().map_err(WorldError::Validation)?;
        if let JointDesc::Distance(distance) = &desc {
            if distance.body_a == distance.body_b {
                return Err(WorldError::Topology(TopologyError::SameBodyJointPair {
                    body: distance.body_a,
                    kind: JointKind::Distance,
                }));
            }
        }
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
        patch.validate().map_err(WorldError::Validation)?;
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
        patch.validate().map_err(WorldError::Validation)?;
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
        patch.validate().map_err(WorldError::Validation)?;
        let expected_kind = patch.kind();
        let actual_kind = self.joint_record(handle)?.desc.kind();
        if expected_kind != actual_kind {
            return Err(WorldError::Handle(HandleError::WrongJointKind {
                handle,
                expected: expected_kind,
                actual: actual_kind,
            }));
        }
        let applied = self.joint_record_mut(handle)?.apply_patch(patch);
        if !applied {
            return Err(WorldError::Handle(HandleError::WrongJointKind {
                handle,
                expected: expected_kind,
                actual: actual_kind,
            }));
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

    fn destroy_collider_internal(&mut self, handle: ColliderHandle) -> Result<(), WorldError> {
        let body = self.collider_record(handle)?.body;
        remove_slot(
            &mut self.colliders,
            &mut self.free_colliders,
            handle,
            super::missing_collider_error(handle),
            super::stale_collider_error(handle),
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
            super::missing_joint_error(handle),
            super::stale_joint_error(handle),
        )?;
        for body in bodies {
            if let Ok(record) = self.body_record_mut(body) {
                record.detach_joint(handle);
            }
        }
        Ok(())
    }

    pub(crate) fn bump_revision(&mut self) {
        self.revision = self.revision.next();
    }
}
