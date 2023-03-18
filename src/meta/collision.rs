use crate::{
    algo::collision::ContactPointPair,
    math::{point::Point, vector::Vector, CommonNum},
};

#[derive(Debug)]
pub struct CollisionInfo {
    pub(crate) collision_element_id_pair: (u32, u32),
    // TODO contact_point_pair should be vector, avoid compute multi times
    pub(crate) contact_point_pair: ContactPointPair,
    pub(crate) mass_effective: Option<CommonNum>,
}

impl CollisionInfo {
    pub fn element_id_a(&self) -> u32 {
        self.collision_element_id_pair.0
    }

    pub fn element_id_b(&self) -> u32 {
        self.collision_element_id_pair.1
    }

    pub fn contact_point_a(&self) -> &Point {
        &self.contact_point_pair.contact_point_a
    }

    pub fn contact_point_b(&self) -> &Point {
        &self.contact_point_pair.contact_point_b
    }

    pub fn contact_points(&self) -> (&Point, &Point) {
        (self.contact_point_a(), self.contact_point_b())
    }

    pub fn depth(&self) -> f32 {
        self.contact_point_pair.depth
    }

    pub fn normal(&self) -> Vector {
        self.contact_point_pair.normal_toward_a
    }

    pub fn mass_effective(&self) -> Option<CommonNum> {
        self.mass_effective
    }

    pub fn set_mass_effective(&mut self, mass_effective: CommonNum) {
        self.mass_effective = Some(mass_effective)
    }
}
