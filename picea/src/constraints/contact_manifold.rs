use std::collections::BTreeMap;

use macro_tools::Deref;

use crate::element::ID;

use super::{contact::ContactConstraint, ConstraintObject};

#[derive(Deref)]
pub struct ContactConstraintManifold<Obj: ConstraintObject> {
    #[deref]
    map: BTreeMap<(ID, ID), ContactConstraint<Obj>>,
}

impl<Obj: ConstraintObject> Default for ContactConstraintManifold<Obj> {
    fn default() -> Self {
        Self {
            map: Default::default(),
        }
    }
}

impl<Obj: ConstraintObject> ContactConstraintManifold<Obj> {
    pub fn insert(&mut self, id_pair: (ID, ID), constraints: ContactConstraint<Obj>) {
        self.map.insert(id_pair, constraints);
    }
}
