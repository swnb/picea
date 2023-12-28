use derive_builder::Builder;

use crate::{
    math::{point::Point, vector::Vector, FloatNum},
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
    #[builder(default = "crate::math::PI()")]
    pub(crate) frequency: FloatNum,
}

impl Default for JoinConstraintConfig {
    fn default() -> Self {
        Self {
            distance: 0.,
            damping_ratio: 1.,
            frequency: crate::math::PI(),
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

pub fn compute_mass_effective<Obj: ConstraintObject>(
    &normal: &Vector,
    object_a: &Obj,
    object_b: &Obj,
    r_a: Vector,
    r_b: Vector,
) -> FloatNum {
    let inv_moment_of_inertia_a = object_a.meta().inv_moment_of_inertia();
    let inv_moment_of_inertia_b = object_b.meta().inv_moment_of_inertia();

    let inv_mass_a = object_a.meta().inv_mass();
    let inv_mass_b = object_b.meta().inv_mass();

    // compute and mass_eff and lambda_n
    let equation_part1 = inv_mass_a;
    let equation_part2 = inv_mass_b;
    let equation_part3 = (r_a ^ normal).powf(2.) * inv_moment_of_inertia_a;
    let equation_part4: f32 = (r_b ^ normal).powf(2.) * inv_moment_of_inertia_b;

    let inv_mass_effective = equation_part1 + equation_part2 + equation_part3 + equation_part4;

    if inv_mass_effective == 0. {
        0.
    } else {
        inv_mass_effective.recip()
    }
}

pub fn compute_soft_constraints_params(
    mass: FloatNum,
    damping_ratio: FloatNum,
    frequency: FloatNum,
    delta_time: FloatNum,
) -> (FloatNum, FloatNum) {
    let spring_constant = mass * frequency.powf(2.);
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
