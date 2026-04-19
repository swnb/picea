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

impl<Obj: ConstraintObject> ContactConstraintManifold<Obj> {
    pub(crate) fn mark_all_inactive(&mut self) {
        self.values_mut().for_each(|manifold| {
            manifold.begin_collision_pass();
        });
    }

    pub(crate) fn refresh_contact_point_pairs_after_warm_start(&mut self) {
        self.values_mut()
            .filter(|manifold| manifold.is_active())
            .for_each(|manifold| {
                manifold.refresh_contact_point_pairs_after_warm_start();
            });
    }
}
