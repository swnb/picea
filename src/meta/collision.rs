use std::{mem, slice::IterMut};

use crate::algo::constraint::{ContactConstraint, ContactManifold, ManifoldsIterMut};

pub struct Manifold {
    pub(crate) collision_element_id_pair: (u32, u32),
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

#[derive(Default)]
pub(crate) struct ManifoldTable {
    pre_manifolds: Vec<Manifold>,
    current_manifolds: Vec<Manifold>,
}

impl ManifoldTable {
    pub fn clear(&mut self) {
        mem::swap(&mut self.pre_manifolds, &mut self.current_manifolds);
        self.current_manifolds.clear();
    }

    pub fn push(&mut self, manifold: Manifold) {
        self.current_manifolds.push(manifold);
    }

    pub fn pre_manifolds(&mut self) -> impl ManifoldsIterMut + '_ {
        &mut self.pre_manifolds[..]
    }

    pub fn current_manifolds(&mut self) -> impl ManifoldsIterMut + '_ {
        &mut self.current_manifolds[..]
    }
}

impl<'a> ManifoldsIterMut for &'a mut [Manifold] {
    type Manifold = Manifold;

    type Iter<'b> = IterMut<'b, Manifold> where Self:'b;

    fn iter_mut(&mut self) -> Self::Iter<'_> {
        <[Manifold]>::iter_mut(self)
    }
}
