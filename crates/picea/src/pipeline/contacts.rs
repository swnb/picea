use std::collections::{BTreeMap, BTreeSet};

use crate::{
    body::Pose,
    collider::{CollisionFilter, ShapeAabb, SharedShape},
    events::{ContactEvent, WorldEvent},
    handles::{BodyHandle, ColliderHandle},
    math::{point::Point, vector::Vector, FloatNum},
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
}

#[derive(Clone, Debug)]
struct ColliderSnapshot {
    handle: ColliderHandle,
    body: BodyHandle,
    world_pose: Pose,
    shape: SharedShape,
    filter: CollisionFilter,
    is_sensor: bool,
}

pub(crate) fn run_contact_phases(
    world: &mut World,
    awake_bodies: &mut BTreeSet<BodyHandle>,
) -> (Vec<WorldEvent>, usize, usize) {
    let contacts = world.collect_contact_observations();
    world.resolve_contacts(&contacts, awake_bodies);
    world.refresh_contact_events(contacts)
}

impl World {
    fn collect_contact_observations(&self) -> Vec<ContactObservation> {
        let colliders = self.live_collider_snapshots();
        let mut observations = Vec::new();

        for index in 0..colliders.len() {
            for other_index in (index + 1)..colliders.len() {
                let collider_a = &colliders[index];
                let collider_b = &colliders[other_index];
                if collider_a.body == collider_b.body
                    || !collider_a.filter.allows(&collider_b.filter)
                {
                    continue;
                }
                let Some((point, normal, depth)) = overlap_from_aabbs(
                    collider_a.shape.aabb(collider_a.world_pose),
                    collider_b.shape.aabb(collider_b.world_pose),
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
                            normal,
                        )
                    } else {
                        (
                            collider_b.handle,
                            collider_a.handle,
                            collider_b.body,
                            collider_a.body,
                            -normal,
                        )
                    };

                observations.push(ContactObservation {
                    key: ContactKey::new(ordered_a, ordered_b),
                    body_a: ordered_body_a,
                    body_b: ordered_body_b,
                    collider_a: ordered_a,
                    collider_b: ordered_b,
                    point,
                    normal: ordered_normal,
                    depth,
                    is_sensor: collider_a.is_sensor || collider_b.is_sensor,
                });
            }
        }

        observations
    }

    fn resolve_contacts(
        &mut self,
        contacts: &[ContactObservation],
        awake_bodies: &mut BTreeSet<BodyHandle>,
    ) {
        for contact in contacts {
            if contact.is_sensor {
                continue;
            }
            self.apply_body_pair_correction(
                contact.body_a,
                contact.body_b,
                contact.normal * (contact.depth * 0.5),
                awake_bodies,
            );
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
                Some(ColliderSnapshot {
                    handle,
                    body: record.body,
                    world_pose: body.pose.compose(record.local_pose),
                    shape: record.shape.clone(),
                    filter: record.filter,
                    is_sensor: record.is_sensor,
                })
            })
            .collect()
    }
}

fn ordered_body_pair(a: BodyHandle, b: BodyHandle) -> (BodyHandle, BodyHandle) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

fn overlap_from_aabbs(a: ShapeAabb, b: ShapeAabb) -> Option<(Point, Vector, FloatNum)> {
    let overlap_x = a.max.x().min(b.max.x()) - a.min.x().max(b.min.x());
    let overlap_y = a.max.y().min(b.max.y()) - a.min.y().max(b.min.y());
    if overlap_x <= 0.0 || overlap_y <= 0.0 {
        return None;
    }

    let point = Point::new(
        (a.min.x().max(b.min.x()) + a.max.x().min(b.max.x())) * 0.5,
        (a.min.y().max(b.min.y()) + a.max.y().min(b.max.y())) * 0.5,
    );
    let center_a = Point::new((a.min.x() + a.max.x()) * 0.5, (a.min.y() + a.max.y()) * 0.5);
    let center_b = Point::new((b.min.x() + b.max.x()) * 0.5, (b.min.y() + b.max.y()) * 0.5);
    let delta = center_a - center_b;

    if overlap_x <= overlap_y {
        let normal = if delta.x() <= 0.0 {
            Vector::new(-1.0, 0.0)
        } else {
            Vector::new(1.0, 0.0)
        };
        Some((point, normal, overlap_x))
    } else {
        let normal = if delta.y() <= 0.0 {
            Vector::new(0.0, -1.0)
        } else {
            Vector::new(0.0, 1.0)
        };
        Some((point, normal, overlap_y))
    }
}
