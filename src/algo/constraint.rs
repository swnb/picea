use crate::{
    math::{num::limit_at_range, point::Point, vector::Vector, FloatNum},
    meta::{collision::Manifold, Meta},
    scene::context::{ConstraintParameters, Context},
};
use std::ops::Deref;

use super::collision::ContactPointPair;

pub(crate) trait ManifoldsIterMut {
    type Item<'a>: Iterator<Item = &'a mut Manifold>
    where
        Self: 'a;

    fn iter_mut(&mut self) -> Self::Item<'_>;
}

/**
 * Sequential Impulse
 * two object collide，object A and object B
 * n is the collision direction normal vector that could separate two object
 * depth define how depth two object collide,
 * the constraint define blow:
 * ( velocity_A + (angular_velocity_A X radius_A) - velocity_B - (angular_velocity_B X radius_B) ) * n == 0
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
fn sequential_impulse() {
    todo!()
}

#[derive(Debug, Clone)]
pub struct ContactPointPairInfo {
    contact_point_pair: ContactPointPair,
    total_friction_lambda: FloatNum,
    total_lambda: FloatNum,
    mass_effective: Option<FloatNum>,
}

impl From<ContactPointPair> for ContactPointPairInfo {
    fn from(contact_point_pair: ContactPointPair) -> Self {
        Self {
            contact_point_pair,
            total_friction_lambda: 0.,
            total_lambda: 0.,
            mass_effective: None,
        }
    }
}

impl Deref for ContactPointPairInfo {
    type Target = ContactPointPair;
    fn deref(&self) -> &Self::Target {
        &self.contact_point_pair
    }
}

pub trait ConstraintObject {
    fn center_point(&self) -> Point;

    fn meta(&self) -> &Meta;

    fn meta_mut(&mut self) -> &mut Meta;

    fn compute_point_velocity(&self, contact_point: &Point) -> Vector;
}

pub(crate) struct ContactSolver<'a: 'b, 'b, 'c, Object: ConstraintObject> {
    object_a: &'a mut Object,
    object_b: &'a mut Object,
    contact_info: &'b mut ContactPointPairInfo,
    constraint_parameters: &'c ConstraintParameters,
}

impl<'a: 'b, 'b, 'c, Object> ContactSolver<'a, 'b, 'c, Object>
where
    Object: ConstraintObject,
{
    pub(crate) fn new(
        object_a: &'a mut Object,
        object_b: &'a mut Object,
        contact_info: &'b mut ContactPointPairInfo,
        constraint_parameters: &'c ConstraintParameters,
    ) -> Self {
        Self {
            object_a,
            object_b,
            contact_info,
            constraint_parameters,
        }
    }

    fn solve_velocity_constraint(&mut self, bias: FloatNum) {
        let mass_effective = match self.contact_info.mass_effective {
            Some(v) => v,
            None => {
                let v = self.compute_mass_effective();
                self.contact_info.mass_effective = Some(v);
                v
            }
        };

        let Self {
            object_a,
            object_b,
            contact_info,
            constraint_parameters,
        } = self;

        let normal = contact_info.normal_toward_a;
        let depth = contact_info.depth;

        let sum_velocity_a = object_a.compute_point_velocity(&contact_info.contact_point_a);

        let sum_velocity_b = object_b.compute_point_velocity(&contact_info.contact_point_b);

        let coefficient = (sum_velocity_a - sum_velocity_b)
            * -normal
            * (1. + constraint_parameters.factor_elastic);

        debug_assert!(depth.is_sign_positive());

        // (coefficient * mass_effective * self.coefficient_friction).abs();

        let lambda = (coefficient + bias) * mass_effective;

        // TODO factor_friction use two element's factor_friction
        let max_friction_lambada_n = (lambda * constraint_parameters.factor_default_friction).abs();

        let previous_total_lambda = contact_info.total_lambda;
        contact_info.total_lambda += lambda;
        contact_info.total_lambda = contact_info.total_lambda.max(0.);
        let lambda = contact_info.total_lambda - previous_total_lambda;

        // TODO better friction algo
        let friction_lambda = -(sum_velocity_a - sum_velocity_b) * !normal * mass_effective * 0.5;

        // let current_friction_lambda = friction_lambda;

        let previous_total_friction_lambda = contact_info.total_friction_lambda;
        contact_info.total_friction_lambda += friction_lambda;
        contact_info.total_friction_lambda = limit_at_range(
            contact_info.total_friction_lambda,
            -max_friction_lambada_n..=max_friction_lambada_n,
        );
        let friction_lambda = contact_info.total_friction_lambda - previous_total_friction_lambda;

        let center_point_a = object_a.center_point();

        object_a.meta_mut().apply_impulse(
            lambda,
            contact_info.normal_toward_a,
            (center_point_a, contact_info.contact_point_a).into(),
        );

        object_a.meta_mut().apply_impulse(
            friction_lambda,
            !contact_info.normal_toward_a,
            (center_point_a, contact_info.contact_point_a).into(),
        );

        let center_point_b = object_b.center_point();

        object_b.meta_mut().apply_impulse(
            lambda,
            -contact_info.normal_toward_a,
            (center_point_b, contact_info.contact_point_b).into(),
        );

        object_b.meta_mut().apply_impulse(
            friction_lambda,
            -!contact_info.normal_toward_a,
            (center_point_b, contact_info.contact_point_b).into(),
        );
    }

    fn solve_position_constraint(&mut self, delta_time: FloatNum) {
        let Self {
            contact_info,
            constraint_parameters,
            ..
        } = &*self;

        // let mut permeate = (contact_info.depth - self.max_allow_permeate).max(0.);

        let permeate = contact_info.depth;

        let bias = constraint_parameters.factor_position_bias * permeate * delta_time.recip();

        self.solve_velocity_constraint(bias);
    }

    fn compute_mass_effective(&self) -> FloatNum {
        let Self {
            object_a,
            object_b,
            contact_info,
            ..
        } = self;

        let center_point_a = object_a.center_point();
        let center_point_b = object_b.center_point();

        let r_a: Vector = (center_point_a, contact_info.contact_point_a).into();
        let r_b: Vector = (center_point_b, contact_info.contact_point_b).into();

        let inv_moment_of_inertia_a = object_a.meta().inv_moment_of_inertia();
        let inv_moment_of_inertia_b = object_b.meta().inv_moment_of_inertia();

        let inv_mass_a = object_a.meta().inv_mass();

        let inv_mass_b = object_b.meta().inv_mass();

        let normal = contact_info.normal_toward_a;

        // compute and mass_eff and lambda_n
        let equation_part1 = inv_mass_a;
        let equation_part2 = inv_mass_b;
        // let equation_part3 = ((normal * (r_a ^ normal)) ^ r_a) * inv_moment_of_inertia_a;
        let equation_part3 = (r_a ^ normal) * (r_a ^ normal) * inv_moment_of_inertia_a;
        let equation_part4 = (r_b ^ normal) * (r_b ^ normal) * inv_moment_of_inertia_b;

        (equation_part1 + equation_part2 + equation_part3 + equation_part4).recip()
    }
}

pub(crate) struct Solver<'z, 'e, M>
where
    M: ManifoldsIterMut + ?Sized,
{
    context: &'e Context,
    contact_manifolds: &'z mut M,
}

const MAX_ITERATOR_TIMES: usize = 10;

impl<'z, 'e, M> Solver<'z, 'e, M>
where
    M: ManifoldsIterMut + ?Sized,
{
    pub(crate) fn new(context: &'e Context, contact_manifolds: &'z mut M) -> Self {
        Self {
            context,
            contact_manifolds,
        }
    }

    pub(crate) fn constraint<'a, 'b: 'a, F, T: 'b>(
        &'a mut self,
        mut query_element_pair: F,
        delta_time: FloatNum,
    ) where
        T: ConstraintObject,
        F: FnMut((u32, u32)) -> Option<(&'b mut T, &'b mut T)>,
    {
        let solve = |(object_a, object_b, manifold): (&'_ mut T, &'_ mut T, &'_ mut Manifold),
                     fix_position: bool| {
            for contact_info in manifold.contact_point_pairs_mut() {
                let mut solver = ContactSolver::new(
                    object_a,
                    object_b,
                    contact_info,
                    &self.context.constraint_parameters,
                );

                if fix_position {
                    solver.solve_position_constraint(delta_time);
                } else {
                    solver.solve_velocity_constraint(delta_time);
                }
            }
        };

        let mut constraint = |fix_position: bool| {
            self.contact_manifolds
                .iter_mut()
                .filter_map(|collision_info| {
                    query_element_pair(collision_info.collision_element_id_pair)
                        .map(|(object_a, object_b)| (object_a, object_b, collision_info))
                })
                .filter(|(object_a, object_b, _)| {
                    !(object_a.meta().is_fixed() && object_b.meta().is_fixed())
                })
                .for_each(|v| solve(v, fix_position));
        };

        for _ in 0..MAX_ITERATOR_TIMES {
            constraint(false);
        }

        constraint(true);
    }
}
