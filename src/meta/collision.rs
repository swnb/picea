use std::{
    collections::{btree_map, BTreeMap},
    iter::{Filter, Map},
    ops::Deref,
    slice::IterMut,
};

use crate::algo::constraint::{ContactManifold, ContactPointPairInfo, ManifoldsIterMut};

pub struct Manifold {
    pub(crate) collision_element_id_pair: (u32, u32),
    pub(crate) is_active: bool,
    pub(crate) contact_point_pairs: Vec<ContactPointPairInfo>,
}

impl ContactManifold for Manifold {
    type IterMut<'a> = IterMut<'a, ContactPointPairInfo> where Self:'a;

    fn collision_element_id_pair(&self) -> (u32, u32) {
        self.collision_element_id_pair
    }

    fn is_active(&self) -> bool {
        true
    }

    fn contact_point_pairs_iter_mut(&mut self) -> Self::IterMut<'_> {
        self.contact_point_pairs.iter_mut()
    }
}

#[derive(Default)]
pub(crate) struct ManifoldStore {
    inner_manifolds_map: BTreeMap<u64, ContactManifoldsWithState>,
}

pub(crate) struct ManifoldStoreIterMutCreator<'a> {
    manifold_store: &'a mut ManifoldStore,
}

impl ManifoldsIterMut for ManifoldStoreIterMutCreator<'_> {
    type Manifold = ContactManifoldsWithState;

    type Iter<'a> = btree_map::ValuesMut<'a, u64, ContactManifoldsWithState> where Self:'a;

    fn iter_mut(&mut self) -> Self::Iter<'_> {
        // TODO performance
        self.manifold_store.inner_manifolds_map.values_mut()
    }
}

pub(crate) struct ContactManifoldsWithState {
    collider_id_a: u32,
    collider_id_b: u32,
    is_active: bool,
    contact_pairs: Vec<ContactPointPairInfoWrapper>,
}

impl ContactManifold for ContactManifoldsWithState {
    type IterMut<'z> = Map<IterMut<'z,ContactPointPairInfoWrapper>,fn(&'z mut ContactPointPairInfoWrapper) -> &'z mut ContactPointPairInfo> where Self:'z;

    fn contact_point_pairs_iter_mut(&'_ mut self) -> Self::IterMut<'_> {
        self.contact_pairs
            .iter_mut()
            .map(|v| &mut v.contact_point_pair_info)
    }

    fn collision_element_id_pair(&self) -> (u32, u32) {
        (self.collider_id_a, self.collider_id_b)
    }

    fn is_active(&self) -> bool {
        self.is_active
    }
}

pub(crate) struct ContactPointPairInfoWrapper {
    reuse_count: u8,
    contact_point_pair_info: ContactPointPairInfo,
}

impl Deref for ContactPointPairInfoWrapper {
    type Target = ContactPointPairInfo;

    fn deref(&self) -> &Self::Target {
        &self.contact_point_pair_info
    }
}

impl ManifoldStore {
    pub fn inactive_all_manifolds(&mut self) {
        self.inner_manifolds_map.values_mut().for_each(|state| {
            state.is_active = false;
        });
    }

    pub fn new() -> Self {
        Default::default()
    }

    pub fn push(
        &mut self,
        Manifold {
            collision_element_id_pair: (id_a, id_b),
            contact_point_pairs,
            ..
        }: Manifold,
    ) {
        let id_pair: u64 = (id_a as u64) << 32 | (id_a as u64);

        if let Some(state) = self.inner_manifolds_map.get_mut(&id_pair) {
            state.is_active = true;
            // TODO max contact point should be two ?
            let length = state.contact_pairs.len();
            for new_contact_pair in contact_point_pairs {
                let mut is_found = false;
                for index in 0..length {
                    let pre_contact_pair = &mut state.contact_pairs[index];
                    let is_equal =
                        // TODO use eq trait;
                        ((&new_contact_pair).contact_point_a == (&pre_contact_pair).contact_point_a
                            && (&new_contact_pair).contact_point_b
                                == (&pre_contact_pair).contact_point_b)
                            || ((&new_contact_pair).contact_point_b
                                == (&pre_contact_pair).contact_point_a
                                && (&new_contact_pair).contact_point_a
                                    == (&pre_contact_pair).contact_point_b);

                    if is_equal {
                        pre_contact_pair.reuse_count = 0;
                        is_found = true;
                        break;
                    }
                }
                if !is_found {
                    state.contact_pairs.push(ContactPointPairInfoWrapper {
                        reuse_count: 0,
                        contact_point_pair_info: new_contact_pair,
                    });
                }
            }
        } else {
            let new_contact_manifolds = ContactManifoldsWithState {
                collider_id_a: id_a,
                collider_id_b: id_b,
                is_active: true,
                contact_pairs: contact_point_pairs
                    .into_iter()
                    .map(|contact_point_pair_info| ContactPointPairInfoWrapper {
                        reuse_count: 0,
                        contact_point_pair_info,
                    })
                    .collect(),
            };
            self.inner_manifolds_map
                .insert(id_pair, new_contact_manifolds);
        }
    }

    pub fn clear(&mut self) {
        self.inner_manifolds_map.clear();
    }

    pub fn update_all_manifolds_usage(&mut self) {
        self.inner_manifolds_map.retain(|_, value| value.is_active);

        self.inner_manifolds_map.values_mut().for_each(|manifold| {
            let contact_pairs = &mut manifold.contact_pairs;
            let mut new_contact_pairs_len = 0;

            for i in 0..contact_pairs.len() {
                if contact_pairs[i].reuse_count < 2 {
                    contact_pairs[i].reuse_count += 1;
                    contact_pairs.swap(new_contact_pairs_len, i);
                    new_contact_pairs_len += 1;
                }
            }
            unsafe {
                contact_pairs.set_len(new_contact_pairs_len);
            }
        });
    }

    pub fn manifolds_iter_mut_creator(&mut self) -> ManifoldStoreIterMutCreator<'_> {
        ManifoldStoreIterMutCreator {
            manifold_store: self,
        }
    }
}
