use std::collections::BTreeMap;

use crate::{
    events::ContactEvent,
    handles::{ColliderHandle, ContactFeatureId, ContactId, ManifoldId},
    world::World,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ContactPairKey {
    collider_a: ColliderHandle,
    collider_b: ColliderHandle,
}

impl ContactPairKey {
    pub(crate) fn new(collider_a: ColliderHandle, collider_b: ColliderHandle) -> Self {
        Self {
            collider_a,
            collider_b,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ContactKey {
    pub(crate) pair: ContactPairKey,
    feature_id: ContactFeatureId,
}

impl ContactKey {
    pub(crate) fn new(
        collider_a: ColliderHandle,
        collider_b: ColliderHandle,
        feature_id: ContactFeatureId,
    ) -> Self {
        Self {
            pair: ContactPairKey::new(collider_a, collider_b),
            feature_id,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ContactRecord {
    pub(crate) contact: ContactEvent,
}

impl World {
    pub(crate) fn take_active_contacts(&mut self) -> BTreeMap<ContactKey, ContactRecord> {
        std::mem::take(&mut self.active_contacts)
    }

    pub(crate) fn replace_active_contacts(
        &mut self,
        active_contacts: BTreeMap<ContactKey, ContactRecord>,
    ) {
        self.active_contacts = active_contacts;
    }

    pub(crate) fn alloc_next_contact_id(&mut self) -> ContactId {
        let raw = self.next_contact_raw;
        self.next_contact_raw = self.next_contact_raw.wrapping_add(1);
        ContactId::from_raw_parts(raw, 0)
    }

    pub(crate) fn alloc_next_manifold_id(&mut self) -> ManifoldId {
        let raw = self.next_manifold_raw;
        self.next_manifold_raw = self.next_manifold_raw.wrapping_add(1);
        ManifoldId::from_raw_parts(raw, 0)
    }
}
