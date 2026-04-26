use std::collections::{BTreeMap, BTreeSet};

use crate::{
    body::Pose,
    collider::{CollisionFilter, Material, ShapeAabb, SharedShape},
    events::{ContactEvent, ContactReductionReason, WarmStartCacheReason, WorldEvent},
    handles::{BodyHandle, ColliderHandle},
    math::{point::Point, vector::Vector, FloatNum},
    pipeline::{
        broadphase::{BroadphaseStats, ColliderProxy},
        narrowphase::contact_from_shapes,
        StepConfig,
    },
    world::{
        contact_state::{ContactKey, ContactPairKey, ContactRecord, WarmStartStats},
        World,
    },
};

const WARM_START_NORMAL_DOT_THRESHOLD: FloatNum = 0.98;
const WARM_START_POINT_DRIFT_THRESHOLD: FloatNum = 0.05;

#[derive(Clone, Debug)]
struct ContactObservation {
    key: ContactKey,
    pair_key: ContactPairKey,
    body_a: BodyHandle,
    body_b: BodyHandle,
    collider_a: ColliderHandle,
    collider_b: ColliderHandle,
    anchor_a: Vector,
    anchor_b: Vector,
    point: Point,
    normal: Vector,
    depth: FloatNum,
    feature_id: crate::handles::ContactFeatureId,
    reduction_reason: ContactReductionReason,
    is_sensor: bool,
    material: Material,
    normal_impulse: FloatNum,
    tangent_impulse: FloatNum,
}

#[derive(Clone, Debug)]
struct ColliderSnapshot {
    handle: ColliderHandle,
    body: BodyHandle,
    shape: SharedShape,
    world_pose: Pose,
    aabb: ShapeAabb,
    material: Material,
    filter: CollisionFilter,
    is_sensor: bool,
}

#[derive(Clone, Copy, Debug)]
struct VelocityUpdate {
    body: BodyHandle,
    linear_velocity: Vector,
    angular_velocity: FloatNum,
}

#[derive(Clone, Copy, Debug, Default)]
struct ContactImpulse {
    normal: FloatNum,
    tangent: FloatNum,
}

pub(crate) fn run_contact_phases(
    world: &mut World,
    config: &StepConfig,
    awake_bodies: &mut BTreeSet<BodyHandle>,
) -> (
    Vec<WorldEvent>,
    usize,
    usize,
    BroadphaseStats,
    WarmStartStats,
) {
    let mut contacts = world.collect_contact_observations();
    let broadphase_stats = contacts.broadphase_stats;
    world.resolve_contacts(&mut contacts.observations, config.dt, awake_bodies);
    let (events, contact_count, manifold_count, warm_start_stats) =
        world.refresh_contact_events(contacts.observations);
    (
        events,
        contact_count,
        manifold_count,
        broadphase_stats,
        warm_start_stats,
    )
}

struct ContactPhaseObservations {
    observations: Vec<ContactObservation>,
    broadphase_stats: BroadphaseStats,
}

impl World {
    fn collect_contact_observations(&mut self) -> ContactPhaseObservations {
        let colliders = self.live_collider_snapshots();
        let proxies = colliders
            .iter()
            .map(|collider| ColliderProxy {
                handle: collider.handle,
                aabb: collider.aabb,
            })
            .collect::<Vec<_>>();
        let mut broadphase = self.update_broadphase(&proxies);
        let mut observations = Vec::new();

        for (index, other_index) in broadphase.candidate_pairs {
            let collider_a = &colliders[index];
            let collider_b = &colliders[other_index];
            if collider_a.body == collider_b.body {
                broadphase.stats.same_body_drop_count += 1;
                continue;
            }
            if !collider_a.filter.allows(&collider_b.filter) {
                broadphase.stats.filter_drop_count += 1;
                continue;
            }
            let Some(contact) = contact_from_shapes(
                &collider_a.shape,
                collider_a.world_pose,
                collider_a.aabb,
                &collider_b.shape,
                collider_b.world_pose,
                collider_b.aabb,
            ) else {
                broadphase.stats.narrowphase_drop_count += 1;
                continue;
            };

            let (
                ordered_a,
                ordered_b,
                ordered_body_a,
                ordered_body_b,
                ordered_pose_a,
                ordered_pose_b,
                ordered_normal,
            ) = if collider_a.handle <= collider_b.handle {
                (
                    collider_a.handle,
                    collider_b.handle,
                    collider_a.body,
                    collider_b.body,
                    collider_a.world_pose,
                    collider_b.world_pose,
                    contact.normal,
                )
            } else {
                (
                    collider_b.handle,
                    collider_a.handle,
                    collider_b.body,
                    collider_a.body,
                    collider_b.world_pose,
                    collider_a.world_pose,
                    -contact.normal,
                )
            };

            for point in contact.points {
                let pair_key = ContactPairKey::new(ordered_a, ordered_b);
                observations.push(ContactObservation {
                    key: ContactKey::new(ordered_a, ordered_b, point.feature_id),
                    pair_key,
                    body_a: ordered_body_a,
                    body_b: ordered_body_b,
                    collider_a: ordered_a,
                    collider_b: ordered_b,
                    anchor_a: point.point - ordered_pose_a.point(),
                    anchor_b: point.point - ordered_pose_b.point(),
                    point: point.point,
                    normal: ordered_normal,
                    depth: point.depth,
                    feature_id: point.feature_id,
                    reduction_reason: contact.reduction_reason,
                    is_sensor: collider_a.is_sensor || collider_b.is_sensor,
                    material: combine_materials(collider_a.material, collider_b.material),
                    normal_impulse: 0.0,
                    tangent_impulse: 0.0,
                });
            }
        }

        ContactPhaseObservations {
            observations,
            broadphase_stats: broadphase.stats,
        }
    }

    fn resolve_contacts(
        &mut self,
        contacts: &mut [ContactObservation],
        dt: FloatNum,
        awake_bodies: &mut BTreeSet<BodyHandle>,
    ) {
        let contact_indices = strongest_contact_per_pair(contacts);
        for index in contact_indices {
            if contacts[index].is_sensor {
                continue;
            }
            let (velocity_updates, impulse) = self.contact_velocity_updates(&contacts[index], dt);
            contacts[index].normal_impulse = impulse.normal;
            contacts[index].tangent_impulse = impulse.tangent;
            self.apply_body_pair_correction(
                contacts[index].body_a,
                contacts[index].body_b,
                contacts[index].normal * (contacts[index].depth * 0.5),
                awake_bodies,
            );
            self.apply_velocity_updates(&velocity_updates, awake_bodies);
        }
    }

    fn refresh_contact_events(
        &mut self,
        contacts: Vec<ContactObservation>,
    ) -> (Vec<WorldEvent>, usize, usize, WarmStartStats) {
        let mut previous = self.take_active_contacts();
        let mut pair_manifold_ids = previous
            .values()
            .map(|record| {
                (
                    ContactPairKey::new(record.contact.collider_a, record.contact.collider_b),
                    record.contact.manifold_id,
                )
            })
            .collect::<BTreeMap<_, _>>();
        let previous_pairs = previous
            .values()
            .map(|record| ContactPairKey::new(record.contact.collider_a, record.contact.collider_b))
            .collect::<BTreeSet<_>>();
        let mut next = BTreeMap::new();
        let mut events = Vec::new();
        let mut warm_start_stats = WarmStartStats::default();

        for contact in contacts {
            let existing = previous.remove(&contact.key);
            let is_persisted = existing.is_some();
            let (warm_start_reason, warm_start_normal_impulse, warm_start_tangent_impulse) =
                warm_start_transfer(
                    existing.as_ref(),
                    &contact,
                    previous_pairs.contains(&contact.pair_key),
                );
            warm_start_stats.record(warm_start_reason);
            let event = if let Some(existing) = existing {
                ContactEvent {
                    contact_id: existing.contact.contact_id,
                    manifold_id: existing.contact.manifold_id,
                    body_a: contact.body_a,
                    body_b: contact.body_b,
                    collider_a: contact.collider_a,
                    collider_b: contact.collider_b,
                    feature_id: contact.feature_id,
                    point: contact.point,
                    normal: contact.normal,
                    depth: contact.depth,
                    reduction_reason: contact.reduction_reason,
                    warm_start_reason,
                    warm_start_normal_impulse,
                    warm_start_tangent_impulse,
                }
            } else {
                let manifold_id = *pair_manifold_ids
                    .entry(contact.pair_key)
                    .or_insert_with(|| self.alloc_next_manifold_id());
                ContactEvent {
                    contact_id: self.alloc_next_contact_id(),
                    manifold_id,
                    body_a: contact.body_a,
                    body_b: contact.body_b,
                    collider_a: contact.collider_a,
                    collider_b: contact.collider_b,
                    feature_id: contact.feature_id,
                    point: contact.point,
                    normal: contact.normal,
                    depth: contact.depth,
                    reduction_reason: contact.reduction_reason,
                    warm_start_reason,
                    warm_start_normal_impulse,
                    warm_start_tangent_impulse,
                }
            };

            if is_persisted {
                events.push(WorldEvent::ContactPersisted(event));
            } else {
                events.push(WorldEvent::ContactStarted(event));
            }
            next.insert(
                contact.key,
                ContactRecord {
                    contact: event,
                    anchor_a: contact.anchor_a,
                    anchor_b: contact.anchor_b,
                    normal_impulse: contact.normal_impulse,
                    tangent_impulse: contact.tangent_impulse,
                },
            );
        }

        for (_, record) in previous {
            events.push(WorldEvent::ContactEnded(record.contact));
        }

        let contact_count = next.len();
        let manifold_count = next
            .keys()
            .map(|key| key.pair)
            .collect::<BTreeSet<_>>()
            .len();

        self.replace_active_contacts(next);
        (events, contact_count, manifold_count, warm_start_stats)
    }

    fn live_collider_snapshots(&self) -> Vec<ColliderSnapshot> {
        self.collider_records()
            .filter_map(|(handle, record)| {
                let body = self.body_record(record.body).ok()?;
                let world_pose = body.pose.compose(record.local_pose);
                Some(ColliderSnapshot {
                    handle,
                    body: record.body,
                    shape: record.shape.clone(),
                    world_pose,
                    aabb: record.shape.aabb(world_pose),
                    material: record.material,
                    filter: record.filter,
                    is_sensor: record.is_sensor,
                })
            })
            .collect()
    }

    fn contact_velocity_updates(
        &self,
        contact: &ContactObservation,
        dt: FloatNum,
    ) -> (Vec<VelocityUpdate>, ContactImpulse) {
        let Ok(body_a) = self.body_record(contact.body_a) else {
            return (Vec::new(), ContactImpulse::default());
        };
        let Ok(body_b) = self.body_record(contact.body_b) else {
            return (Vec::new(), ContactImpulse::default());
        };
        let inv_mass_a = body_a.mass_properties.inverse_mass;
        let inv_mass_b = body_b.mass_properties.inverse_mass;
        let inv_mass_sum = inv_mass_a + inv_mass_b;
        if inv_mass_sum <= FloatNum::EPSILON {
            return (Vec::new(), ContactImpulse::default());
        }

        let normal = contact.normal.normalized_or_zero();
        if normal.length() <= FloatNum::EPSILON {
            return (Vec::new(), ContactImpulse::default());
        }

        let mut velocity_a = body_a.linear_velocity;
        let mut velocity_b = body_b.linear_velocity;
        let relative_velocity = velocity_a - velocity_b;
        let normal_speed = relative_velocity.dot(normal);
        let mut normal_impulse = 0.0;
        if normal_speed < 0.0 {
            let restitution = contact.material.restitution.clamp(0.0, 1.0);
            normal_impulse = -(1.0 + restitution) * normal_speed / inv_mass_sum;
            velocity_a += normal * (normal_impulse * inv_mass_a);
            velocity_b -= normal * (normal_impulse * inv_mass_b);
        }

        let tangent = normal.perp().normalized_or_zero();
        let mut tangent_impulse = 0.0;
        if tangent.length() > FloatNum::EPSILON && normal_speed <= 0.0 {
            let tangent_speed = (velocity_a - velocity_b).dot(tangent);
            let friction = contact.material.friction.clamp(0.0, 1.0);
            // Until the full sequential impulse solver lands, use the penetration correction as
            // a support budget for resting contacts. Separating contacts receive no friction.
            let support_impulse = contact.depth.max(0.0) / dt.max(FloatNum::EPSILON);
            let max_friction_impulse = friction * normal_impulse.max(support_impulse);
            tangent_impulse =
                (-tangent_speed / inv_mass_sum).clamp(-max_friction_impulse, max_friction_impulse);
            velocity_a += tangent * (tangent_impulse * inv_mass_a);
            velocity_b -= tangent * (tangent_impulse * inv_mass_b);
        }

        let updates = [
            (
                contact.body_a,
                velocity_a,
                body_a.angular_velocity,
                inv_mass_a,
            ),
            (
                contact.body_b,
                velocity_b,
                body_b.angular_velocity,
                inv_mass_b,
            ),
        ]
        .into_iter()
        .filter_map(|(body, linear_velocity, angular_velocity, inv_mass)| {
            (inv_mass > 0.0).then_some(VelocityUpdate {
                body,
                linear_velocity,
                angular_velocity,
            })
        })
        .collect();
        (
            updates,
            ContactImpulse {
                normal: normal_impulse,
                tangent: tangent_impulse,
            },
        )
    }

    fn apply_velocity_updates(
        &mut self,
        updates: &[VelocityUpdate],
        awake_bodies: &mut BTreeSet<BodyHandle>,
    ) {
        for update in updates {
            let Ok(record) = self.body_record_mut(update.body) else {
                continue;
            };
            if !record.body_type.is_dynamic() {
                continue;
            }
            record.linear_velocity = update.linear_velocity;
            record.angular_velocity = update.angular_velocity;
            record.sleeping = false;
            record.sleep_idle_time = 0.0;
            awake_bodies.insert(update.body);
        }
    }
}

fn warm_start_transfer(
    previous: Option<&ContactRecord>,
    contact: &ContactObservation,
    had_previous_pair: bool,
) -> (WarmStartCacheReason, FloatNum, FloatNum) {
    if contact.is_sensor {
        return (WarmStartCacheReason::SkippedSensor, 0.0, 0.0);
    }

    let Some(previous) = previous else {
        return if had_previous_pair {
            (WarmStartCacheReason::MissFeatureId, 0.0, 0.0)
        } else {
            (WarmStartCacheReason::MissNoPrevious, 0.0, 0.0)
        };
    };

    if previous.contact.warm_start_reason == WarmStartCacheReason::SkippedSensor {
        return (WarmStartCacheReason::MissPreviousSensor, 0.0, 0.0);
    }

    if !previous.normal_impulse.is_finite() || !previous.tangent_impulse.is_finite() {
        return (WarmStartCacheReason::DroppedInvalidImpulse, 0.0, 0.0);
    }

    let previous_normal = previous.contact.normal.normalized_or_zero();
    let current_normal = contact.normal.normalized_or_zero();
    // Normal mismatch means the old impulse would push along the wrong
    // constraint row. Feature ids alone are not enough after a normal flip.
    if previous_normal.length() <= FloatNum::EPSILON
        || current_normal.length() <= FloatNum::EPSILON
        || previous_normal.dot(current_normal) < WARM_START_NORMAL_DOT_THRESHOLD
    {
        return (WarmStartCacheReason::DroppedNormalMismatch, 0.0, 0.0);
    }

    // Feature ids are local geometric names, not raw world-space guarantees.
    // Compare contact anchors relative to both colliders so a pair translating
    // together keeps its cache, while contact movement on either shape drops it.
    let drift_a = (contact.anchor_a - previous.anchor_a).length();
    let drift_b = (contact.anchor_b - previous.anchor_b).length();
    let drift = drift_a.max(drift_b);
    if !drift.is_finite() || drift > WARM_START_POINT_DRIFT_THRESHOLD {
        return (WarmStartCacheReason::DroppedPointDrift, 0.0, 0.0);
    }

    (
        WarmStartCacheReason::Hit,
        previous.normal_impulse,
        previous.tangent_impulse,
    )
}

fn strongest_contact_per_pair(contacts: &[ContactObservation]) -> Vec<usize> {
    let mut strongest = BTreeMap::<ContactPairKey, usize>::new();
    for (index, contact) in contacts.iter().enumerate() {
        strongest
            .entry(contact.pair_key)
            .and_modify(|current| {
                if contact.depth > contacts[*current].depth {
                    *current = index;
                }
            })
            .or_insert(index);
    }
    strongest.into_values().collect()
}

fn combine_materials(a: Material, b: Material) -> Material {
    Material {
        friction: (a.friction.max(0.0) * b.friction.max(0.0)).sqrt(),
        restitution: a.restitution.max(b.restitution).max(0.0),
    }
}
