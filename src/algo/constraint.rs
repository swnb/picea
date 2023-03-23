use std::marker::PhantomData;

use crate::{
    element::Element,
    math::{
        num::limit_at_range,
        point::Point,
        vector::{Vector, Vector3},
        CommonNum,
    },
    meta::{collision::CollisionInfo, Meta},
};

// pub trait Element {
//     fn translate(&mut self, vector: &Vector);

//     fn rotate(&mut self, deg: f32);

//     fn center_point(&self) -> Point;

//     fn meta(&self) -> &Meta;

//     fn meta_mut(&mut self) -> &mut Meta;

//     fn compute_point_velocity(&self, contact_point: Point) -> Vector;
// }

// pub fn update_elements_by_duration<T: Element>(element: &mut T, delta_t: f32) {
//     use std::f32::consts::TAU;

//     let meta = element.meta();

//     let inv_m = meta.inv_mass();

//     let force_group = meta.force_group();
//     let f = if force_group.is_empty() {
//         None
//     } else {
//         Some(force_group.sum_force())
//     };

//     let origin_v = meta.velocity();
//     let origin_w = meta.angular_velocity();

//     let a = f.map(|f| f * inv_m);

//     if let Some(a) = a {
//         let inc_v = a * delta_t;
//         element.meta_mut().set_velocity(|pre_v| pre_v + inc_v);
//     }

//     // compute_constraint(element, delta_t);

//     let meta = element.meta();

//     let current_v = meta.velocity();
//     let current_w = meta.angular_velocity();

//     let deg = (current_w + (current_w - origin_w) * 0.5) * delta_t;
//     element.meta_mut().set_angular(|pre| (pre + deg) % TAU);

//     let delta_s = (origin_v * 0.5 + current_v * 0.5) * delta_t;

//     element.translate(&delta_s);
//     element.rotate(deg);
// }

// pub fn compute_constraint<T: Element>(element: &mut T, delta_t: f32) {
//     if !element.meta().is_collision() {
//         return;
//     } else {
//         element.meta_mut().mark_collision(false);
//     }

//     let center_point = element.center_point();

//     let inv_mass = element.meta().inv_mass();

//     let Some(collision_info) = element.meta().collision_infos().next() else {
//         return
//     };

//     let contact_point = collision_info.contact_point;

//     let r: Vector = (center_point, contact_point).into();
//     let mut normal = collision_info.normal;

//     if normal * r > 0. {
//         normal = -normal
//     }

//     let mass_eff = element.meta().compute_mass_eff(normal, r);
//     let inv_moment_of_inertia = element.meta().inv_moment_of_inertia();
//     let depth = collision_info.depth;

//     let lambda = mass_eff;

//     // let B 0..1 ;let h = t; let b = B/h * depth
//     const B: f32 = 0.1;

//     let v = element.compute_point_velocity(contact_point);

//     // let velocity_reducer = move |pre_velocity: Vector| {
//     //     pre_velocity + normal * ((v * -normal + B * depth * delta_t.recip()) * lambda * inv_mass)
//     // };

//     // let angular_velocity_reducer = move |pre_angular_velocity| {
//     //     pre_angular_velocity - (r ^ normal) * lambda * inv_moment_of_inertia
//     // };

//     // element
//     //     .meta_mut()
//     //     .set_velocity(velocity_reducer)
//     //     .set_angular_velocity(angular_velocity_reducer);
// }

pub(crate) trait ManifoldIterMut {
    type Item<'a>: Iterator<Item = &'a mut CollisionInfo>
    where
        Self: 'a;

    fn iter_mut(&self) -> Self::Item<'_>;
}

pub(crate) fn constraint<'a, 'b, M, F>(
    contact_manifold: M,
    mut query_elements: F,
    delta_time: CommonNum,
    constraint_position: bool,
) where
    M: Iterator<Item = &'b mut CollisionInfo>,
    F: FnMut((u32, u32)) -> Option<(&'a mut Element, &'a mut Element)>,
{
    contact_manifold
        .filter_map(|collision_info| {
            query_elements(collision_info.collision_element_id_pair)
                .map(|elements| (elements, collision_info))
        })
        .filter(|((e_a, e_b), _)| !(e_a.meta().is_fixed() && e_b.meta().is_fixed()))
        .for_each(|((element_a, element_b), collision_info)| {
            let mut contact_info = ContactInfo {
                contact_point_a: *collision_info.contact_point_a(),
                contact_point_b: *collision_info.contact_point_b(),
                normal: collision_info.normal(),
                depth: collision_info.depth(),
                total_friction_lambda: 0.,
                total_lambda: 0.,
            };

            let mut solver = ContactSolver::new(element_a, element_b, &mut contact_info);
            if constraint_position {
                solver.solve_position_constraint(delta_time);
            } else {
                solver.solve_velocity_constraint(delta_time);
            }
            return;

            let mass_effective = match collision_info.mass_effective() {
                Some(v) => v,
                None => {
                    let mass_effective =
                        compute_mass_effective(element_a, element_b, &contact_info);
                    collision_info.set_mass_effective(mass_effective);
                    mass_effective
                }
            };

            let (lambda, friction_lambda) = compute_impulse(
                element_a,
                element_b,
                &contact_info,
                mass_effective,
                delta_time,
                constraint_position,
            );

            let center_point_a = element_a.center_point();

            element_a.meta_mut().apply_impulse(
                lambda,
                contact_info.normal,
                (center_point_a, *collision_info.contact_point_a()).into(),
            );

            element_a.meta_mut().apply_impulse(
                friction_lambda,
                !contact_info.normal,
                (center_point_a, *collision_info.contact_point_a()).into(),
            );

            let center_point_b = element_b.center_point();

            element_b.meta_mut().apply_impulse(
                lambda,
                -contact_info.normal,
                (center_point_b, *collision_info.contact_point_b()).into(),
            );

            element_b.meta_mut().apply_impulse(
                friction_lambda,
                -!contact_info.normal,
                (center_point_b, *collision_info.contact_point_b()).into(),
            );
        });
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
pub(crate) struct ContactInfo {
    contact_point_a: Point,
    contact_point_b: Point,
    normal: Vector,
    depth: f32,
    total_friction_lambda: CommonNum,
    total_lambda: CommonNum,
}

// TODO shrink this trait
pub trait ConstraintObject {
    fn center_point(&self) -> Point;

    fn meta(&self) -> &Meta;

    fn meta_mut(&mut self) -> &mut Meta;

    fn compute_point_velocity(&self, contact_point: &Point) -> Vector;
}

fn compute_mass_effective<Obj: ConstraintObject>(
    object_a: &mut Obj,
    object_b: &mut Obj,
    contact_info: &ContactInfo,
) -> CommonNum {
    let center_point_a = object_a.center_point();
    let center_point_b = object_b.center_point();

    let r_a: Vector = (center_point_a, contact_info.contact_point_a).into();
    let r_b: Vector = (center_point_b, contact_info.contact_point_b).into();

    let inv_moment_of_inertia_a = object_a.meta().inv_moment_of_inertia();
    let inv_moment_of_inertia_b = object_b.meta().inv_moment_of_inertia();

    let inv_mass_a = object_a.meta().inv_mass();

    let inv_mass_b = object_b.meta().inv_mass();

    let normal = contact_info.normal;

    // compute and mass_eff and lambda_n
    let equation_part1 = inv_mass_a;
    let equation_part2 = inv_mass_b;
    // let equation_part3 = ((normal * (r_a ^ normal)) ^ r_a) * inv_moment_of_inertia_a;
    let equation_part3 = (r_a ^ normal) * (r_a ^ normal) * inv_moment_of_inertia_a;
    let equation_part4 = (r_b ^ normal) * (r_b ^ normal) * inv_moment_of_inertia_b;

    (equation_part1 + equation_part2 + equation_part3 + equation_part4).recip()
}

fn compute_impulse<Obj: ConstraintObject>(
    object_a: &mut Obj,
    object_b: &mut Obj,
    contact_info: &ContactInfo,
    mass_effective: CommonNum,
    delta_time: CommonNum,
    should_use_bias: bool,
) -> (CommonNum, CommonNum) {
    let normal = contact_info.normal;
    let depth = contact_info.depth;

    let velocity_a = object_a.meta().velocity();
    let velocity_b = object_b.meta().velocity();

    let w_a = object_a.meta().angular_velocity();
    let w_b = object_b.meta().angular_velocity();

    let center_point_a = object_a.center_point();
    let center_point_b = object_b.center_point();

    let r_a: Vector = (center_point_a, contact_info.contact_point_a).into();
    let r_b: Vector = (center_point_b, contact_info.contact_point_b).into();

    let w_a = Vector3::from((0., 0., w_a));
    let r_a = Vector3::from(r_a);
    let w_velocity_a: Vector = (w_a ^ r_a).into();

    let w_b = Vector3::from((0., 0., w_b));
    let r_b = Vector3::from(r_b);
    let w_velocity_b: Vector = (w_b ^ r_b).into();

    let sum_velocity_a = velocity_a + w_velocity_a;

    let sum_velocity_b = velocity_b + w_velocity_b;

    // TODO set B into context
    const B: CommonNum = 0.5;

    const Cr: CommonNum = 0.1;

    let bias = if should_use_bias {
        // B * (depth - 0.02) * delta_time.recip()
        B * depth * delta_time.recip()
    } else {
        0.
    };

    let coefficient = (sum_velocity_a - sum_velocity_b) * -normal * (1. + Cr);

    debug_assert!(depth.is_sign_positive());

    let max_friction_lambada_n = (coefficient * mass_effective * 1.2).abs();

    let lambda_n = (coefficient + bias * 0.8) * mass_effective;

    let friction_lambda_n = -(sum_velocity_a - sum_velocity_b) * !normal * mass_effective;

    dbg!(max_friction_lambada_n, friction_lambda_n);

    let friction_lambda_n = limit_at_range(
        friction_lambda_n,
        (-max_friction_lambada_n)..=(max_friction_lambada_n),
    );

    let friction_lambda_n = friction_lambda_n * 0.1;

    (lambda_n, friction_lambda_n)
}

pub(crate) struct ContactSolver<'a: 'b, 'b, Object: ConstraintObject> {
    coefficient_bias: CommonNum,
    coefficient_elastic: CommonNum,
    max_allow_permeate: CommonNum,
    coefficient_friction: CommonNum,
    object_a: &'a mut Object,
    object_b: &'a mut Object,
    contact_info: &'b mut ContactInfo,
}

impl<'a: 'b, 'b, Object> ContactSolver<'a, 'b, Object>
where
    Object: ConstraintObject,
{
    pub(crate) fn new(
        object_a: &'a mut Object,
        object_b: &'a mut Object,
        contact_info: &'b mut ContactInfo,
    ) -> Self {
        Self {
            coefficient_bias: 0.8,
            coefficient_elastic: 0.01,
            max_allow_permeate: 0.2,
            coefficient_friction: 0.2,
            object_a,
            object_b,
            contact_info,
        }
    }

    fn solve_velocity_constraint(&mut self, bias: CommonNum) {
        // TODO cache this
        let mass_effective = self.compute_mass_effective();

        let Self {
            object_a,
            object_b,
            contact_info,
            ..
        } = self;

        let normal = contact_info.normal;
        let depth = contact_info.depth;

        let sum_velocity_a = object_a.compute_point_velocity(&contact_info.contact_point_a);

        let sum_velocity_b = object_b.compute_point_velocity(&contact_info.contact_point_b);

        let coefficient =
            (sum_velocity_a - sum_velocity_b) * -normal * (1. + self.coefficient_elastic);

        debug_assert!(depth.is_sign_positive());

        let max_friction_lambada_n =
            (coefficient * mass_effective * self.coefficient_friction).abs();

        let lambda = (coefficient + bias) * mass_effective;

        let previous_total_lambda = contact_info.total_lambda;
        contact_info.total_lambda += lambda;
        contact_info.total_lambda = contact_info.total_lambda.max(0.);
        let lambda = contact_info.total_lambda - previous_total_lambda;

        let friction_lambda = -(sum_velocity_a - sum_velocity_b) * !normal * mass_effective;

        let previous_total_friction_lambda = contact_info.total_friction_lambda;
        contact_info.total_friction_lambda += friction_lambda;
        contact_info.total_friction_lambda = limit_at_range(
            contact_info.total_friction_lambda,
            -max_friction_lambada_n..=max_friction_lambada_n,
        );
        let friction_lambda = contact_info.total_friction_lambda - previous_total_friction_lambda;

        // let friction_lambda = friction_lambda * 0.1;

        let center_point_a = object_a.center_point();

        object_a.meta_mut().apply_impulse(
            lambda,
            contact_info.normal,
            (center_point_a, contact_info.contact_point_a).into(),
        );

        object_a.meta_mut().apply_impulse(
            friction_lambda,
            !contact_info.normal,
            (center_point_a, contact_info.contact_point_a).into(),
        );

        let center_point_b = object_b.center_point();

        object_b.meta_mut().apply_impulse(
            lambda,
            -contact_info.normal,
            (center_point_b, contact_info.contact_point_b).into(),
        );

        object_b.meta_mut().apply_impulse(
            friction_lambda,
            -!contact_info.normal,
            (center_point_b, contact_info.contact_point_b).into(),
        );
    }

    fn solve_position_constraint(&mut self, delta_time: CommonNum) {
        let Self { contact_info, .. } = &*self;

        // let mut permeate = (contact_info.depth - self.max_allow_permeate).max(0.);

        let permeate = contact_info.depth;

        let bias = self.coefficient_bias * permeate * delta_time.recip();

        self.solve_velocity_constraint(bias);
    }

    fn compute_mass_effective(&self) -> CommonNum {
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

        let normal = contact_info.normal;

        // compute and mass_eff and lambda_n
        let equation_part1 = inv_mass_a;
        let equation_part2 = inv_mass_b;
        // let equation_part3 = ((normal * (r_a ^ normal)) ^ r_a) * inv_moment_of_inertia_a;
        let equation_part3 = (r_a ^ normal) * (r_a ^ normal) * inv_moment_of_inertia_a;
        let equation_part4 = (r_b ^ normal) * (r_b ^ normal) * inv_moment_of_inertia_b;

        (equation_part1 + equation_part2 + equation_part3 + equation_part4).recip()
    }
}

pub(crate) struct Solver<T: ConstraintObject> {
    _marker: PhantomData<T>,
}

const MAX_ITERATOR_TIMES: usize = 10;

impl<T: ConstraintObject> Solver<T> {
    pub(crate) fn constraint<'a: 'b, 'b, F>(&'a self, query_element_pair: F)
    where
        F: FnMut((u32, u32)) -> (&'b mut T, &'b mut T),
    {
        for _ in 0..MAX_ITERATOR_TIMES {}
    }
}
