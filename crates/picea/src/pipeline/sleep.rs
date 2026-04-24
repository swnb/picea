use std::collections::{BTreeMap, BTreeSet};

use crate::{
    body::BodyType,
    events::{SleepEvent, WorldEvent},
    handles::BodyHandle,
    math::FloatNum,
    pipeline::StepConfig,
    world::World,
};

const SLEEP_STABILITY_SECONDS: FloatNum = 0.5;
const SLEEP_LINEAR_THRESHOLD: FloatNum = 0.0001;
const SLEEP_ANGULAR_THRESHOLD: FloatNum = 0.0001;

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
                    record.sleep_idle_time = 0.0;
                }
                BodyType::Dynamic => {
                    if awake_bodies.contains(&handle) {
                        record.sleeping = false;
                        record.sleep_idle_time = 0.0;
                    } else if config.enable_sleep && enable_sleep && record.can_sleep {
                        let low_motion = record.linear_velocity.length() < SLEEP_LINEAR_THRESHOLD
                            && record.angular_velocity.abs() < SLEEP_ANGULAR_THRESHOLD;
                        if low_motion {
                            if was_sleeping {
                                record.sleep_idle_time = SLEEP_STABILITY_SECONDS;
                                record.sleeping = true;
                            } else {
                                record.sleep_idle_time = (record.sleep_idle_time + config.dt)
                                    .min(SLEEP_STABILITY_SECONDS);
                                record.sleeping = record.sleep_idle_time >= SLEEP_STABILITY_SECONDS;
                            }
                        } else {
                            record.sleeping = false;
                            record.sleep_idle_time = 0.0;
                        }
                    } else {
                        record.sleeping = false;
                        record.sleep_idle_time = 0.0;
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
                    record.sleep_idle_time = 0.0;
                    active_body_count += 1;
                }
            }
        }

        (events, transitions, active_body_count)
    }
}
