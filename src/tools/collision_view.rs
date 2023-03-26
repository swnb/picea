use crate::{
    algo::collision::{
        compute_minkowski, epa_compute_collision_edge, gjk_collision_detective, ContactPointPair,
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
    minkowski_different: Vec<Point>,
    minkowski_different_points: Vec<[Point; 3]>,
    collision_infos: Vec<ContactPointPair>,
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

        let center_point_a = a.shape().center_point();
        let center_point_b = b.shape().center_point();

        let first_approximation_vector: Vector = (center_point_a, center_point_b).into();

        let Some(simplex) = gjk_collision_detective(first_approximation_vector, compute_support_point) else {
            return;
        };

        self.minkowski_different_points.push({
            let simplex = simplex.clone();
            simplex.map(|ref p| p.vector.to_point())
        });

        self.minkowski_different = compute_minkowski(compute_support_point)
            .into_iter()
            .map(|different_point| different_point.vector.to_point())
            .collect();

        let edge = epa_compute_collision_edge(simplex, compute_support_point);

        let contact_point_pairs = edge.get_contact_info(center_point_a, center_point_b);

        // let info = CollisionInfo {
        //     points_a: contact_point_pairs
        //         .iter()
        //         .map(|pair| pair.contact_point_a)
        //         .collect(),
        //     points_b: contact_point_pairs
        //         .iter()
        //         .map(|pair| pair.contact_point_b)
        //         .collect(),
        //     vector: edge.normal,
        // };

        self.collision_infos.extend(contact_point_pairs);
    }

    // TODO chore rename
    pub fn get_minkowski_different_points(&self) -> &[[Point; 3]] {
        &self.minkowski_different_points
    }

    pub fn get_collision_infos(&self) -> &[ContactPointPair] {
        &self.collision_infos
    }

    // TODO chore rename
    pub fn get_all_minkowski_different_points(&self) -> &[Point] {
        &self.minkowski_different
    }
}
