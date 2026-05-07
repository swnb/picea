use std::collections::BTreeMap;

use crate::{
    events::ContactReductionReason,
    events::SleepTransitionReason,
    handles::BodyHandle,
    math::{point::Point, vector::Vector, FloatNum},
    pipeline::{contacts::ContactObservation, island, sleep, StepConfig},
    world::{contact_state::ContactPairKey, World},
};

const POSITION_CORRECTION_PERCENT: FloatNum = 0.8;
const POSITION_CORRECTION_SLOP: FloatNum = 0.005;
const POSITION_CORRECTION_SLEEP_RESET_TRANSLATION: FloatNum = POSITION_CORRECTION_SLOP;
const CONTACT_VELOCITY_BIAS: FloatNum = 0.5;
const SHALLOW_SUPPORT_FRICTION_BUDGET_SCALE: FloatNum = 0.5;
const MANIFOLD_BLOCK_SOLVE_TANGENT_SPEED_THRESHOLD: FloatNum = 0.25;
// Dense contact graphs are the matrix-stack case: many overlapping rows can
// reuse stale frame-start depth and over-correct the same bodies. Small stacks
// keep the legacy path because it currently satisfies the quiet behavior lock.
const DENSE_POSITION_CORRECTION_CONTACT_THRESHOLD: usize = 16;

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
    pair_key: ContactPairKey,
    body_a_slot: usize,
    body_b_slot: usize,
    normal: Vector,
    tangent: Vector,
    anchor_a: Vector,
    anchor_b: Vector,
    normal_mass: FloatNum,
    tangent_mass: FloatNum,
    friction: FloatNum,
    reduction_reason: ContactReductionReason,
    support_friction_impulse: FloatNum,
    restitution_bias: FloatNum,
    position_bias: FloatNum,
    initial_normal_speed: FloatNum,
    initial_tangent_speed: FloatNum,
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
    normal_pair_partners: Vec<Option<usize>>,
}

#[derive(Default)]
struct PositionCorrectionStats {
    input_contact_count: usize,
    input_max_depth: FloatNum,
    input_total_depth: FloatNum,
    corrected_body_count: usize,
    max_translation: FloatNum,
    total_translation: FloatNum,
}

#[derive(Clone, Copy, Debug)]
struct ContactPositionRow {
    contact_index: usize,
    body_a: BodyHandle,
    body_b: BodyHandle,
    normal: Vector,
    inverse_mass_a: FloatNum,
    inverse_mass_b: FloatNum,
    frame_start_depth: FloatNum,
    raw_depth: FloatNum,
}

#[derive(Clone, Copy, Debug, Default)]
struct QueuedPositionTranslation {
    translation: Vector,
    distance_sum: FloatNum,
    reset_idle: bool,
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

    let (mut batches, mut stats) =
        contact_solver_row_batches(world, contacts, &islands, plan, config);

    for batch in &mut batches {
        let mut solver_bodies = solver_body_cache(world, &batch.body_slots);
        for row in &batch.rows {
            let warm_start_impulse =
                row.normal * row.normal_impulse + row.tangent * row.tangent_impulse;
            apply_solver_impulse(&mut solver_bodies, row, warm_start_impulse);
        }

        for _ in 0..config.velocity_iterations {
            for row_index in 0..batch.rows.len() {
                match batch.normal_pair_partners[row_index] {
                    Some(partner_index) if partner_index > row_index => {
                        if !solve_normal_impulse_pair(
                            &mut solver_bodies,
                            &mut batch.rows,
                            row_index,
                            partner_index,
                        ) {
                            solve_normal_impulse(&mut solver_bodies, &mut batch.rows[row_index]);
                            solve_normal_impulse(
                                &mut solver_bodies,
                                &mut batch.rows[partner_index],
                            );
                        }
                    }
                    Some(_) => {}
                    None => solve_normal_impulse(&mut solver_bodies, &mut batch.rows[row_index]),
                }
                solve_tangent_impulse(&mut solver_bodies, &mut batch.rows[row_index]);
            }
        }

        for row in &batch.rows {
            let contact = &mut contacts[row.contact_index];
            contact.normal_impulse = row.normal_impulse.max(0.0);
            contact.tangent_impulse = row.tangent_impulse;
            contact.solver_initial_normal_speed = row.initial_normal_speed;
            contact.solver_initial_tangent_speed = row.initial_tangent_speed;
            let (final_normal_speed, final_tangent_speed) =
                contact_row_relative_speeds(&solver_bodies, row)
                    .unwrap_or((row.initial_normal_speed, row.initial_tangent_speed));
            contact.solver_final_normal_speed = final_normal_speed;
            contact.solver_final_tangent_speed = final_tangent_speed;
            contact.solver_position_bias = row.position_bias;
            contact.solver_restitution_bias = row.restitution_bias;
            contact.solver_support_friction_impulse = row.support_friction_impulse;
            contact.solver_normal_impulse_delta =
                contact.normal_impulse - contact.warm_start_normal_impulse;
            contact.solver_tangent_impulse_delta =
                contact.tangent_impulse - contact.warm_start_tangent_impulse;
            contact.normal_impulse_clamped = row.normal_impulse_clamped;
            contact.tangent_impulse_clamped = row.tangent_impulse_clamped;
            contact.restitution_velocity_threshold = row.restitution_velocity_threshold;
            contact.restitution_applied = row.restitution_applied;
        }
        record_contact_impulse_wakes_for_rows(world, contacts, &batch.rows, wake_reasons);
        write_solver_velocities(world, &batch.body_slots, &solver_bodies, wake_reasons);
    }

    let position_correction_stats = apply_residual_contact_position_correction(
        world,
        contacts,
        config.position_iterations,
        wake_reasons,
    );
    stats.position_correction_input_contact_count = position_correction_stats.input_contact_count;
    stats.position_correction_input_max_depth = position_correction_stats.input_max_depth;
    stats.position_correction_input_total_depth = position_correction_stats.input_total_depth;
    stats.position_correction_body_count = position_correction_stats.corrected_body_count;
    stats.position_correction_max_translation = position_correction_stats.max_translation;
    stats.position_correction_total_translation = position_correction_stats.total_translation;
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
            let dense_contact_graph =
                island.contact_rows.len() >= DENSE_POSITION_CORRECTION_CONTACT_THRESHOLD;
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
                        dense_contact_graph,
                    )
                })
                .collect::<Vec<_>>();
            let normal_pair_partners = manifold_normal_pair_partners(&rows, dense_contact_graph);
            (!rows.is_empty()).then_some(ContactSolveBatch {
                body_slots: island.body_slots,
                rows,
                normal_pair_partners,
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
        ..island::SolverStepStats::default()
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
    dense_contact_graph: bool,
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

    let initial_velocity = relative_contact_velocity(body_a, body_b, anchor_a, anchor_b);
    let relative_normal_speed = initial_velocity.dot(normal);
    let relative_tangent_speed = initial_velocity.dot(tangent);
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
    let support_friction_impulse = if dense_contact_graph
        && contact.depth > 0.0
        && contact.depth <= POSITION_CORRECTION_SLOP
        && relative_normal_speed >= 0.0
    {
        (contact.depth * SHALLOW_SUPPORT_FRICTION_BUDGET_SCALE / config.dt) * normal_mass
    } else {
        0.0
    };
    let normal_impulse = contact.warm_start_normal_impulse.max(0.0);
    let max_friction = friction * normal_impulse.max(support_friction_impulse);
    let tangent_impulse = contact
        .warm_start_tangent_impulse
        .clamp(-max_friction, max_friction);

    Some(ContactSolverRow {
        contact_index,
        pair_key: contact.pair_key,
        body_a_slot,
        body_b_slot,
        normal,
        tangent,
        anchor_a,
        anchor_b,
        normal_mass,
        tangent_mass,
        friction,
        reduction_reason: contact.reduction_reason,
        support_friction_impulse,
        restitution_bias,
        position_bias,
        initial_normal_speed: relative_normal_speed,
        initial_tangent_speed: relative_tangent_speed,
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
    contacts: &mut [ContactObservation],
    iterations: u16,
    wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
) -> PositionCorrectionStats {
    let mut stats = PositionCorrectionStats::default();
    if iterations == 0 {
        return stats;
    }

    let eligible_contact_count = contacts
        .iter()
        .filter(|contact| !contact.is_sensor && residual_correction_depth(contact) > 0.0)
        .count();
    let dense_correction_mode =
        eligible_contact_count >= DENSE_POSITION_CORRECTION_CONTACT_THRESHOLD;

    let mut queued_translations = BTreeMap::<BodyHandle, QueuedPositionTranslation>::new();
    // Dense stacks reuse frame-start contact depths across many overlapping
    // rows. Track the pseudo-translation accumulated in this correction phase
    // so later rows see less stale penetration. The writeback is deferred until
    // all rows have accumulated into a pseudo-position state, which is the seam
    // needed for the later Box2D-style position row pass.
    let mut body_translation_vectors = BTreeMap::<BodyHandle, Vector>::new();
    let rows = contact_position_rows(world, contacts);
    for iteration in 0..iterations {
        for row in &rows {
            if iteration == 0 {
                stats.input_contact_count += 1;
                stats.input_max_depth = stats.input_max_depth.max(row.raw_depth.max(0.0));
                stats.input_total_depth += row.raw_depth.max(0.0);
            }
            let depth = if dense_correction_mode {
                let translation_a = body_translation_vectors
                    .get(&row.body_a)
                    .copied()
                    .unwrap_or_default();
                let translation_b = body_translation_vectors
                    .get(&row.body_b)
                    .copied()
                    .unwrap_or_default();
                ((row.raw_depth - (translation_a - translation_b).dot(row.normal)).max(0.0)
                    - POSITION_CORRECTION_SLOP)
                    .max(0.0)
            } else {
                row.frame_start_depth
            };
            if depth <= FloatNum::EPSILON {
                continue;
            }
            let contact = &mut contacts[row.contact_index];
            contact.solver_position_correction_depth += depth;
            let correction = depth * POSITION_CORRECTION_PERCENT / FloatNum::from(iterations);
            let inv_mass_sum = row.inverse_mass_a + row.inverse_mass_b;
            let correction_a = row.normal * (correction * row.inverse_mass_a / inv_mass_sum);
            let correction_b = -row.normal * (correction * row.inverse_mass_b / inv_mass_sum);
            let wake_a =
                contact_counterpart_can_wake_with_queued(world, row.body_b, &queued_translations);
            let wake_b =
                contact_counterpart_can_wake_with_queued(world, row.body_a, &queued_translations);
            if let Some(distance) = queue_position_translation(
                world,
                &mut queued_translations,
                row.body_a,
                correction_a,
                wake_a,
            ) {
                contact.solver_position_correction_body_a_translation += distance;
                if dense_correction_mode {
                    *body_translation_vectors.entry(row.body_a).or_default() += correction_a;
                }
            }
            if let Some(distance) = queue_position_translation(
                world,
                &mut queued_translations,
                row.body_b,
                correction_b,
                wake_b,
            ) {
                contact.solver_position_correction_body_b_translation += distance;
                if dense_correction_mode {
                    *body_translation_vectors.entry(row.body_b).or_default() += correction_b;
                }
            }
        }
    }
    stats.corrected_body_count = queued_translations.len();
    stats.max_translation = queued_translations
        .values()
        .map(|translation| translation.distance_sum)
        .fold(0.0, FloatNum::max);
    stats.total_translation = queued_translations
        .values()
        .map(|translation| translation.distance_sum)
        .sum();
    apply_queued_position_translations(world, queued_translations, wake_reasons);
    stats
}

fn contact_position_rows(
    world: &World,
    contacts: &[ContactObservation],
) -> Vec<ContactPositionRow> {
    contacts
        .iter()
        .enumerate()
        .filter_map(|(contact_index, contact)| contact_position_row(world, contact_index, contact))
        .collect()
}

fn contact_position_row(
    world: &World,
    contact_index: usize,
    contact: &ContactObservation,
) -> Option<ContactPositionRow> {
    if contact.is_sensor {
        return None;
    }
    let normal = contact.normal.normalized_or_zero();
    let frame_start_depth = residual_correction_depth(contact);
    if normal.length() <= FloatNum::EPSILON || frame_start_depth <= FloatNum::EPSILON {
        return None;
    }
    let inverse_mass_a = world
        .body_record(contact.body_a)
        .map(|record| record.mass_properties.inverse_mass)
        .unwrap_or(0.0);
    let inverse_mass_b = world
        .body_record(contact.body_b)
        .map(|record| record.mass_properties.inverse_mass)
        .unwrap_or(0.0);
    if inverse_mass_a + inverse_mass_b <= FloatNum::EPSILON {
        return None;
    }
    Some(ContactPositionRow {
        contact_index,
        body_a: contact.body_a,
        body_b: contact.body_b,
        normal,
        inverse_mass_a,
        inverse_mass_b,
        frame_start_depth,
        raw_depth: contact.depth,
    })
}

fn residual_correction_depth(contact: &ContactObservation) -> FloatNum {
    (contact.depth - POSITION_CORRECTION_SLOP).max(0.0)
}

fn contact_counterpart_can_wake_with_queued(
    world: &World,
    other: BodyHandle,
    queued_translations: &BTreeMap<BodyHandle, QueuedPositionTranslation>,
) -> bool {
    if queued_translations.contains_key(&other) {
        return true;
    }
    contact_counterpart_can_wake(world, other)
}

fn queue_position_translation(
    world: &World,
    queued_translations: &mut BTreeMap<BodyHandle, QueuedPositionTranslation>,
    body: BodyHandle,
    translation: Vector,
    wake_sleeping_body: bool,
) -> Option<FloatNum> {
    if translation.length() <= FloatNum::EPSILON {
        return None;
    }
    let Ok(record) = world.body_record(body) else {
        return None;
    };
    if !record.body_type.is_dynamic() {
        return None;
    }
    let already_queued = queued_translations.contains_key(&body);
    if record.sleeping && !wake_sleeping_body && !already_queued {
        return None;
    }
    let distance = translation.length();
    let queued = queued_translations.entry(body).or_default();
    queued.translation += translation;
    queued.distance_sum += distance;
    queued.reset_idle |= record.sleeping || distance > POSITION_CORRECTION_SLEEP_RESET_TRANSLATION;
    Some(distance)
}

fn apply_queued_position_translations(
    world: &mut World,
    queued_translations: BTreeMap<BodyHandle, QueuedPositionTranslation>,
    wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
) {
    for (body, queued) in queued_translations {
        if queued.translation.length() <= FloatNum::EPSILON {
            continue;
        }
        let Ok(record) = world.body_record_mut(body) else {
            continue;
        };
        if !record.body_type.is_dynamic() {
            continue;
        }
        let was_sleeping = record.sleeping;
        crate::solver::body_state::translate_pose(&mut record.pose, queued.translation, 0.0);
        record.sleeping = false;
        if queued.reset_idle {
            record.sleep_idle_time = 0.0;
        }
        if was_sleeping {
            sleep::record_wake_reason(wake_reasons, body, SleepTransitionReason::ContactImpulse);
        }
    }
}

fn manifold_normal_pair_partners(
    rows: &[ContactSolverRow],
    dense_contact_graph: bool,
) -> Vec<Option<usize>> {
    let mut partners = vec![None; rows.len()];
    if dense_contact_graph {
        return partners;
    }

    let mut by_pair = BTreeMap::<ContactPairKey, Vec<usize>>::new();
    for (index, row) in rows.iter().enumerate() {
        if !matches!(
            row.reduction_reason,
            ContactReductionReason::Clipped | ContactReductionReason::DuplicateReduced
        ) {
            continue;
        }
        by_pair.entry(row.pair_key).or_default().push(index);
    }

    for indices in by_pair.values() {
        if indices.len() != 2 {
            continue;
        }
        let first = indices[0];
        let second = indices[1];
        let row_a = &rows[first];
        let row_b = &rows[second];
        let same_solver_pair = row_a.body_a_slot == row_b.body_a_slot
            && row_a.body_b_slot == row_b.body_b_slot
            && row_a.normal.dot(row_b.normal) > 0.999;
        let sliding_velocity_pair = row_a.position_bias <= FloatNum::EPSILON
            && row_b.position_bias <= FloatNum::EPSILON
            && row_a
                .initial_tangent_speed
                .abs()
                .max(row_b.initial_tangent_speed.abs())
                >= MANIFOLD_BLOCK_SOLVE_TANGENT_SPEED_THRESHOLD;
        if same_solver_pair && sliding_velocity_pair {
            partners[first] = Some(second);
            partners[second] = Some(first);
        }
    }

    partners
}

fn solve_normal_impulse_pair(
    bodies: &mut [SolverBody],
    rows: &mut [ContactSolverRow],
    first_index: usize,
    second_index: usize,
) -> bool {
    debug_assert!(first_index < second_index);
    let (head, tail) = rows.split_at_mut(second_index);
    let first = &mut head[first_index];
    let second = &mut tail[0];
    solve_normal_impulse_pair_rows(bodies, first, second)
}

fn solve_normal_impulse_pair_rows(
    bodies: &mut [SolverBody],
    first: &mut ContactSolverRow,
    second: &mut ContactSolverRow,
) -> bool {
    if first.normal_mass <= 0.0 || second.normal_mass <= 0.0 {
        return false;
    }
    if first.body_a_slot != second.body_a_slot || first.body_b_slot != second.body_b_slot {
        return false;
    }
    let Some((body_a, body_b)) = solver_pair(bodies, first) else {
        return false;
    };

    let k11 = normal_effective_denominator(body_a, body_b, first, first);
    let k22 = normal_effective_denominator(body_a, body_b, second, second);
    let k12 = normal_effective_denominator(body_a, body_b, first, second);
    let determinant = k11 * k22 - k12 * k12;
    if !determinant.is_finite() || determinant <= FloatNum::EPSILON {
        return false;
    }

    let vn1 =
        relative_contact_velocity(body_a, body_b, first.anchor_a, first.anchor_b).dot(first.normal);
    let vn2 = relative_contact_velocity(body_a, body_b, second.anchor_a, second.anchor_b)
        .dot(second.normal);
    let bias1 = first.restitution_bias + first.position_bias;
    let bias2 = second.restitution_bias + second.position_bias;
    let old1 = first.normal_impulse;
    let old2 = second.normal_impulse;
    let rhs1 = bias1 - vn1 + k11 * old1 + k12 * old2;
    let rhs2 = bias2 - vn2 + k12 * old1 + k22 * old2;

    let inv_det = 1.0 / determinant;
    let x1 = inv_det * (k22 * rhs1 - k12 * rhs2);
    let x2 = inv_det * (k11 * rhs2 - k12 * rhs1);
    if x1 >= 0.0 && x2 >= 0.0 {
        apply_normal_pair_solution(bodies, first, second, x1, x2);
        return true;
    }

    let x1 = 0.0;
    let x2 = rhs2 / k22;
    if x2 >= 0.0 {
        let vn1_after_bias = vn1 - bias1 + k11 * (x1 - old1) + k12 * (x2 - old2);
        if vn1_after_bias >= -1.0e-5 {
            apply_normal_pair_solution(bodies, first, second, x1, x2);
            first.normal_impulse_clamped = true;
            return true;
        }
    }

    let x1 = rhs1 / k11;
    let x2 = 0.0;
    if x1 >= 0.0 {
        let vn2_after_bias = vn2 - bias2 + k12 * (x1 - old1) + k22 * (x2 - old2);
        if vn2_after_bias >= -1.0e-5 {
            apply_normal_pair_solution(bodies, first, second, x1, x2);
            second.normal_impulse_clamped = true;
            return true;
        }
    }

    let x1 = 0.0;
    let x2 = 0.0;
    let vn1_after_bias = vn1 - bias1 + k11 * (x1 - old1) + k12 * (x2 - old2);
    let vn2_after_bias = vn2 - bias2 + k12 * (x1 - old1) + k22 * (x2 - old2);
    if vn1_after_bias >= -1.0e-5 && vn2_after_bias >= -1.0e-5 {
        apply_normal_pair_solution(bodies, first, second, x1, x2);
        first.normal_impulse_clamped = true;
        second.normal_impulse_clamped = true;
        return true;
    }

    false
}

fn normal_effective_denominator(
    body_a: SolverBody,
    body_b: SolverBody,
    first: &ContactSolverRow,
    second: &ContactSolverRow,
) -> FloatNum {
    body_a.inverse_mass
        + body_b.inverse_mass
        + body_a.inverse_inertia
            * first.anchor_a.cross(first.normal)
            * second.anchor_a.cross(second.normal)
        + body_b.inverse_inertia
            * first.anchor_b.cross(first.normal)
            * second.anchor_b.cross(second.normal)
}

fn apply_normal_pair_solution(
    bodies: &mut [SolverBody],
    first: &mut ContactSolverRow,
    second: &mut ContactSolverRow,
    impulse1: FloatNum,
    impulse2: FloatNum,
) {
    let delta1 = impulse1 - first.normal_impulse;
    let delta2 = impulse2 - second.normal_impulse;
    first.normal_impulse = impulse1.max(0.0);
    second.normal_impulse = impulse2.max(0.0);
    apply_solver_impulse(bodies, first, first.normal * delta1);
    apply_solver_impulse(bodies, second, second.normal * delta2);
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
    let max_friction = row.friction * row.normal_impulse.max(row.support_friction_impulse);
    row.tangent_impulse = candidate.clamp(-max_friction, max_friction);
    row.tangent_impulse_clamped |= (row.tangent_impulse - candidate).abs() > FloatNum::EPSILON;
    let delta = row.tangent_impulse - previous;
    apply_solver_impulse(bodies, row, row.tangent * delta);
}

fn solver_pair(bodies: &[SolverBody], row: &ContactSolverRow) -> Option<(SolverBody, SolverBody)> {
    Some((*bodies.get(row.body_a_slot)?, *bodies.get(row.body_b_slot)?))
}

fn contact_row_relative_speeds(
    bodies: &[SolverBody],
    row: &ContactSolverRow,
) -> Option<(FloatNum, FloatNum)> {
    let (body_a, body_b) = solver_pair(bodies, row)?;
    let relative = relative_contact_velocity(body_a, body_b, row.anchor_a, row.anchor_b);
    Some((relative.dot(row.normal), relative.dot(row.tangent)))
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
