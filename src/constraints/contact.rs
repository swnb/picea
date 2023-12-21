use crate::{
    algo::collision::ContactPointPair,
    element::{Element, ID},
    math::{num::limit_at_range, vector::Vector, FloatNum},
    scene::context::ConstraintParameters,
};

use super::{compute_mass_effective, ConstraintObject};

// TODO if two element is still collide in current frame, we can reuse this
// contact info , is two element is not collide anymore , we don't need this frame
pub struct ContactConstraint<Obj: ConstraintObject = Element> {
    contact_point_pair: ContactPointPair,
    total_friction_lambda: FloatNum,
    total_lambda: FloatNum,
    mass_effective: FloatNum,
    tangent_mass_effective: FloatNum,
    // vector from center point to  contact point
    r_a: Vector,
    r_b: Vector,
    // two collide obj
    obj_id_a: ID,
    obj_id_b: ID,
    obj_a: *mut Obj,
    obj_b: *mut Obj,
}

impl<Obj: ConstraintObject> ContactConstraint<Obj> {
    pub fn new(obj_id_a: ID, obj_id_b: ID, contact_point_pair: ContactPointPair) -> Self {
        Self {
            contact_point_pair,
            total_friction_lambda: 0.,
            total_lambda: 0.,
            mass_effective: 0.,
            tangent_mass_effective: 0.,
            r_a: Default::default(),
            r_b: Default::default(),
            obj_id_a,
            obj_id_b,
            obj_a: std::ptr::null_mut(),
            obj_b: std::ptr::null_mut(),
        }
    }

    pub fn obj_id_pair(&self) -> (ID, ID) {
        (self.obj_id_a, self.obj_id_b)
    }

    pub fn reset_total_lambda(&mut self) {
        self.total_friction_lambda = 0.;
        self.total_friction_lambda = 0.;
    }

    // restrict total lambda must big than zero
    fn restrict_lambda(&mut self, lambda: FloatNum) -> FloatNum {
        let previous_total_lambda = self.total_lambda;
        self.total_lambda = (self.total_lambda + lambda).max(0.);
        self.total_lambda - previous_total_lambda
    }

    pub unsafe fn reset_params(
        &mut self,
        parameters: &ConstraintParameters,
        (obj_a, obj_b): (*mut Obj, *mut Obj),
        delta_time: FloatNum,
    ) {
        self.obj_a = obj_a;
        self.obj_b = obj_b;
        self.total_friction_lambda = 0.;

        let object_a = &mut *self.obj_a;
        let object_b: &mut Obj = &mut *self.obj_b;
        let contact_point_pair = &self.contact_point_pair;

        let r_a = (object_a.center_point(), contact_point_pair.contact_point_a).into();
        let r_b = (object_b.center_point(), contact_point_pair.contact_point_b).into();

        let normal = contact_point_pair.normal_toward_a;

        let mass_effective = compute_mass_effective(&normal, object_a, object_b, r_a, r_b);

        let tangent_normal = !normal;

        let tangent_mass_effective =
            compute_mass_effective(&tangent_normal, object_a, object_b, r_a, r_b);

        self.mass_effective = mass_effective;
        self.tangent_mass_effective = tangent_mass_effective;
        self.r_a = r_a;
        self.r_b = r_b;
    }

    pub(crate) unsafe fn solve(
        &mut self,
        parameters: &ConstraintParameters,
        delta_time: FloatNum,
        fix_position: bool,
    ) {
        if fix_position {
            self.solve_position_constraint(parameters, delta_time);
        } else {
            self.solve_velocity_constraint(parameters, 0.);
        }
    }

    unsafe fn solve_velocity_constraint(
        &mut self,
        parameters: &ConstraintParameters,
        bias: FloatNum,
    ) {
        let obj_a = &mut *self.obj_a;
        let obj_b = &mut *self.obj_b;
        let contact_info = &mut self.contact_point_pair;

        let normal = contact_info.normal_toward_a;
        let mass_effective = self.mass_effective;
        let depth = contact_info.depth;

        let sum_velocity_a = obj_a.compute_point_velocity(&contact_info.contact_point_a);

        let sum_velocity_b = obj_b.compute_point_velocity(&contact_info.contact_point_b);

        let coefficient = (sum_velocity_a - sum_velocity_b) * -normal * (parameters.factor_elastic);

        debug_assert!(depth.is_sign_positive());

        let lambda = (coefficient + bias) * mass_effective;

        let lambda = self.restrict_lambda(lambda);

        let max_friction_lambda = self.total_lambda * parameters.factor_default_friction;

        let impulse = normal * lambda;

        obj_a.meta_mut().apply_impulse(impulse, self.r_a);

        obj_b.meta_mut().apply_impulse(-impulse, self.r_b);

        if !parameters.skip_friction_constraints {
            // TODO factor_friction use two element's factor_friction
            self.solve_friction_constraint(max_friction_lambda);
        }
    }

    // TODO add static friction , make object static
    unsafe fn solve_friction_constraint(&mut self, max_friction_lambda: FloatNum) {
        let obj_a = &mut *self.obj_a;
        let obj_b = &mut *self.obj_b;
        let contact_info = &mut self.contact_point_pair;

        let mass_effective = self.tangent_mass_effective;

        let sum_velocity_a = obj_a.compute_point_velocity(&contact_info.contact_point_a);

        let sum_velocity_b = obj_b.compute_point_velocity(&contact_info.contact_point_b);

        let tangent_normal = !contact_info.normal_toward_a;

        let friction_lambda = (sum_velocity_a - sum_velocity_b) * tangent_normal * mass_effective;

        let previous_total_friction_lambda = self.total_friction_lambda;
        self.total_friction_lambda += friction_lambda;
        self.total_friction_lambda = limit_at_range(
            self.total_friction_lambda,
            -(max_friction_lambda.abs())..=(max_friction_lambda.abs()),
        );
        let friction_lambda = self.total_friction_lambda - previous_total_friction_lambda;

        let friction_impulse: Vector = tangent_normal * friction_lambda;

        obj_a.meta_mut().apply_impulse(-friction_impulse, self.r_a);

        obj_b.meta_mut().apply_impulse(friction_impulse, self.r_b);
    }

    unsafe fn solve_position_constraint(
        &mut self,
        parameters: &ConstraintParameters,
        delta_time: FloatNum,
    ) {
        let contact_info = &mut self.contact_point_pair;

        // let permeate = (contact_info.depth - constraint_parameters.max_allow_permeate).max(0.);

        // let bias = constraint_parameters.factor_position_bias * permeate * delta_time.recip();

        // REVIEW
        let mut permeate = contact_info.depth - parameters.max_allow_permeate;

        if !parameters.allow_permeate_negative {
            permeate = permeate.max(0.)
        }

        let bias = permeate * delta_time.recip();

        self.solve_velocity_constraint(parameters, bias);
    }
}
