use std::collections::BTreeMap;

use crate::{
    events::ContactReductionReason,
    events::SleepTransitionReason,
    handles::BodyHandle,
    math::{point::Point, vector::Vector, FloatNum},
    pipeline::{
        contacts::ContactObservation, island, sleep, ContactPositionCorrectionPolicy, StepConfig,
    },
    world::{contact_state::ContactPairKey, World},
};

const POSITION_CORRECTION_PERCENT: FloatNum = 0.8;
const POSITION_CORRECTION_SLOP: FloatNum = 0.005;
const DENSE_POSITION_CORRECTION_SLOP: FloatNum = 0.0025;
const POSITION_CORRECTION_SLEEP_RESET_TRANSLATION: FloatNum = POSITION_CORRECTION_SLOP;
const CONTACT_VELOCITY_BIAS: FloatNum = 0.5;
const SHALLOW_SUPPORT_FRICTION_BUDGET_SCALE: FloatNum = 0.6;
const SPARSE_STACK_BLOCK_SOLVE_TANGENT_SPEED_THRESHOLD: FloatNum = 0.02;
const SPARSE_STACK_BLOCK_SOLVE_BODY_SLOT_THRESHOLD: usize = 5;
const SPARSE_STACK_BLOCK_SOLVE_ROW_THRESHOLD: usize = 8;
// Dense contact graphs are the matrix-stack case: many overlapping rows can
// reuse stale frame-start depth and over-correct the same bodies. Keep the
// velocity-side dense damping scoped there.
const DENSE_CONTACT_GRAPH_CONTACT_THRESHOLD: usize = 16;
// Position-row correction is also useful for sparse vertical support chains:
// four-box stacks need pseudo-position depth re-evaluation even though they are
// not dense enough to justify the dense velocity damping path.
const POSITION_ROW_CORRECTION_CONTACT_THRESHOLD: usize = 1;
// Mature iterative solvers rely on warm-starting plus a small amount of
// resting-contact energy dissipation to keep dense stacks from converting
// correction noise into long-lived sliding/spinning motion. Keep this scoped to
// dense contact islands so ordinary sparse contacts retain the legacy response.
const DENSE_CONTACT_TANGENT_DAMPING_PER_SECOND: FloatNum = 20.0;
const DENSE_CONTACT_ANGULAR_DAMPING_PER_SECOND: FloatNum = 20.0;
const SHALLOW_SUPPORT_TANGENT_DAMPING_PER_SECOND: FloatNum = 8.0;
const SHALLOW_SUPPORT_ANGULAR_DAMPING_PER_SECOND: FloatNum = 8.0;
const SHALLOW_SUPPORT_MAX_SEPARATING_SPEED: FloatNum = 0.25;
const DENSE_POSITION_ANGULAR_CORRECTION_SCALE: FloatNum = 0.0;
// The red dense-stack lock asserts output work below input depth * 0.78. Keep
// the private dense budget slightly under that ceiling so rounding and row
// ordering cannot spend the whole assertion margin. The design doc's 0.76 was
// an illustrative value; the final 0.775 is the least aggressive limiter that
// still leaves matrix/aligned artifact gates as support-retention guards.
const DENSE_PRESSURE_TOTAL_TRANSLATION_BUDGET_RATIO: FloatNum = 0.775;
// Very small tail fragments are skipped instead of creating near-zero pseudo
// deltas. This only discards the final sliver once the frame budget is spent;
// larger partial rows still use one shared scale for queued and pseudo motion.
const DENSE_PRESSURE_MIN_PARTIAL_CORRECTION_SCALE: FloatNum = 0.05;
// E5 reuses the artifact dry-run motion threshold so the residual-correction
// gate only suppresses source rows that are already acting like high-motion
// pressure handoffs rather than quiet resting support.
const SOURCE_ROW_PRESSURE_GATE_MOTION_THRESHOLD: FloatNum = 0.05;
const HIGH_RESTITUTION_POSITION_CORRECTION_THRESHOLD: FloatNum = 0.9;

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
    speculative_velocity_target: FloatNum,
    restitution_candidate_bias: FloatNum,
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
    base_normal_solve_executed: bool,
    resting_shallow_support: bool,
}

struct ContactSolveBatch {
    body_slots: Vec<BodyHandle>,
    // Solver body cache captured while the rows were built. Islands partition
    // dynamic bodies disjointly, so no other batch mutates these bodies before
    // this batch solves; reusing the snapshot avoids a redundant rebuild.
    solver_bodies: Vec<SolverBody>,
    rows: Vec<ContactSolverRow>,
    normal_pair_partners: Vec<Option<usize>>,
    dense_contact_graph: bool,
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
    inverse_inertia_a: FloatNum,
    inverse_inertia_b: FloatNum,
    frame_start_depth: FloatNum,
    raw_depth: FloatNum,
    source_row_continuity_candidate: bool,
    normal_impulse: FloatNum,
    solver_position_bias: FloatNum,
    solver_support_friction_impulse: FloatNum,
    anchor_a: Vector,
    anchor_b: Vector,
    body_a_linear_velocity: Vector,
    body_b_linear_velocity: Vector,
    body_a_angular_velocity: FloatNum,
    body_b_angular_velocity: FloatNum,
}

#[derive(Clone, Copy, Debug, Default)]
struct PositionSolveBody {
    // Pseudo-position is a solver-local shadow state: it tracks the temporary
    // pose delta accumulated during the dense position pass so later rows can
    // re-evaluate penetration without mutating the real world pose.
    pseudo_translation: Vector,
    pseudo_rotation: FloatNum,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PositionRowCorrection {
    correction_a: Vector,
    correction_b: Vector,
    rotation_a: FloatNum,
    rotation_b: FloatNum,
}

impl PositionRowCorrection {
    fn scaled(self, scale: FloatNum) -> Self {
        let scale = finite_unit_scale(scale);
        Self {
            correction_a: self.correction_a * scale,
            correction_b: self.correction_b * scale,
            rotation_a: self.rotation_a * scale,
            rotation_b: self.rotation_b * scale,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct QueuedPositionTranslation {
    translation: Vector,
    rotation: FloatNum,
    distance_sum: FloatNum,
    reset_idle: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum PositionRowDecision {
    Correct { scale: FloatNum },
    PressureLimited { scale: FloatNum },
    BudgetExhausted,
    SkipResolved,
    SkipSourcePressureGate,
}

struct DensePressureBudget {
    total_translation_limit: FloatNum,
    consumed_translation: FloatNum,
    min_partial_correction_scale: FloatNum,
}

impl DensePressureBudget {
    fn new(input_total_depth: FloatNum) -> Self {
        let total_translation_limit = if input_total_depth.is_finite() && input_total_depth > 0.0 {
            input_total_depth * DENSE_PRESSURE_TOTAL_TRANSLATION_BUDGET_RATIO
        } else {
            0.0
        };

        Self {
            total_translation_limit,
            consumed_translation: 0.0,
            min_partial_correction_scale: DENSE_PRESSURE_MIN_PARTIAL_CORRECTION_SCALE,
        }
    }

    fn row_decision(
        &self,
        row: &ContactPositionRow,
        depth: FloatNum,
        correction: Option<PositionRowCorrection>,
        candidate_translation_work: FloatNum,
    ) -> PositionRowDecision {
        if dense_contact_position_correction_gate(row, depth) {
            return PositionRowDecision::SkipSourcePressureGate;
        }
        let Some(_) = correction else {
            return PositionRowDecision::SkipResolved;
        };
        if !candidate_translation_work.is_finite()
            || candidate_translation_work <= FloatNum::EPSILON
        {
            return PositionRowDecision::SkipResolved;
        }
        if !self.total_translation_limit.is_finite() || self.total_translation_limit <= 0.0 {
            return PositionRowDecision::BudgetExhausted;
        }

        let remaining = self.total_translation_limit - self.consumed_translation;
        if !remaining.is_finite() || remaining <= FloatNum::EPSILON {
            return PositionRowDecision::BudgetExhausted;
        }
        if candidate_translation_work <= remaining {
            return PositionRowDecision::Correct { scale: 1.0 };
        }

        let scale = finite_unit_scale(remaining / candidate_translation_work);
        if scale < self.min_partial_correction_scale {
            PositionRowDecision::BudgetExhausted
        } else {
            PositionRowDecision::PressureLimited { scale }
        }
    }

    fn consume(&mut self, translation_work: FloatNum) {
        if translation_work.is_finite() && translation_work > 0.0 {
            self.consumed_translation =
                (self.consumed_translation + translation_work).min(self.total_translation_limit);
        }
    }
}

struct DensePositionSolveContext {
    dense_correction_mode: bool,
    iterations: u16,
    rows: Vec<ContactPositionRow>,
    pressure_budget: Option<DensePressureBudget>,
    queued_translations: BTreeMap<BodyHandle, QueuedPositionTranslation>,
    // Pseudo-position belongs to this residual position pass only. It lets
    // dense rows re-read already-consumed overlap without mutating world poses
    // before all queued corrections are ready to apply.
    position_solve_bodies: BTreeMap<BodyHandle, PositionSolveBody>,
    stats: PositionCorrectionStats,
}

impl DensePositionSolveContext {
    fn new(
        world: &World,
        contacts: &[ContactObservation],
        iterations: u16,
        policy: ContactPositionCorrectionPolicy,
    ) -> Self {
        let eligible_contact_count = contacts
            .iter()
            .filter(|contact| {
                !contact.is_sensor
                    && contact_position_correction_policy_allows(
                        policy,
                        contact.material.restitution,
                    )
                    && residual_correction_depth(contact) > 0.0
            })
            .count();
        let dense_correction_mode =
            eligible_contact_count >= POSITION_ROW_CORRECTION_CONTACT_THRESHOLD;
        let rows = contact_position_rows(world, contacts, dense_correction_mode, policy);
        let pressure_budget = dense_correction_mode.then(|| {
            DensePressureBudget::new(
                rows.iter()
                    .map(|row| row.raw_depth.max(0.0))
                    .sum::<FloatNum>(),
            )
        });

        Self {
            dense_correction_mode,
            iterations,
            rows,
            pressure_budget,
            queued_translations: BTreeMap::new(),
            position_solve_bodies: BTreeMap::new(),
            stats: PositionCorrectionStats::default(),
        }
    }

    fn solve(&mut self, world: &World, contacts: &mut [ContactObservation]) {
        for iteration in 0..self.iterations {
            for row_index in 0..self.rows.len() {
                let row = self.rows[row_index];
                self.solve_row(world, contacts, row, iteration);
            }
        }
    }

    fn into_queued_translations(
        mut self,
    ) -> (
        PositionCorrectionStats,
        BTreeMap<BodyHandle, QueuedPositionTranslation>,
    ) {
        self.stats.corrected_body_count = self.queued_translations.len();
        self.stats.max_translation = self
            .queued_translations
            .values()
            .map(|translation| translation.distance_sum)
            .fold(0.0, FloatNum::max);
        self.stats.total_translation = self
            .queued_translations
            .values()
            .map(|translation| translation.distance_sum)
            .sum();
        (self.stats, self.queued_translations)
    }

    fn solve_row(
        &mut self,
        world: &World,
        contacts: &mut [ContactObservation],
        row: ContactPositionRow,
        iteration: u16,
    ) {
        if iteration == 0 {
            self.stats.input_contact_count += 1;
            self.stats.input_max_depth = self.stats.input_max_depth.max(row.raw_depth.max(0.0));
            self.stats.input_total_depth += row.raw_depth.max(0.0);
        }

        let depth = self.row_depth(&row);
        let Some(correction) = self.decide_row_correction(world, &row, depth) else {
            return;
        };

        let contact = &mut contacts[row.contact_index];
        contact.solver_position_correction_depth += depth;
        let applied_a = self.queue_body_correction(
            world,
            contact,
            row.body_a,
            row.body_b,
            correction.correction_a,
            correction.rotation_a,
            true,
        );
        let applied_b = self.queue_body_correction(
            world,
            contact,
            row.body_b,
            row.body_a,
            correction.correction_b,
            correction.rotation_b,
            false,
        );
        if let Some(budget) = &mut self.pressure_budget {
            budget.consume(applied_a + applied_b);
        }
    }

    fn row_depth(&self, row: &ContactPositionRow) -> FloatNum {
        if self.dense_correction_mode {
            dense_position_row_depth(row, &self.position_solve_bodies)
        } else {
            row.frame_start_depth
        }
    }

    fn decide_row_correction(
        &mut self,
        world: &World,
        row: &ContactPositionRow,
        depth: FloatNum,
    ) -> Option<PositionRowCorrection> {
        let correction =
            position_row_correction(row, depth, self.iterations, self.dense_correction_mode);
        if !self.dense_correction_mode {
            return correction;
        }
        let candidate_work = correction
            .map(|candidate| position_row_translation_work(world, row, candidate))
            .unwrap_or(0.0);
        let decision = self
            .pressure_budget
            .as_ref()
            .map(|budget| budget.row_decision(row, depth, correction, candidate_work))
            .unwrap_or(PositionRowDecision::Correct { scale: 1.0 });

        match decision {
            PositionRowDecision::Correct { scale }
            | PositionRowDecision::PressureLimited { scale } => {
                correction.map(|candidate| candidate.scaled(scale))
            }
            PositionRowDecision::BudgetExhausted
            | PositionRowDecision::SkipResolved
            | PositionRowDecision::SkipSourcePressureGate => None,
        }
    }

    // Cohesive per-body position-correction inputs threaded with solver context;
    // bundling them would obscure the single-body correction call sites.
    #[allow(clippy::too_many_arguments)]
    fn queue_body_correction(
        &mut self,
        world: &World,
        contact: &mut ContactObservation,
        body: BodyHandle,
        counterpart: BodyHandle,
        translation: Vector,
        rotation: FloatNum,
        body_a: bool,
    ) -> FloatNum {
        let wake_sleeping_body =
            contact_counterpart_can_wake_with_queued(world, counterpart, &self.queued_translations);
        if let Some(distance) = queue_position_correction(
            world,
            &mut self.queued_translations,
            body,
            translation,
            rotation,
            wake_sleeping_body,
        ) {
            if body_a {
                contact.solver_position_correction_body_a_translation += distance;
            } else {
                contact.solver_position_correction_body_b_translation += distance;
            }
            if self.dense_correction_mode {
                record_shadow_position_delta(
                    &mut self.position_solve_bodies,
                    body,
                    translation,
                    rotation,
                );
            }
            distance
        } else {
            0.0
        }
    }
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
        contact.base_normal_solve_executed = false;
    }

    let (mut batches, mut stats) =
        contact_solver_row_batches(world, contacts, &islands, plan, config);

    for batch in &mut batches {
        let mut solver_bodies = std::mem::take(&mut batch.solver_bodies);
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
        // Restitution is intentionally a second pass: speculative rows must first
        // prove a positive non-penetration impulse before they are allowed to
        // convert the saved pre-solve approach speed into bounce.
        for row in &mut batch.rows {
            solve_restitution_impulse(&mut solver_bodies, row);
        }
        stabilize_resting_contact_graph(batch, &mut solver_bodies, config);

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
            contact.base_normal_solve_executed = row.base_normal_solve_executed;
        }
        record_contact_impulse_wakes_for_rows(world, contacts, &batch.rows, wake_reasons);
        write_solver_velocities(world, &batch.body_slots, &solver_bodies, wake_reasons);
    }

    let position_correction_stats =
        apply_residual_contact_position_correction(world, contacts, config, wake_reasons);
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
            let small_world_body_count =
                world.bodies().count() <= SPARSE_STACK_BLOCK_SOLVE_BODY_SLOT_THRESHOLD;
            let dense_contact_graph =
                island.contact_rows.len() >= DENSE_CONTACT_GRAPH_CONTACT_THRESHOLD;
            let sparse_stack_stabilization_candidate = !dense_contact_graph
                && small_world_body_count
                && island.body_slots.len() >= 3
                && island.body_slots.len() <= SPARSE_STACK_BLOCK_SOLVE_BODY_SLOT_THRESHOLD
                && island.contact_rows.len() >= 2
                && island.contact_rows.len() <= SPARSE_STACK_BLOCK_SOLVE_ROW_THRESHOLD;
            let mut rows = island
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
            let sparse_vertical_stack_graph =
                sparse_stack_stabilization_candidate && sparse_vertical_stack_support_graph(&rows);
            let stabilized_contact_graph = dense_contact_graph || sparse_vertical_stack_graph;
            if sparse_vertical_stack_graph {
                rows = island
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
                            stabilized_contact_graph,
                        )
                    })
                    .collect::<Vec<_>>();
            }
            let sparse_stack_block_solve_guard = !stabilized_contact_graph
                && small_world_body_count
                && island.body_slots.len() <= SPARSE_STACK_BLOCK_SOLVE_BODY_SLOT_THRESHOLD
                && rows.len() <= SPARSE_STACK_BLOCK_SOLVE_ROW_THRESHOLD;
            let normal_pair_partners =
                manifold_normal_pair_partners(&rows, sparse_stack_block_solve_guard);
            (!rows.is_empty()).then_some(ContactSolveBatch {
                body_slots: island.body_slots,
                solver_bodies: bodies,
                rows,
                normal_pair_partners,
                dense_contact_graph: stabilized_contact_graph,
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

fn stabilize_resting_contact_graph(
    batch: &ContactSolveBatch,
    bodies: &mut [SolverBody],
    config: &StepConfig,
) {
    if config.dt <= 0.0 {
        return;
    }
    let shallow_support_graph = batch.rows.iter().any(|row| row.resting_shallow_support);
    if !batch.dense_contact_graph && !shallow_support_graph {
        return;
    }

    let tangent_rate = if batch.dense_contact_graph {
        DENSE_CONTACT_TANGENT_DAMPING_PER_SECOND
    } else {
        SHALLOW_SUPPORT_TANGENT_DAMPING_PER_SECOND
    };
    let angular_rate = if batch.dense_contact_graph {
        DENSE_CONTACT_ANGULAR_DAMPING_PER_SECOND
    } else {
        SHALLOW_SUPPORT_ANGULAR_DAMPING_PER_SECOND
    };
    let tangent_scale = damping_scale(tangent_rate, config.dt);
    let angular_scale = damping_scale(angular_rate, config.dt);
    if tangent_scale <= FloatNum::EPSILON && angular_scale <= FloatNum::EPSILON {
        return;
    }

    let mut tangent_accumulators = vec![Vector::default(); bodies.len()];
    let mut tangent_counts = vec![0usize; bodies.len()];
    for row in &batch.rows {
        if !batch.dense_contact_graph && !row.resting_shallow_support {
            continue;
        }
        accumulate_contact_tangent_damping(
            bodies,
            &mut tangent_accumulators,
            &mut tangent_counts,
            row.body_a_slot,
            row.tangent,
        );
        accumulate_contact_tangent_damping(
            bodies,
            &mut tangent_accumulators,
            &mut tangent_counts,
            row.body_b_slot,
            row.tangent,
        );
    }

    for (index, body) in bodies.iter_mut().enumerate() {
        if !body.dynamic {
            continue;
        }
        if let Some(count) = tangent_counts
            .get(index)
            .copied()
            .filter(|count| *count > 0)
        {
            if let Some(tangent_velocity) = tangent_accumulators.get(index).copied() {
                body.linear_velocity -= tangent_velocity * (tangent_scale / count as FloatNum);
            }
        }
        body.angular_velocity *= 1.0 - angular_scale;
    }
}

fn damping_scale(rate_per_second: FloatNum, dt: FloatNum) -> FloatNum {
    (rate_per_second * dt).clamp(0.0, 1.0)
}

fn accumulate_contact_tangent_damping(
    bodies: &[SolverBody],
    tangent_accumulators: &mut [Vector],
    tangent_counts: &mut [usize],
    slot: usize,
    tangent: Vector,
) {
    let Some(body) = bodies.get(slot) else {
        return;
    };
    if !body.dynamic || tangent.length() <= FloatNum::EPSILON {
        return;
    }
    if let Some(accumulator) = tangent_accumulators.get_mut(slot) {
        *accumulator += tangent * body.linear_velocity.dot(tangent);
    }
    if let Some(count) = tangent_counts.get_mut(slot) {
        *count += 1;
    }
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
    let restitution_candidate =
        -relative_normal_speed > restitution_threshold && contact.material.restitution > 0.0;
    let restitution_candidate_bias = if restitution_candidate {
        contact.material.restitution.clamp(0.0, 1.0) * -relative_normal_speed
    } else {
        0.0
    };
    let resting_shallow_support = is_resting_shallow_support_contact(
        body_a,
        body_b,
        contact.depth,
        relative_normal_speed,
        restitution_candidate,
        contact.material.friction,
    );
    // Penetration velocity bias gives resting overlap a support impulse during the
    // velocity solve, so Coulomb friction has a real normal budget to clamp against.
    let position_bias = if relative_normal_speed.abs() <= 1.0e-4 {
        (contact.depth - POSITION_CORRECTION_SLOP).max(0.0) * CONTACT_VELOCITY_BIAS / config.dt
    } else {
        0.0
    };
    let speculative_velocity_target = if config.dt > 0.0 && contact.signed_separation.is_finite() {
        -(contact.signed_separation - POSITION_CORRECTION_SLOP).max(0.0) / config.dt
    } else {
        0.0
    };
    let friction = contact.material.friction.max(0.0);
    let support_friction_impulse = if (dense_contact_graph || resting_shallow_support)
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
        speculative_velocity_target,
        restitution_candidate_bias,
        restitution_bias: 0.0,
        position_bias,
        initial_normal_speed: relative_normal_speed,
        initial_tangent_speed: relative_tangent_speed,
        normal_impulse,
        tangent_impulse,
        normal_impulse_clamped: false,
        tangent_impulse_clamped: (tangent_impulse - contact.warm_start_tangent_impulse).abs()
            > FloatNum::EPSILON,
        restitution_velocity_threshold: restitution_threshold,
        restitution_applied: false,
        base_normal_solve_executed: false,
        resting_shallow_support,
    })
}

fn sparse_vertical_stack_support_graph(rows: &[ContactSolverRow]) -> bool {
    rows.iter()
        .filter(|row| row.normal.y().abs() >= 0.9)
        .count()
        >= 2
}

fn is_resting_shallow_support_contact(
    body_a: SolverBody,
    body_b: SolverBody,
    depth: FloatNum,
    relative_normal_speed: FloatNum,
    restitution_applied: bool,
    friction: FloatNum,
) -> bool {
    (body_a.dynamic || body_b.dynamic)
        && depth > 0.0
        && (0.0..=SHALLOW_SUPPORT_MAX_SEPARATING_SPEED).contains(&relative_normal_speed)
        && !restitution_applied
        && friction > 0.0
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
    config: &StepConfig,
    wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
) -> PositionCorrectionStats {
    if config.position_iterations == 0
        || matches!(
            config.contact_position_correction,
            ContactPositionCorrectionPolicy::Disabled
        )
    {
        return PositionCorrectionStats::default();
    }

    let mut context = DensePositionSolveContext::new(
        world,
        contacts,
        config.position_iterations,
        config.contact_position_correction,
    );
    context.solve(world, contacts);
    let (stats, queued_translations) = context.into_queued_translations();
    apply_queued_position_translations(world, queued_translations, wake_reasons);
    stats
}

fn contact_position_rows(
    world: &World,
    contacts: &[ContactObservation],
    dense_correction_mode: bool,
    policy: ContactPositionCorrectionPolicy,
) -> Vec<ContactPositionRow> {
    contacts
        .iter()
        .enumerate()
        .filter_map(|(contact_index, contact)| {
            contact_position_row(world, contact_index, contact, dense_correction_mode, policy)
        })
        .collect()
}

fn contact_position_row(
    world: &World,
    contact_index: usize,
    contact: &ContactObservation,
    dense_correction_mode: bool,
    policy: ContactPositionCorrectionPolicy,
) -> Option<ContactPositionRow> {
    if contact.is_sensor {
        return None;
    }
    if !contact_position_correction_policy_allows(policy, contact.material.restitution) {
        return None;
    }
    let normal = contact.normal.normalized_or_zero();
    let correction_slop = if dense_correction_mode {
        DENSE_POSITION_CORRECTION_SLOP
    } else {
        POSITION_CORRECTION_SLOP
    };
    let frame_start_depth = residual_correction_depth_from_raw(contact.depth, correction_slop);
    if normal.length() <= FloatNum::EPSILON || frame_start_depth <= FloatNum::EPSILON {
        return None;
    }
    let record_a = world.body_record(contact.body_a).ok();
    let record_b = world.body_record(contact.body_b).ok();
    let inverse_mass_a = record_a
        .map(|record| record.mass_properties.inverse_mass)
        .unwrap_or(0.0);
    let inverse_mass_b = record_b
        .map(|record| record.mass_properties.inverse_mass)
        .unwrap_or(0.0);
    let inverse_inertia_a = record_a
        .map(|record| record.mass_properties.inverse_inertia)
        .unwrap_or(0.0);
    let inverse_inertia_b = record_b
        .map(|record| record.mass_properties.inverse_inertia)
        .unwrap_or(0.0);
    if inverse_mass_a + inverse_mass_b <= FloatNum::EPSILON {
        return None;
    }
    let center_a = record_a.map(|record| {
        record
            .pose
            .transform_point(record.mass_properties.local_center_of_mass)
    });
    let center_b = record_b.map(|record| {
        record
            .pose
            .transform_point(record.mass_properties.local_center_of_mass)
    });
    Some(ContactPositionRow {
        contact_index,
        body_a: contact.body_a,
        body_b: contact.body_b,
        normal,
        inverse_mass_a,
        inverse_mass_b,
        inverse_inertia_a,
        inverse_inertia_b,
        frame_start_depth,
        raw_depth: contact.depth,
        source_row_continuity_candidate: contact.source_row_continuity_candidate,
        normal_impulse: contact.normal_impulse.max(0.0),
        solver_position_bias: contact.solver_position_bias,
        solver_support_friction_impulse: contact.solver_support_friction_impulse,
        anchor_a: center_a
            .map(|center| contact.point - center)
            .unwrap_or_default(),
        anchor_b: center_b
            .map(|center| contact.point - center)
            .unwrap_or_default(),
        body_a_linear_velocity: record_a
            .map(|record| record.linear_velocity)
            .unwrap_or_default(),
        body_b_linear_velocity: record_b
            .map(|record| record.linear_velocity)
            .unwrap_or_default(),
        body_a_angular_velocity: record_a
            .map(|record| record.angular_velocity)
            .unwrap_or(0.0),
        body_b_angular_velocity: record_b
            .map(|record| record.angular_velocity)
            .unwrap_or(0.0),
    })
}

fn contact_position_correction_policy_allows(
    policy: ContactPositionCorrectionPolicy,
    restitution: FloatNum,
) -> bool {
    match policy {
        ContactPositionCorrectionPolicy::Enabled => true,
        ContactPositionCorrectionPolicy::Conservative => {
            !restitution.is_finite() || restitution < HIGH_RESTITUTION_POSITION_CORRECTION_THRESHOLD
        }
        ContactPositionCorrectionPolicy::Disabled => false,
    }
}

fn residual_correction_depth(contact: &ContactObservation) -> FloatNum {
    residual_correction_depth_from_raw(contact.depth, POSITION_CORRECTION_SLOP)
}

fn residual_correction_depth_from_raw(raw_depth: FloatNum, slop: FloatNum) -> FloatNum {
    (raw_depth - slop).max(0.0)
}

fn finite_unit_scale(scale: FloatNum) -> FloatNum {
    if !scale.is_finite() {
        return 0.0;
    }
    scale.clamp(0.0, 1.0)
}

fn dense_position_row_depth(
    row: &ContactPositionRow,
    position_solve_bodies: &BTreeMap<BodyHandle, PositionSolveBody>,
) -> FloatNum {
    let translation_a = position_solve_bodies
        .get(&row.body_a)
        .copied()
        .unwrap_or_default()
        .pseudo_translation;
    let translation_b = position_solve_bodies
        .get(&row.body_b)
        .copied()
        .unwrap_or_default()
        .pseudo_translation;
    let rotation_a = position_solve_bodies
        .get(&row.body_a)
        .copied()
        .unwrap_or_default()
        .pseudo_rotation;
    let rotation_b = position_solve_bodies
        .get(&row.body_b)
        .copied()
        .unwrap_or_default()
        .pseudo_rotation;
    let point_delta_a = translation_a + angular_point_velocity(rotation_a, row.anchor_a);
    let point_delta_b = translation_b + angular_point_velocity(rotation_b, row.anchor_b);
    residual_correction_depth_from_raw(
        (row.raw_depth - (point_delta_a - point_delta_b).dot(row.normal)).max(0.0),
        DENSE_POSITION_CORRECTION_SLOP,
    )
}

fn position_row_translation_work(
    world: &World,
    row: &ContactPositionRow,
    correction: PositionRowCorrection,
) -> FloatNum {
    let mut work = 0.0;
    if body_can_queue_position_correction(world, row.body_a) {
        work += correction.correction_a.length();
    }
    if body_can_queue_position_correction(world, row.body_b) {
        work += correction.correction_b.length();
    }
    if work.is_finite() {
        work
    } else {
        0.0
    }
}

fn body_can_queue_position_correction(world: &World, body: BodyHandle) -> bool {
    world
        .body_record(body)
        .map(|record| record.body_type.is_dynamic())
        .unwrap_or(false)
}

fn dense_contact_position_correction_gate(row: &ContactPositionRow, depth: FloatNum) -> bool {
    let tangent = row.normal.perp().normalized_or_zero();
    if tangent.length() <= FloatNum::EPSILON {
        return false;
    }

    let tangent_speed = source_row_relative_tangent_speed(row, tangent).abs();
    let counterpart_motion = source_row_counterpart_motion(row, tangent);

    if !row.source_row_continuity_candidate
        || depth <= FloatNum::EPSILON
        || row.solver_position_bias.abs() > FloatNum::EPSILON
        || row.solver_support_friction_impulse.abs() > FloatNum::EPSILON
        || depth <= row.normal_impulse.max(0.0)
    {
        return false;
    }
    counterpart_motion > SOURCE_ROW_PRESSURE_GATE_MOTION_THRESHOLD
        || tangent_speed > SOURCE_ROW_PRESSURE_GATE_MOTION_THRESHOLD
}

fn source_row_relative_tangent_speed(row: &ContactPositionRow, tangent: Vector) -> FloatNum {
    let body_a = SolverBody {
        dynamic: row.inverse_mass_a > FloatNum::EPSILON,
        inverse_mass: row.inverse_mass_a,
        inverse_inertia: 0.0,
        center: Point::default(),
        linear_velocity: row.body_a_linear_velocity,
        angular_velocity: row.body_a_angular_velocity,
    };
    let body_b = SolverBody {
        dynamic: row.inverse_mass_b > FloatNum::EPSILON,
        inverse_mass: row.inverse_mass_b,
        inverse_inertia: 0.0,
        center: Point::default(),
        linear_velocity: row.body_b_linear_velocity,
        angular_velocity: row.body_b_angular_velocity,
    };
    relative_contact_velocity(body_a, body_b, row.anchor_a, row.anchor_b).dot(tangent)
}

fn source_row_counterpart_motion(row: &ContactPositionRow, tangent: Vector) -> FloatNum {
    let body_a = source_row_body_motion(
        row.inverse_mass_a,
        row.body_a_linear_velocity,
        row.body_a_angular_velocity,
        row.anchor_a,
        tangent,
    );
    let body_b = source_row_body_motion(
        row.inverse_mass_b,
        row.body_b_linear_velocity,
        row.body_b_angular_velocity,
        row.anchor_b,
        tangent,
    );
    if body_a.max_component() >= body_b.max_component() {
        body_a.max_component()
    } else {
        body_b.max_component()
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct SourceRowBodyMotion {
    linear_speed: FloatNum,
    angular_speed: FloatNum,
    tangent_point_speed: FloatNum,
}

impl SourceRowBodyMotion {
    fn max_component(self) -> FloatNum {
        self.linear_speed
            .max(self.angular_speed)
            .max(self.tangent_point_speed)
    }
}

fn source_row_body_motion(
    inverse_mass: FloatNum,
    linear_velocity: Vector,
    angular_velocity: FloatNum,
    anchor: Vector,
    tangent: Vector,
) -> SourceRowBodyMotion {
    if inverse_mass <= FloatNum::EPSILON {
        return SourceRowBodyMotion::default();
    }
    SourceRowBodyMotion {
        linear_speed: linear_velocity.length(),
        angular_speed: angular_velocity.abs(),
        tangent_point_speed: contact_point_velocity(
            SolverBody {
                dynamic: true,
                inverse_mass,
                inverse_inertia: 0.0,
                center: Point::default(),
                linear_velocity,
                angular_velocity,
            },
            anchor,
        )
        .dot(tangent)
        .abs(),
    }
}

fn position_row_correction(
    row: &ContactPositionRow,
    depth: FloatNum,
    iterations: u16,
    angular_effective_mass: bool,
) -> Option<PositionRowCorrection> {
    if depth <= FloatNum::EPSILON || iterations == 0 {
        return None;
    }
    if !angular_effective_mass {
        let inv_mass_sum = row.inverse_mass_a + row.inverse_mass_b;
        if inv_mass_sum <= FloatNum::EPSILON {
            return None;
        }
        let correction = depth * POSITION_CORRECTION_PERCENT / FloatNum::from(iterations);
        return Some(PositionRowCorrection {
            correction_a: row.normal * (correction * row.inverse_mass_a / inv_mass_sum),
            correction_b: -row.normal * (correction * row.inverse_mass_b / inv_mass_sum),
            rotation_a: 0.0,
            rotation_b: 0.0,
        });
    }
    let anchor_a_cross = row.anchor_a.cross(row.normal);
    let anchor_b_cross = row.anchor_b.cross(row.normal);
    let denominator = row.inverse_mass_a
        + row.inverse_mass_b
        + DENSE_POSITION_ANGULAR_CORRECTION_SCALE
            * (row.inverse_inertia_a * anchor_a_cross * anchor_a_cross
                + row.inverse_inertia_b * anchor_b_cross * anchor_b_cross);
    if denominator <= FloatNum::EPSILON || !denominator.is_finite() {
        return None;
    }
    let correction = depth * POSITION_CORRECTION_PERCENT / FloatNum::from(iterations);
    let impulse = correction / denominator;
    let normal_impulse = row.normal * impulse;
    Some(PositionRowCorrection {
        correction_a: normal_impulse * row.inverse_mass_a,
        correction_b: -normal_impulse * row.inverse_mass_b,
        rotation_a: -row.anchor_a.cross(normal_impulse)
            * row.inverse_inertia_a
            * DENSE_POSITION_ANGULAR_CORRECTION_SCALE,
        rotation_b: row.anchor_b.cross(normal_impulse)
            * row.inverse_inertia_b
            * DENSE_POSITION_ANGULAR_CORRECTION_SCALE,
    })
}

fn record_shadow_position_delta(
    position_solve_bodies: &mut BTreeMap<BodyHandle, PositionSolveBody>,
    body: BodyHandle,
    translation: Vector,
    rotation: FloatNum,
) {
    if translation.length() <= FloatNum::EPSILON && rotation.abs() <= FloatNum::EPSILON {
        return;
    }
    let solve_body = position_solve_bodies.entry(body).or_default();
    solve_body.pseudo_translation += translation;
    solve_body.pseudo_rotation += rotation;
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

fn queue_position_correction(
    world: &World,
    queued_translations: &mut BTreeMap<BodyHandle, QueuedPositionTranslation>,
    body: BodyHandle,
    translation: Vector,
    rotation: FloatNum,
    wake_sleeping_body: bool,
) -> Option<FloatNum> {
    if translation.length() <= FloatNum::EPSILON && rotation.abs() <= FloatNum::EPSILON {
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
    queued.rotation += rotation;
    queued.distance_sum += distance;
    queued.reset_idle |= record.sleeping
        || distance > POSITION_CORRECTION_SLEEP_RESET_TRANSLATION
        || rotation.abs() > POSITION_CORRECTION_SLEEP_RESET_TRANSLATION;
    Some(distance)
}

fn apply_queued_position_translations(
    world: &mut World,
    queued_translations: BTreeMap<BodyHandle, QueuedPositionTranslation>,
    wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
) {
    for (body, queued) in queued_translations {
        if queued.translation.length() <= FloatNum::EPSILON
            && queued.rotation.abs() <= FloatNum::EPSILON
        {
            continue;
        }
        let Ok(record) = world.body_record_mut(body) else {
            continue;
        };
        if !record.body_type.is_dynamic() {
            continue;
        }
        let was_sleeping = record.sleeping;
        crate::solver::body_state::translate_pose(
            &mut record.pose,
            queued.translation,
            queued.rotation,
        );
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
    sparse_stack_block_solve_guard: bool,
) -> Vec<Option<usize>> {
    let mut partners = vec![None; rows.len()];

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
        let tangent_speed_threshold = if sparse_stack_block_solve_guard {
            SPARSE_STACK_BLOCK_SOLVE_TANGENT_SPEED_THRESHOLD
        } else {
            0.0
        };
        let stable_support_pair = !sparse_stack_block_solve_guard
            || (row_a.position_bias <= FloatNum::EPSILON
                && row_b.position_bias <= FloatNum::EPSILON);
        let sliding_velocity_pair = stable_support_pair
            && row_a
                .initial_tangent_speed
                .abs()
                .max(row_b.initial_tangent_speed.abs())
                >= tangent_speed_threshold;
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
    let bias1 = first.position_bias + first.speculative_velocity_target;
    let bias2 = second.position_bias + second.speculative_velocity_target;
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
    first.base_normal_solve_executed = true;
    second.base_normal_solve_executed = true;
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
    row.base_normal_solve_executed = true;
    let normal_speed =
        relative_contact_velocity(body_a, body_b, row.anchor_a, row.anchor_b).dot(row.normal);
    // Accumulated impulses are clamped, not per-iteration deltas. This lets a row
    // give impulse back on later iterations without ever pulling bodies together.
    let previous = row.normal_impulse;
    let velocity_target = row.position_bias + row.speculative_velocity_target;
    let candidate = previous - (normal_speed - velocity_target) * row.normal_mass;
    row.normal_impulse = candidate.max(0.0);
    row.normal_impulse_clamped |= candidate < 0.0;
    let delta = row.normal_impulse - previous;
    apply_solver_impulse(bodies, row, row.normal * delta);
}

fn solve_restitution_impulse(bodies: &mut [SolverBody], row: &mut ContactSolverRow) {
    if row.normal_mass <= 0.0
        || !row.base_normal_solve_executed
        || row.normal_impulse <= FloatNum::EPSILON
        || row.restitution_candidate_bias <= 0.0
    {
        return;
    }
    let Some((body_a, body_b)) = solver_pair(bodies, row) else {
        return;
    };
    let normal_speed =
        relative_contact_velocity(body_a, body_b, row.anchor_a, row.anchor_b).dot(row.normal);
    let previous = row.normal_impulse;
    let candidate = previous - (normal_speed - row.restitution_candidate_bias) * row.normal_mass;
    let next = candidate.max(previous);
    let delta = next - previous;
    if delta <= FloatNum::EPSILON {
        return;
    }
    row.normal_impulse = next;
    row.restitution_bias = row.restitution_candidate_bias;
    row.restitution_applied = true;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handles::ColliderHandle;

    fn body(index: u32) -> BodyHandle {
        BodyHandle::from_raw_parts(index, 0)
    }

    fn collider(index: u32) -> ColliderHandle {
        ColliderHandle::from_raw_parts(index, 0)
    }

    fn block_solve_pair() -> ContactPairKey {
        ContactPairKey::new(collider(0), collider(1))
    }

    fn block_solve_row(
        contact_index: usize,
        initial_tangent_speed: FloatNum,
        position_bias: FloatNum,
    ) -> ContactSolverRow {
        ContactSolverRow {
            contact_index,
            pair_key: block_solve_pair(),
            body_a_slot: 0,
            body_b_slot: 1,
            normal: Vector::new(0.0, 1.0),
            tangent: Vector::new(-1.0, 0.0),
            anchor_a: Vector::new(-0.25 + contact_index as FloatNum * 0.5, 0.5),
            anchor_b: Vector::new(-0.25 + contact_index as FloatNum * 0.5, -0.5),
            normal_mass: 1.0,
            tangent_mass: 1.0,
            friction: 0.2,
            reduction_reason: ContactReductionReason::Clipped,
            support_friction_impulse: 0.0,
            speculative_velocity_target: 0.0,
            restitution_candidate_bias: 0.0,
            restitution_bias: 0.0,
            position_bias,
            initial_normal_speed: 0.0,
            initial_tangent_speed,
            normal_impulse: 0.0,
            tangent_impulse: 0.0,
            normal_impulse_clamped: false,
            tangent_impulse_clamped: false,
            restitution_velocity_threshold: 1.0,
            restitution_applied: false,
            base_normal_solve_executed: false,
            resting_shallow_support: false,
        }
    }

    #[test]
    fn restitution_pass_requires_base_normal_solve_execution() {
        let mut row = block_solve_row(0, 0.0, 0.0);
        row.normal_impulse = 0.25;
        row.restitution_candidate_bias = 1.0;
        let mut bodies = vec![
            SolverBody {
                dynamic: true,
                inverse_mass: 1.0,
                inverse_inertia: 0.0,
                center: Point::default(),
                linear_velocity: Vector::default(),
                angular_velocity: 0.0,
            },
            SolverBody {
                dynamic: false,
                inverse_mass: 0.0,
                inverse_inertia: 0.0,
                center: Point::default(),
                linear_velocity: Vector::default(),
                angular_velocity: 0.0,
            },
        ];

        solve_restitution_impulse(&mut bodies, &mut row);

        assert_eq!(row.normal_impulse, 0.25);
        assert!(!row.restitution_applied);
        assert_eq!(row.restitution_bias, 0.0);
    }

    #[test]
    fn block_solve_skips_resting_two_point_manifold_without_sliding_tangent_speed() {
        let rows = vec![block_solve_row(0, 0.0, 0.0), block_solve_row(1, 0.0, 0.0)];

        let partners = manifold_normal_pair_partners(&rows, true);

        assert_eq!(partners, vec![None, None]);
    }

    #[test]
    fn block_solve_allows_guarded_stack_when_two_point_manifold_is_sliding() {
        let rows = vec![block_solve_row(0, 0.3, 0.0), block_solve_row(1, 0.3, 0.0)];

        let partners = manifold_normal_pair_partners(&rows, true);

        assert_eq!(partners, vec![Some(1), Some(0)]);
    }

    #[test]
    fn block_solve_allows_non_dense_sliding_two_point_manifold() {
        let rows = vec![block_solve_row(0, 0.3, 0.0), block_solve_row(1, 0.3, 0.0)];

        let partners = manifold_normal_pair_partners(&rows, false);

        assert_eq!(partners, vec![Some(1), Some(0)]);
    }

    #[test]
    fn conservative_position_policy_skips_high_restitution_contacts() {
        assert!(contact_position_correction_policy_allows(
            ContactPositionCorrectionPolicy::Enabled,
            1.0,
        ));
        assert!(!contact_position_correction_policy_allows(
            ContactPositionCorrectionPolicy::Conservative,
            1.0,
        ));
        assert!(contact_position_correction_policy_allows(
            ContactPositionCorrectionPolicy::Conservative,
            0.2,
        ));
        assert!(!contact_position_correction_policy_allows(
            ContactPositionCorrectionPolicy::Disabled,
            0.2,
        ));
    }

    #[test]
    fn block_solve_allows_slow_sliding_above_resting_noise_floor() {
        let rows = vec![block_solve_row(0, 0.02, 0.0), block_solve_row(1, 0.02, 0.0)];

        let partners = manifold_normal_pair_partners(&rows, true);

        assert_eq!(partners, vec![Some(1), Some(0)]);
    }

    #[test]
    fn block_solve_skips_low_speed_duplicate_reduced_rows() {
        let mut rows = vec![block_solve_row(0, 0.01, 0.0), block_solve_row(1, 0.01, 0.0)];
        rows[0].reduction_reason = ContactReductionReason::DuplicateReduced;
        rows[1].reduction_reason = ContactReductionReason::DuplicateReduced;

        let partners = manifold_normal_pair_partners(&rows, true);

        assert_eq!(partners, vec![None, None]);
    }

    #[test]
    fn block_solve_allows_high_speed_duplicate_reduced_rows() {
        let mut rows = vec![block_solve_row(0, 0.02, 0.0), block_solve_row(1, 0.02, 0.0)];
        rows[0].reduction_reason = ContactReductionReason::DuplicateReduced;
        rows[1].reduction_reason = ContactReductionReason::DuplicateReduced;

        let partners = manifold_normal_pair_partners(&rows, true);

        assert_eq!(partners, vec![Some(1), Some(0)]);
    }

    #[test]
    fn block_solve_allows_dense_slow_sliding_below_sparse_threshold() {
        let rows = vec![block_solve_row(0, 0.01, 0.0), block_solve_row(1, 0.01, 0.0)];

        let partners = manifold_normal_pair_partners(&rows, false);

        assert_eq!(partners, vec![Some(1), Some(0)]);
    }

    fn dense_row(raw_depth: FloatNum) -> ContactPositionRow {
        ContactPositionRow {
            contact_index: 0,
            body_a: body(0),
            body_b: body(1),
            normal: Vector::new(0.0, 1.0),
            inverse_mass_a: 1.0,
            inverse_mass_b: 1.0,
            inverse_inertia_a: 0.0,
            inverse_inertia_b: 0.0,
            frame_start_depth: residual_correction_depth_from_raw(
                raw_depth,
                DENSE_POSITION_CORRECTION_SLOP,
            ),
            raw_depth,
            source_row_continuity_candidate: false,
            normal_impulse: 0.0,
            solver_position_bias: 0.0,
            solver_support_friction_impulse: 0.0,
            anchor_a: Vector::new(0.0, 0.5),
            anchor_b: Vector::new(0.0, -0.5),
            body_a_linear_velocity: Vector::default(),
            body_b_linear_velocity: Vector::default(),
            body_a_angular_velocity: 0.0,
            body_b_angular_velocity: 0.0,
        }
    }

    #[test]
    fn dense_position_row_shadow_depth_shrinks_after_accumulated_pseudo_translation() {
        let row = dense_row(0.03);
        let mut shadow_bodies = BTreeMap::new();
        shadow_bodies.insert(
            row.body_a,
            PositionSolveBody {
                pseudo_translation: Vector::new(0.0, 0.01),
                ..PositionSolveBody::default()
            },
        );

        let stale_depth = dense_position_row_depth(&row, &BTreeMap::new());
        let pseudo_depth = dense_position_row_depth(&row, &shadow_bodies);

        assert!((stale_depth - (0.03 - DENSE_POSITION_CORRECTION_SLOP)).abs() < 1.0e-6);
        assert!((pseudo_depth - (0.02 - DENSE_POSITION_CORRECTION_SLOP)).abs() < 1.0e-6);
        assert!(pseudo_depth < stale_depth);
    }

    #[test]
    fn dense_position_row_shadow_depth_skips_already_resolved_row() {
        let row = dense_row(0.03);
        let mut shadow_bodies = BTreeMap::new();
        shadow_bodies.insert(
            row.body_a,
            PositionSolveBody {
                pseudo_translation: Vector::new(0.0, row.frame_start_depth),
                ..PositionSolveBody::default()
            },
        );

        let pseudo_depth = dense_position_row_depth(&row, &shadow_bodies);

        assert!(pseudo_depth.abs() < 1.0e-6);
    }

    #[test]
    fn dense_position_row_shadow_only_updates_pseudo_translation_state() {
        let row = dense_row(0.02);
        let mut shadow_bodies = BTreeMap::new();

        let correction = position_row_correction(&row, row.frame_start_depth, 2, true)
            .expect("row should produce a correction");
        record_shadow_position_delta(
            &mut shadow_bodies,
            row.body_a,
            correction.correction_a,
            correction.rotation_a,
        );
        record_shadow_position_delta(
            &mut shadow_bodies,
            row.body_b,
            correction.correction_b,
            correction.rotation_b,
        );

        let expected = row.frame_start_depth * POSITION_CORRECTION_PERCENT / 2.0 / 2.0;
        assert_eq!(correction.correction_a, Vector::new(0.0, expected));
        assert_eq!(correction.correction_b, Vector::new(0.0, -expected));
        assert_eq!(
            shadow_bodies
                .get(&row.body_a)
                .expect("body a shadow state should exist")
                .pseudo_translation,
            correction.correction_a
        );
        assert_eq!(
            shadow_bodies
                .get(&row.body_b)
                .expect("body b shadow state should exist")
                .pseudo_translation,
            correction.correction_b
        );
    }

    fn budget_test_correction() -> PositionRowCorrection {
        PositionRowCorrection {
            correction_a: Vector::new(0.0, 0.06),
            correction_b: Vector::new(0.0, -0.04),
            rotation_a: 0.02,
            rotation_b: -0.01,
        }
    }

    #[test]
    fn dense_pressure_budget_allows_full_row_when_work_fits() {
        let row = dense_row(0.02);
        let budget = DensePressureBudget::new(1.0);

        let decision = budget.row_decision(
            &row,
            row.frame_start_depth,
            Some(budget_test_correction()),
            0.1,
        );

        assert_eq!(decision, PositionRowDecision::Correct { scale: 1.0 });
    }

    #[test]
    fn dense_pressure_budget_partial_scales_row_correction_uniformly() {
        let row = dense_row(0.02);
        let mut budget = DensePressureBudget::new(1.0);
        budget.consume(0.7);

        let decision = budget.row_decision(
            &row,
            row.frame_start_depth,
            Some(budget_test_correction()),
            0.1,
        );

        let PositionRowDecision::PressureLimited { scale } = decision else {
            panic!("expected partial pressure limit, got {decision:?}");
        };
        assert!((scale - 0.75).abs() < 1.0e-5);
        let scaled = budget_test_correction().scaled(scale);
        assert!((scaled.correction_a.y() - 0.045).abs() < 1.0e-5);
        assert!((scaled.correction_b.y() + 0.03).abs() < 1.0e-5);
        assert!((scaled.rotation_a - 0.015).abs() < 1.0e-5);
        assert!((scaled.rotation_b + 0.0075).abs() < 1.0e-5);
    }

    #[test]
    fn dense_pressure_budget_exhausted_skips_row() {
        let row = dense_row(0.02);
        let mut budget = DensePressureBudget::new(1.0);
        budget.consume(1.0);

        let decision = budget.row_decision(
            &row,
            row.frame_start_depth,
            Some(budget_test_correction()),
            0.1,
        );

        assert_eq!(decision, PositionRowDecision::BudgetExhausted);
    }

    #[test]
    fn dense_pressure_budget_resolved_row_skips_without_consuming_budget() {
        let row = dense_row(0.02);
        let budget = DensePressureBudget::new(1.0);

        let decision = budget.row_decision(&row, 0.0, None, 0.0);

        assert_eq!(decision, PositionRowDecision::SkipResolved);
        assert_eq!(budget.consumed_translation, 0.0);
    }

    #[test]
    fn position_row_correction_uses_angular_effective_mass_for_off_center_contact() {
        let mut row = dense_row(0.02);
        row.anchor_a = Vector::new(1.0, 0.0);
        row.anchor_b = Vector::new(-1.0, 0.0);
        row.inverse_inertia_a = 1.0;
        row.inverse_inertia_b = 1.0;

        let correction = position_row_correction(&row, row.frame_start_depth, 2, true)
            .expect("row should produce off-center correction");

        let scalar = row.frame_start_depth * POSITION_CORRECTION_PERCENT / 2.0;
        let scaled_angular_mass = DENSE_POSITION_ANGULAR_CORRECTION_SCALE * 2.0;
        let expected_impulse = scalar / (2.0 + scaled_angular_mass);
        let expected_rotation = -expected_impulse * DENSE_POSITION_ANGULAR_CORRECTION_SCALE;
        assert_eq!(correction.correction_a, Vector::new(0.0, expected_impulse));
        assert_eq!(correction.correction_b, Vector::new(0.0, -expected_impulse));
        assert!((correction.rotation_a - expected_rotation).abs() < 1.0e-6);
        assert!((correction.rotation_b - expected_rotation).abs() < 1.0e-6);
    }

    #[test]
    fn contact_point_velocity_and_impulse_share_clockwise_positive_convention() {
        let mut bodies = vec![SolverBody {
            dynamic: true,
            inverse_mass: 1.0,
            inverse_inertia: 1.0,
            center: Point::default(),
            linear_velocity: Vector::default(),
            angular_velocity: 0.0,
        }];
        let anchor = Vector::new(0.0, 1.0);

        apply_impulse_to_body(&mut bodies, 0, anchor, Vector::new(1.0, 0.0));

        assert!(
            bodies[0].angular_velocity > 0.0,
            "positive angular velocity is clockwise in Picea's screen-space rotation convention"
        );
        assert_eq!(
            contact_point_velocity(bodies[0], anchor),
            Vector::new(2.0, 0.0),
            "rightward impulse above center should add clockwise point velocity at that anchor"
        );
    }

    #[test]
    fn resting_shallow_support_requires_shallow_frictional_contact() {
        let dynamic = SolverBody {
            dynamic: true,
            inverse_mass: 1.0,
            inverse_inertia: 1.0,
            center: Point::default(),
            linear_velocity: Vector::default(),
            angular_velocity: 0.0,
        };
        let static_body = SolverBody {
            dynamic: false,
            inverse_mass: 0.0,
            inverse_inertia: 0.0,
            center: Point::default(),
            linear_velocity: Vector::default(),
            angular_velocity: 0.0,
        };

        assert!(is_resting_shallow_support_contact(
            dynamic,
            static_body,
            0.001,
            0.05,
            false,
            0.8,
        ));
        assert!(is_resting_shallow_support_contact(
            dynamic, dynamic, 0.001, 0.05, false, 0.8,
        ));
        assert!(!is_resting_shallow_support_contact(
            dynamic,
            static_body,
            0.001,
            SHALLOW_SUPPORT_MAX_SEPARATING_SPEED + 0.001,
            false,
            0.8,
        ));
        assert!(!is_resting_shallow_support_contact(
            dynamic,
            static_body,
            0.001,
            0.05,
            false,
            0.0,
        ));
    }

    #[test]
    fn dense_source_row_pressure_gate_rejects_high_motion_counterpart() {
        let mut row = dense_row(0.013);
        row.source_row_continuity_candidate = true;
        row.normal_impulse = 0.001;
        row.body_b_linear_velocity = Vector::new(0.12, 0.0);
        row.body_b_angular_velocity = 0.08;

        assert!(dense_contact_position_correction_gate(
            &row,
            row.frame_start_depth
        ));
    }

    #[test]
    fn dense_position_row_decision_applies_source_gate_before_budget() {
        let mut row = dense_row(0.013);
        row.source_row_continuity_candidate = true;
        row.normal_impulse = 0.001;
        row.body_b_linear_velocity = Vector::new(0.12, 0.0);
        row.body_b_angular_velocity = 0.08;
        let mut budget = DensePressureBudget::new(1.0);
        budget.consume(1.0);

        let decision = budget.row_decision(
            &row,
            row.frame_start_depth,
            Some(budget_test_correction()),
            0.1,
        );

        assert_eq!(decision, PositionRowDecision::SkipSourcePressureGate);
    }

    #[test]
    fn dense_source_row_pressure_gate_preserves_quiet_resting_support() {
        let mut row = dense_row(0.013);
        row.source_row_continuity_candidate = true;
        row.normal_impulse = 0.001;
        row.body_a_linear_velocity = Vector::new(0.01, 0.0);
        row.body_b_linear_velocity = Vector::new(0.02, 0.0);
        row.body_a_angular_velocity = 0.01;
        row.body_b_angular_velocity = 0.02;

        assert!(!dense_contact_position_correction_gate(
            &row,
            row.frame_start_depth
        ));
    }
}
