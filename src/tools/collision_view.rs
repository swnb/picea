use crate::{
    algo::collision::{
        epa_compute_collision_edge, get_collision_contact_point, gjk_collision_detective,
    },
    element::Element,
    math::{point::Point, vector::Vector},
    scene::Scene,
};

pub struct CollisionInfo {
    pub points: Vec<Point<f32>>,
    pub vector: Vector<f32>,
}

#[derive(Default)]
pub struct CollisionStatusViewer {
    minkowski_different_points: Vec<[Point<f32>; 3]>,
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
        let compute_support_point = |reference_vector: Vector<f32>| {
            let (_, max_point_a) = a.shape().projection_on_vector(&reference_vector);
            let (_, max_point_b) = b.shape().projection_on_vector(&-reference_vector);
            (max_point_a, max_point_b).into()
        };

        let first_approximation_vector: Vector<f32> =
            (a.shape().center_point(), b.shape().center_point()).into();

        let Some(simplex) = gjk_collision_detective(first_approximation_vector, compute_support_point) else {
            return;
        };

        self.minkowski_different_points.push({
            let simplex = simplex.clone();
            simplex.map(|ref p| p.vector.to_point())
        });

        let edge = epa_compute_collision_edge(simplex, compute_support_point);

        // let (contact_info_a, contact_info_b) =
        let contact_points =
            get_collision_contact_point(&edge, a.shape().center_point(), b.shape().center_point());

        let (_, point1) = a.shape().projection_on_vector(&edge.normal);

        let (_, point2) = b.shape().projection_on_vector(&-edge.normal);

        let info = CollisionInfo {
            points: contact_points,
            vector: edge.normal,
        };

        self.collision_infos.push(info);
    }

    pub fn get_minkowski_different_points(&self) -> &[[Point<f32>; 3]] {
        &self.minkowski_different_points
    }

    pub fn get_collision_infos(&self) -> &[CollisionInfo] {
        &self.collision_infos
    }
}
