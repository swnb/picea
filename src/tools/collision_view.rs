use crate::{
    algo::collision::{gjk_collision_detective, GJKDifferencePoint},
    element::Element,
    math::{point::Point, vector::Vector},
    scene::Scene,
};

#[derive(Default)]
pub struct CollisionStatusViewer {
    gjk_point_groups: Vec<[(Point<f32>, Point<f32>); 3]>,
}

impl CollisionStatusViewer {
    pub fn on_update(&mut self, scene: &mut Scene) {
        self.gjk_point_groups.clear();
        for a in scene.elements_iter() {
            for b in scene.elements_iter() {
                if let Some(gjk_points) = self.detective_element_collision(a, b) {
                    self.gjk_point_groups
                        .push(gjk_points.map(|p| (p.start_point, p.end_point)));
                }
            }
        }
    }

    fn detective_element_collision(
        &mut self,
        a: &Element,
        b: &Element,
    ) -> Option<[GJKDifferencePoint; 3]> {
        let compute_support_point = |reference_vector: Vector<f32>| -> GJKDifferencePoint {
            let (_, max_point_a) = a.shape().projection_on_vector(&reference_vector);
            let (_, max_point_b) = b.shape().projection_on_vector(&-reference_vector);
            (max_point_b, max_point_a).into()
        };

        let first_approximation_vector: Vector<f32> =
            (a.shape().center_point(), b.shape().center_point()).into();

        gjk_collision_detective(first_approximation_vector, compute_support_point)
    }

    pub fn get_gjk_point_groups(&self) -> &[[(Point<f32>, Point<f32>); 3]] {
        &self.gjk_point_groups
    }
}
