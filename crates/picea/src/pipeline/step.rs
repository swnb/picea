use std::collections::{BTreeMap, BTreeSet};

use crate::{
    handles::BodyHandle,
    pipeline::{StepConfig, StepOutcome, StepStats},
    world::World,
};

pub(crate) fn simulate_world_step(world: &mut World, config: &StepConfig) -> StepOutcome {
    let previous_sleep_states = world
        .bodies()
        .map(|handle| {
            (
                handle,
                world
                    .body_record(handle)
                    .map(|record| record.sleeping)
                    .unwrap_or(false),
            )
        })
        .collect::<BTreeMap<BodyHandle, bool>>();
    let mut events = world.take_pending_events();
    let mut awake_bodies = BTreeSet::new();
    let mut numeric_warnings = Vec::new();

    crate::pipeline::integrate::run_integration_phase(world, config, &mut numeric_warnings);
    crate::pipeline::joints::solve_joint_phase(
        world,
        config.dt,
        &mut awake_bodies,
        &mut numeric_warnings,
    );
    let (contact_events, contact_count, manifold_count) =
        crate::pipeline::contacts::run_contact_phases(world, config, &mut awake_bodies);
    events.extend(contact_events);
    let (sleep_events, sleep_transition_count, active_body_count) =
        crate::pipeline::sleep::refresh_sleep_phase(
            world,
            config,
            &previous_sleep_states,
            &awake_bodies,
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
        contact_count,
        manifold_count,
        velocity_iterations: config.velocity_iterations,
        position_iterations: config.position_iterations,
        sleep_transition_count,
        numeric_warnings: numeric_warnings.len(),
    };

    world.commit_step(config, stats, events)
}
