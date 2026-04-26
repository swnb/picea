use std::collections::BTreeMap;

use crate::{
    events::{ContactEvent, WarmStartCacheReason},
    handles::{ColliderHandle, ContactFeatureId, ContactId, ManifoldId},
    math::FloatNum,
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
    pub(crate) anchor_a: crate::math::vector::Vector,
    pub(crate) anchor_b: crate::math::vector::Vector,
    pub(crate) normal_impulse: FloatNum,
    pub(crate) tangent_impulse: FloatNum,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct WarmStartStats {
    pub(crate) hit_count: usize,
    pub(crate) miss_count: usize,
    pub(crate) drop_count: usize,
}

impl WarmStartStats {
    pub(crate) fn record(&mut self, reason: WarmStartCacheReason) {
        if reason.is_hit() {
            self.hit_count += 1;
        } else if reason.is_miss() {
            self.miss_count += 1;
        } else if reason.is_drop() {
            self.drop_count += 1;
        }
    }
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
