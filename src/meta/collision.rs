use crate::algo::constraint::ContactPointPairInfo;

#[derive(Debug)]
pub struct Manifold {
    pub(crate) collision_element_id_pair: (u32, u32),
    pub(crate) contact_point_pairs: Vec<ContactPointPairInfo>,
}

impl Manifold {
    pub fn element_id_a(&self) -> u32 {
        self.collision_element_id_pair.0
    }

    pub fn element_id_b(&self) -> u32 {
        self.collision_element_id_pair.1
    }

    pub fn contact_point_pairs(&self) -> impl Iterator<Item = &'_ ContactPointPairInfo> {
        self.contact_point_pairs.iter()
    }

    pub fn contact_point_pairs_mut(
        &mut self,
    ) -> impl Iterator<Item = &'_ mut ContactPointPairInfo> {
        self.contact_point_pairs.iter_mut()
    }
}
