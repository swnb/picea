use std::collections::BTreeMap;

use crate::{
    handles::BodyHandle,
    pipeline::{StepConfig, StepOutcome, StepStats},
    world::World,
};

pub(crate) fn simulate_world_step(world: &mut World, config: &StepConfig) -> StepOutcome {
    let mut wake_reasons = world.take_pending_wake_reasons();
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
    let mut events = world.take_pending_events();
    let mut numeric_warnings = Vec::new();

    crate::pipeline::integrate::run_integration_phase(world, config, &mut numeric_warnings);
    crate::pipeline::joints::solve_joint_phase(
        world,
        config.dt,
        &mut wake_reasons,
        &mut numeric_warnings,
    );
    let (contact_events, contact_count, manifold_count, broadphase_stats, warm_start_stats) =
        crate::pipeline::contacts::run_contact_phases(world, config, &mut wake_reasons);
    events.extend(contact_events);
    let (sleep_events, sleep_transition_count, active_body_count) =
        crate::pipeline::sleep::refresh_sleep_phase(
            world,
            config,
            &previous_sleep_states,
            &wake_reasons,
            &events,
        );
    events.extend(sleep_events);
    events.extend(
        numeric_warnings
            .iter()
            .cloned()
            .map(crate::events::WorldEvent::NumericsWarning),
    );

    let stats = StepStats {
        step_index: world.last_step_stats().step_index.saturating_add(1),
        body_count: world.bodies().count(),
        collider_count: world.collider_records().count(),
        joint_count: world.joints().count(),
        active_body_count,
        broadphase_candidate_count: broadphase_stats.candidate_count,
        broadphase_update_count: broadphase_stats.update_count,
        broadphase_stale_proxy_drop_count: broadphase_stats.stale_proxy_drop_count,
        broadphase_same_body_drop_count: broadphase_stats.same_body_drop_count,
        broadphase_filter_drop_count: broadphase_stats.filter_drop_count,
        broadphase_narrowphase_drop_count: broadphase_stats.narrowphase_drop_count,
        broadphase_rebuild_count: broadphase_stats.rebuild_count,
        broadphase_tree_depth: broadphase_stats.tree_depth,
        contact_count,
        manifold_count,
        warm_start_hit_count: warm_start_stats.hit_count,
        warm_start_miss_count: warm_start_stats.miss_count,
        warm_start_drop_count: warm_start_stats.drop_count,
        velocity_iterations: config.velocity_iterations,
        position_iterations: config.position_iterations,
        sleep_transition_count,
        numeric_warnings: numeric_warnings.len(),
    };

    world.commit_step(config, stats, events)
}
