use crate::{
    collision::ContactPointPair,
    element::ID,
    math::{num::limit_at_range, vector::Vector, FloatNum, TAU},
    scene::context::ConstraintParameters,
};

use super::{compute_inv_mass_effective, ConstraintObject};

// TODO if two element is still collide in current frame, we can reuse this
// contact info , is two element is not collide anymore , we don't need this frame
pub struct ContactConstraint<Obj: ConstraintObject> {
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
    max_allow_restrict_impulse: FloatNum,
    inv_delta_time: FloatNum,
    is_active: bool,
    friction: FloatNum,
    factor_restitution: FloatNum,
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
            inv_delta_time: 0.,
            max_allow_restrict_impulse: 0.,
            is_active: true,
            friction: 0.,
            factor_restitution: 0.,
        }
    }

    pub fn contact_point_pair(&self) -> &ContactPointPair {
        &self.contact_point_pair
    }

    pub fn update_contact_point_pair(&mut self, contact_point_pair: ContactPointPair) {
        self.contact_point_pair = contact_point_pair
    }

    pub fn set_active(&mut self, is_active: bool) {
        self.is_active = is_active
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }

    pub fn obj_id_pair(&self) -> (ID, ID) {
        (self.obj_id_a, self.obj_id_b)
    }

    pub unsafe fn object_a_mut(&mut self) -> &mut Obj {
        &mut *self.obj_a
    }

    pub unsafe fn object_b_mut(&mut self) -> &mut Obj {
        &mut *self.obj_b
    }

    pub unsafe fn object_a(&self) -> &Obj {
        &*self.obj_a
    }

    pub unsafe fn object_b(&self) -> &Obj {
        &*self.obj_b
    }

    // restrict total lambda must big than zero
    pub(crate) fn restrict_contact_lambda(&mut self, lambda: FloatNum) -> FloatNum {
        // if speed is very large , than sequence impulse is bad when resolve large speed
        if lambda > self.max_allow_restrict_impulse {
            self.total_lambda = 0.;
            return lambda;
        }

        let previous_total_lambda = self.total_lambda;
        self.total_lambda = (self.total_lambda + lambda).max(0.);
        self.total_lambda - previous_total_lambda
    }

    pub(crate) unsafe fn reset_params(
        &mut self,
        (obj_a, obj_b): (*mut Obj, *mut Obj),
        delta_time: FloatNum,
        parameters: &ConstraintParameters,
    ) {
        self.obj_a = obj_a;
        self.obj_b = obj_b;
        self.total_friction_lambda = 0.;
        self.max_allow_restrict_impulse =
            parameters.max_allow_restrict_force_for_contact_solve * delta_time;
        self.inv_delta_time = delta_time.recip();

        let object_a = &mut *self.obj_a;
        let object_b = &mut *self.obj_b;
        let contact_point_pair = &self.contact_point_pair;

        *object_a.meta_mut().contact_count_mut() += 1;
        *object_b.meta_mut().contact_count_mut() += 1;

        self.friction = (object_a.meta().friction() * object_b.meta().friction()).sqrt();

        self.factor_restitution =
            (object_a.meta().factor_restitution() * object_b.meta().factor_restitution()).sqrt();

        let contact_point = ((contact_point_pair.point_a().to_vector()
            + contact_point_pair.point_b().to_vector())
            * 0.5)
            .to_point();

        let r_a = (&object_a.center_point(), &contact_point).into();
        let r_b = (&object_b.center_point(), &contact_point).into();

        let normal = contact_point_pair.normal_toward_a();

        let mass_effective =
            compute_inv_mass_effective(&normal, (object_a, object_b), r_a, r_b).recip();

        let tangent_normal = !normal;

        let tangent_mass_effective =
            compute_inv_mass_effective(&tangent_normal, (object_a, object_b), r_a, r_b).recip();

        self.mass_effective = mass_effective;
        self.tangent_mass_effective = tangent_mass_effective;
        self.r_a = r_a;
        self.r_b = r_b;
    }

    pub(crate) unsafe fn solve_velocity_constraint(
        &mut self,
        parameters: &ConstraintParameters,
        iter_count: u8,
    ) {
        let contact_info = &mut self.contact_point_pair;

        let obj_a = &mut *self.obj_a;
        let obj_b = &mut *self.obj_b;
        let jv_a = contact_info.normal_toward_a();
        let jv_b = -jv_a;

        let contact_point_a = contact_info.point_a();
        let contact_point_b = contact_info.point_b();
        let contact_point_b_ = *contact_info.point_b();

        let contact_point =
            ((contact_point_a.to_vector() + contact_point_b.to_vector()) * 0.5).to_point();

        let v_a = obj_a.compute_point_velocity(&contact_point);
        let v_b = obj_b.compute_point_velocity(&contact_point);

        let jv = v_a * jv_a + v_b * jv_b;

        let position_bias =
            (contact_info.depth() - parameters.max_allow_permeate).max(0.) * self.inv_delta_time;

        let bias = if parameters.split_position_fix {
            0.
        } else {
            -position_bias
        };

        let lambda = -((1. + self.factor_restitution) * jv + (bias * 0.3)) * self.mass_effective;
        let lambda = self.restrict_contact_lambda(lambda);

        // let contact_count_a = obj_a.meta().contact_count();
        // let contact_count_b = obj_b.meta().contact_count();

        obj_a.meta_mut().apply_impulse(jv_a * lambda, self.r_a);

        obj_b.meta_mut().apply_impulse(jv_b * lambda, self.r_b);

        // {

        let v_b = obj_b.compute_point_velocity(&contact_point_b_);

        // }

        if iter_count >= 10 && !parameters.skip_friction_constraints {
            self.solve_friction_constraint(lambda + position_bias);
        }
    }

    // TODO add static friction , make object static
    pub(crate) unsafe fn solve_friction_constraint(&mut self, max_friction_lambda: FloatNum) {
        let obj_a = &mut *self.obj_a;
        let obj_b = &mut *self.obj_b;
        let contact_info = &mut self.contact_point_pair;

        let mass_effective = self.tangent_mass_effective;

        let contact_point_a = contact_info.point_a();
        let contact_point_b = contact_info.point_b();

        let contact_point =
            ((contact_point_a.to_vector() + contact_point_b.to_vector()) * 0.5).to_point();

        let sum_velocity_a = obj_a.compute_point_velocity(&contact_point);

        let sum_velocity_b = obj_b.compute_point_velocity(&contact_point);

        let tangent_normal = !contact_info.normal_toward_a();

        let mut friction_lambda =
            (sum_velocity_a - sum_velocity_b) * tangent_normal * mass_effective * self.friction;

        // if friction_lambda > (2.0 * self.inv_delta_time.recip()) {
        //     self.total_friction_lambda = 0.;
        // } else {
        let previous_total_friction_lambda: f32 = self.total_friction_lambda;
        self.total_friction_lambda += friction_lambda;
        self.total_friction_lambda = limit_at_range(
            self.total_friction_lambda,
            -(max_friction_lambda.abs())..=(max_friction_lambda.abs()),
        );
        friction_lambda = self.total_friction_lambda - previous_total_friction_lambda;
        // }

        let friction_impulse = tangent_normal * friction_lambda;

        obj_a.meta_mut().apply_impulse(-friction_impulse, self.r_a);

        obj_b.meta_mut().apply_impulse(friction_impulse, self.r_b);
    }

    pub(crate) unsafe fn prepare_solve_position_constraint(&self) -> ContactPointPair {
        let delta_angle_a = self.object_a().meta().get_delta_angle();

        let delta_angle_b = self.object_b().meta().get_delta_angle();

        let pre_position_a = self.object_a().meta().pre_position();
        let position_a = self.object_a().meta().position();

        let pre_position_b = self.object_b().meta().pre_position();
        let position_b = self.object_b().meta().position();

        let contact_info = &self.contact_point_pair;

        let point_a = contact_info.point_a();
        let point_b = contact_info.point_b();

        let mut r_a: Vector = (pre_position_a, point_a).into();
        r_a.affine_transformation_rotate_self(delta_angle_a);
        let point_a = position_a + &r_a;

        let mut r_b: Vector = (pre_position_b, point_b).into();
        r_b.affine_transformation_rotate_self(delta_angle_b);
        let point_b = position_b + &r_b;

        let normal: Vector = (point_a, point_b).into();

        let normal_toward_a = if normal * (*position_a - *position_b) < 0. {
            -normal
        } else {
            normal
        };

        ContactPointPair::new(
            point_a,
            point_b,
            normal_toward_a.normalize(),
            (*position_a - *position_b).abs(),
        )
    }

    pub(crate) unsafe fn solve_position_constraint(
        &mut self,
        parameters: &ConstraintParameters,
        index: usize,
    ) {
        let delta_angle_a = self.object_a().meta().get_delta_angle();
        let delta_position_a = self.object_a().meta().get_delta_position();
        let delta_angle_b = self.object_b().meta().get_delta_angle();
        let delta_position_b = self.object_b().meta().get_delta_position();

        let normal_toward_a = self.contact_point_pair.normal_toward_a();

        let contact_point_pair = self.prepare_solve_position_constraint();

        // let n = contact_point_pair.normal_toward_a();
        let n = normal_toward_a;

        let obj_a = &mut *self.obj_a;
        let obj_b = &mut *self.obj_b;
        let obj_a_meta = obj_a.meta();
        let obj_b_meta = obj_b.meta();

        let r_a = self.r_a.affine_transformation_rotate(delta_angle_a);
        let r_b = self.r_b.affine_transformation_rotate(delta_angle_b);

        let inv_mass_effective = obj_a_meta.inv_mass()
            + obj_a_meta.inv_mass()
            + obj_a_meta.inv_moment_of_inertia() * (r_a ^ n).powf(2.)
            + obj_b_meta.inv_moment_of_inertia() * (r_b ^ n).powf(2.);

        let contact_count_a = obj_a_meta.contact_count();
        let contact_count_b = obj_b_meta.contact_count();

        let permeate: FloatNum =
            n * (*contact_point_pair.point_b() - *contact_point_pair.point_a());

        let mut depth_fix = permeate - parameters.max_allow_permeate;

        // FIXME impossible
        // debug_assert!(depth_fix.is_sign_positive());
        // if depth_fix < 0. {
        //     return;
        // }

        // if obj_a_meta.is_fixed() || obj_b_meta.is_fixed() {
        //     depth_fix *= 2.;
        // }

        const POSITION_DAMPEN: FloatNum = 0.1;

        depth_fix *= POSITION_DAMPEN;

        let c = n * depth_fix;

        let f = c * inv_mass_effective.recip();

        obj_a.apply_position_fix(f * (contact_count_a as FloatNum).recip(), r_a);

        obj_b.apply_position_fix(-f * (contact_count_b as FloatNum).recip(), r_b);
    }
}
