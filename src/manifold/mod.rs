pub(crate) mod join;

use std::{
    collections::{btree_map::ValuesMut, BTreeMap},
    mem,
    slice::IterMut,
};

use crate::algo::constraint::{ContactConstraint, ContactManifold, ManifoldsIterMut};

pub struct Manifold {
    pub(crate) collision_element_id_pair: (u32, u32),
    pub(crate) reusable: bool,
    pub(crate) contact_constraints: Vec<ContactConstraint>,
}

impl ContactManifold for Manifold {
    type IterMut<'a> = IterMut<'a, ContactConstraint> where Self:'a;

    fn collision_element_id_pair(&self) -> (u32, u32) {
        self.collision_element_id_pair
    }

    fn contact_constraints_iter_mut(&mut self) -> Self::IterMut<'_> {
        self.contact_constraints.iter_mut()
    }
}

struct Manifolds<'a>(&'a mut BTreeMap<u64, Manifold>);

#[derive(Default)]
pub(crate) struct ManifoldTable {
    pre_manifolds: BTreeMap<u64, Manifold>,
    current_manifolds: BTreeMap<u64, Manifold>,
}

impl ManifoldTable {
    pub fn flip(&mut self) {
        mem::swap(&mut self.pre_manifolds, &mut self.current_manifolds);
        self.current_manifolds.clear();
    }

    pub fn push(&mut self, manifold: Manifold) {
        let (id_a, id_b) = manifold.collision_element_id_pair();
        let id_pair = ((id_a as u64) << 32) | id_b as u64;

        if let Some(manifold) = self.pre_manifolds.get_mut(&id_pair) {
            manifold.reusable = true
        }

        if let Some(origin_manifold) = self.current_manifolds.get_mut(&id_pair) {
            origin_manifold
                .contact_constraints
                .extend(manifold.contact_constraints);
        } else {
            self.current_manifolds.insert(id_pair, manifold);
        }
    }

    pub fn shrink_pre_manifolds(&mut self) {
        self.pre_manifolds.retain(|_, manifold| manifold.reusable);
    }

    pub fn pre_manifolds(&mut self) -> impl ManifoldsIterMut + '_ {
        Manifolds(&mut self.pre_manifolds)
    }

    pub fn current_manifolds(&mut self) -> impl ManifoldsIterMut + '_ {
        Manifolds(&mut self.current_manifolds)
    }

    pub fn clear(&mut self) {
        self.pre_manifolds.clear();
        self.current_manifolds.clear();
    }
}

impl<'a> ManifoldsIterMut for Manifolds<'a> {
    type Manifold = Manifold;

    type Iter<'b> = ValuesMut<'b,u64, Manifold> where Self:'b;

    fn iter_mut(&mut self) -> Self::Iter<'_> {
        self.0.values_mut()
    }
}
