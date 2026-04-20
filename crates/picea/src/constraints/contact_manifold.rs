use std::collections::BTreeMap;

use crate::{collision::ContactPointPair, element::ID};

use super::{contact::ContactConstraint, ConstraintObject};

pub struct ContactConstraintManifold<Obj: ConstraintObject> {
    manager: ContactManager<Obj>,
}

pub(crate) struct ContactManager<Obj: ConstraintObject> {
    map: BTreeMap<(ID, ID), ContactManifoldEntry<Obj>>,
}

struct ContactManifoldEntry<Obj: ConstraintObject> {
    constraint: ContactConstraint<Obj>,
    is_active: bool,
    was_active_last_pass: bool,
    pending_contact_point_pairs: Option<Vec<ContactPointPair>>,
}

impl<Obj: ConstraintObject> ContactManifoldEntry<Obj> {
    fn new_active(obj_id_a: ID, obj_id_b: ID, contact_point_pairs: Vec<ContactPointPair>) -> Self {
        let mut constraint = ContactConstraint::new(obj_id_a, obj_id_b, contact_point_pairs);
        constraint.set_is_active(true);

        Self {
            constraint,
            is_active: true,
            was_active_last_pass: false,
            pending_contact_point_pairs: None,
        }
    }

    fn begin_collision_pass(&mut self) {
        self.pending_contact_point_pairs = None;
        self.was_active_last_pass = self.is_active;
        self.is_active = false;
        self.constraint.set_is_active(false);
    }

    fn ingest_contact_point_pairs(&mut self, contact_point_pairs: Vec<ContactPointPair>) {
        if self.is_active {
            self.extend_current_contact_point_pairs(contact_point_pairs);
            return;
        }

        self.is_active = true;
        self.constraint.set_is_active(true);
        if self.was_active_last_pass {
            self.pending_contact_point_pairs = Some(contact_point_pairs);
        } else {
            self.constraint
                .replace_contact_point_pairs(contact_point_pairs);
        }
    }

    fn extend_current_contact_point_pairs(
        &mut self,
        mut contact_point_pairs: Vec<ContactPointPair>,
    ) {
        if let Some(pending_contact_point_pairs) = &mut self.pending_contact_point_pairs {
            pending_contact_point_pairs.append(&mut contact_point_pairs);
        } else {
            self.constraint
                .extend_contact_point_pairs(contact_point_pairs);
        }
    }

    fn refresh_contact_point_pairs_after_warm_start(&mut self) {
        if let Some(contact_point_pairs) = self.pending_contact_point_pairs.take() {
            if self.was_active_last_pass && self.is_active {
                self.constraint
                    .replace_contact_point_pairs_with_cached_impulse_transfer(contact_point_pairs);
            } else {
                self.constraint
                    .replace_contact_point_pairs(contact_point_pairs);
            }
        }
    }

    fn is_active(&self) -> bool {
        self.is_active
    }

    fn can_warm_start_current_pass(&self) -> bool {
        self.is_active && self.was_active_last_pass
    }

    #[cfg(test)]
    fn has_pending_refresh(&self) -> bool {
        self.pending_contact_point_pairs.is_some()
    }
}

impl<Obj: ConstraintObject> Default for ContactManager<Obj> {
    fn default() -> Self {
        Self {
            map: Default::default(),
        }
    }
}

impl<Obj: ConstraintObject> Default for ContactConstraintManifold<Obj> {
    fn default() -> Self {
        Self {
            manager: Default::default(),
        }
    }
}

impl<Obj: ConstraintObject> ContactConstraintManifold<Obj> {
    pub(crate) fn begin_collision_pass(&mut self) {
        self.manager.begin_collision_pass();
    }

    pub(crate) fn clear(&mut self) {
        self.manager.clear();
    }

    pub fn len(&self) -> usize {
        self.manager.len()
    }

    pub fn is_empty(&self) -> bool {
        self.manager.is_empty()
    }

    pub fn get(&self, pair: &(ID, ID)) -> Option<&ContactConstraint<Obj>> {
        self.manager.get(pair)
    }

    pub fn get_mut(&mut self, pair: &(ID, ID)) -> Option<&mut ContactConstraint<Obj>> {
        self.manager.get_mut(pair)
    }

    pub fn contains_key(&self, pair: &(ID, ID)) -> bool {
        self.manager.contains_key(pair)
    }

    pub fn keys(&self) -> impl Iterator<Item = &(ID, ID)> {
        self.manager.keys()
    }

    pub fn values(&self) -> impl Iterator<Item = &ContactConstraint<Obj>> {
        self.manager.values()
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut ContactConstraint<Obj>> {
        self.manager.values_mut()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&(ID, ID), &ContactConstraint<Obj>)> {
        self.manager.iter()
    }

    pub(crate) fn ingest_contact_point_pairs(
        &mut self,
        obj_id_a: ID,
        obj_id_b: ID,
        contact_point_pairs: Vec<ContactPointPair>,
    ) {
        self.manager
            .ingest_contact_point_pairs(obj_id_a, obj_id_b, contact_point_pairs);
    }

    pub(crate) fn is_pair_active(&self, pair: (ID, ID)) -> bool {
        self.manager.is_pair_active(pair)
    }

    pub(crate) fn active_constraints(&self) -> impl Iterator<Item = &ContactConstraint<Obj>> {
        self.manager.active_constraints()
    }

    pub(crate) fn active_constraints_with_keys(
        &self,
    ) -> impl Iterator<Item = (&(ID, ID), &ContactConstraint<Obj>)> {
        self.manager.active_constraints_with_keys()
    }

    pub(crate) fn active_constraints_mut(
        &mut self,
    ) -> impl Iterator<Item = &mut ContactConstraint<Obj>> {
        self.manager.active_constraints_mut()
    }

    pub(crate) fn warm_start_constraints_mut(
        &mut self,
    ) -> impl Iterator<Item = &mut ContactConstraint<Obj>> {
        self.manager.warm_start_constraints_mut()
    }

    pub(crate) fn refresh_contact_point_pairs_after_warm_start(&mut self) {
        self.manager.refresh_contact_point_pairs_after_warm_start();
    }
}

impl<Obj: ConstraintObject> ContactManager<Obj> {
    pub(crate) fn begin_collision_pass(&mut self) {
        self.map
            .values_mut()
            .for_each(ContactManifoldEntry::begin_collision_pass);
    }

    pub(crate) fn clear(&mut self) {
        self.map.clear();
    }

    pub(crate) fn len(&self) -> usize {
        self.map.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub(crate) fn get(&self, pair: &(ID, ID)) -> Option<&ContactConstraint<Obj>> {
        self.map.get(pair).map(|entry| &entry.constraint)
    }

    pub(crate) fn get_mut(&mut self, pair: &(ID, ID)) -> Option<&mut ContactConstraint<Obj>> {
        self.map.get_mut(pair).map(|entry| &mut entry.constraint)
    }

    pub(crate) fn contains_key(&self, pair: &(ID, ID)) -> bool {
        self.map.contains_key(pair)
    }

    pub(crate) fn keys(&self) -> impl Iterator<Item = &(ID, ID)> {
        self.map.keys()
    }

    pub(crate) fn values(&self) -> impl Iterator<Item = &ContactConstraint<Obj>> {
        self.map.values().map(|entry| &entry.constraint)
    }

    pub(crate) fn values_mut(&mut self) -> impl Iterator<Item = &mut ContactConstraint<Obj>> {
        self.map.values_mut().map(|entry| &mut entry.constraint)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&(ID, ID), &ContactConstraint<Obj>)> {
        self.map.iter().map(|(key, entry)| (key, &entry.constraint))
    }

    pub(crate) fn ingest_contact_point_pairs(
        &mut self,
        obj_id_a: ID,
        obj_id_b: ID,
        contact_point_pairs: Vec<ContactPointPair>,
    ) {
        let pair = (obj_id_a, obj_id_b);
        if let Some(entry) = self.map.get_mut(&pair) {
            entry.ingest_contact_point_pairs(contact_point_pairs);
        } else {
            self.map.insert(
                pair,
                ContactManifoldEntry::new_active(obj_id_a, obj_id_b, contact_point_pairs),
            );
        }
    }

    pub(crate) fn is_pair_active(&self, pair: (ID, ID)) -> bool {
        self.map
            .get(&pair)
            .map_or(false, ContactManifoldEntry::is_active)
    }

    #[cfg(test)]
    pub(crate) fn can_warm_start_pair(&self, pair: (ID, ID)) -> bool {
        self.map
            .get(&pair)
            .map_or(false, ContactManifoldEntry::can_warm_start_current_pass)
    }

    #[cfg(test)]
    pub(crate) fn has_pending_refresh(&self, pair: (ID, ID)) -> bool {
        self.map
            .get(&pair)
            .map_or(false, ContactManifoldEntry::has_pending_refresh)
    }

    pub(crate) fn active_constraints(&self) -> impl Iterator<Item = &ContactConstraint<Obj>> {
        self.map
            .values()
            .filter(|entry| entry.is_active())
            .map(|entry| &entry.constraint)
    }

    pub(crate) fn active_constraints_with_keys(
        &self,
    ) -> impl Iterator<Item = (&(ID, ID), &ContactConstraint<Obj>)> {
        self.map
            .iter()
            .filter(|(_, entry)| entry.is_active())
            .map(|(key, entry)| (key, &entry.constraint))
    }

    pub(crate) fn active_constraints_mut(
        &mut self,
    ) -> impl Iterator<Item = &mut ContactConstraint<Obj>> {
        self.map
            .values_mut()
            .filter(|entry| entry.is_active())
            .map(|entry| &mut entry.constraint)
    }

    pub(crate) fn warm_start_constraints_mut(
        &mut self,
    ) -> impl Iterator<Item = &mut ContactConstraint<Obj>> {
        self.map
            .values_mut()
            .filter(|entry| entry.can_warm_start_current_pass())
            .map(|entry| &mut entry.constraint)
    }

    pub(crate) fn refresh_contact_point_pairs_after_warm_start(&mut self) {
        self.map
            .values_mut()
            .filter(|entry| entry.is_active())
            .for_each(ContactManifoldEntry::refresh_contact_point_pairs_after_warm_start);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{collision::ContactPointPair, element::Element, math::FloatNum};

    const CONTACT_DEPTH: FloatNum = 0.25;
    const PAIR: (ID, ID) = (1, 2);

    fn contact_at_x(x: FloatNum) -> ContactPointPair {
        ContactPointPair::new(
            (x, 0.).into(),
            (x, CONTACT_DEPTH).into(),
            (0., -1.).into(),
            CONTACT_DEPTH,
        )
    }

    fn manager_with_active_pair() -> ContactManager<Element<()>> {
        let mut manager = ContactManager::default();
        manager.begin_collision_pass();
        manager.ingest_contact_point_pairs(PAIR.0, PAIR.1, vec![contact_at_x(0.)]);
        manager
    }

    fn cached_impulses(manager: &ContactManager<Element<()>>) -> Vec<(FloatNum, FloatNum)> {
        manager
            .get(&PAIR)
            .expect("managed contact exists")
            .cached_impulses_for_test()
    }

    fn set_cached_impulse(
        manager: &mut ContactManager<Element<()>>,
        total_lambda: FloatNum,
        total_friction_lambda: FloatNum,
    ) {
        manager
            .get_mut(&PAIR)
            .expect("managed contact exists")
            .set_cached_impulse_for_test(0, total_lambda, total_friction_lambda);
    }

    #[test]
    fn public_compat_surface_keeps_constraint_lifecycle_and_map_like_methods() {
        let mut constraint = ContactConstraint::<Element<()>>::new(1, 2, vec![contact_at_x(0.)]);
        assert!(constraint.is_active());
        constraint.set_is_active(false);
        assert!(!constraint.is_active());

        let mut manifold = ContactConstraintManifold::<Element<()>>::default();
        manifold.begin_collision_pass();
        manifold.ingest_contact_point_pairs(PAIR.0, PAIR.1, vec![contact_at_x(0.)]);

        assert!(manifold.contains_key(&PAIR));
        assert_eq!(manifold.keys().copied().collect::<Vec<_>>(), vec![PAIR]);
        assert_eq!(manifold.values().count(), 1);
        assert_eq!(manifold.iter().count(), 1);
        assert_eq!(manifold.values_mut().count(), 1);
    }

    #[test]
    fn contact_manager_keeps_lifecycle_outside_solver_constraint() {
        let mut manager = ContactManager::<Element<()>>::default();

        manager.begin_collision_pass();
        manager.ingest_contact_point_pairs(PAIR.0, PAIR.1, vec![contact_at_x(0.)]);

        assert!(manager.is_pair_active(PAIR));
        assert!(!manager.can_warm_start_pair(PAIR));
        assert!(!manager.has_pending_refresh(PAIR));

        manager.begin_collision_pass();

        assert!(!manager.is_pair_active(PAIR));
        assert!(!manager.can_warm_start_pair(PAIR));
        assert!(!manager.has_pending_refresh(PAIR));
    }

    #[test]
    fn contact_manager_recontact_after_inactive_replaces_without_warm_start() {
        let mut manager = manager_with_active_pair();
        set_cached_impulse(&mut manager, 4., 1.);

        manager.begin_collision_pass();
        manager.begin_collision_pass();
        manager.ingest_contact_point_pairs(PAIR.0, PAIR.1, vec![contact_at_x(0.0004)]);

        assert!(manager.is_pair_active(PAIR));
        assert!(!manager.can_warm_start_pair(PAIR));
        assert!(!manager.has_pending_refresh(PAIR));
        assert_eq!(cached_impulses(&manager), vec![(0., 0.)]);
    }

    #[test]
    fn contact_manager_continuing_pair_preserves_warm_start_until_refresh() {
        let mut manager = manager_with_active_pair();
        set_cached_impulse(&mut manager, 2.5, -0.75);

        manager.begin_collision_pass();
        manager.ingest_contact_point_pairs(PAIR.0, PAIR.1, vec![contact_at_x(0.0004)]);

        assert!(manager.is_pair_active(PAIR));
        assert!(manager.can_warm_start_pair(PAIR));
        assert!(manager.has_pending_refresh(PAIR));
        assert_eq!(cached_impulses(&manager), vec![(2.5, -0.75)]);

        manager.refresh_contact_point_pairs_after_warm_start();

        assert!(!manager.has_pending_refresh(PAIR));
        assert_eq!(cached_impulses(&manager), vec![(2.5, -0.75)]);
    }
}
