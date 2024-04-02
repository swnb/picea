use std::collections::BTreeMap;

use picea_macro_tools::Deref;

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
