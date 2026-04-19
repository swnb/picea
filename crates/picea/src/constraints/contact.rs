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

fn can_solve_with_effective_mass(mass_effective: FloatNum) -> bool {
    mass_effective.is_finite() && mass_effective > 0.
}

fn can_solve_with_inv_mass_effective(inv_mass_effective: FloatNum) -> bool {
    inv_mass_effective.is_finite() && inv_mass_effective > 0.
}

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
    was_active_last_pass: bool,
    pending_contact_point_pairs: Option<Vec<ContactPointPair>>,
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
    contact_point_pair: ContactPointPair,
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
    pub(crate) fn warm_start_impulse_toward_a<Obj: ConstraintObject>(
        &self,
        object_a: &Obj,
        object_b: &Obj,
    ) -> Option<Vector> {
        if !self.total_lambda.is_finite() || !self.total_friction_lambda.is_finite() {
            return None;
        }

        let has_normal_lambda = self.total_lambda != 0.;
        let has_friction_lambda = self.total_friction_lambda != 0.;
        if !has_normal_lambda && !has_friction_lambda {
            return None;
        }

        let normal = self.normal_toward_a();
        if has_normal_lambda {
            let inv_mass_effective =
                compute_inv_mass_effective(&normal, (object_a, object_b), self.r_a, self.r_b);
            if !can_solve_with_inv_mass_effective(inv_mass_effective) {
                return None;
            }
        }

        if has_friction_lambda {
            let tangent = !normal;
            let inv_mass_effective =
                compute_inv_mass_effective(&tangent, (object_a, object_b), self.r_a, self.r_b);
            if !can_solve_with_inv_mass_effective(inv_mass_effective) {
                return None;
            }
        }

        let impulse = (normal * self.total_lambda) + (-!normal * self.total_friction_lambda);
        if impulse.x().is_finite() && impulse.y().is_finite() && !impulse.is_zero() {
            Some(impulse)
        } else {
            None
        }
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

    pub(crate) fn prepare_solve_position_constraint<Obj: ConstraintObject>(
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
            * object_a_meta.effective_inv_mass()
    }

    pub fn delta_angle_for_a(&self, object_a_meta: &Meta) -> FloatNum {
        ((self.r_a ^ (self.normal_toward_a() * self.real_total_lambda))
            + (self.r_a ^ (!self.normal_toward_a() * -self.real_total_friction_lambda)))
            * object_a_meta.effective_inv_moment_of_inertia()
    }

    pub fn delta_velocity_for_b(&self, object_b_meta: &Meta) -> Vector {
        ((self.normal_toward_a() * -self.real_total_lambda)
            + (!self.normal_toward_a() * self.real_total_friction_lambda))
            * object_b_meta.effective_inv_mass()
    }

    pub fn delta_angle_for_b(&self, object_b_meta: &Meta) -> FloatNum {
        ((self.r_b ^ (self.normal_toward_a() * -self.real_total_lambda))
            + (self.r_b ^ (!self.normal_toward_a() * self.real_total_friction_lambda)))
            * object_b_meta.effective_inv_moment_of_inertia()
    }
}

impl<Obj: ConstraintObject> ContactConstraint<Obj> {
    pub fn new(obj_id_a: ID, obj_id_b: ID, contact_point_pairs: Vec<ContactPointPair>) -> Self {
        let contact_point_pair_constraint_infos = contact_point_pairs
            .into_iter()
            .map(|v| ContactPointPairConstraintInfo {
                contact_point_pair: v,
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
            was_active_last_pass: false,
            pending_contact_point_pairs: None,
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
                contact_point_pair: v,
                ..Default::default()
            })
            .collect()
    }

    pub(crate) fn begin_collision_pass(&mut self) {
        self.pending_contact_point_pairs = None;
        self.was_active_last_pass = self.is_active;
        self.is_active = false;
    }

    pub(crate) fn queue_contact_point_pairs_for_warm_started_refresh(
        &mut self,
        contact_point_pairs: Vec<ContactPointPair>,
    ) {
        self.is_active = true;
        if self.was_active_last_pass {
            self.pending_contact_point_pairs = Some(contact_point_pairs);
        } else {
            self.replace_contact_point_pairs(contact_point_pairs);
        }
    }

    pub(crate) fn can_warm_start_current_pass(&self) -> bool {
        self.is_active && self.was_active_last_pass
    }

    pub(crate) fn extend_current_contact_point_pairs(
        &mut self,
        mut contact_point_pairs: Vec<ContactPointPair>,
    ) {
        if let Some(pending_contact_point_pairs) = &mut self.pending_contact_point_pairs {
            pending_contact_point_pairs.append(&mut contact_point_pairs);
        } else {
            self.extend_contact_point_pairs(contact_point_pairs);
        }
    }

    pub(crate) fn refresh_contact_point_pairs_after_warm_start(&mut self) {
        if let Some(contact_point_pairs) = self.pending_contact_point_pairs.take() {
            self.replace_contact_point_pairs(contact_point_pairs);
        }
    }

    pub fn extend_contact_point_pairs(&mut self, contact_point_pairs: Vec<ContactPointPair>) {
        self.contact_point_pair_constraint_infos
            .extend(
                contact_point_pairs
                    .into_iter()
                    .map(|v| ContactPointPairConstraintInfo {
                        contact_point_pair: v,
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
                    parameters.max_allow_restrict_force_for_contact_solve() * delta_time;
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

                let r_a = (
                    &object_a.center_point(),
                    contact_point_pair_constraint_info.point_a(),
                )
                    .into();

                let r_b = (
                    &object_b.center_point(),
                    contact_point_pair_constraint_info.point_b(),
                )
                    .into();

                contact_point_pair_constraint_info.r_a = r_a;

                contact_point_pair_constraint_info.r_b = r_b;

                let normal = contact_point_pair_constraint_info.normal_toward_a();

                let inv_mass_effective =
                    compute_inv_mass_effective(&normal, (object_a, object_b), r_a, r_b);
                let mass_effective = if can_solve_with_inv_mass_effective(inv_mass_effective) {
                    inv_mass_effective.recip()
                } else {
                    0.
                };

                let tangent_normal: Vector = !normal;

                let tangent_inv_mass_effective =
                    compute_inv_mass_effective(&tangent_normal, (object_a, object_b), r_a, r_b);
                let tangent_mass_effective =
                    if can_solve_with_inv_mass_effective(tangent_inv_mass_effective) {
                        tangent_inv_mass_effective.recip()
                    } else {
                        0.
                    };

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

                if !can_solve_with_effective_mass(contact_info.mass_effective) {
                    return;
                }

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

                if !lambda.is_finite() {
                    return;
                }

                let lambda = contact_info.restrict_contact_lambda(lambda);

                contact_info.real_total_lambda += lambda;

                obj_a
                    .meta_mut()
                    .apply_impulse(jv_a * lambda, contact_info.r_a);
                obj_b
                    .meta_mut()
                    .apply_impulse(jv_b * lambda, contact_info.r_b);
            });

        if iter_count >= 5 && !parameters.skip_friction_constraints() {
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
                if !can_solve_with_effective_mass(mass_effective) {
                    return;
                }

                let contact_point = contact_info.point();

                let sum_velocity_a = obj_a.compute_point_velocity(contact_point);

                let sum_velocity_b = obj_b.compute_point_velocity(contact_point);

                let tangent_normal = !contact_info.normal_toward_a();

                let friction_lambda = (sum_velocity_a - sum_velocity_b)
                    * tangent_normal
                    * mass_effective
                    * self.factor_friction;

                if !friction_lambda.is_finite() {
                    return;
                }

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

    pub(crate) fn solve_position_constraint(&mut self) {
        self.contact_point_pair_constraint_infos
            .iter()
            .for_each(|contact_info| {
                let obj_a = unsafe { &mut *self.obj_a };
                let obj_b = unsafe { &mut *self.obj_b };

                let (point_a, point_b, r_a, r_b) =
                    contact_info.prepare_solve_position_constraint(obj_a, obj_b);

                // let delta_angle_a = obj_a_meta.delta_angle();
                // let delta_position_a = obj_a_meta.delta_position();
                // let delta_angle_b = obj_b_meta.delta_angle();
                // let delta_position_b = obj_b_meta.delta_position();

                // REVIEW
                let n = contact_info.normal_toward_a();

                // let n = contact_point_pair.normal_toward_a();
                // let n = normal_toward_a;

                let inv_mass_effective = compute_inv_mass_effective(&n, (obj_a, obj_b), r_a, r_b);
                if !can_solve_with_inv_mass_effective(inv_mass_effective) {
                    return;
                }

                let obj_a_meta = obj_a.meta();
                let obj_b_meta = obj_b.meta();

                let contact_count_a = obj_a_meta.contact_count();
                let contact_count_b = obj_b_meta.contact_count();

                let permeate: FloatNum = n * (point_b - point_a);

                let mut depth_fix = permeate;

                // FIXME impossible
                // debug_assert!(depth_fix.is_sign_positive());

                // if obj_a_meta.is_fixed() || obj_b_meta.is_fixed() {
                //     depth_fix *= 2.;
                // }

                const POSITION_DAMPEN: FloatNum = 0.2;

                depth_fix *= POSITION_DAMPEN;

                let c = n * depth_fix;

                let f = c * inv_mass_effective.recip();

                obj_a.apply_position_fix(f * (contact_count_a as FloatNum).recip(), r_a);

                obj_b.apply_position_fix(-f * (contact_count_b as FloatNum).recip(), r_b);
            })
    }

    pub(crate) fn get_position_constraint_result(&self) -> Vec<FloatNum> {
        unsafe {
            self.contact_point_pair_constraint_infos
                .iter()
                .map(|contact_info| {
                    let obj_a = &mut *self.obj_a;
                    let obj_b = &mut *self.obj_b;

                    let (point_a, point_b, _r_a, _r_b) =
                        contact_info.prepare_solve_position_constraint(obj_a, obj_b);
                    // REVIEW
                    let n = contact_info.normal_toward_a();
                    let permeate: FloatNum = n * (point_b - point_a);

                    permeate
                })
                .collect()
        }
    }

    pub(crate) fn contact_pair_constraint_infos_iter(
        &self,
    ) -> impl Iterator<Item = &ContactPointPairConstraintInfo> {
        self.contact_point_pair_constraint_infos.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        element::{Element, ElementBuilder},
        math::FloatNum,
        meta::MetaBuilder,
        scene::context::ConstraintParameters,
        shape::Circle,
    };

    const STEP_DT: FloatNum = 1. / 60.;
    const EPSILON: FloatNum = 0.00001;
    const CONTACT_DEPTH: FloatNum = 0.5;
    const POSITION_DAMPEN: FloatNum = 0.2;

    fn circle_element(center_x: FloatNum, mass: FloatNum, is_fixed: bool) -> Element<()> {
        ElementBuilder::new(
            Circle::new((center_x, 0.), 1.),
            MetaBuilder::new().mass(mass).is_fixed(is_fixed),
            (),
        )
        .into()
    }

    fn central_overlap_contact() -> ContactPointPair {
        ContactPointPair::new(
            (1., 0.).into(),
            (0.5, 0.).into(),
            (-1., 0.).into(),
            CONTACT_DEPTH,
        )
    }

    fn solve_position_once(
        mass_a: FloatNum,
        mass_b: FloatNum,
        is_a_fixed: bool,
        is_b_fixed: bool,
    ) -> (FloatNum, FloatNum) {
        let mut object_a = circle_element(0., mass_a, is_a_fixed);
        let mut object_b = circle_element(1.5, mass_b, is_b_fixed);
        let mut constraint = ContactConstraint::new(1, 2, vec![central_overlap_contact()]);

        unsafe {
            constraint.pre_solve(
                (&mut object_a as *mut _, &mut object_b as *mut _),
                STEP_DT,
                &ConstraintParameters::default(),
            );
        }
        constraint.solve_position_constraint();

        (
            object_a.meta().delta_transform().translation().x(),
            object_b.meta().delta_transform().translation().x(),
        )
    }

    fn solve_velocity_once(
        mass_a: FloatNum,
        velocity_a: Vector,
        mass_b: FloatNum,
        velocity_b: Vector,
    ) -> (Vector, FloatNum, Vector, FloatNum) {
        let mut object_a = circle_element(0., mass_a, false);
        let mut object_b = circle_element(1.5, mass_b, false);
        *object_a.meta_mut().velocity_mut() = velocity_a;
        *object_b.meta_mut().velocity_mut() = velocity_b;

        let mut constraint = ContactConstraint::new(1, 2, vec![central_overlap_contact()]);
        unsafe {
            constraint.pre_solve(
                (&mut object_a as *mut _, &mut object_b as *mut _),
                STEP_DT,
                &ConstraintParameters::default(),
            );
            constraint.solve_velocity_constraint(&ConstraintParameters::default(), 0);
        }

        (
            *object_a.meta().velocity(),
            object_a.meta().angle_velocity(),
            *object_b.meta().velocity(),
            object_b.meta().angle_velocity(),
        )
    }

    fn solve_friction_once(
        mass_a: FloatNum,
        velocity_a: Vector,
        mass_b: FloatNum,
        velocity_b: Vector,
    ) -> (Vector, FloatNum, Vector, FloatNum) {
        let mut object_a = circle_element(0., mass_a, false);
        let mut object_b = circle_element(1.5, mass_b, false);
        *object_a.meta_mut().velocity_mut() = velocity_a;
        *object_b.meta_mut().velocity_mut() = velocity_b;

        let mut constraint = ContactConstraint::new(1, 2, vec![central_overlap_contact()]);
        unsafe {
            constraint.pre_solve(
                (&mut object_a as *mut _, &mut object_b as *mut _),
                STEP_DT,
                &ConstraintParameters::default(),
            );
            constraint.solve_friction_constraint();
        }

        (
            *object_a.meta().velocity(),
            object_a.meta().angle_velocity(),
            *object_b.meta().velocity(),
            object_b.meta().angle_velocity(),
        )
    }

    fn assert_float_close(actual: FloatNum, expected: FloatNum) {
        assert!(
            (actual - expected).abs() <= EPSILON,
            "expected {actual} to be within {EPSILON} of {expected}"
        );
    }

    fn assert_vector_finite(vector: Vector) {
        assert!(vector.x().is_finite(), "expected finite x, got {vector}");
        assert!(vector.y().is_finite(), "expected finite y, got {vector}");
    }

    fn assert_velocity_state_finite(state: (Vector, FloatNum, Vector, FloatNum)) {
        let (velocity_a, angle_velocity_a, velocity_b, angle_velocity_b) = state;
        assert_vector_finite(velocity_a);
        assert!(angle_velocity_a.is_finite());
        assert_vector_finite(velocity_b);
        assert!(angle_velocity_b.is_finite());
    }

    fn assert_velocity_state_unchanged(
        state: (Vector, FloatNum, Vector, FloatNum),
        expected_velocity_a: Vector,
        expected_velocity_b: Vector,
    ) {
        assert_velocity_state_finite(state);
        assert_eq!(state.0, expected_velocity_a);
        assert_float_close(state.1, 0.);
        assert_eq!(state.2, expected_velocity_b);
        assert_float_close(state.3, 0.);
    }

    #[test]
    fn position_solver_uses_b_inverse_mass_in_effective_mass() {
        let (delta_a_x, delta_b_x) = solve_position_once(1., 4., false, false);

        let expected_total_correction = CONTACT_DEPTH * POSITION_DAMPEN;
        let inv_mass_a = 1.;
        let inv_mass_b = 0.25;
        let inv_mass_sum = inv_mass_a + inv_mass_b;

        assert_float_close(
            delta_a_x,
            -expected_total_correction * inv_mass_a / inv_mass_sum,
        );
        assert_float_close(
            delta_b_x,
            expected_total_correction * inv_mass_b / inv_mass_sum,
        );
        assert_float_close(delta_b_x - delta_a_x, expected_total_correction);
    }

    #[test]
    fn fixed_body_does_not_contribute_to_position_effective_mass() {
        let (delta_a_x, delta_b_x) = solve_position_once(1., 4., true, false);

        assert_float_close(delta_a_x, 0.);
        assert_float_close(delta_b_x, CONTACT_DEPTH * POSITION_DAMPEN);
    }

    #[test]
    fn normal_velocity_solver_noops_for_zero_effective_mass_without_nan() {
        let velocity_a = (2., 0.).into();
        let velocity_b = (-2., 0.).into();

        assert_velocity_state_unchanged(
            solve_velocity_once(0., velocity_a, 1., velocity_b),
            velocity_a,
            velocity_b,
        );
    }

    #[test]
    fn normal_velocity_solver_noops_for_non_finite_effective_mass_without_nan() {
        let velocity_a = (2., 0.).into();
        let velocity_b = (-2., 0.).into();

        assert_velocity_state_unchanged(
            solve_velocity_once(FloatNum::NAN, velocity_a, 1., velocity_b),
            velocity_a,
            velocity_b,
        );
    }

    #[test]
    fn normal_velocity_solver_handles_very_small_finite_effective_mass() {
        let state = solve_velocity_once(
            0.0000000001,
            (2., 0.).into(),
            0.0000000001,
            (-2., 0.).into(),
        );

        assert_velocity_state_finite(state);
        assert!(
            state.0.x() < 0.,
            "finite tiny effective mass should still solve normal impulse for A"
        );
        assert!(
            state.2.x() > 0.,
            "finite tiny effective mass should still solve normal impulse for B"
        );
    }

    #[test]
    fn friction_solver_noops_for_zero_effective_mass_without_nan() {
        let velocity_a = (0., 2.).into();
        let velocity_b = (0., -2.).into();

        assert_velocity_state_unchanged(
            solve_friction_once(0., velocity_a, 1., velocity_b),
            velocity_a,
            velocity_b,
        );
    }

    #[test]
    fn friction_solver_noops_for_non_finite_effective_mass_without_nan() {
        let velocity_a = (0., 2.).into();
        let velocity_b = (0., -2.).into();

        assert_velocity_state_unchanged(
            solve_friction_once(FloatNum::NAN, velocity_a, 1., velocity_b),
            velocity_a,
            velocity_b,
        );
    }
}
