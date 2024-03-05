use std::ops::Deref;

use crate::{
    collision::ContactPointPair,
    element::ID,
    math::{point::Point, vector::Vector},
    scene::Scene,
};

pub struct CollisionInfo {
    pub points_a: Vec<Point>,
    pub points_b: Vec<Point>,
    pub vector: Vector,
}

#[derive(Default)]
pub struct CollisionStatusViewer {
    minkowski_different_gathers: Vec<Point>,
    minkowski_simplexes: Vec<[Point; 3]>,
    collision_infos: Vec<ContactInfos>,
}

#[derive(Debug)]
pub struct ContactInfos {
    object_id_pair: (ID, ID),
    contact_point_pair: ContactPointPair,
}

impl ContactInfos {
    pub fn object_id_pair(&self) -> (ID, ID) {
        self.object_id_pair
    }
}

impl Deref for ContactInfos {
    type Target = ContactPointPair;

    fn deref(&self) -> &Self::Target {
        &self.contact_point_pair
    }
}

impl CollisionStatusViewer {
    pub fn on_update<T>(&mut self, scene: &Scene<T>)
    where
        T: Clone + Default,
    {
        self.minkowski_simplexes.clear();
        self.collision_infos.truncate(0);

        scene
            .element_store
            .clone()
            .detective_collision(|element_a, element_b, contact_pairs| {
                self.collision_infos
                    .extend(contact_pairs.into_iter().map(|contact_pair| ContactInfos {
                        object_id_pair: (element_a.id(), element_b.id()),
                        contact_point_pair: contact_pair,
                    }))
            });
    }

    pub fn get_collision_infos(&self) -> &[ContactInfos] {
        &self.collision_infos
    }
}
