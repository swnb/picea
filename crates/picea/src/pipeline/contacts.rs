use std::collections::{BTreeMap, BTreeSet};

use crate::{
    body::Pose,
    collider::{CollisionFilter, Material, ShapeAabb, SharedShape},
    events::{ContactEvent, WorldEvent},
    handles::{BodyHandle, ColliderHandle},
    math::{point::Point, vector::Vector, FloatNum},
    pipeline::{
        broadphase::{ColliderProxy, DynamicAabbTree},
        narrowphase::contact_from_shapes,
        StepConfig,
    },
    world::{
        contact_state::{ContactKey, ContactRecord},
        World,
    },
};

#[derive(Clone, Debug)]
struct ContactObservation {
    key: ContactKey,
    body_a: BodyHandle,
    body_b: BodyHandle,
    collider_a: ColliderHandle,
    collider_b: ColliderHandle,
    point: Point,
    normal: Vector,
    depth: FloatNum,
    is_sensor: bool,
    material: Material,
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

pub(crate) fn run_contact_phases(
    world: &mut World,
    config: &StepConfig,
    awake_bodies: &mut BTreeSet<BodyHandle>,
) -> (Vec<WorldEvent>, usize, usize) {
    let contacts = world.collect_contact_observations();
    world.resolve_contacts(&contacts, config.dt, awake_bodies);
    world.refresh_contact_events(contacts)
}

impl World {
    fn collect_contact_observations(&self) -> Vec<ContactObservation> {
        let colliders = self.live_collider_snapshots();
        let proxies = colliders
            .iter()
            .map(|collider| ColliderProxy {
                handle: collider.handle,
                aabb: collider.aabb,
            })
            .collect::<Vec<_>>();
        let candidate_pairs = DynamicAabbTree::from_proxies(&proxies).candidate_pairs();
        let mut observations = Vec::new();

        for (index, other_index) in candidate_pairs {
            let collider_a = &colliders[index];
            let collider_b = &colliders[other_index];
            if collider_a.body == collider_b.body || !collider_a.filter.allows(&collider_b.filter) {
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
                continue;
            };

            let (ordered_a, ordered_b, ordered_body_a, ordered_body_b, ordered_normal) =
                if collider_a.handle <= collider_b.handle {
                    (
                        collider_a.handle,
                        collider_b.handle,
                        collider_a.body,
                        collider_b.body,
                        contact.normal,
                    )
                } else {
                    (
                        collider_b.handle,
                        collider_a.handle,
                        collider_b.body,
                        collider_a.body,
                        -contact.normal,
                    )
                };

            observations.push(ContactObservation {
                key: ContactKey::new(ordered_a, ordered_b),
                body_a: ordered_body_a,
                body_b: ordered_body_b,
                collider_a: ordered_a,
                collider_b: ordered_b,
                point: contact.point,
                normal: ordered_normal,
                depth: contact.depth,
                is_sensor: collider_a.is_sensor || collider_b.is_sensor,
                material: combine_materials(collider_a.material, collider_b.material),
            });
        }

        observations
    }

    fn resolve_contacts(
        &mut self,
        contacts: &[ContactObservation],
        dt: FloatNum,
        awake_bodies: &mut BTreeSet<BodyHandle>,
    ) {
        for contact in contacts {
            if contact.is_sensor {
                continue;
            }
            let velocity_updates = self.contact_velocity_updates(contact, dt);
            self.apply_body_pair_correction(
                contact.body_a,
                contact.body_b,
                contact.normal * (contact.depth * 0.5),
                awake_bodies,
            );
            self.apply_velocity_updates(&velocity_updates, awake_bodies);
        }
    }

    fn refresh_contact_events(
        &mut self,
        contacts: Vec<ContactObservation>,
    ) -> (Vec<WorldEvent>, usize, usize) {
        let mut previous = self.take_active_contacts();
        let mut next = BTreeMap::new();
        let mut events = Vec::new();

        for contact in contacts {
            let existing = previous.remove(&contact.key);
            let is_persisted = existing.is_some();
            let event = if let Some(existing) = existing {
                ContactEvent {
                    contact_id: existing.contact.contact_id,
                    manifold_id: existing.contact.manifold_id,
                    body_a: contact.body_a,
                    body_b: contact.body_b,
                    collider_a: contact.collider_a,
                    collider_b: contact.collider_b,
                    point: contact.point,
                    normal: contact.normal,
                    depth: contact.depth,
                }
            } else {
                ContactEvent {
                    contact_id: self.alloc_next_contact_id(),
                    manifold_id: self.alloc_next_manifold_id(),
                    body_a: contact.body_a,
                    body_b: contact.body_b,
                    collider_a: contact.collider_a,
                    collider_b: contact.collider_b,
                    point: contact.point,
                    normal: contact.normal,
                    depth: contact.depth,
                }
            };

            if is_persisted {
                events.push(WorldEvent::ContactPersisted(event));
            } else {
                events.push(WorldEvent::ContactStarted(event));
            }
            next.insert(contact.key, ContactRecord { contact: event });
        }

        for (_, record) in previous {
            events.push(WorldEvent::ContactEnded(record.contact));
        }

        let contact_count = next.len();
        let manifold_count = next
            .values()
            .map(|record| ordered_body_pair(record.contact.body_a, record.contact.body_b))
            .collect::<BTreeSet<_>>()
            .len();

        self.replace_active_contacts(next);
        (events, contact_count, manifold_count)
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
    ) -> Vec<VelocityUpdate> {
        let Ok(body_a) = self.body_record(contact.body_a) else {
            return Vec::new();
        };
        let Ok(body_b) = self.body_record(contact.body_b) else {
            return Vec::new();
        };
        let inv_mass_a = if body_a.body_type.is_dynamic() {
            1.0
        } else {
            0.0
        };
        let inv_mass_b = if body_b.body_type.is_dynamic() {
            1.0
        } else {
            0.0
        };
        let inv_mass_sum = inv_mass_a + inv_mass_b;
        if inv_mass_sum <= FloatNum::EPSILON {
            return Vec::new();
        }

        let normal = contact.normal.normalized_or_zero();
        if normal.length() <= FloatNum::EPSILON {
            return Vec::new();
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
        if tangent.length() > FloatNum::EPSILON && normal_speed <= 0.0 {
            let tangent_speed = (velocity_a - velocity_b).dot(tangent);
            let friction = contact.material.friction.clamp(0.0, 1.0);
            // Until the full sequential impulse solver lands, use the penetration correction as
            // a support budget for resting contacts. Separating contacts receive no friction.
            let support_impulse = contact.depth.max(0.0) / dt.max(FloatNum::EPSILON);
            let max_friction_impulse = friction * normal_impulse.max(support_impulse);
            let tangent_impulse =
                (-tangent_speed / inv_mass_sum).clamp(-max_friction_impulse, max_friction_impulse);
            velocity_a += tangent * (tangent_impulse * inv_mass_a);
            velocity_b -= tangent * (tangent_impulse * inv_mass_b);
        }

        [
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
        .collect()
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

fn ordered_body_pair(a: BodyHandle, b: BodyHandle) -> (BodyHandle, BodyHandle) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

fn combine_materials(a: Material, b: Material) -> Material {
    Material {
        friction: (a.friction.max(0.0) * b.friction.max(0.0)).sqrt(),
        restitution: a.restitution.max(b.restitution).max(0.0),
    }
}
