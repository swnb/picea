use picea_macro_tools::Fields;

use crate::{
    element::ID,
    math::{point::Point, vector::Vector, FloatNum},
    scene::context::ConstraintParameters,
};

use super::{
    can_solve_with_positive_denominator, compute_soft_constraints_params, ConstraintObject,
    JoinConstraintConfig,
};

#[derive(Fields)]
pub struct PointConstraint<Obj: ConstraintObject> {
    #[r]
    id: u32,
    #[r]
    obj_id: ID,
    #[r]
    #[w]
    fixed_point: Point,
    #[r]
    #[w(vis(pub(crate)))]
    move_point: Point, // bind with element
    #[r]
    total_lambda: FloatNum,
    // force_soft_factor: FloatNum,
    // position_fix_factor: FloatNum,
    // distance must large than zero
    position_bias: FloatNum,
    soft_part: FloatNum,
    mass_effective: FloatNum,
    obj: *mut Obj,
    #[r]
    #[w]
    config: JoinConstraintConfig,
}

impl<Obj: ConstraintObject> PointConstraint<Obj> {
    pub fn new(
        id: u32,
        obj_id: ID,
        fixed_point: Point,
        move_point: Point,
        config: JoinConstraintConfig,
    ) -> Self {
        assert!(
            config.distance >= 0.,
            "distance must large than or equal to zero"
        );

        Self {
            id,
            obj_id,
            fixed_point,
            move_point,
            total_lambda: 0.,
            position_bias: 0.,
            soft_part: 0.,
            mass_effective: 0.,
            obj: std::ptr::null_mut(),
            config,
        }
    }

    pub fn stretch_length(&self) -> Vector {
        (self.move_point, self.fixed_point).into()
    }

    pub(crate) unsafe fn reset_params(
        &mut self,
        move_point: Point,
        obj: *mut Obj,
        delta_time: FloatNum,
    ) {
        self.move_point = move_point;
        self.total_lambda = 0.;

        let meta = (*obj).meta();
        let mass = meta.mass();
        let inv_mass = meta.effective_inv_mass();
        let inv_moment_of_inertia = meta.effective_inv_moment_of_inertia();

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

        let strength_length = self.stretch_length();
        let n = -strength_length.normalize();

        let position_bias = position_fix_factor
            * (strength_length.abs() - self.config.distance)
            * delta_time.recip();

        let soft_part = force_soft_factor * delta_time.recip();

        let r_t: Vector = ((*obj).center_point(), self.move_point).into();

        let mass_effective = inv_mass + (r_t ^ n).powf(2.) * inv_moment_of_inertia;

        // self.force_soft_factor = force_soft_factor;
        // self.position_fix_factor = position_fix_factor;
        self.position_bias = position_bias;
        self.soft_part = soft_part;
        self.mass_effective = mass_effective;
        self.obj = obj;
    }

    pub(crate) unsafe fn solve(&mut self, parameters: &ConstraintParameters) {
        let strength_length = self.stretch_length();
        if strength_length.abs() < parameters.max_allow_permeate() {
            // no constraint if there is no need
            return;
        }

        let obj = &mut *self.obj;

        let &mut Self {
            position_bias,
            mass_effective,
            soft_part,
            ..
        } = self;

        let denominator = soft_part + mass_effective;
        if !can_solve_with_positive_denominator(denominator) {
            return;
        }

        let r_t: Vector = (obj.center_point(), self.move_point).into();

        let n = -strength_length.normalize();

        let v: Vector = obj.compute_point_velocity(&self.move_point);

        let jv_b: f32 = -(v * n + position_bias);

        let lambda = jv_b * denominator.recip();
        if !lambda.is_finite() {
            return;
        }

        let impulse = n * lambda;

        // TODO restrict here

        obj.meta_mut().apply_impulse(impulse, r_t);
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

    fn hard_point_config(distance: FloatNum) -> JoinConstraintConfig {
        JoinConstraintConfigBuilder::new()
            .distance(distance)
            .hard(true)
            .into()
    }

    fn solve_center_point_once(mass: FloatNum, is_fixed: bool) -> (Vector, FloatNum) {
        let mut object = circle_element(2., mass, is_fixed);
        let mut constraint = PointConstraint::new(
            1,
            1,
            (0., 0.).into(),
            (2., 0.).into(),
            hard_point_config(1.),
        );

        unsafe {
            constraint.reset_params((2., 0.).into(), &mut object as *mut _, STEP_DT);
            constraint.solve(&ConstraintParameters::default());
        }

        (*object.meta().velocity(), object.meta().angle_velocity())
    }

    fn assert_vector_finite(vector: Vector) {
        assert!(vector.x().is_finite(), "expected finite x, got {vector}");
        assert!(vector.y().is_finite(), "expected finite y, got {vector}");
    }

    fn assert_velocity_state_finite(state: &(Vector, FloatNum)) {
        assert_vector_finite(state.0);
        assert!(state.1.is_finite());
    }

    #[test]
    fn hard_point_noops_for_zero_mass_dynamic_body_without_nan() {
        let state = solve_center_point_once(0., false);

        assert_velocity_state_finite(&state);
        assert_eq!(state.0, Vector::default());
        assert_eq!(state.1, 0.);
    }

    #[test]
    fn hard_point_noops_for_non_finite_mass_dynamic_body_without_nan() {
        let state = solve_center_point_once(FloatNum::NAN, false);

        assert_velocity_state_finite(&state);
        assert_eq!(state.0, Vector::default());
        assert_eq!(state.1, 0.);
    }

    #[test]
    fn hard_point_with_very_small_finite_denominator_solves() {
        let state = solve_center_point_once(10000000000., false);

        assert_velocity_state_finite(&state);
        assert!(
            state.0.x() < 0.,
            "finite tiny denominator should still move the dynamic body"
        );
    }
}
