use std::ops::Deref;

use crate::{
    collision::ContactPointPair,
    element::ID,
    math::{num::limit_at_range, vector::Vector, FloatNum},
    meta::Meta,
    scene::context::ConstraintParameters,
};

use super::{compute_inv_mass_effective, ConstraintObject};

// TODO if two element is still collide in current frame, we can reuse this
// contact info , is two element is not collide anymore , we don't need this frame
pub struct ContactConstraint<Obj: ConstraintObject> {
    contact_point_pair_constraint_infos: Vec<ContactPointPairConstraintInfo>,
    total_lambda: Vector,
    total_friction_lambda: Vector,
    // two collide obj
    obj_id_a: ID,
    obj_id_b: ID,
    obj_a: *mut Obj,
    obj_b: *mut Obj,
    max_allow_restrict_impulse: FloatNum,
    inv_delta_time: FloatNum,
    is_active: bool,
    factor_friction: FloatNum,
    factor_restitution: FloatNum,
    velocity_a: Vector,
    velocity_b: Vector,
    angle_velocity_a: FloatNum,
    angle_velocity_b: FloatNum,
}

#[derive(Default)]
pub struct ContactPointPairConstraintInfo {
    concat_point_pair: ContactPointPair,
    r_a: Vector,
    r_b: Vector,
    mass_effective: FloatNum,
    tangent_mass_effective: FloatNum,
    max_allow_restrict_impulse: FloatNum,
    real_total_lambda: FloatNum,
    total_lambda: FloatNum,
    real_total_friction_lambda: FloatNum,
    total_friction_lambda: FloatNum,
    velocity_bias: FloatNum,
}

impl ContactPointPairConstraintInfo {
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

    pub(crate) fn restrict_contact_friction_lambda(
        &mut self,
        mut friction_lambda: FloatNum,
        max_friction_lambda: FloatNum,
    ) -> FloatNum {
        // if friction_lambda.abs() > (2.0 * self.inv_delta_time.recip()) {
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

        friction_lambda
    }

    pub(crate) unsafe fn prepare_solve_position_constraint(
        &self,
        object_a_meta: &Meta,
        object_b_meta: &Meta,
    ) -> ContactPointPair {
        let delta_angle_a = object_a_meta.get_delta_angle();

        let delta_angle_b = object_b_meta.get_delta_angle();

        let pre_position_a = object_a_meta.pre_position();
        let position_a = object_a_meta.position();

        let pre_position_b = object_b_meta.pre_position();
        let position_b = object_b_meta.position();

        let point_a = self.point_a();
        let point_b = self.point_b();

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

    pub fn delta_velocity_for_a(&self, object_a_meta: &Meta) -> Vector {
        ((self.normal_toward_a() * self.real_total_lambda)
            + (!self.normal_toward_a() * -self.real_total_friction_lambda))
            * object_a_meta.inv_mass()
    }

    pub fn delta_angle_for_a(&self, object_a_meta: &Meta) -> FloatNum {
        ((self.r_a ^ (self.normal_toward_a() * self.real_total_lambda))
            + (self.r_a ^ (!self.normal_toward_a() * -self.real_total_friction_lambda)))
            * object_a_meta.inv_moment_of_inertia()
    }

    pub fn delta_velocity_for_b(&self, object_b_meta: &Meta) -> Vector {
        ((self.normal_toward_a() * -self.real_total_lambda)
            + (!self.normal_toward_a() * self.real_total_friction_lambda))
            * object_b_meta.inv_mass()
    }

    pub fn delta_angle_for_b(&self, object_b_meta: &Meta) -> FloatNum {
        ((self.r_b ^ (self.normal_toward_a() * -self.real_total_lambda))
            + (self.r_b ^ (!self.normal_toward_a() * self.real_total_friction_lambda)))
            * object_b_meta.inv_moment_of_inertia()
    }

    pub fn real_total_lambda(&self) -> FloatNum {
        self.real_total_lambda
    }

    pub fn real_total_friction_lambda(&self) -> FloatNum {
        self.real_total_friction_lambda
    }

    pub fn r_a(&self) -> &Vector {
        &self.r_a
    }

    pub fn r_b(&self) -> &Vector {
        &self.r_b
    }

    pub fn total_lambda(&self) -> FloatNum {
        self.total_lambda
    }

    pub fn total_friction_lambda(&self) -> FloatNum {
        self.total_friction_lambda
    }
}

impl Deref for ContactPointPairConstraintInfo {
    type Target = ContactPointPair;

    fn deref(&self) -> &Self::Target {
        &self.concat_point_pair
    }
}

impl<Obj: ConstraintObject> ContactConstraint<Obj> {
    pub fn new(obj_id_a: ID, obj_id_b: ID, contact_point_pairs: Vec<ContactPointPair>) -> Self {
        Self {
            contact_point_pair_constraint_infos: contact_point_pairs
                .into_iter()
                .map(|v| ContactPointPairConstraintInfo {
                    concat_point_pair: v,
                    ..Default::default()
                })
                .collect(),
            total_friction_lambda: Default::default(),
            total_lambda: Default::default(),
            obj_id_a,
            obj_id_b,
            obj_a: std::ptr::null_mut(),
            obj_b: std::ptr::null_mut(),
            inv_delta_time: 0.,
            max_allow_restrict_impulse: 0.,
            is_active: true,
            factor_friction: 0.,
            factor_restitution: 0.,
            velocity_a: Default::default(),
            velocity_b: Default::default(),
            angle_velocity_a: 0.,
            angle_velocity_b: 0.,
        }
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

    // TODO without compute inv_mass
    pub fn compute_delta_velocity_for_a(&self) -> Vector {
        self.contact_point_pair_constraint_infos.iter().fold(
            Vector::default(),
            |delta_velocity, contact_info| unsafe {
                delta_velocity + contact_info.delta_velocity_for_a(self.object_a().meta())
            },
        )
    }

    pub fn compute_delta_angle_for_a(&self) -> FloatNum {
        self.contact_point_pair_constraint_infos.iter().fold(
            0.,
            |delta_angle, contact_info| unsafe {
                delta_angle + contact_info.delta_angle_for_a(self.object_a().meta())
            },
        )
    }

    pub fn compute_total_friction_lambda_toward_a(&self) -> Vector {
        self.contact_point_pair_constraint_infos.iter().fold(
            Default::default(),
            |total_friction_lambda, contact_info| {
                let tangent_normal = !contact_info.normal_toward_a();
                let current_friction_lambda =
                    tangent_normal * -contact_info.real_total_friction_lambda;
                total_friction_lambda + current_friction_lambda
            },
        )
    }

    pub fn compute_total_lambda_toward_a(&self) -> Vector {
        self.contact_point_pair_constraint_infos.iter().fold(
            Default::default(),
            |total_friction_lambda, contact_info| {
                let lambda = contact_info.normal_toward_a() * contact_info.real_total_lambda;
                total_friction_lambda + lambda
            },
        )
    }

    pub fn delta_velocity_for_a(&self) -> Vector {
        unsafe { self.object_a().meta().velocity() - self.velocity_a }
    }

    pub fn delta_velocity_for_b(&self) -> Vector {
        unsafe { self.object_b().meta().velocity() - self.velocity_b }
    }

    pub fn delta_angle_velocity_for_a(&self) -> FloatNum {
        unsafe { self.object_a().meta().angle_velocity() - self.angle_velocity_a }
    }

    pub fn delta_angle_velocity_for_b(&self) -> FloatNum {
        unsafe { self.object_b().meta().angle_velocity() - self.angle_velocity_b }
    }

    pub(crate) unsafe fn reset_params(
        &mut self,
        (obj_a, obj_b): (*mut Obj, *mut Obj),
        delta_time: FloatNum,
        parameters: &ConstraintParameters,
    ) {
        self.obj_a = obj_a;
        self.obj_b = obj_b;

        self.contact_point_pair_constraint_infos
            .iter_mut()
            .for_each(|constraint| {
                constraint.max_allow_restrict_impulse =
                    parameters.max_allow_restrict_force_for_contact_solve * delta_time;
            });

        self.inv_delta_time = delta_time.recip();

        let object_a = &mut *self.obj_a;
        let object_b = &mut *self.obj_b;
        let contact_point_pairs = &self.contact_point_pair_constraint_infos;

        *object_a.meta_mut().contact_count_mut() += contact_point_pairs.len() as u16;
        *object_b.meta_mut().contact_count_mut() += contact_point_pairs.len() as u16;

        self.velocity_a = object_a.meta().velocity();
        self.velocity_b = object_b.meta().velocity();
        self.angle_velocity_a = object_a.meta().angle_velocity();
        self.angle_velocity_b = object_b.meta().angle_velocity();

        self.factor_friction = (object_a.meta().friction() * object_b.meta().friction()).sqrt();

        self.factor_restitution =
            (object_a.meta().factor_restitution() * object_b.meta().factor_restitution()).sqrt();

        self.contact_point_pair_constraint_infos
            .iter_mut()
            .for_each(|contact_point_pair_constraint_info| {
                let contact_point = &contact_point_pair_constraint_info.point().clone();

                let r_a = (&object_a.center_point(), contact_point).into();

                let r_b = (&object_b.center_point(), contact_point).into();

                contact_point_pair_constraint_info.r_a = r_a;

                contact_point_pair_constraint_info.r_b = r_b;

                let normal = contact_point_pair_constraint_info.normal_toward_a();

                let mass_effective =
                    compute_inv_mass_effective(&normal, (object_a, object_b), r_a, r_b).recip();

                let tangent_normal: Vector = !normal;

                let tangent_mass_effective =
                    compute_inv_mass_effective(&tangent_normal, (object_a, object_b), r_a, r_b)
                        .recip();

                contact_point_pair_constraint_info.mass_effective = mass_effective;
                contact_point_pair_constraint_info.tangent_mass_effective = tangent_mass_effective;

                let vn = normal
                    * (object_a.compute_point_velocity(contact_point)
                        - object_b.compute_point_velocity(contact_point));

                let mut velocity_bias = 0.;

                if vn < -1. {
                    velocity_bias = -vn * self.factor_restitution;
                }

                contact_point_pair_constraint_info.velocity_bias = velocity_bias;
            });
    }

    pub(crate) unsafe fn solve_velocity_constraint(
        &mut self,
        parameters: &ConstraintParameters,
        iter_count: u8,
    ) {
        let obj_a = &mut *self.obj_a;
        let obj_b = &mut *self.obj_b;

        self.contact_point_pair_constraint_infos
            .iter_mut()
            .for_each(|contact_info| {
                let jv_a = contact_info.normal_toward_a();
                let jv_b = -jv_a;

                let v_a = obj_a.compute_point_velocity(contact_info.point());
                let v_b = obj_b.compute_point_velocity(contact_info.point());

                let jv = v_a * jv_a + v_b * jv_b;

                let position_bias = (contact_info.depth() - parameters.max_allow_permeate).max(0.)
                    * self.inv_delta_time;

                let bias = if parameters.split_position_fix {
                    0.
                } else {
                    -position_bias
                };

                let lambda = -(jv * (1. + self.factor_restitution)) * contact_info.mass_effective;

                let lambda = contact_info.restrict_contact_lambda(lambda);

                contact_info.real_total_lambda += lambda;

                obj_a
                    .meta_mut()
                    .apply_impulse(jv_a * lambda, contact_info.r_a);
                obj_b
                    .meta_mut()
                    .apply_impulse(jv_b * lambda, contact_info.r_b);
            });

        if iter_count >= 5 && !parameters.skip_friction_constraints {
            self.solve_friction_constraint();
        }
    }

    // TODO add static friction , make object static
    pub(crate) unsafe fn solve_friction_constraint(&mut self) {
        let obj_a = &mut *self.obj_a;
        let obj_b = &mut *self.obj_b;

        self.contact_point_pair_constraint_infos
            .iter_mut()
            .for_each(|contact_info| {
                let mass_effective = contact_info.tangent_mass_effective;

                let contact_point = contact_info.point();

                let sum_velocity_a = obj_a.compute_point_velocity(contact_point);

                let sum_velocity_b = obj_b.compute_point_velocity(contact_point);

                let tangent_normal = !contact_info.normal_toward_a();

                let friction_lambda = (sum_velocity_a - sum_velocity_b)
                    * tangent_normal
                    * mass_effective
                    * self.factor_friction;

                let friction_lambda = contact_info.restrict_contact_friction_lambda(
                    friction_lambda,
                    contact_info.real_total_lambda,
                );

                contact_info.real_total_friction_lambda += friction_lambda;

                let friction_impulse = tangent_normal * friction_lambda;

                obj_a
                    .meta_mut()
                    .apply_impulse(-friction_impulse, contact_info.r_a);

                obj_b
                    .meta_mut()
                    .apply_impulse(friction_impulse, contact_info.r_b);
            });
    }

    pub(crate) unsafe fn solve_position_constraint(
        &mut self,
        parameters: &ConstraintParameters,
        index: usize,
    ) {
        self.contact_point_pair_constraint_infos
            .iter_mut()
            .for_each(|contact_info| {
                let obj_a = &mut *self.obj_a;
                let obj_b = &mut *self.obj_b;
                let obj_a_meta = obj_a.meta();
                let obj_b_meta = obj_b.meta();

                let delta_angle_a = obj_a_meta.get_delta_angle();
                let delta_position_a = obj_a_meta.get_delta_position();
                let delta_angle_b = obj_b_meta.get_delta_angle();
                let delta_position_b = obj_b_meta.get_delta_position();

                let normal_toward_a = contact_info.normal_toward_a();

                let contact_point_pair =
                    contact_info.prepare_solve_position_constraint(obj_a_meta, obj_b_meta);

                // let n = contact_point_pair.normal_toward_a();
                let n = normal_toward_a;

                let r_a = contact_info.r_a.affine_transformation_rotate(delta_angle_a);
                let r_b = contact_info.r_b.affine_transformation_rotate(delta_angle_b);

                let inv_mass_effective = obj_a_meta.inv_mass()
                    + obj_a_meta.inv_mass()
                    + obj_a_meta.inv_moment_of_inertia() * (r_a ^ n).powf(2.)
                    + obj_b_meta.inv_moment_of_inertia() * (r_b ^ n).powf(2.);

                let contact_count_a = obj_a_meta.contact_count();
                let contact_count_b = obj_b_meta.contact_count();

                let permeate: FloatNum =
                    n * (*contact_point_pair.point_b() - *contact_point_pair.point_a());

                let mut depth_fix = permeate;

                // FIXME impossible
                // debug_assert!(depth_fix.is_sign_positive());

                // if obj_a_meta.is_fixed() || obj_b_meta.is_fixed() {
                //     depth_fix *= 2.;
                // }

                const POSITION_DAMPEN: FloatNum = 0.08;

                depth_fix *= POSITION_DAMPEN;

                let c = n * depth_fix;

                let f = c * inv_mass_effective.recip();

                obj_a.apply_position_fix(f * (contact_count_a as FloatNum).recip(), r_a);

                obj_b.apply_position_fix(-f * (contact_count_b as FloatNum).recip(), r_b);
            })
    }

    pub(crate) fn contact_pair_constraint_infos_iter(
        &self,
    ) -> impl Iterator<Item = &ContactPointPairConstraintInfo> {
        self.contact_point_pair_constraint_infos.iter()
    }
}
