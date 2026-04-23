use std::collections::{BTreeMap, BTreeSet};

use crate::{
    body::BodyType,
    events::{SleepEvent, WorldEvent},
    handles::BodyHandle,
    pipeline::StepConfig,
    world::World,
};

pub(crate) fn refresh_sleep_phase(
    world: &mut World,
    config: &StepConfig,
    previous_sleep_states: &BTreeMap<BodyHandle, bool>,
    awake_bodies: &BTreeSet<BodyHandle>,
) -> (Vec<WorldEvent>, usize, usize) {
    world.refresh_sleep_states(config, previous_sleep_states, awake_bodies)
}

impl World {
    pub(crate) fn refresh_sleep_states(
        &mut self,
        config: &StepConfig,
        previous_sleep_states: &BTreeMap<BodyHandle, bool>,
        awake_bodies: &BTreeSet<BodyHandle>,
    ) -> (Vec<WorldEvent>, usize, usize) {
        let body_handles = self.bodies().collect::<Vec<_>>();
        let enable_sleep = self.desc().enable_sleep;
        let mut events = Vec::new();
        let mut transitions = 0usize;
        let mut active_body_count = 0usize;

        for handle in body_handles {
            let record = self
                .body_record_mut(handle)
                .expect("live body handles must resolve during sleep refresh");
            let was_sleeping = previous_sleep_states.get(&handle).copied().unwrap_or(false);

            match record.body_type {
                BodyType::Static => {
                    record.sleeping = false;
                }
                BodyType::Dynamic => {
                    if awake_bodies.contains(&handle) {
                        record.sleeping = false;
                    } else if config.enable_sleep && enable_sleep && record.can_sleep {
                        record.sleeping = record.linear_velocity.length() < 0.0001
                            && record.angular_velocity.abs() < 0.0001;
                    } else {
                        record.sleeping = false;
                    }

                    if !record.sleeping {
                        active_body_count += 1;
                    }
                    if record.sleeping != was_sleeping {
                        transitions += 1;
                        events.push(WorldEvent::SleepChanged(SleepEvent {
                            body: handle,
                            is_sleeping: record.sleeping,
                        }));
                    }
                }
                BodyType::Kinematic => {
                    record.sleeping = false;
                    active_body_count += 1;
                }
            }
        }

        (events, transitions, active_body_count)
    }
}
