use std::collections::BTreeMap;

use crate::{
    body::BodyType,
    events::{SleepEvent, SleepTransitionReason, WorldEvent},
    handles::BodyHandle,
    math::FloatNum,
    pipeline::StepConfig,
    world::World,
};

const SLEEP_STABILITY_SECONDS: FloatNum = 0.5;
const SLEEP_LINEAR_THRESHOLD: FloatNum = 0.0001;
const SLEEP_ANGULAR_THRESHOLD: FloatNum = 0.0001;

#[derive(Clone, Debug)]
struct Island {
    id: u32,
    bodies: Vec<BodyHandle>,
}

/// Internal solve batch identity for M12 active islands.
///
/// An island is a deterministic group of non-static bodies linked by contacts
/// or joints. Sleeping islands stay visible to events/debug, but inactive
/// islands should not allocate contact or joint rows in the hot solver phase.
#[derive(Clone, Debug)]
pub(crate) struct SolverIsland {
    pub(crate) id: u32,
    pub(crate) bodies: Vec<BodyHandle>,
    pub(crate) active: bool,
}

pub(crate) fn record_wake_reason(
    wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
    body: BodyHandle,
    reason: SleepTransitionReason,
) {
    wake_reasons
        .entry(body)
        .and_modify(|current| {
            if wake_reason_priority(reason) > wake_reason_priority(*current) {
                *current = reason;
            }
        })
        .or_insert(reason);
}

pub(crate) fn refresh_sleep_phase(
    world: &mut World,
    config: &StepConfig,
    previous_sleep_states: &BTreeMap<BodyHandle, bool>,
    wake_reasons: &BTreeMap<BodyHandle, SleepTransitionReason>,
    step_events: &[WorldEvent],
) -> (Vec<WorldEvent>, usize, usize) {
    world.refresh_sleep_states(config, previous_sleep_states, wake_reasons, step_events)
}

pub(crate) fn build_active_solver_islands<I>(
    world: &World,
    contact_pairs: I,
    wake_reasons: &BTreeMap<BodyHandle, SleepTransitionReason>,
) -> Vec<SolverIsland>
where
    I: IntoIterator<Item = (BodyHandle, BodyHandle)>,
{
    build_islands_from_pairs(world, contact_pairs)
        .into_iter()
        .map(|island| {
            let active = island.bodies.iter().any(|body| {
                world
                    .body_record(*body)
                    .map(|record| !record.sleeping || wake_reasons.contains_key(body))
                    .unwrap_or(false)
            });
            SolverIsland {
                id: island.id,
                bodies: island.bodies,
                active,
            }
        })
        .collect()
}

impl World {
    pub(crate) fn refresh_sleep_states(
        &mut self,
        config: &StepConfig,
        previous_sleep_states: &BTreeMap<BodyHandle, bool>,
        wake_reasons: &BTreeMap<BodyHandle, SleepTransitionReason>,
        step_events: &[WorldEvent],
    ) -> (Vec<WorldEvent>, usize, usize) {
        let islands = build_islands(self, step_events);
        let island_wake_reasons = expand_wake_reasons(&islands, wake_reasons);
        let enable_sleep = self.desc().enable_sleep;
        let mut events = Vec::new();
        let mut transitions = 0usize;
        let mut active_body_count = 0usize;

        for handle in self.bodies().collect::<Vec<_>>() {
            let record = self
                .body_record_mut(handle)
                .expect("live body handles must resolve during sleep refresh");
            record.island_id = None;
            if record.body_type.is_static() {
                record.sleeping = false;
                record.sleep_idle_time = 0.0;
            }
        }

        for island in islands {
            for body in &island.bodies {
                if let Ok(record) = self.body_record_mut(*body) {
                    record.island_id = Some(island.id);
                }
            }

            if let Some(reason) = island_wake_reasons.get(&island.id).copied() {
                active_body_count +=
                    self.wake_island(&island, previous_sleep_states, reason, &mut events);
                continue;
            }

            if !(config.enable_sleep && enable_sleep) {
                active_body_count += self.force_island_awake(
                    &island,
                    previous_sleep_states,
                    SleepTransitionReason::SleepDisabled,
                    &mut events,
                );
                continue;
            }

            if !self.island_can_sleep(&island) {
                active_body_count += self.force_island_awake(
                    &island,
                    previous_sleep_states,
                    SleepTransitionReason::Unknown,
                    &mut events,
                );
                continue;
            }

            let should_sleep = island.bodies.iter().all(|body| {
                self.body_record(*body)
                    .map(|record| {
                        record.body_type.is_dynamic()
                            && is_low_motion(record)
                            && (previous_sleep_states.get(body).copied().unwrap_or(false)
                                || record.sleep_idle_time + config.dt >= SLEEP_STABILITY_SECONDS)
                    })
                    .unwrap_or(false)
            });

            for body in island.bodies {
                let was_sleeping = previous_sleep_states.get(&body).copied().unwrap_or(false);
                let record = self
                    .body_record_mut(body)
                    .expect("island bodies must remain live during sleep refresh");

                if should_sleep {
                    record.sleep_idle_time = SLEEP_STABILITY_SECONDS;
                    record.sleeping = true;
                } else {
                    record.sleep_idle_time = if is_low_motion(record) {
                        (record.sleep_idle_time + config.dt).min(SLEEP_STABILITY_SECONDS)
                    } else {
                        0.0
                    };
                    record.sleeping = false;
                    active_body_count += 1;
                }

                if record.sleeping != was_sleeping {
                    transitions += 1;
                    events.push(WorldEvent::SleepChanged(SleepEvent {
                        body,
                        is_sleeping: record.sleeping,
                        island_id: record.island_id.unwrap_or(0),
                        reason: if record.sleeping {
                            SleepTransitionReason::StabilityWindow
                        } else {
                            SleepTransitionReason::Unknown
                        },
                    }));
                }
            }
        }

        (events, transitions, active_body_count)
    }

    fn wake_island(
        &mut self,
        island: &Island,
        previous_sleep_states: &BTreeMap<BodyHandle, bool>,
        reason: SleepTransitionReason,
        events: &mut Vec<WorldEvent>,
    ) -> usize {
        self.force_island_awake(island, previous_sleep_states, reason, events)
    }

    fn force_island_awake(
        &mut self,
        island: &Island,
        previous_sleep_states: &BTreeMap<BodyHandle, bool>,
        reason: SleepTransitionReason,
        events: &mut Vec<WorldEvent>,
    ) -> usize {
        let mut active = 0usize;
        for body in &island.bodies {
            let was_sleeping = previous_sleep_states.get(body).copied().unwrap_or(false);
            let record = self
                .body_record_mut(*body)
                .expect("island bodies must remain live during sleep refresh");
            if !record.body_type.is_static() {
                active += 1;
            }
            record.sleeping = false;
            record.sleep_idle_time = 0.0;
            if was_sleeping {
                events.push(WorldEvent::SleepChanged(SleepEvent {
                    body: *body,
                    is_sleeping: false,
                    island_id: island.id,
                    reason,
                }));
            }
        }
        active
    }

    fn island_can_sleep(&self, island: &Island) -> bool {
        island.bodies.iter().all(|body| {
            self.body_record(*body)
                .map(|record| record.body_type.is_dynamic() && record.can_sleep)
                .unwrap_or(false)
        })
    }
}

fn build_islands(world: &World, step_events: &[WorldEvent]) -> Vec<Island> {
    build_islands_from_pairs(world, step_events.iter().filter_map(active_contact_bodies))
}

fn build_islands_from_pairs<I>(world: &World, contact_pairs: I) -> Vec<Island>
where
    I: IntoIterator<Item = (BodyHandle, BodyHandle)>,
{
    let bodies = world
        .bodies()
        .filter(|handle| {
            world
                .body_record(*handle)
                .map(|record| !matches!(record.body_type, BodyType::Static))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    let mut union = UnionFind::new(&bodies);

    for (body_a, body_b) in contact_pairs {
        union_if_non_static(world, &mut union, body_a, body_b);
    }

    for (_, joint) in world.joint_records() {
        let bodies = joint.body_handles();
        if bodies.len() == 2 {
            union_if_non_static(world, &mut union, bodies[0], bodies[1]);
        }
    }

    let mut groups: BTreeMap<BodyHandle, Vec<BodyHandle>> = BTreeMap::new();
    for body in bodies {
        let root = union.find(body);
        groups.entry(root).or_default().push(body);
    }

    groups
        .into_values()
        .enumerate()
        .map(|(index, mut bodies)| {
            bodies.sort();
            Island {
                id: index as u32,
                bodies,
            }
        })
        .collect()
}

fn active_contact_bodies(event: &WorldEvent) -> Option<(BodyHandle, BodyHandle)> {
    active_contact_event(event).map(|contact| (contact.body_a, contact.body_b))
}

fn union_if_non_static(
    world: &World,
    union: &mut UnionFind,
    body_a: BodyHandle,
    body_b: BodyHandle,
) {
    if body_a == body_b {
        return;
    }
    let pair_can_connect = [body_a, body_b].iter().all(|body| {
        world
            .body_record(*body)
            .map(|record| !record.body_type.is_static())
            .unwrap_or(false)
    });
    if pair_can_connect {
        union.union(body_a, body_b);
    }
}

fn active_contact_event(event: &WorldEvent) -> Option<&crate::events::ContactEvent> {
    match event {
        WorldEvent::ContactStarted(contact) | WorldEvent::ContactPersisted(contact) => {
            Some(contact)
        }
        _ => None,
    }
}

fn expand_wake_reasons(
    islands: &[Island],
    wake_reasons: &BTreeMap<BodyHandle, SleepTransitionReason>,
) -> BTreeMap<u32, SleepTransitionReason> {
    let mut island_reasons = BTreeMap::new();
    for island in islands {
        for body in &island.bodies {
            if let Some(reason) = wake_reasons.get(body).copied() {
                island_reasons
                    .entry(island.id)
                    .and_modify(|current| {
                        if wake_reason_priority(reason) > wake_reason_priority(*current) {
                            *current = reason;
                        }
                    })
                    .or_insert(reason);
            }
        }
    }
    island_reasons
}

fn is_low_motion(record: &crate::body::BodyRecord) -> bool {
    record.linear_velocity.length() < SLEEP_LINEAR_THRESHOLD
        && record.angular_velocity.abs() < SLEEP_ANGULAR_THRESHOLD
}

fn wake_reason_priority(reason: SleepTransitionReason) -> u8 {
    match reason {
        SleepTransitionReason::Unknown => 0,
        SleepTransitionReason::StabilityWindow => 1,
        SleepTransitionReason::SleepDisabled => 2,
        SleepTransitionReason::JointCorrection => 3,
        SleepTransitionReason::ContactImpulse => 4,
        SleepTransitionReason::Impact => 5,
        SleepTransitionReason::UserPatch => 6,
        SleepTransitionReason::TransformEdit => 7,
        SleepTransitionReason::VelocityEdit => 8,
    }
}

#[derive(Clone, Debug)]
struct UnionFind {
    parents: BTreeMap<BodyHandle, BodyHandle>,
}

impl UnionFind {
    fn new(bodies: &[BodyHandle]) -> Self {
        Self {
            parents: bodies.iter().copied().map(|body| (body, body)).collect(),
        }
    }

    fn find(&mut self, body: BodyHandle) -> BodyHandle {
        let parent = *self.parents.get(&body).unwrap_or(&body);
        if parent == body {
            body
        } else {
            let root = self.find(parent);
            self.parents.insert(body, root);
            root
        }
    }

    fn union(&mut self, a: BodyHandle, b: BodyHandle) {
        if !self.parents.contains_key(&a) || !self.parents.contains_key(&b) {
            return;
        }
        let root_a = self.find(a);
        let root_b = self.find(b);
        if root_a == root_b {
            return;
        }
        let (root, child) = if root_a <= root_b {
            (root_a, root_b)
        } else {
            (root_b, root_a)
        };
        self.parents.insert(child, root);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        body::{BodyDesc, BodyType, Pose},
        collider::ColliderDesc,
        math::vector::Vector,
        world::{World, WorldDesc},
    };

    #[test]
    fn static_contact_does_not_join_dynamic_islands() {
        let mut world = World::new(WorldDesc {
            gravity: Vector::default(),
            enable_sleep: true,
        });
        let static_body = world
            .create_body(BodyDesc {
                body_type: BodyType::Static,
                ..BodyDesc::default()
            })
            .expect("static body");
        let left = world
            .create_body(BodyDesc {
                pose: Pose::from_xy_angle(-1.0, 0.0, 0.0),
                ..BodyDesc::default()
            })
            .expect("left body");
        let right = world
            .create_body(BodyDesc {
                pose: Pose::from_xy_angle(1.0, 0.0, 0.0),
                ..BodyDesc::default()
            })
            .expect("right body");
        let static_collider = world
            .create_collider(static_body, ColliderDesc::default())
            .expect("static collider");
        let left_collider = world
            .create_collider(left, ColliderDesc::default())
            .expect("left collider");
        let right_collider = world
            .create_collider(right, ColliderDesc::default())
            .expect("right collider");
        let contacts = [
            WorldEvent::ContactStarted(crate::events::ContactEvent {
                body_a: static_body,
                body_b: left,
                collider_a: static_collider,
                collider_b: left_collider,
                ..crate::events::ContactEvent::default()
            }),
            WorldEvent::ContactStarted(crate::events::ContactEvent {
                body_a: static_body,
                body_b: right,
                collider_a: static_collider,
                collider_b: right_collider,
                ..crate::events::ContactEvent::default()
            }),
        ];

        let islands = build_islands(&world, &contacts);

        assert_eq!(islands.len(), 2);
        assert!(islands.iter().any(|island| island.bodies == vec![left]));
        assert!(islands.iter().any(|island| island.bodies == vec![right]));
    }

    #[test]
    fn stronger_wake_reason_wins() {
        let body = BodyHandle::from_raw_parts(1, 0);
        let mut reasons = BTreeMap::new();

        record_wake_reason(&mut reasons, body, SleepTransitionReason::JointCorrection);
        record_wake_reason(&mut reasons, body, SleepTransitionReason::Impact);

        assert_eq!(reasons[&body], SleepTransitionReason::Impact);
    }
}
