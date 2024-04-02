use picea_macro_tools::{Deref, Fields};

use crate::{
    collision::ContactPointPair,
    element::ID,
    math::{num::limit_at_range, vector::Vector, FloatNum},
    meta::Meta,
    prelude::Point,
    scene::context::ConstraintParameters,
};

use super::{compute_inv_mass_effective, ConstraintObject};

// TODO if two element is still collide in current frame, we can reuse this
// contact info , is two element is not collide anymore , we don't need this frame
#[derive(Fields)]
pub struct ContactConstraint<Obj: ConstraintObject> {
    contact_point_pair_constraint_infos: Vec<ContactPointPairConstraintInfo>,
    // two collide obj
    obj_id_a: ID,
    obj_id_b: ID,
    obj_a: *mut Obj,
    obj_b: *mut Obj,
    // max_allow_restrict_impulse: FloatNum,
    inv_delta_time: FloatNum,
    #[r]
    #[w(set)]
    is_active: bool,
    factor_friction: FloatNum,
    factor_restitution: FloatNum,
    velocity_a: Vector,
    velocity_b: Vector,
    angle_velocity_a: FloatNum,
    angle_velocity_b: FloatNum,
}

#[derive(Default, Deref, Fields)]
pub struct ContactPointPairConstraintInfo {
    #[deref]
    #[r]
    concat_point_pair: ContactPointPair,
    #[r]
    r_a: Vector,
    #[r]
    r_b: Vector,
    mass_effective: FloatNum,
    tangent_mass_effective: FloatNum,
    max_allow_restrict_impulse: FloatNum,
    #[r]
    real_total_lambda: FloatNum,
    #[r]
    total_lambda: FloatNum,
    #[r]
    real_total_friction_lambda: FloatNum,
    #[r]
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

    pub(crate) unsafe fn prepare_solve_position_constraint<Obj: ConstraintObject>(
        &self,
        object_a: &Obj,
        object_b: &Obj,
    ) -> (Point, Point, Vector, Vector) {
        let object_a_meta = object_a.meta();
        let object_b_meta = object_b.meta();

        let (delta_position_a, delta_angle_a) = object_a_meta.delta_transform().split();
        let (delta_position_b, delta_angle_b) = object_b_meta.delta_transform().split();

        // let pre_position_a = object_a_meta.pre_position();
        let position_a = object_a.center_point() + delta_position_a;

        // let pre_position_b = object_b_meta.pre_position();
        let position_b = object_b.center_point() + delta_position_b;

        // let point_a = self.point_a();
        // let point_b = self.point_b();

        // let mut r_a: Vector = (pre_position_a, point_a).into();
        let r_a = self.r_a.affine_transformation_rotate(delta_angle_a);
        let point_a = position_a + r_a;

        // let mut r_b: Vector = (pre_position_b, point_b).into();
        let r_b = self.r_b.affine_transformation_rotate(delta_angle_b);
        let point_b = position_b + r_b;

        // let normal: Vector = (point_a, point_b).into();

        // let normal_toward_a = if normal * (position_a - position_b) < 0. {
        //     -normal
        // } else {
        //     normal
        // };

        (point_a, point_b, r_a, r_b)
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
}

impl<Obj: ConstraintObject> ContactConstraint<Obj> {
    pub fn new(obj_id_a: ID, obj_id_b: ID, contact_point_pairs: Vec<ContactPointPair>) -> Self {
        let contact_point_pair_constraint_infos = contact_point_pairs
            .into_iter()
            .map(|v| ContactPointPairConstraintInfo {
                concat_point_pair: v,
                ..Default::default()
            })
            .collect();

        Self {
            contact_point_pair_constraint_infos,
            obj_id_a,
            obj_id_b,
            obj_a: std::ptr::null_mut(),
            obj_b: std::ptr::null_mut(),
            inv_delta_time: 0.,
            is_active: true,
            factor_friction: 0.,
            factor_restitution: 0.,
            velocity_a: Default::default(),
            velocity_b: Default::default(),
            angle_velocity_a: 0.,
            angle_velocity_b: 0.,
        }
    }

    pub fn replace_contact_point_pairs(&mut self, contact_point_pairs: Vec<ContactPointPair>) {
        self.contact_point_pair_constraint_infos = contact_point_pairs
            .into_iter()
            .map(|v| ContactPointPairConstraintInfo {
                concat_point_pair: v,
                ..Default::default()
            })
            .collect()
    }

    pub fn extend_contact_point_pairs(&mut self, contact_point_pairs: Vec<ContactPointPair>) {
        self.contact_point_pair_constraint_infos
            .extend(
                contact_point_pairs
                    .into_iter()
                    .map(|v| ContactPointPairConstraintInfo {
                        concat_point_pair: v,
                        ..Default::default()
                    }),
            )
    }

    pub fn contact_point_pair_len(&self) -> usize {
        self.contact_point_pair_constraint_infos.len()
    }

    pub fn filter_contact_point_pairs(&mut self, predicate: impl Fn(&ContactPointPair) -> bool) {
        self.contact_point_pair_constraint_infos =
            std::mem::take(&mut self.contact_point_pair_constraint_infos)
                .into_iter()
                .filter(|v| predicate(v))
                .collect();
    }

    pub fn obj_id_pair(&self) -> (ID, ID) {
        (self.obj_id_a, self.obj_id_b)
    }

    pub fn object_a(&self) -> &Obj {
        unsafe { &*self.obj_a }
    }

    pub fn object_b(&self) -> &Obj {
        unsafe { &*self.obj_b }
    }

    // TODO without compute inv_mass
    pub fn compute_delta_velocity_for_a(&self) -> Vector {
        self.contact_point_pair_constraint_infos.iter().fold(
            Vector::default(),
            |delta_velocity, contact_info| {
                delta_velocity + contact_info.delta_velocity_for_a(self.object_a().meta())
            },
        )
    }

    pub fn compute_delta_angle_for_a(&self) -> FloatNum {
        self.contact_point_pair_constraint_infos
            .iter()
            .fold(0., |delta_angle, contact_info| {
                delta_angle + contact_info.delta_angle_for_a(self.object_a().meta())
            })
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
        {
            *self.object_a().meta().velocity() - self.velocity_a
        }
    }

    pub fn delta_velocity_for_b(&self) -> Vector {
        *self.object_b().meta().velocity() - self.velocity_b
    }

    pub fn delta_angle_velocity_for_a(&self) -> FloatNum {
        self.object_a().meta().angle_velocity() - self.angle_velocity_a
    }

    pub fn delta_angle_velocity_for_b(&self) -> FloatNum {
        self.object_b().meta().angle_velocity() - self.angle_velocity_b
    }

    pub(crate) unsafe fn pre_solve(
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

        self.velocity_a = *object_a.meta().velocity();
        self.velocity_b = *object_b.meta().velocity();
        self.angle_velocity_a = object_a.meta().angle_velocity();
        self.angle_velocity_b = object_b.meta().angle_velocity();

        self.factor_friction =
            (object_a.meta().factor_friction() * object_b.meta().factor_friction()).sqrt();

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

                // let position_bias = (contact_info.depth() - parameters.max_allow_permeate).max(0.)
                //     * self.inv_delta_time;

                // let bias = if parameters.split_position_fix {
                //     0.
                // } else {
                //     -position_bias
                // };

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

    pub(crate) unsafe fn solve_position_constraint(&mut self) {
        self.contact_point_pair_constraint_infos
            .iter()
            .for_each(|contact_info| {
                let obj_a = &mut *self.obj_a;
                let obj_b = &mut *self.obj_b;

                let (point_a, point_b, r_a, r_b) =
                    contact_info.prepare_solve_position_constraint(obj_a, obj_b);

                let obj_a_meta = obj_a.meta();
                let obj_b_meta = obj_b.meta();

                // let delta_angle_a = obj_a_meta.delta_angle();
                // let delta_position_a = obj_a_meta.delta_position();
                // let delta_angle_b = obj_b_meta.delta_angle();
                // let delta_position_b = obj_b_meta.delta_position();

                // REVIEW
                let n = contact_info.normal_toward_a();

                // let n = contact_point_pair.normal_toward_a();
                // let n = normal_toward_a;

                let inv_mass_effective = obj_a_meta.inv_mass()
                    + obj_a_meta.inv_mass()
                    + obj_a_meta.inv_moment_of_inertia() * (r_a ^ n).powf(2.)
                    + obj_b_meta.inv_moment_of_inertia() * (r_b ^ n).powf(2.);

                let contact_count_a = obj_a_meta.contact_count();
                let contact_count_b = obj_b_meta.contact_count();

                let permeate: FloatNum = n * (point_b - point_a);

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
