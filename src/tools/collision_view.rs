use crate::{
    algo::collision::{
        epa_compute_collision_edge, get_collision_contact_point, gjk_collision_detective,
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
    minkowski_different_points: Vec<[Point; 3]>,
    collision_infos: Vec<CollisionInfo>,
}

impl CollisionStatusViewer {
    pub fn on_update(&mut self, scene: &mut Scene) {
        self.minkowski_different_points.clear();
        self.collision_infos.clear();
        let elements: Vec<&Element> = scene.elements_iter().collect();
        for i in 0..elements.len() {
            for j in (i + 1)..elements.len() {
                let a = elements[i];
                let b = elements[j];
                self.detective_element_collision(a, b);
            }
        }
    }

    fn detective_element_collision(&mut self, a: &Element, b: &Element) {
        let compute_support_point = |reference_vector: Vector| {
            let (_, max_point_a) = a.shape().projection_on_vector(&reference_vector);
            let (_, max_point_b) = b.shape().projection_on_vector(&-reference_vector);
            (max_point_a, max_point_b).into()
        };

        let first_approximation_vector: Vector =
            (a.shape().center_point(), b.shape().center_point()).into();

        let Some(simplex) = gjk_collision_detective(first_approximation_vector, compute_support_point) else {
            return;
        };

        self.minkowski_different_points.push({
            let simplex = simplex.clone();
            simplex.map(|ref p| p.vector.to_point())
        });

        let edge = epa_compute_collision_edge(simplex, compute_support_point);

        let contact_points =
            get_collision_contact_point(&edge, a.shape().center_point(), b.shape().center_point());

        let info = CollisionInfo {
            points_a: contact_points.iter().map(|v| v.0.contact_point).collect(),
            points_b: contact_points.iter().map(|v| v.1.contact_point).collect(),
            vector: edge.normal,
        };

        self.collision_infos.push(info);
    }

    pub fn get_minkowski_different_points(&self) -> &[[Point; 3]] {
        &self.minkowski_different_points
    }

    pub fn get_collision_infos(&self) -> &[CollisionInfo] {
        &self.collision_infos
    }
}
