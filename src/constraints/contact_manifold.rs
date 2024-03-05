use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
};

use crate::element::ID;

use super::{contact::ContactConstraint, ConstraintObject};

pub struct ContactConstraintManifold<Obj: ConstraintObject> {
    map: BTreeMap<(ID, ID), ContactConstraint<Obj>>,
}

impl<Obj: ConstraintObject> Default for ContactConstraintManifold<Obj> {
    fn default() -> Self {
        Self {
            map: Default::default(),
        }
    }
}

impl<Obj: ConstraintObject> Deref for ContactConstraintManifold<Obj> {
    type Target = BTreeMap<(ID, ID), ContactConstraint<Obj>>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl<Obj: ConstraintObject> DerefMut for ContactConstraintManifold<Obj> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}

impl<Obj: ConstraintObject> ContactConstraintManifold<Obj> {
    pub fn insert(&mut self, id_pair: (ID, ID), constraints: ContactConstraint<Obj>) {
        self.map.insert(id_pair, constraints);
    }
}
