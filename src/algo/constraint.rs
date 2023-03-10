use crate::{
    math::{point::Point, vector::Vector},
    meta::{collision::ContactType, Meta},
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
