use std::collections::BTreeMap;

use crate::{
    events::{SleepTransitionReason, WorldEvent},
    handles::BodyHandle,
    math::FloatNum,
    pipeline::{
        broadphase::{BroadphaseOutput, ColliderProxy},
        StepConfig, StepOutcome, StepStats,
    },
    world::World,
};

impl World {
    pub(crate) fn last_step_stats(&self) -> StepStats {
        self.last_step_stats
    }

    pub(crate) fn last_step_dt(&self) -> FloatNum {
        self.last_step_dt
    }

    pub(crate) fn simulated_time(&self) -> f64 {
        self.simulated_time
    }

    pub(crate) fn last_step_events(&self) -> &[WorldEvent] {
        &self.last_step_events
    }

    pub(crate) fn take_pending_events(&mut self) -> Vec<WorldEvent> {
        std::mem::take(&mut self.pending_events)
    }

    pub(crate) fn take_pending_wake_reasons(
        &mut self,
    ) -> BTreeMap<BodyHandle, SleepTransitionReason> {
        std::mem::take(&mut self.pending_wake_reasons)
    }

    pub(crate) fn update_broadphase(&mut self, proxies: &[ColliderProxy]) -> BroadphaseOutput {
        self.broadphase.update(proxies)
    }

    pub(crate) fn commit_step(
        &mut self,
        config: &StepConfig,
        stats: StepStats,
        events: Vec<WorldEvent>,
    ) -> StepOutcome {
        self.last_step_stats = stats;
        self.last_step_dt = config.dt;
        self.simulated_time += f64::from(config.dt);
        self.last_step_events = events.clone();
        self.bump_revision();

        StepOutcome {
            revision: self.revision(),
            stats,
            events,
        }
    }
}
