use crate::{
    algo::constraint,
    math::{
        point::Point,
        vector::{Vector, Vector3},
        CommonNum,
    },
    meta::Meta,
};

pub trait Element {
    fn translate(&mut self, vector: &Vector);

    fn rotate(&mut self, deg: f32);

    fn center_point(&self) -> Point;

    fn meta(&self) -> &Meta;

    fn meta_mut(&mut self) -> &mut Meta;

    fn compute_point_velocity(&self, contact_point: Point) -> Vector;
}

pub fn update_elements_by_duration<T: Element>(element: &mut T, delta_t: f32) {
    use std::f32::consts::TAU;

    let meta = element.meta();

    let inv_m = meta.inv_mass();

    let force_group = meta.force_group();
    let f = if force_group.is_empty() {
        None
    } else {
        Some(force_group.sum_force())
    };

    let origin_v = meta.velocity();
    let origin_w = meta.angular_velocity();

    let a = f.map(|f| f * inv_m);

    if let Some(a) = a {
        let inc_v = a * delta_t;
        element.meta_mut().set_velocity(|pre_v| pre_v + inc_v);
    }

    compute_constraint(element, delta_t);

    let meta = element.meta();

    let current_v = meta.velocity();
    let current_w = meta.angular_velocity();

    let deg = (current_w + (current_w - origin_w) * 0.5) * delta_t;
    element.meta_mut().set_angular(|pre| (pre + deg) % TAU);

    let delta_s = (origin_v * 0.5 + current_v * 0.5) * delta_t;

    element.translate(&delta_s);
    element.rotate(deg);
}

pub fn compute_constraint<T: Element>(element: &mut T, delta_t: f32) {
    if !element.meta().is_collision() {
        return;
    } else {
        element.meta_mut().mark_collision(false);
    }

    let center_point = element.center_point();

    let inv_mass = element.meta().inv_mass();

    let Some(collision_info) = element.meta().collision_infos().next() else {
        return
    };

    let contact_point = collision_info.contact_point;

    let r: Vector = (center_point, contact_point).into();
    let mut normal = collision_info.normal;

    if normal * r > 0. {
        normal = -normal
    }

    let mass_eff = element.meta().compute_mass_eff(normal, r);
    let inv_moment_of_inertia = element.meta().inv_moment_of_inertia();
    let depth = collision_info.depth;

    let lambda = mass_eff;

    // let B 0..1 ;let h = t; let b = B/h * depth
    const B: f32 = 0.1;

    let v = element.compute_point_velocity(contact_point);

    let velocity_reducer = move |pre_velocity: Vector| {
        pre_velocity + normal * ((v * -normal + B * depth * delta_t.recip()) * lambda * inv_mass)
    };

    let angular_velocity_reducer = move |pre_angular_velocity| {
        pre_angular_velocity - (r ^ normal) * lambda * inv_moment_of_inertia
    };

    element
        .meta_mut()
        .set_velocity(velocity_reducer)
        .set_angular_velocity(angular_velocity_reducer);
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

pub(crate) struct ContactInfo {
    normal: Vector,
    depth: f32,
    contact_point_a: Point,
    contact_point_b: Point,
}

// TODO shrink this trait
pub trait ConstraintObject {
    fn translate(&mut self, vector: &Vector);

    fn rotate(&mut self, deg: f32);

    fn center_point(&self) -> Point;

    fn meta(&self) -> &Meta;

    fn meta_mut(&mut self) -> &mut Meta;

    fn compute_point_velocity(&self, contact_point: Point) -> Vector;
}

fn compute_mass_effective<Obj: ConstraintObject>(
    object_a: &mut Obj,
    object_b: &mut Obj,
    contact_infos: &ContactInfo,
) -> CommonNum {
    const Cr: CommonNum = 0.9; // loss energy when collide happen

    let center_point_a = object_a.center_point();
    let center_point_b = object_b.center_point();

    let r_a: Vector = (center_point_a, contact_infos.contact_point_a).into();
    let r_b: Vector = (center_point_b, contact_infos.contact_point_b).into();

    let inv_moment_of_inertia_a = object_a.meta().inv_moment_of_inertia();
    let inv_moment_of_inertia_b = object_b.meta().inv_moment_of_inertia();

    let inv_mass_a = object_a.meta().inv_mass();
    let inv_mass_b = object_b.meta().inv_mass();

    let normal = contact_infos.normal;

    // compute and mass_eff and lambda_n
    let equation_part1 = inv_mass_a;
    let equation_part2 = inv_mass_b;
    let equation_part3 = ((normal * (r_a ^ normal)) ^ r_a) * inv_moment_of_inertia_a;
    let equation_part4 = ((normal * (r_b ^ normal)) ^ r_b) * inv_moment_of_inertia_b;

    (equation_part1 + equation_part2 + equation_part3 + equation_part4).recip()
}

fn compute_impulse<Obj: ConstraintObject>(
    object_a: &mut Obj,
    object_b: &mut Obj,
    contact_infos: &ContactInfo,
    mass_effective: CommonNum,
) -> Vector {
    let normal = contact_infos.normal;

    let velocity_a = object_a.meta().velocity();
    let velocity_b = object_b.meta().velocity();

    let w_a = object_a.meta().angular_velocity();
    let w_b = object_b.meta().angular_velocity();

    let center_point_a = object_a.center_point();
    let center_point_b = object_b.center_point();

    let r_a: Vector = (center_point_a, contact_infos.contact_point_a).into();
    let r_b: Vector = (center_point_b, contact_infos.contact_point_b).into();

    let w_a = Vector3::from((0., 0., w_a));
    let r_a = Vector3::from(r_a);
    let w_velocity_a: Vector = (w_a ^ r_a).into();

    let w_b = Vector3::from((0., 0., w_b));
    let r_b = Vector3::from(r_b);
    let w_velocity_b: Vector = (w_b ^ r_b).into();

    let sum_velocity_a = velocity_a + w_velocity_a;

    let sum_velocity_b = velocity_b + w_velocity_b;

    let coefficient = (sum_velocity_a - sum_velocity_b) * normal;

    let lambda_n = -coefficient * mass_effective;

    normal * lambda_n
}
