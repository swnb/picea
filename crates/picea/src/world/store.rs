use crate::{
    body::BodyRecord,
    collider::ColliderRecord,
    handles::{BodyHandle, ColliderHandle, JointHandle},
    joint::JointRecord,
    world::World,
    world::WorldError,
};

#[derive(Clone, Debug)]
pub(crate) struct Slot<T> {
    pub(crate) generation: u32,
    pub(crate) value: Option<T>,
}

impl<T> Default for Slot<T> {
    fn default() -> Self {
        Self {
            generation: 0,
            value: None,
        }
    }
}

pub(crate) fn allocate_slot<T, H: Copy>(
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

pub(crate) fn remove_slot<T, H>(
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

pub(crate) fn slot_checked<T, H: HandleLike>(
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

pub(crate) fn slot_checked_mut<T, H: HandleLike>(
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

pub(crate) trait HandleLike: Copy {
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

impl World {
    pub(crate) fn body_record(&self, handle: BodyHandle) -> Result<&BodyRecord, WorldError> {
        self.body_slot_checked(handle)?
            .value
            .as_ref()
            .ok_or_else(|| super::stale_body_error(handle))
    }

    pub(crate) fn body_record_mut(
        &mut self,
        handle: BodyHandle,
    ) -> Result<&mut BodyRecord, WorldError> {
        self.body_slot_checked_mut(handle)?
            .value
            .as_mut()
            .ok_or_else(|| super::stale_body_error(handle))
    }

    pub(crate) fn collider_record(
        &self,
        handle: ColliderHandle,
    ) -> Result<&ColliderRecord, WorldError> {
        self.collider_slot_checked(handle)?
            .value
            .as_ref()
            .ok_or_else(|| super::stale_collider_error(handle))
    }

    pub(crate) fn collider_record_mut(
        &mut self,
        handle: ColliderHandle,
    ) -> Result<&mut ColliderRecord, WorldError> {
        self.collider_slot_checked_mut(handle)?
            .value
            .as_mut()
            .ok_or_else(|| super::stale_collider_error(handle))
    }

    pub(crate) fn joint_record(&self, handle: JointHandle) -> Result<&JointRecord, WorldError> {
        self.joint_slot_checked(handle)?
            .value
            .as_ref()
            .ok_or_else(|| super::stale_joint_error(handle))
    }

    pub(crate) fn joint_record_mut(
        &mut self,
        handle: JointHandle,
    ) -> Result<&mut JointRecord, WorldError> {
        self.joint_slot_checked_mut(handle)?
            .value
            .as_mut()
            .ok_or_else(|| super::stale_joint_error(handle))
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

    fn body_slot_checked(&self, handle: BodyHandle) -> Result<&Slot<BodyRecord>, WorldError> {
        slot_checked(
            &self.bodies,
            handle,
            super::missing_body_error(handle),
            super::stale_body_error(handle),
        )
    }

    fn body_slot_checked_mut(
        &mut self,
        handle: BodyHandle,
    ) -> Result<&mut Slot<BodyRecord>, WorldError> {
        slot_checked_mut(
            &mut self.bodies,
            handle,
            super::missing_body_error(handle),
            super::stale_body_error(handle),
        )
    }

    fn collider_slot_checked(
        &self,
        handle: ColliderHandle,
    ) -> Result<&Slot<ColliderRecord>, WorldError> {
        slot_checked(
            &self.colliders,
            handle,
            super::missing_collider_error(handle),
            super::stale_collider_error(handle),
        )
    }

    fn collider_slot_checked_mut(
        &mut self,
        handle: ColliderHandle,
    ) -> Result<&mut Slot<ColliderRecord>, WorldError> {
        slot_checked_mut(
            &mut self.colliders,
            handle,
            super::missing_collider_error(handle),
            super::stale_collider_error(handle),
        )
    }

    fn joint_slot_checked(&self, handle: JointHandle) -> Result<&Slot<JointRecord>, WorldError> {
        slot_checked(
            &self.joints,
            handle,
            super::missing_joint_error(handle),
            super::stale_joint_error(handle),
        )
    }

    fn joint_slot_checked_mut(
        &mut self,
        handle: JointHandle,
    ) -> Result<&mut Slot<JointRecord>, WorldError> {
        slot_checked_mut(
            &mut self.joints,
            handle,
            super::missing_joint_error(handle),
            super::stale_joint_error(handle),
        )
    }
}
