use derive_builder::Builder;

use crate::{
    math::{point::Point, vector::Vector, FloatNum, TAU},
    meta::Meta,
};

pub mod contact;
pub mod join;
pub mod point;

#[derive(Builder, Clone)]
#[builder(pattern = "immutable")]
pub struct JoinConstraintConfig {
    #[builder(default = "0.")]
    pub(crate) distance: FloatNum,
    #[builder(default = "1.")]
    pub(crate) damping_ratio: FloatNum,
    #[builder(default = "0.5")]
    pub(crate) frequency: FloatNum,
    #[builder(default = "false")]
    pub(crate) hard: bool,
}

impl Default for JoinConstraintConfig {
    fn default() -> Self {
        Self {
            distance: 0.,
            damping_ratio: 1.,
            frequency: 0.5,
            hard: false,
        }
    }
}

impl JoinConstraintConfig {
    pub fn distance(&self) -> FloatNum {
        self.distance
    }

    pub fn damping_ratio(&self) -> FloatNum {
        self.damping_ratio
    }

    pub fn frequency(&self) -> FloatNum {
        self.frequency
    }
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
    fn center_point(&self) -> Point;

    fn meta(&self) -> &Meta;

    fn meta_mut(&mut self) -> &mut Meta;

    fn compute_point_velocity(&self, contact_point: &Point) -> Vector;
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
