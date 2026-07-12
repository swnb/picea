use std::collections::BTreeMap;

use crate::{
    events::{NumericsWarningEvent, SleepTransitionReason, WorldEvent},
    handles::BodyHandle,
    pipeline::{
        broadphase::BroadphaseStats, ccd::CcdPoseClampOutcome, island::SolverStepStats, StepConfig,
        StepOutcome, StepStats,
    },
    world::contact_state::WarmStartStats,
    world::World,
};

pub(crate) fn simulate_world_step(world: &mut World, config: &StepConfig) -> StepOutcome {
    let mut step = StepContext::new(world);

    // Reordered step: gravity lands in velocities first, then predictive CCD
    // advances fast movers to their time-of-impact so the contact phase sees
    // them touching. Contacts collide and solve against those poses, and only
    // then do positions integrate with the solved velocities. Running CCD
    // before contacts (instead of after position integration) keeps its traces
    // in the same step that consumes them, while the velocity-first solve still
    // spares resting stacks the integrate-then-correct penetration cycle.
    crate::pipeline::integrate::run_velocity_integration_phase(
        world,
        config,
        &mut step.numeric_warnings,
    );
    step.pose_clamp = crate::pipeline::ccd::run_pose_clamp_phase(world, config.dt);
    let joint_solver_stats = crate::pipeline::joints::solve_joint_phase(
        world,
        config.dt,
        &step.pose_clamp.clamped_bodies,
        &mut step.wake_reasons,
        &mut step.numeric_warnings,
    );
    step.record_solver_stats(joint_solver_stats);
    let predicted_body_poses = crate::pipeline::integrate::preintegrated_body_poses(
        world,
        config.dt,
        &step.pose_clamp.clamped_bodies,
    );
    let pending_contacts = crate::pipeline::contacts::run_contact_solve_phase(
        world,
        config,
        &mut step.wake_reasons,
        &step.pose_clamp.traces,
        &predicted_body_poses,
    );
    if config.joint_velocity_projection {
        crate::pipeline::joints::solve_joint_velocity_phase(world, &mut step.wake_reasons);
    }
    crate::pipeline::integrate::run_position_integration_phase(
        world,
        config,
        &mut step.numeric_warnings,
        &step.pose_clamp.clamped_bodies,
    );
    let (
        contact_events,
        contact_count,
        manifold_count,
        broadphase_stats,
        warm_start_stats,
        contact_solver_stats,
    ) = crate::pipeline::contacts::finalize_contact_phases(world, pending_contacts);
    step.record_contacts(
        contact_events,
        contact_count,
        manifold_count,
        broadphase_stats,
        warm_start_stats,
        contact_solver_stats,
    );
    let (sleep_events, sleep_transition_count, active_body_count) =
        crate::pipeline::sleep::refresh_sleep_phase(
            world,
            config,
            &step.previous_sleep_states,
            &step.wake_reasons,
            &step.events,
        );
    step.record_sleep(sleep_events, sleep_transition_count, active_body_count);
    step.record_numeric_warnings();

    let stats = step.stats(world, config);
    world.commit_step(config, stats, step.events)
}

struct StepContext {
    previous_sleep_states: BTreeMap<BodyHandle, bool>,
    wake_reasons: BTreeMap<BodyHandle, SleepTransitionReason>,
    events: Vec<WorldEvent>,
    numeric_warnings: Vec<NumericsWarningEvent>,
    pose_clamp: CcdPoseClampOutcome,
    broadphase_stats: BroadphaseStats,
    warm_start_stats: WarmStartStats,
    solver_stats: SolverStepStats,
    contact_count: usize,
    manifold_count: usize,
    sleep_transition_count: usize,
    active_body_count: usize,
}

impl StepContext {
    fn new(world: &mut World) -> Self {
        let wake_reasons = world.take_pending_wake_reasons();
        let previous_sleep_states = world
            .bodies()
            .map(|handle| {
                (
                    handle,
                    world
                        .body_record(handle)
                        .map(|record| record.sleeping || wake_reasons.contains_key(&handle))
                        .unwrap_or(false),
                )
            })
            .collect::<BTreeMap<BodyHandle, bool>>();
        Self {
            previous_sleep_states,
            wake_reasons,
            events: world.take_pending_events(),
            numeric_warnings: Vec::new(),
            pose_clamp: CcdPoseClampOutcome::default(),
            broadphase_stats: BroadphaseStats::default(),
            warm_start_stats: WarmStartStats::default(),
            solver_stats: SolverStepStats::default(),
            contact_count: 0,
            manifold_count: 0,
            sleep_transition_count: 0,
            active_body_count: 0,
        }
    }

    fn record_contacts(
        &mut self,
        events: Vec<WorldEvent>,
        contact_count: usize,
        manifold_count: usize,
        broadphase_stats: BroadphaseStats,
        warm_start_stats: WarmStartStats,
        solver_stats: SolverStepStats,
    ) {
        self.events.extend(events);
        self.contact_count = contact_count;
        self.manifold_count = manifold_count;
        self.broadphase_stats = broadphase_stats;
        self.warm_start_stats = warm_start_stats;
        self.record_solver_stats(solver_stats);
    }

    fn record_solver_stats(&mut self, stats: SolverStepStats) {
        self.solver_stats.accumulate(stats);
    }

    fn record_sleep(
        &mut self,
        events: Vec<WorldEvent>,
        sleep_transition_count: usize,
        active_body_count: usize,
    ) {
        self.events.extend(events);
        self.sleep_transition_count = sleep_transition_count;
        self.active_body_count = active_body_count;
    }

    fn record_numeric_warnings(&mut self) {
        self.events.extend(
            self.numeric_warnings
                .iter()
                .cloned()
                .map(WorldEvent::NumericsWarning),
        );
    }

    fn stats(&self, world: &World, config: &StepConfig) -> StepStats {
        StepStats {
            step_index: world.last_step_stats().step_index.saturating_add(1),
            body_count: world.bodies().count(),
            collider_count: world.collider_records().count(),
            joint_count: world.joints().count(),
            active_body_count: self.active_body_count,
            broadphase_candidate_count: self.broadphase_stats.candidate_count,
            broadphase_update_count: self.broadphase_stats.update_count,
            broadphase_stale_proxy_drop_count: self.broadphase_stats.stale_proxy_drop_count,
            broadphase_same_body_drop_count: self.broadphase_stats.same_body_drop_count,
            broadphase_filter_drop_count: self.broadphase_stats.filter_drop_count,
            broadphase_narrowphase_drop_count: self.broadphase_stats.narrowphase_drop_count,
            broadphase_traversal_count: self.broadphase_stats.traversal_count,
            broadphase_pruned_count: self.broadphase_stats.pruned_count,
            broadphase_rebuild_count: self.broadphase_stats.rebuild_count,
            broadphase_tree_depth: self.broadphase_stats.tree_depth,
            contact_count: self.contact_count,
            manifold_count: self.manifold_count,
            island_count: self.solver_stats.island_count,
            active_island_count: self.solver_stats.active_island_count,
            sleeping_island_skip_count: self.solver_stats.sleeping_island_skip_count,
            solver_body_slot_count: self.solver_stats.body_slot_count,
            contact_row_count: self.solver_stats.contact_row_count,
            joint_row_count: self.solver_stats.joint_row_count,
            warm_start_hit_count: self.warm_start_stats.hit_count,
            warm_start_miss_count: self.warm_start_stats.miss_count,
            warm_start_drop_count: self.warm_start_stats.drop_count,
            ccd_candidate_count: self.pose_clamp.stats.candidate_count,
            ccd_hit_count: self.pose_clamp.stats.hit_count,
            ccd_miss_count: self.pose_clamp.stats.miss_count,
            ccd_clamp_count: self.pose_clamp.stats.clamp_count,
            position_correction_input_contact_count: self
                .solver_stats
                .position_correction_input_contact_count,
            position_correction_input_max_depth: self
                .solver_stats
                .position_correction_input_max_depth,
            position_correction_input_total_depth: self
                .solver_stats
                .position_correction_input_total_depth,
            position_correction_body_count: self.solver_stats.position_correction_body_count,
            position_correction_max_translation: self
                .solver_stats
                .position_correction_max_translation,
            position_correction_total_translation: self
                .solver_stats
                .position_correction_total_translation,
            velocity_iterations: config.velocity_iterations,
            position_iterations: config.position_iterations,
            sleep_transition_count: self.sleep_transition_count,
            numeric_warnings: self.numeric_warnings.len(),
        }
    }
}
