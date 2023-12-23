use crate::{
    collision::{
        compute_minkowski, epa_compute_collision_edge, gjk_collision_detective, Collider,
        ContactPointPair, SubCollider,
    },
    element::Element,
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
    pub fn on_update(&mut self, scene: &mut Scene) {
        self.minkowski_simplexes.clear();
        self.collision_infos.clear();
        let elements: Vec<&Element> = scene.elements_iter().collect();
        for i in 0..elements.len() {
            for j in (i + 1)..elements.len() {
                let a = elements[i];
                let b = elements[j];
                self.detective_collider_collision(a, b);
            }
        }
    }

    fn detective_collider_collision<A, B>(&mut self, a: &A, b: &B)
    where
        A: Collider,
        B: Collider,
    {
        let sub_colliders_a = a.sub_colliders();
        let sub_colliders_b = b.sub_colliders();

        match (sub_colliders_a, sub_colliders_b) {
            // TODO
            (Some(sub_colliders_a), Some(sub_colliders_b)) => {
                for collider_a in sub_colliders_a {
                    let sub_colliders_b = b.sub_colliders().unwrap();
                    for collider_b in sub_colliders_b {
                        self.detective_sub_collider_collision(collider_a, collider_b);
                    }
                }
            }
            (Some(sub_colliders_a), None) => {
                for collider_a in sub_colliders_a {
                    self.detective_sub_collider_collision(collider_a, b);
                }
            }
            (None, Some(sub_colliders_b)) => {
                for collider_b in sub_colliders_b {
                    self.detective_sub_collider_collision(a, collider_b);
                }
            }
            (None, None) => {
                self.detective_sub_collider_collision(a, b);
            }
        }
    }

    fn detective_sub_collider_collision(&mut self, a: &dyn SubCollider, b: &dyn SubCollider) {
        let compute_support_point = |reference_vector: Vector| {
            let (_, max_point_a) = a.projection_on_vector(&reference_vector);
            let (_, max_point_b) = b.projection_on_vector(&-reference_vector);
            (max_point_a, max_point_b).into()
        };

        let center_point_a = a.center_point();
        let center_point_b = b.center_point();

        let first_approximation_vector: Vector = (center_point_a, center_point_b).into();

        let Some(simplex) =
            gjk_collision_detective(first_approximation_vector, compute_support_point)
        else {
            return;
        };

        self.minkowski_simplexes.push({
            let simplex = simplex.clone();
            simplex.map(|ref p| p.to_point())
        });

        self.minkowski_different_gathers = compute_minkowski(compute_support_point)
            .into_iter()
            .map(|different_point| different_point.to_point())
            .collect();

        let edge = epa_compute_collision_edge(simplex, compute_support_point);

        let contact_constraints = edge.get_contact_info(a, b, true);

        self.collision_infos.extend(contact_constraints);
    }

    pub fn get_minkowski_simplexes(&self) -> &[[Point; 3]] {
        &self.minkowski_simplexes
    }

    pub fn get_collision_infos(&self) -> &[ContactPointPair] {
        &self.collision_infos
    }

    // TODO chore rename
    pub fn get_all_minkowski_different_gathers(&self) -> &[Point] {
        &self.minkowski_different_gathers
    }
}
