use crate::{
    algo::constraint::{compute_soft_constraints_params, ConstraintObject},
    element::{Element, ID},
    math::{point::Point, vector::Vector, FloatNum},
    scene::context::ConstraintParameters,
};

pub struct PointConstraint<Obj: ConstraintObject = Element> {
    id: u32,
    element_id: ID,
    fixed_point: Point,
    move_point: Point, // bind with element
    total_lambda: FloatNum,
    // force_soft_factor: FloatNum,
    // position_fix_factor: FloatNum,
    position_bias: FloatNum,
    soft_part: FloatNum,
    mass_effective: FloatNum,
    obj: *mut Obj,
}

impl<Obj: ConstraintObject> PointConstraint<Obj> {
    pub fn new(id: u32, element_id: ID, fixed_point: Point) -> Self {
        Self {
            id,
            element_id,
            fixed_point,
            move_point: Default::default(),
            total_lambda: 0.,
            position_bias: 0.,
            soft_part: 0.,
            mass_effective: 0.,
            obj: std::ptr::null_mut(),
        }
    }
    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn element_id(&self) -> ID {
        self.element_id
    }

    pub fn stretch_length(&self) -> Vector {
        (self.move_point, self.fixed_point).into()
    }

    pub unsafe fn reset_params(
        &mut self,
        move_point: Point,
        damping_ratio: FloatNum,
        frequency: FloatNum,
        obj: *mut Obj,
        delta_time: FloatNum,
    ) {
        self.move_point = move_point;
        self.total_lambda = 0.;

        let meta = (*obj).meta();
        let mass = meta.mass();
        let inv_mass = meta.inv_mass();
        let inv_moment_of_inertia = meta.inv_moment_of_inertia();

        let (force_soft_factor, position_fix_factor) =
            compute_soft_constraints_params(mass, damping_ratio, frequency, delta_time);

        let strength_length = self.stretch_length();
        let n = -strength_length.normalize();

        let position_bias = position_fix_factor * strength_length.abs() * delta_time.recip();

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

    pub unsafe fn solve(&mut self, parameters: &ConstraintParameters) {
        let strength_length = self.stretch_length();
        if strength_length.abs() < parameters.max_allow_permeate {
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

        let r_t: Vector = (obj.center_point(), self.move_point).into();

        let n = -strength_length.normalize();

        let v: Vector = obj.compute_point_velocity(&self.move_point);

        let jv_b: f32 = -(v * n + position_bias);

        let lambda = jv_b * (soft_part + mass_effective).recip();

        let impulse = n * lambda;

        // TODO restrict here

        obj.meta_mut().apply_impulse(impulse, r_t);
    }
}
