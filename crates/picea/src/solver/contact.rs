use std::collections::BTreeMap;

use crate::{
    events::SleepTransitionReason,
    handles::BodyHandle,
    math::{point::Point, vector::Vector, FloatNum},
    pipeline::{contacts::ContactObservation, island, sleep, StepConfig},
    world::World,
};

const POSITION_CORRECTION_PERCENT: FloatNum = 0.8;
const POSITION_CORRECTION_SLOP: FloatNum = 0.005;
const CONTACT_VELOCITY_BIAS: FloatNum = 0.5;

#[derive(Clone, Copy, Debug)]
struct SolverBody {
    dynamic: bool,
    inverse_mass: FloatNum,
    inverse_inertia: FloatNum,
    center: Point,
    linear_velocity: Vector,
    angular_velocity: FloatNum,
}

#[derive(Clone, Debug)]
struct ContactSolverRow {
    contact_index: usize,
    body_a_slot: usize,
    body_b_slot: usize,
    normal: Vector,
    tangent: Vector,
    anchor_a: Vector,
    anchor_b: Vector,
    normal_mass: FloatNum,
    tangent_mass: FloatNum,
    friction: FloatNum,
    restitution_bias: FloatNum,
    position_bias: FloatNum,
    normal_impulse: FloatNum,
    tangent_impulse: FloatNum,
    normal_impulse_clamped: bool,
    tangent_impulse_clamped: bool,
    restitution_velocity_threshold: FloatNum,
    restitution_applied: bool,
}

struct ContactSolveBatch {
    body_slots: Vec<BodyHandle>,
    rows: Vec<ContactSolverRow>,
}

pub(crate) fn resolve_contacts(
    world: &mut World,
    contacts: &mut [ContactObservation],
    config: &StepConfig,
    wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
) -> island::SolverStepStats {
    let islands = sleep::build_active_solver_islands(
        world,
        contacts
            .iter()
            .filter(|contact| !contact.is_sensor)
            .map(|contact| (contact.body_a, contact.body_b)),
        wake_reasons,
    );
    let plan = island::build_island_solve_plan(
        &islands,
        contacts
            .iter()
            .enumerate()
            .filter(|(_, contact)| !contact.is_sensor)
            .map(|(index, contact)| (index, contact.body_a, contact.body_b)),
        std::iter::empty(),
    );
    for contact in contacts.iter_mut() {
        contact.normal_impulse = 0.0;
        contact.tangent_impulse = 0.0;
        contact.normal_impulse_clamped = false;
        contact.tangent_impulse_clamped = false;
        contact.restitution_velocity_threshold = 0.0;
        contact.restitution_applied = false;
    }

    let (mut batches, stats) = contact_solver_row_batches(world, contacts, &islands, plan, config);

    for batch in &mut batches {
        let mut solver_bodies = solver_body_cache(world, &batch.body_slots);
        for row in &batch.rows {
            let warm_start_impulse =
                row.normal * row.normal_impulse + row.tangent * row.tangent_impulse;
            apply_solver_impulse(&mut solver_bodies, row, warm_start_impulse);
        }

        for _ in 0..config.velocity_iterations {
            for row in &mut batch.rows {
                solve_normal_impulse(&mut solver_bodies, row);
                solve_tangent_impulse(&mut solver_bodies, row);
            }
        }

        for row in &batch.rows {
            let contact = &mut contacts[row.contact_index];
            contact.normal_impulse = row.normal_impulse.max(0.0);
            contact.tangent_impulse = row.tangent_impulse;
            contact.normal_impulse_clamped = row.normal_impulse_clamped;
            contact.tangent_impulse_clamped = row.tangent_impulse_clamped;
            contact.restitution_velocity_threshold = row.restitution_velocity_threshold;
            contact.restitution_applied = row.restitution_applied;
        }
        record_contact_impulse_wakes_for_rows(world, contacts, &batch.rows, wake_reasons);
        write_solver_velocities(world, &batch.body_slots, &solver_bodies, wake_reasons);
    }

    apply_residual_contact_position_correction(
        world,
        contacts,
        config.position_iterations,
        wake_reasons,
    );
    stats
}

fn contact_solver_row_batches(
    world: &World,
    contacts: &[ContactObservation],
    islands: &[sleep::SolverIsland],
    plan: island::IslandSolvePlan,
    config: &StepConfig,
) -> (Vec<ContactSolveBatch>, island::SolverStepStats) {
    let batches = plan
        .islands
        .into_iter()
        .filter_map(|island| {
            let bodies = solver_body_cache(world, &island.body_slots);
            let rows = island
                .contact_rows
                .iter()
                .filter_map(|row| {
                    contact_solver_row(
                        row.contact_index,
                        row.body_a_slot,
                        row.body_b_slot,
                        &contacts[row.contact_index],
                        &bodies,
                        config,
                    )
                })
                .collect::<Vec<_>>();
            (!rows.is_empty()).then_some(ContactSolveBatch {
                body_slots: island.body_slots,
                rows,
            })
        })
        .collect::<Vec<_>>();
    let stats = island::SolverStepStats {
        island_count: islands.len(),
        active_island_count: islands.iter().filter(|island| island.active).count(),
        sleeping_island_skip_count: islands.iter().filter(|island| !island.active).count(),
        body_slot_count: batches.iter().map(|batch| batch.body_slots.len()).sum(),
        contact_row_count: batches.iter().map(|batch| batch.rows.len()).sum(),
        joint_row_count: 0,
    };
    (batches, stats)
}

fn solver_body_cache(world: &World, body_slots: &[BodyHandle]) -> Vec<SolverBody> {
    body_slots
        .iter()
        .map(|handle| {
            let record = world
                .body_record(*handle)
                .expect("live contact body handles must resolve");
            let mass = record.mass_properties;
            SolverBody {
                dynamic: record.body_type.is_dynamic(),
                inverse_mass: mass.inverse_mass,
                inverse_inertia: mass.inverse_inertia,
                center: record.pose.transform_point(mass.local_center_of_mass),
                linear_velocity: record.linear_velocity,
                angular_velocity: record.angular_velocity,
            }
        })
        .collect()
}

fn contact_solver_row(
    contact_index: usize,
    body_a_slot: usize,
    body_b_slot: usize,
    contact: &ContactObservation,
    bodies: &[SolverBody],
    config: &StepConfig,
) -> Option<ContactSolverRow> {
    if contact.is_sensor {
        return None;
    }
    let body_a = *bodies.get(body_a_slot)?;
    let body_b = *bodies.get(body_b_slot)?;
    let normal = contact.normal.normalized_or_zero();
    let tangent = normal.perp().normalized_or_zero();
    if normal.length() <= FloatNum::EPSILON || tangent.length() <= FloatNum::EPSILON {
        return None;
    }
    let anchor_a = contact.point - body_a.center;
    let anchor_b = contact.point - body_b.center;
    let normal_mass = effective_mass(body_a, body_b, anchor_a, anchor_b, normal);
    let tangent_mass = effective_mass(body_a, body_b, anchor_a, anchor_b, tangent);
    if normal_mass <= 0.0 && tangent_mass <= 0.0 {
        return None;
    }

    let relative_normal_speed =
        relative_contact_velocity(body_a, body_b, anchor_a, anchor_b).dot(normal);
    let restitution_threshold = config.restitution_velocity_threshold;
    let restitution_applied =
        -relative_normal_speed > restitution_threshold && contact.material.restitution > 0.0;
    let restitution_bias = if restitution_applied {
        contact.material.restitution.clamp(0.0, 1.0) * -relative_normal_speed
    } else {
        0.0
    };
    // Penetration velocity bias gives resting overlap a support impulse during the
    // velocity solve, so Coulomb friction has a real normal budget to clamp against.
    let position_bias = if relative_normal_speed.abs() <= 1.0e-4 {
        (contact.depth - POSITION_CORRECTION_SLOP).max(0.0) * CONTACT_VELOCITY_BIAS / config.dt
    } else {
        0.0
    };
    let friction = contact.material.friction.max(0.0);
    let normal_impulse = contact.warm_start_normal_impulse.max(0.0);
    let max_friction = friction * normal_impulse;
    let tangent_impulse = contact
        .warm_start_tangent_impulse
        .clamp(-max_friction, max_friction);

    Some(ContactSolverRow {
        contact_index,
        body_a_slot,
        body_b_slot,
        normal,
        tangent,
        anchor_a,
        anchor_b,
        normal_mass,
        tangent_mass,
        friction,
        restitution_bias,
        position_bias,
        normal_impulse,
        tangent_impulse,
        normal_impulse_clamped: false,
        tangent_impulse_clamped: (tangent_impulse - contact.warm_start_tangent_impulse).abs()
            > FloatNum::EPSILON,
        restitution_velocity_threshold: restitution_threshold,
        restitution_applied,
    })
}

fn write_solver_velocities(
    world: &mut World,
    body_slots: &[BodyHandle],
    bodies: &[SolverBody],
    wake_reasons: &BTreeMap<BodyHandle, SleepTransitionReason>,
) {
    for (handle, body) in body_slots.iter().zip(bodies.iter()) {
        if !body.dynamic {
            continue;
        };
        let Ok(record) = world.body_record_mut(*handle) else {
            continue;
        };
        if record.sleeping && !wake_reasons.contains_key(handle) {
            continue;
        }
        record.linear_velocity = body.linear_velocity;
        record.angular_velocity = body.angular_velocity;
    }
}

fn record_contact_impulse_wakes_for_rows(
    world: &World,
    contacts: &[ContactObservation],
    rows: &[ContactSolverRow],
    wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
) {
    for contact in rows
        .iter()
        .map(|row| &contacts[row.contact_index])
        .filter(|contact| !contact.is_sensor && contact.normal_impulse > FloatNum::EPSILON)
    {
        record_contact_wake_if_sleeping(world, contact.body_a, contact.body_b, wake_reasons);
        record_contact_wake_if_sleeping(world, contact.body_b, contact.body_a, wake_reasons);
    }
}

fn record_contact_wake_if_sleeping(
    world: &World,
    body: BodyHandle,
    other: BodyHandle,
    wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
) {
    if world
        .body_record(body)
        .map(|record| record.sleeping && record.body_type.is_dynamic())
        .unwrap_or(false)
        && contact_counterpart_can_wake(world, other)
    {
        sleep::record_wake_reason(wake_reasons, body, SleepTransitionReason::Impact);
    }
}

fn contact_counterpart_can_wake(world: &World, other: BodyHandle) -> bool {
    world
        .body_record(other)
        .map(|record| !record.body_type.is_static() && !record.sleeping)
        .unwrap_or(false)
}

fn apply_residual_contact_position_correction(
    world: &mut World,
    contacts: &[ContactObservation],
    iterations: u16,
    wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
) {
    if iterations == 0 {
        return;
    }

    for _ in 0..iterations {
        for contact in contacts.iter().filter(|contact| !contact.is_sensor) {
            let normal = contact.normal.normalized_or_zero();
            let depth = (contact.depth - POSITION_CORRECTION_SLOP).max(0.0);
            if normal.length() <= FloatNum::EPSILON || depth <= FloatNum::EPSILON {
                continue;
            }
            let inv_mass_a = world
                .body_record(contact.body_a)
                .map(|record| record.mass_properties.inverse_mass)
                .unwrap_or(0.0);
            let inv_mass_b = world
                .body_record(contact.body_b)
                .map(|record| record.mass_properties.inverse_mass)
                .unwrap_or(0.0);
            let inv_mass_sum = inv_mass_a + inv_mass_b;
            if inv_mass_sum <= FloatNum::EPSILON {
                continue;
            }
            let correction = depth * POSITION_CORRECTION_PERCENT / FloatNum::from(iterations);
            let correction_a = normal * (correction * inv_mass_a / inv_mass_sum);
            let correction_b = -normal * (correction * inv_mass_b / inv_mass_sum);
            let wake_a = contact_counterpart_can_wake(world, contact.body_b);
            let wake_b = contact_counterpart_can_wake(world, contact.body_a);
            apply_position_translation(world, contact.body_a, correction_a, wake_a, wake_reasons);
            apply_position_translation(world, contact.body_b, correction_b, wake_b, wake_reasons);
        }
    }
}

fn apply_position_translation(
    world: &mut World,
    body: BodyHandle,
    translation: Vector,
    wake_sleeping_body: bool,
    wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
) {
    if translation.length() <= FloatNum::EPSILON {
        return;
    }
    let Ok(record) = world.body_record_mut(body) else {
        return;
    };
    if !record.body_type.is_dynamic() {
        return;
    }
    let was_sleeping = record.sleeping;
    if was_sleeping && !wake_sleeping_body {
        return;
    }
    crate::solver::body_state::translate_pose(&mut record.pose, translation, 0.0);
    record.sleeping = false;
    record.sleep_idle_time = 0.0;
    if was_sleeping {
        sleep::record_wake_reason(wake_reasons, body, SleepTransitionReason::ContactImpulse);
    }
}

fn solve_normal_impulse(bodies: &mut [SolverBody], row: &mut ContactSolverRow) {
    if row.normal_mass <= 0.0 {
        return;
    }
    let Some((body_a, body_b)) = solver_pair(bodies, row) else {
        return;
    };
    let normal_speed =
        relative_contact_velocity(body_a, body_b, row.anchor_a, row.anchor_b).dot(row.normal);
    // Accumulated impulses are clamped, not per-iteration deltas. This lets a row
    // give impulse back on later iterations without ever pulling bodies together.
    let previous = row.normal_impulse;
    let candidate =
        previous - (normal_speed - row.restitution_bias - row.position_bias) * row.normal_mass;
    row.normal_impulse = candidate.max(0.0);
    row.normal_impulse_clamped |= candidate < 0.0;
    let delta = row.normal_impulse - previous;
    apply_solver_impulse(bodies, row, row.normal * delta);
}

fn solve_tangent_impulse(bodies: &mut [SolverBody], row: &mut ContactSolverRow) {
    if row.tangent_mass <= 0.0 {
        return;
    }
    let Some((body_a, body_b)) = solver_pair(bodies, row) else {
        return;
    };
    let tangent_speed =
        relative_contact_velocity(body_a, body_b, row.anchor_a, row.anchor_b).dot(row.tangent);
    let previous = row.tangent_impulse;
    let candidate = previous - tangent_speed * row.tangent_mass;
    // Coulomb friction limits the tangent row by the normal support impulse
    // solved so far: |jt| <= mu * jn.
    let max_friction = row.friction * row.normal_impulse;
    row.tangent_impulse = candidate.clamp(-max_friction, max_friction);
    row.tangent_impulse_clamped |= (row.tangent_impulse - candidate).abs() > FloatNum::EPSILON;
    let delta = row.tangent_impulse - previous;
    apply_solver_impulse(bodies, row, row.tangent * delta);
}

fn solver_pair(bodies: &[SolverBody], row: &ContactSolverRow) -> Option<(SolverBody, SolverBody)> {
    Some((*bodies.get(row.body_a_slot)?, *bodies.get(row.body_b_slot)?))
}

fn apply_solver_impulse(bodies: &mut [SolverBody], row: &ContactSolverRow, impulse: Vector) {
    apply_impulse_to_body(bodies, row.body_a_slot, row.anchor_a, impulse);
    apply_impulse_to_body(bodies, row.body_b_slot, row.anchor_b, -impulse);
}

fn apply_impulse_to_body(bodies: &mut [SolverBody], slot: usize, anchor: Vector, impulse: Vector) {
    let Some(body) = bodies.get_mut(slot) else {
        return;
    };
    if !body.dynamic {
        return;
    }
    body.linear_velocity += impulse * body.inverse_mass;
    body.angular_velocity -= anchor.cross(impulse) * body.inverse_inertia;
}

fn effective_mass(
    body_a: SolverBody,
    body_b: SolverBody,
    anchor_a: Vector,
    anchor_b: Vector,
    direction: Vector,
) -> FloatNum {
    let anchor_a_cross = anchor_a.cross(direction);
    let anchor_b_cross = anchor_b.cross(direction);
    // Effective mass is the scalar inverse of "how hard is it to change
    // relative velocity along this row", including angular inertia through r x n.
    let denominator = body_a.inverse_mass
        + body_b.inverse_mass
        + body_a.inverse_inertia * anchor_a_cross * anchor_a_cross
        + body_b.inverse_inertia * anchor_b_cross * anchor_b_cross;
    if denominator.is_finite() && denominator > FloatNum::EPSILON {
        1.0 / denominator
    } else {
        0.0
    }
}

fn relative_contact_velocity(
    body_a: SolverBody,
    body_b: SolverBody,
    anchor_a: Vector,
    anchor_b: Vector,
) -> Vector {
    contact_point_velocity(body_a, anchor_a) - contact_point_velocity(body_b, anchor_b)
}

fn contact_point_velocity(body: SolverBody, anchor: Vector) -> Vector {
    body.linear_velocity + angular_point_velocity(body.angular_velocity, anchor)
}

fn angular_point_velocity(angular_velocity: FloatNum, anchor: Vector) -> Vector {
    Vector::new(
        angular_velocity * anchor.y(),
        -angular_velocity * anchor.x(),
    )
}
