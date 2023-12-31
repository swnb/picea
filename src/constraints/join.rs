use crate::{
    element::ID,
    math::{point::Point, vector::Vector, FloatNum},
    scene::context::ConstraintParameters,
};

use super::{compute_soft_constraints_params, ConstraintObject, JoinConstraintConfig};

pub struct JoinConstraint<Obj: ConstraintObject> {
    id: u32,
    obj_a_id: ID,
    obj_b_id: ID,
    obj_a: *mut Obj,
    obj_b: *mut Obj,
    move_point_with_a: Point,
    move_point_with_b: Point,
    total_lambda: FloatNum,
    // force_soft_factor: FloatNum,
    // position_fix_factor: FloatNum,
    // distance must large than or equal to zero
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

    pub fn id(&self) -> u32 {
        self.id
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
        let inv_mass_a = meta_a.inv_mass();
        let inv_mass_b = meta_b.inv_mass();
        let inv_i_a = meta_a.inv_moment_of_inertia();
        let inv_i_b = meta_b.inv_moment_of_inertia();
        let mass = meta_a.mass() + meta_b.mass();

        let (force_soft_factor, position_fix_factor) = compute_soft_constraints_params(
            mass,
            self.config.damping_ratio,
            self.config.frequency,
            delta_time,
        );

        let r_a: Vector = (obj_a.center_point(), move_point_with_a).into();

        let r_b: Vector = (obj_b.center_point(), move_point_with_b).into();

        let distance = self.stretch_length();

        let n = -distance.normalize();

        let inv_mass_effective =
            inv_mass_a + inv_mass_b + inv_i_a * (r_a ^ n).powf(2.) + inv_i_b * (r_b ^ n).powf(2.);

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

        if distance.abs() < parameters.max_allow_permeate {
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

        let lambda = jv_b * (inv_mass_effective + soft_part).recip();

        obj_a.meta_mut().apply_impulse(n * lambda, r_a);
        obj_b.meta_mut().apply_impulse(-n * lambda, r_b);
    }
}
