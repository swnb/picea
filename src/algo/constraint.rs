use crate::{element::Element, math::vector::Vector, meta::collision::ContactType, shape::Shape};

pub fn update_elements_by_duration(element: &mut Element, delta_t: f32) {
    use std::f32::consts::TAU;

    let meta = element.meta();

    let inv_m = meta.inv_mass();

    let force_group = meta.force();
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
    let delta_s = (current_v + (origin_v - current_v) * 0.5) * delta_t;

    let shape = element.shape_mut();
    shape.translate(&delta_s);
    shape.rotate(deg);
}

pub fn compute_constraint(element: &mut Element, delta_t: f32) {
    if !element.meta().is_collision() {
        return;
    } else {
        element.meta_mut().mark_collision(false);
    }

    let center_point = element.shape().compute_center_point();

    let collision_info = element.meta().collision_infos().next();

    let inv_mass = element.meta().inv_mass();

    if let Some((velocity_reducer, angular_velocity_reducer)) =
        collision_info.map(|collision_info| {
            use ContactType::*;
            let contact_point = match collision_info.contact_points() {
                (Edge(edge_a), Edge(edge_b)) => {
                    // TODO 物体堆叠
                    unimplemented!();
                }
                (Edge(edge_a), Point(point_b)) => *point_b,
                (Point(point_a), Edge(edge_b)) => *point_a,
                (Point(point_a), Point(point_b)) => *point_a,
            };

            let r: Vector<f32> = (center_point, contact_point).into();
            let mut normal = collision_info.normal;

            if normal * r > 0. {
                normal = -normal
            }

            let mass_eff = element.meta().compute_mass_eff(normal, r);
            let inv_moment_of_inertia = element.meta().inv_moment_of_inertia();
            let depth = collision_info.depth;

            let lambda = mass_eff;

            // let B 0..1 ;let h = t; let b = B/h * depth
            const B: f32 = 0.9;

            let v = element.compute_point_velocity(contact_point);

            let velocity_reducer = move |pre_velocity: Vector<f32>| {
                pre_velocity + normal * ((v * -normal + B * depth / delta_t) * lambda * inv_mass)
            };

            let angular_velocity_reducer = move |pre_angular_velocity| {
                pre_angular_velocity - (r ^ normal) * lambda * inv_moment_of_inertia
            };

            (velocity_reducer, angular_velocity_reducer)
        })
    {
        element.meta_mut().set_velocity(velocity_reducer);
        element
            .meta_mut()
            .set_angular_velocity(angular_velocity_reducer)
    }
}
