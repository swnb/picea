use picea_macro_tools::Fields;

use crate::{
    element::ID,
    math::{point::Point, vector::Vector, FloatNum},
    scene::context::ConstraintParameters,
};

use super::{
    can_solve_with_positive_denominator, compute_inv_mass_effective,
    compute_soft_constraints_params, ConstraintObject, JoinConstraintConfig,
};

#[derive(Fields)]
pub struct JoinConstraint<Obj: ConstraintObject> {
    #[r]
    id: u32,
    obj_a_id: ID,
    obj_b_id: ID,
    obj_a: *mut Obj,
    obj_b: *mut Obj,
    #[w(vis(pub(crate)))]
    move_point_with_a: Point,
    #[w(vis(pub(crate)))]
    move_point_with_b: Point,
    total_lambda: FloatNum,
    #[r]
    #[w]
    config: JoinConstraintConfig,
    position_bias: FloatNum,
    soft_part: FloatNum,
    inv_mass_effective: FloatNum,
}

impl<Obj: ConstraintObject> JoinConstraint<Obj> {
    pub fn new(
        id: u32,
        (obj_a_id, obj_b_id): (ID, ID),
        (move_point_with_a, move_point_with_b): (Point, Point),
        config: JoinConstraintConfig,
    ) -> Self {
        assert!(
            config.distance >= 0.,
            "distance must large than or equal to zero"
        );

        Self {
            id,
            obj_a_id,
            obj_b_id,
            move_point_with_a,
            move_point_with_b,
            obj_a: std::ptr::null_mut(),
            obj_b: std::ptr::null_mut(),
            total_lambda: 0.,
            // force_soft_factor: 0.,
            // position_fix_factor: 0.,
            config,
            position_bias: 0.,
            soft_part: 0.,
            inv_mass_effective: 0.,
        }
    }

    pub fn obj_id_pair(&self) -> (ID, ID) {
        (self.obj_a_id, self.obj_b_id)
    }

    pub fn move_point_pair(&self) -> (&Point, &Point) {
        (&self.move_point_with_a, &self.move_point_with_b)
    }

    pub fn stretch_length(&self) -> Vector {
        (self.move_point_with_b, self.move_point_with_a).into()
    }

    pub(crate) unsafe fn reset_params(
        &mut self,
        (obj_a, obj_b): (*mut Obj, *mut Obj),
        (move_point_with_a, move_point_with_b): (Point, Point),
        delta_time: FloatNum,
    ) {
        self.move_point_with_a = move_point_with_a;
        self.move_point_with_b = move_point_with_b;
        self.total_lambda = 0.;

        let inv_delta_time = delta_time.recip();

        let obj_a = &mut *obj_a;
        let obj_b = &mut *obj_b;

        let meta_a = obj_a.meta();
        let meta_b = obj_b.meta();
        let mass = meta_a.mass() + meta_b.mass();

        let (force_soft_factor, position_fix_factor) = if self.config.hard {
            (0., 1.)
        } else {
            compute_soft_constraints_params(
                mass,
                self.config.damping_ratio,
                self.config.frequency,
                delta_time,
            )
        };

        let r_a: Vector = (obj_a.center_point(), move_point_with_a).into();

        let r_b: Vector = (obj_b.center_point(), move_point_with_b).into();

        let distance = self.stretch_length();

        let n = -distance.normalize();

        let inv_mass_effective = compute_inv_mass_effective(&n, (obj_a, obj_b), r_a, r_b);

        let position_fix = (distance.abs() - self.config.distance).max(0.);

        let position_bias = position_fix_factor * position_fix * inv_delta_time;

        let force_soft_part = force_soft_factor * inv_delta_time;

        self.inv_mass_effective = inv_mass_effective;
        self.position_bias = position_bias;
        self.soft_part = force_soft_part;
        self.obj_a = obj_a;
        self.obj_b = obj_b;
    }

    pub(crate) unsafe fn solve(&mut self, parameters: &ConstraintParameters) {
        let obj_a = &mut *self.obj_a;
        let obj_b = &mut *self.obj_b;
        let point_a = self.move_point_with_a;
        let point_b = self.move_point_with_b;

        let point_a_v = obj_a.compute_point_velocity(&point_a);
        let point_b_v = obj_b.compute_point_velocity(&point_b);

        let distance = self.stretch_length();

        if distance.abs() < parameters.max_allow_permeate() {
            return;
        }

        let n = distance.normalize();

        let r_a: Vector = (obj_a.center_point(), point_a).into();

        let r_b: Vector = (obj_b.center_point(), point_b).into();

        let &mut Self {
            inv_mass_effective,
            position_bias,
            soft_part,
            ..
        } = self;

        let jv_b = -(n * (point_a_v - point_b_v) + position_bias);

        let denominator = inv_mass_effective + soft_part;
        if !can_solve_with_positive_denominator(denominator) {
            return;
        }

        let lambda = jv_b * denominator.recip();
        if !lambda.is_finite() {
            return;
        }

        obj_a.meta_mut().apply_impulse(n * lambda, r_a);
        obj_b.meta_mut().apply_impulse(-n * lambda, r_b);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        constraints::JoinConstraintConfigBuilder,
        element::{Element, ElementBuilder},
        meta::MetaBuilder,
        shape::Circle,
    };

    const STEP_DT: FloatNum = 1. / 60.;

    fn circle_element(center_x: FloatNum, mass: FloatNum, is_fixed: bool) -> Element<()> {
        ElementBuilder::new(
            Circle::new((center_x, 0.), 1.),
            MetaBuilder::new().mass(mass).is_fixed(is_fixed),
            (),
        )
        .into()
    }

    fn hard_join_config(distance: FloatNum) -> JoinConstraintConfig {
        JoinConstraintConfigBuilder::new()
            .distance(distance)
            .hard(true)
            .into()
    }

    fn solve_center_join_once(
        mass_a: FloatNum,
        is_a_fixed: bool,
        mass_b: FloatNum,
        is_b_fixed: bool,
    ) -> (Vector, FloatNum, Vector, FloatNum) {
        let mut object_a = circle_element(0., mass_a, is_a_fixed);
        let mut object_b = circle_element(2., mass_b, is_b_fixed);
        let mut constraint = JoinConstraint::new(
            1,
            (1, 2),
            ((0., 0.).into(), (2., 0.).into()),
            hard_join_config(1.),
        );

        unsafe {
            constraint.reset_params(
                (&mut object_a as *mut _, &mut object_b as *mut _),
                ((0., 0.).into(), (2., 0.).into()),
                STEP_DT,
            );
            constraint.solve(&ConstraintParameters::default());
        }

        (
            *object_a.meta().velocity(),
            object_a.meta().angle_velocity(),
            *object_b.meta().velocity(),
            object_b.meta().angle_velocity(),
        )
    }

    fn assert_vector_finite(vector: Vector) {
        assert!(vector.x().is_finite(), "expected finite x, got {vector}");
        assert!(vector.y().is_finite(), "expected finite y, got {vector}");
    }

    fn assert_velocity_state_finite(state: &(Vector, FloatNum, Vector, FloatNum)) {
        assert_vector_finite(state.0);
        assert!(state.1.is_finite());
        assert_vector_finite(state.2);
        assert!(state.3.is_finite());
    }

    #[test]
    fn hard_join_between_two_fixed_bodies_noops() {
        let state = solve_center_join_once(1., true, 1., true);

        assert_velocity_state_finite(&state);
        assert_eq!(state.0, Vector::default());
        assert_eq!(state.2, Vector::default());
    }

    #[test]
    fn hard_join_noops_for_zero_mass_dynamic_body_without_nan() {
        let state = solve_center_join_once(1., true, 0., false);

        assert_velocity_state_finite(&state);
        assert_eq!(state.2, Vector::default());
    }

    #[test]
    fn hard_join_noops_for_non_finite_mass_dynamic_body_without_nan() {
        let state = solve_center_join_once(1., true, FloatNum::NAN, false);

        assert_velocity_state_finite(&state);
        assert_eq!(state.2, Vector::default());
    }

    #[test]
    fn hard_join_between_fixed_and_dynamic_body_moves_dynamic_side() {
        let state = solve_center_join_once(1., true, 1., false);

        assert_velocity_state_finite(&state);
        assert_eq!(state.0, Vector::default());
        assert!(
            state.2.x() < 0.,
            "dynamic body should be pulled toward the fixed body"
        );
    }

    #[test]
    fn hard_join_with_very_small_finite_denominator_solves() {
        let state = solve_center_join_once(10000000000., false, 10000000000., false);

        assert_velocity_state_finite(&state);
        assert!(
            state.0.x() > 0.,
            "finite tiny denominator should still move object A"
        );
        assert!(
            state.2.x() < 0.,
            "finite tiny denominator should still move object B"
        );
    }
}
