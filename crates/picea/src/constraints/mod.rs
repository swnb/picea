use picea_macro_tools::{Builder, Fields};

use crate::{
    element::ID,
    math::{point::Point, vector::Vector, FloatNum, TAU},
    meta::Meta,
};

pub mod contact;
pub mod contact_manifold;
pub mod join;
pub mod point;

#[derive(Clone, Builder, Fields)]
#[r]
#[w]
pub struct JoinConstraintConfig {
    #[default = 0.]
    distance: FloatNum,
    #[default = 1.]
    damping_ratio: FloatNum,
    #[default = 0.5]
    frequency: FloatNum,
    #[default = false]
    hard: bool,
}

pub fn compute_inv_mass_effective<Obj: ConstraintObject>(
    &normal: &Vector,
    object_pair: (&Obj, &Obj),
    r_a: Vector,
    r_b: Vector,
) -> FloatNum {
    let (obj_a, obj_b) = object_pair;
    let meta_a = obj_a.meta();
    let meta_b = obj_b.meta();

    let inv_moment_of_inertia_a = meta_a.inv_moment_of_inertia();
    let inv_moment_of_inertia_b = meta_b.inv_moment_of_inertia();

    let inv_mass_a = meta_a.inv_mass();
    let inv_mass_b = meta_b.inv_mass();

    let inv_mass_effective = inv_mass_a
        + inv_mass_b
        + (r_a ^ normal).powf(2.) * inv_moment_of_inertia_a
        + (r_b ^ normal).powf(2.) * inv_moment_of_inertia_b;

    inv_mass_effective
}

pub fn compute_soft_constraints_params(
    mass: FloatNum,
    damping_ratio: FloatNum,
    frequency: FloatNum,
    delta_time: FloatNum,
) -> (FloatNum, FloatNum) {
    let spring_constant = mass * (TAU() * frequency).powf(2.);
    let damping_coefficient = 2. * mass * damping_ratio * frequency;

    let tmp1 = delta_time * spring_constant; // h * k
    let tmp2 = damping_coefficient + tmp1;

    let force_soft_factor = tmp2.recip();
    let position_fix_factor = tmp1 * tmp2.recip();

    (force_soft_factor, position_fix_factor)
}

pub trait ConstraintObject {
    // this method doesn't need to impl, it is for debug only
    fn id(&self) -> ID {
        0
    }

    fn center_point(&self) -> Point;

    fn meta(&self) -> &Meta;

    fn meta_mut(&mut self) -> &mut Meta;

    fn compute_point_velocity(&self, contact_point: &Point) -> Vector;

    fn apply_position_fix(&mut self, fix: Vector, r: Vector);
}

/**
 * Sequential Impulse
 * two object collide，object A and object B
 * n is the collision direction normal vector that could separate two object
 * depth define how depth two object collide,
 * the constraint define blow:
 * ( velocity_A + (angle_velocity_A X radius_A) - velocity_B - (angle_velocity_B X radius_B) ) * n == 0
 * which mean ，the **relatively** velocity in the collide direction must be zero，that is what constraint means.
 * and previous velocity plus delta_velocity equal current velocity, what change object's velocity is F * t , also know as **Impulse** (I)
 * velocity = previous_velocity + delta_velocity
 * delta_velocity = I / mass
 * **Impulse** is same for both object
 * I = n * L
 * some the purpose of constraint is find the L, and by L find Impulse , and finlay get the **delta_velocity** .
 * after we fix the velocity , two object is still collide, wo need to separate two element in the next tick
 * we need to add prefix
 * prefix = B * (depth / delta_time)
 * B from 0 to 1
 * prefix need to add into the constraint, so the equation become
 * (.........) * n - prefix = 0
 *
 * more details , visit https://zhuanlan.zhihu.com/p/411876276
 */
#[allow(unused)]
fn sequential_impulse() -> ! {
    panic!("don't use")
}
