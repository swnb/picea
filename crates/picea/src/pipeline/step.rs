use std::collections::BTreeMap;

use crate::{
    body::Pose,
    events::{NumericsWarningEvent, SleepTransitionReason, WorldEvent},
    handles::BodyHandle,
    pipeline::{
        broadphase::BroadphaseStats, ccd::CcdPoseClampOutcome, StepConfig, StepOutcome, StepStats,
    },
    world::contact_state::WarmStartStats,
    world::World,
};

pub(crate) fn simulate_world_step(world: &mut World, config: &StepConfig) -> StepOutcome {
    let mut step = StepContext::new(world);

    crate::pipeline::integrate::run_integration_phase(world, config, &mut step.numeric_warnings);
    crate::pipeline::joints::solve_joint_phase(
        world,
        config.dt,
        &mut step.wake_reasons,
        &mut step.numeric_warnings,
    );
    step.pose_clamp = crate::pipeline::ccd::run_pose_clamp_phase(world, &step.previous_body_poses);
    let (contact_events, contact_count, manifold_count, broadphase_stats, warm_start_stats) =
        crate::pipeline::contacts::run_contact_phases(
            world,
            config,
            &mut step.wake_reasons,
            &step.pose_clamp.traces,
        );
    step.record_contacts(
        contact_events,
        contact_count,
        manifold_count,
        broadphase_stats,
        warm_start_stats,
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
    previous_body_poses: BTreeMap<BodyHandle, Pose>,
    previous_sleep_states: BTreeMap<BodyHandle, bool>,
    wake_reasons: BTreeMap<BodyHandle, SleepTransitionReason>,
    events: Vec<WorldEvent>,
    numeric_warnings: Vec<NumericsWarningEvent>,
    pose_clamp: CcdPoseClampOutcome,
    broadphase_stats: BroadphaseStats,
    warm_start_stats: WarmStartStats,
    contact_count: usize,
    manifold_count: usize,
    sleep_transition_count: usize,
    active_body_count: usize,
}

impl StepContext {
    fn new(world: &mut World) -> Self {
        let previous_body_poses = world
            .body_records()
            .map(|(handle, record)| (handle, record.pose))
            .collect::<BTreeMap<BodyHandle, Pose>>();
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
            previous_body_poses,
            previous_sleep_states,
            wake_reasons,
            events: world.take_pending_events(),
            numeric_warnings: Vec::new(),
            pose_clamp: CcdPoseClampOutcome::default(),
            broadphase_stats: BroadphaseStats::default(),
            warm_start_stats: WarmStartStats::default(),
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
    ) {
        self.events.extend(events);
        self.contact_count = contact_count;
        self.manifold_count = manifold_count;
        self.broadphase_stats = broadphase_stats;
        self.warm_start_stats = warm_start_stats;
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
            broadphase_rebuild_count: self.broadphase_stats.rebuild_count,
            broadphase_tree_depth: self.broadphase_stats.tree_depth,
            contact_count: self.contact_count,
            manifold_count: self.manifold_count,
            warm_start_hit_count: self.warm_start_stats.hit_count,
            warm_start_miss_count: self.warm_start_stats.miss_count,
            warm_start_drop_count: self.warm_start_stats.drop_count,
            ccd_candidate_count: self.pose_clamp.stats.candidate_count,
            ccd_hit_count: self.pose_clamp.stats.hit_count,
            ccd_miss_count: self.pose_clamp.stats.miss_count,
            ccd_clamp_count: self.pose_clamp.stats.clamp_count,
            velocity_iterations: config.velocity_iterations,
            position_iterations: config.position_iterations,
            sleep_transition_count: self.sleep_transition_count,
            numeric_warnings: self.numeric_warnings.len(),
        }
    }
}
