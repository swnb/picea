use crate::{
    collision::ContactPointPair,
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
    collision_infos: Vec<ContactPointPair>,
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
            .detective_collision(|_, _, contact_pairs| {
                self.collision_infos.extend(contact_pairs);
            });
    }

    pub fn get_collision_infos(&self) -> &[ContactPointPair] {
        &self.collision_infos
    }
}
