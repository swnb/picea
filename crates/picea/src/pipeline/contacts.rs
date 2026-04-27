use std::collections::{BTreeMap, BTreeSet};

use crate::{
    body::Pose,
    collider::{CollisionFilter, Material, ShapeAabb, SharedShape},
    events::{
        ContactEvent, ContactReductionReason, GenericConvexTrace, SleepTransitionReason,
        WarmStartCacheReason, WorldEvent,
    },
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
const POSITION_CORRECTION_PERCENT: FloatNum = 0.8;
const POSITION_CORRECTION_SLOP: FloatNum = 0.005;
const CONTACT_VELOCITY_BIAS: FloatNum = 0.5;

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
    warm_start_reason: WarmStartCacheReason,
    warm_start_normal_impulse: FloatNum,
    warm_start_tangent_impulse: FloatNum,
    normal_impulse_clamped: bool,
    tangent_impulse_clamped: bool,
    restitution_velocity_threshold: FloatNum,
    restitution_applied: bool,
    generic_convex_trace: Option<GenericConvexTrace>,
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
struct SolverBody {
    dynamic: bool,
    inverse_mass: FloatNum,
    inverse_inertia: FloatNum,
    center: Point,
    linear_velocity: Vector,
    angular_velocity: FloatNum,
}

#[derive(Clone, Debug)]
struct ContactSolverRow {
    contact_index: usize,
    body_a: BodyHandle,
    body_b: BodyHandle,
    normal: Vector,
    tangent: Vector,
    anchor_a: Vector,
    anchor_b: Vector,
    normal_mass: FloatNum,
    tangent_mass: FloatNum,
    friction: FloatNum,
    restitution_bias: FloatNum,
    position_bias: FloatNum,
    normal_impulse: FloatNum,
    tangent_impulse: FloatNum,
    normal_impulse_clamped: bool,
    tangent_impulse_clamped: bool,
    restitution_velocity_threshold: FloatNum,
    restitution_applied: bool,
}

pub(crate) fn run_contact_phases(
    world: &mut World,
    config: &StepConfig,
    wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
) -> (
    Vec<WorldEvent>,
    usize,
    usize,
    BroadphaseStats,
    WarmStartStats,
) {
    let mut contacts = world.collect_contact_observations();
    let broadphase_stats = contacts.broadphase_stats;
    let previous_contacts = world.take_active_contacts();
    world.prepare_contact_warm_start(&mut contacts.observations, &previous_contacts);
    world.resolve_contacts(&mut contacts.observations, config, wake_reasons);
    let (events, contact_count, manifold_count, warm_start_stats) =
        world.refresh_contact_events(contacts.observations, previous_contacts);
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
                    warm_start_reason: WarmStartCacheReason::MissNoPrevious,
                    warm_start_normal_impulse: 0.0,
                    warm_start_tangent_impulse: 0.0,
                    normal_impulse_clamped: false,
                    tangent_impulse_clamped: false,
                    restitution_velocity_threshold: 0.0,
                    restitution_applied: false,
                    generic_convex_trace: contact.generic_convex_trace,
                });
            }
        }

        ContactPhaseObservations {
            observations,
            broadphase_stats: broadphase.stats,
        }
    }

    fn prepare_contact_warm_start(
        &self,
        contacts: &mut [ContactObservation],
        previous_contacts: &BTreeMap<ContactKey, ContactRecord>,
    ) {
        let previous_pairs = previous_contacts
            .values()
            .map(|record| ContactPairKey::new(record.contact.collider_a, record.contact.collider_b))
            .collect::<BTreeSet<_>>();

        for contact in contacts {
            let (reason, normal_impulse, tangent_impulse) = warm_start_transfer(
                previous_contacts.get(&contact.key),
                contact,
                previous_pairs.contains(&contact.pair_key),
            );
            contact.warm_start_reason = reason;
            contact.warm_start_normal_impulse = normal_impulse;
            contact.warm_start_tangent_impulse = tangent_impulse;
            contact.normal_impulse = normal_impulse.max(0.0);
            contact.tangent_impulse = tangent_impulse;
        }
    }

    fn resolve_contacts(
        &mut self,
        contacts: &mut [ContactObservation],
        config: &StepConfig,
        wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
    ) {
        let mut solver_bodies = self.solver_body_cache(contacts);
        let mut rows = self.contact_solver_rows(contacts, &solver_bodies, config);
        for contact in contacts.iter_mut() {
            contact.normal_impulse = 0.0;
            contact.tangent_impulse = 0.0;
            contact.normal_impulse_clamped = false;
            contact.tangent_impulse_clamped = false;
            contact.restitution_velocity_threshold = 0.0;
            contact.restitution_applied = false;
        }

        for row in &rows {
            let warm_start_impulse =
                row.normal * row.normal_impulse + row.tangent * row.tangent_impulse;
            apply_solver_impulse(&mut solver_bodies, row, warm_start_impulse);
        }

        for _ in 0..config.velocity_iterations {
            for row in &mut rows {
                solve_normal_impulse(&mut solver_bodies, row);
                solve_tangent_impulse(&mut solver_bodies, row);
            }
        }

        for row in rows {
            let contact = &mut contacts[row.contact_index];
            contact.normal_impulse = row.normal_impulse.max(0.0);
            contact.tangent_impulse = row.tangent_impulse;
            contact.normal_impulse_clamped = row.normal_impulse_clamped;
            contact.tangent_impulse_clamped = row.tangent_impulse_clamped;
            contact.restitution_velocity_threshold = row.restitution_velocity_threshold;
            contact.restitution_applied = row.restitution_applied;
        }

        self.record_contact_impulse_wakes(contacts, wake_reasons);
        self.write_solver_velocities(&solver_bodies, wake_reasons);
        self.apply_residual_contact_position_correction(
            contacts,
            config.position_iterations,
            wake_reasons,
        );
    }

    fn refresh_contact_events(
        &mut self,
        contacts: Vec<ContactObservation>,
        mut previous: BTreeMap<ContactKey, ContactRecord>,
    ) -> (Vec<WorldEvent>, usize, usize, WarmStartStats) {
        let mut pair_manifold_ids = previous
            .values()
            .map(|record| {
                (
                    ContactPairKey::new(record.contact.collider_a, record.contact.collider_b),
                    record.contact.manifold_id,
                )
            })
            .collect::<BTreeMap<_, _>>();
        let mut next = BTreeMap::new();
        let mut events = Vec::new();
        let mut warm_start_stats = WarmStartStats::default();

        for contact in contacts {
            let existing = previous.remove(&contact.key);
            let is_persisted = existing.is_some();
            warm_start_stats.record(contact.warm_start_reason);
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
                    warm_start_reason: contact.warm_start_reason,
                    warm_start_normal_impulse: contact.warm_start_normal_impulse,
                    warm_start_tangent_impulse: contact.warm_start_tangent_impulse,
                    solver_normal_impulse: contact.normal_impulse,
                    solver_tangent_impulse: contact.tangent_impulse,
                    normal_impulse_clamped: contact.normal_impulse_clamped,
                    tangent_impulse_clamped: contact.tangent_impulse_clamped,
                    restitution_velocity_threshold: contact.restitution_velocity_threshold,
                    restitution_applied: contact.restitution_applied,
                    generic_convex_trace: contact.generic_convex_trace,
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
                    warm_start_reason: contact.warm_start_reason,
                    warm_start_normal_impulse: contact.warm_start_normal_impulse,
                    warm_start_tangent_impulse: contact.warm_start_tangent_impulse,
                    solver_normal_impulse: contact.normal_impulse,
                    solver_tangent_impulse: contact.tangent_impulse,
                    normal_impulse_clamped: contact.normal_impulse_clamped,
                    tangent_impulse_clamped: contact.tangent_impulse_clamped,
                    restitution_velocity_threshold: contact.restitution_velocity_threshold,
                    restitution_applied: contact.restitution_applied,
                    generic_convex_trace: contact.generic_convex_trace,
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

    fn solver_body_cache(
        &self,
        contacts: &[ContactObservation],
    ) -> BTreeMap<BodyHandle, SolverBody> {
        let mut bodies = BTreeMap::new();
        for contact in contacts.iter().filter(|contact| !contact.is_sensor) {
            for handle in [contact.body_a, contact.body_b] {
                bodies.entry(handle).or_insert_with(|| {
                    let record = self
                        .body_record(handle)
                        .expect("live contact body handles must resolve");
                    let mass = record.mass_properties;
                    SolverBody {
                        dynamic: record.body_type.is_dynamic(),
                        inverse_mass: mass.inverse_mass,
                        inverse_inertia: mass.inverse_inertia,
                        center: record.pose.transform_point(mass.local_center_of_mass),
                        linear_velocity: record.linear_velocity,
                        angular_velocity: record.angular_velocity,
                    }
                });
            }
        }
        bodies
    }

    fn contact_solver_rows(
        &self,
        contacts: &[ContactObservation],
        bodies: &BTreeMap<BodyHandle, SolverBody>,
        config: &StepConfig,
    ) -> Vec<ContactSolverRow> {
        contacts
            .iter()
            .enumerate()
            .filter_map(|(contact_index, contact)| {
                if contact.is_sensor {
                    return None;
                }
                let body_a = *bodies.get(&contact.body_a)?;
                let body_b = *bodies.get(&contact.body_b)?;
                let normal = contact.normal.normalized_or_zero();
                let tangent = normal.perp().normalized_or_zero();
                if normal.length() <= FloatNum::EPSILON || tangent.length() <= FloatNum::EPSILON {
                    return None;
                }
                let anchor_a = contact.point - body_a.center;
                let anchor_b = contact.point - body_b.center;
                let normal_mass = effective_mass(body_a, body_b, anchor_a, anchor_b, normal);
                let tangent_mass = effective_mass(body_a, body_b, anchor_a, anchor_b, tangent);
                if normal_mass <= 0.0 && tangent_mass <= 0.0 {
                    return None;
                }

                let relative_normal_speed =
                    relative_contact_velocity(body_a, body_b, anchor_a, anchor_b).dot(normal);
                let restitution_threshold = config.restitution_velocity_threshold;
                let restitution_applied = -relative_normal_speed > restitution_threshold
                    && contact.material.restitution > 0.0;
                let restitution_bias = if restitution_applied {
                    contact.material.restitution.clamp(0.0, 1.0) * -relative_normal_speed
                } else {
                    0.0
                };
                // Penetration velocity bias gives resting overlap a support impulse during the
                // velocity solve, so Coulomb friction has a real normal budget to clamp against.
                let position_bias = if relative_normal_speed.abs() <= 1.0e-4 {
                    (contact.depth - POSITION_CORRECTION_SLOP).max(0.0) * CONTACT_VELOCITY_BIAS
                        / config.dt
                } else {
                    0.0
                };
                let friction = contact.material.friction.max(0.0);
                let normal_impulse = contact.warm_start_normal_impulse.max(0.0);
                let max_friction = friction * normal_impulse;
                let tangent_impulse = contact
                    .warm_start_tangent_impulse
                    .clamp(-max_friction, max_friction);

                Some(ContactSolverRow {
                    contact_index,
                    body_a: contact.body_a,
                    body_b: contact.body_b,
                    normal,
                    tangent,
                    anchor_a,
                    anchor_b,
                    normal_mass,
                    tangent_mass,
                    friction,
                    restitution_bias,
                    position_bias,
                    normal_impulse,
                    tangent_impulse,
                    normal_impulse_clamped: false,
                    tangent_impulse_clamped: (tangent_impulse - contact.warm_start_tangent_impulse)
                        .abs()
                        > FloatNum::EPSILON,
                    restitution_velocity_threshold: restitution_threshold,
                    restitution_applied,
                })
            })
            .collect()
    }

    fn write_solver_velocities(
        &mut self,
        bodies: &BTreeMap<BodyHandle, SolverBody>,
        wake_reasons: &BTreeMap<BodyHandle, SleepTransitionReason>,
    ) {
        for (handle, body) in bodies {
            if !body.dynamic {
                continue;
            };
            let Ok(record) = self.body_record_mut(*handle) else {
                continue;
            };
            if record.sleeping && !wake_reasons.contains_key(handle) {
                continue;
            }
            record.linear_velocity = body.linear_velocity;
            record.angular_velocity = body.angular_velocity;
        }
    }

    fn record_contact_impulse_wakes(
        &self,
        contacts: &[ContactObservation],
        wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
    ) {
        for contact in contacts
            .iter()
            .filter(|contact| !contact.is_sensor && contact.normal_impulse > FloatNum::EPSILON)
        {
            self.record_contact_wake_if_sleeping(contact.body_a, contact.body_b, wake_reasons);
            self.record_contact_wake_if_sleeping(contact.body_b, contact.body_a, wake_reasons);
        }
    }

    fn record_contact_wake_if_sleeping(
        &self,
        body: BodyHandle,
        other: BodyHandle,
        wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
    ) {
        if self
            .body_record(body)
            .map(|record| record.sleeping && record.body_type.is_dynamic())
            .unwrap_or(false)
            && self.contact_counterpart_can_wake(other)
        {
            crate::pipeline::sleep::record_wake_reason(
                wake_reasons,
                body,
                SleepTransitionReason::Impact,
            );
        }
    }

    fn contact_counterpart_can_wake(&self, other: BodyHandle) -> bool {
        self.body_record(other)
            .map(|record| !record.body_type.is_static() && !record.sleeping)
            .unwrap_or(false)
    }

    fn apply_residual_contact_position_correction(
        &mut self,
        contacts: &[ContactObservation],
        iterations: u16,
        wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
    ) {
        if iterations == 0 {
            return;
        }

        for _ in 0..iterations {
            for contact in contacts.iter().filter(|contact| !contact.is_sensor) {
                let normal = contact.normal.normalized_or_zero();
                let depth = (contact.depth - POSITION_CORRECTION_SLOP).max(0.0);
                if normal.length() <= FloatNum::EPSILON || depth <= FloatNum::EPSILON {
                    continue;
                }
                let inv_mass_a = self
                    .body_record(contact.body_a)
                    .map(|record| record.mass_properties.inverse_mass)
                    .unwrap_or(0.0);
                let inv_mass_b = self
                    .body_record(contact.body_b)
                    .map(|record| record.mass_properties.inverse_mass)
                    .unwrap_or(0.0);
                let inv_mass_sum = inv_mass_a + inv_mass_b;
                if inv_mass_sum <= FloatNum::EPSILON {
                    continue;
                }
                let correction = depth * POSITION_CORRECTION_PERCENT / FloatNum::from(iterations);
                let correction_a = normal * (correction * inv_mass_a / inv_mass_sum);
                let correction_b = -normal * (correction * inv_mass_b / inv_mass_sum);
                let wake_a = self.contact_counterpart_can_wake(contact.body_b);
                let wake_b = self.contact_counterpart_can_wake(contact.body_a);
                self.apply_position_translation(contact.body_a, correction_a, wake_a, wake_reasons);
                self.apply_position_translation(contact.body_b, correction_b, wake_b, wake_reasons);
            }
        }
    }

    fn apply_position_translation(
        &mut self,
        body: BodyHandle,
        translation: Vector,
        wake_sleeping_body: bool,
        wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
    ) {
        if translation.length() <= FloatNum::EPSILON {
            return;
        }
        let Ok(record) = self.body_record_mut(body) else {
            return;
        };
        if !record.body_type.is_dynamic() {
            return;
        }
        let was_sleeping = record.sleeping;
        if was_sleeping && !wake_sleeping_body {
            return;
        }
        crate::solver::body_state::translate_pose(&mut record.pose, translation, 0.0);
        record.sleeping = false;
        record.sleep_idle_time = 0.0;
        if was_sleeping {
            crate::pipeline::sleep::record_wake_reason(
                wake_reasons,
                body,
                SleepTransitionReason::ContactImpulse,
            );
        }
    }
}

fn solve_normal_impulse(bodies: &mut BTreeMap<BodyHandle, SolverBody>, row: &mut ContactSolverRow) {
    if row.normal_mass <= 0.0 {
        return;
    }
    let Some((body_a, body_b)) = solver_pair(bodies, row) else {
        return;
    };
    let normal_speed =
        relative_contact_velocity(body_a, body_b, row.anchor_a, row.anchor_b).dot(row.normal);
    // Accumulated impulses are clamped, not per-iteration deltas. This lets a row
    // give impulse back on later iterations without ever pulling bodies together.
    let previous = row.normal_impulse;
    let candidate =
        previous - (normal_speed - row.restitution_bias - row.position_bias) * row.normal_mass;
    row.normal_impulse = candidate.max(0.0);
    row.normal_impulse_clamped |= candidate < 0.0;
    let delta = row.normal_impulse - previous;
    apply_solver_impulse(bodies, row, row.normal * delta);
}

fn solve_tangent_impulse(
    bodies: &mut BTreeMap<BodyHandle, SolverBody>,
    row: &mut ContactSolverRow,
) {
    if row.tangent_mass <= 0.0 {
        return;
    }
    let Some((body_a, body_b)) = solver_pair(bodies, row) else {
        return;
    };
    let tangent_speed =
        relative_contact_velocity(body_a, body_b, row.anchor_a, row.anchor_b).dot(row.tangent);
    let previous = row.tangent_impulse;
    let candidate = previous - tangent_speed * row.tangent_mass;
    // Coulomb friction limits the tangent row by the normal support impulse
    // solved so far: |jt| <= mu * jn.
    let max_friction = row.friction * row.normal_impulse;
    row.tangent_impulse = candidate.clamp(-max_friction, max_friction);
    row.tangent_impulse_clamped |= (row.tangent_impulse - candidate).abs() > FloatNum::EPSILON;
    let delta = row.tangent_impulse - previous;
    apply_solver_impulse(bodies, row, row.tangent * delta);
}

fn solver_pair(
    bodies: &BTreeMap<BodyHandle, SolverBody>,
    row: &ContactSolverRow,
) -> Option<(SolverBody, SolverBody)> {
    Some((*bodies.get(&row.body_a)?, *bodies.get(&row.body_b)?))
}

fn apply_solver_impulse(
    bodies: &mut BTreeMap<BodyHandle, SolverBody>,
    row: &ContactSolverRow,
    impulse: Vector,
) {
    apply_impulse_to_body(bodies, row.body_a, row.anchor_a, impulse);
    apply_impulse_to_body(bodies, row.body_b, row.anchor_b, -impulse);
}

fn apply_impulse_to_body(
    bodies: &mut BTreeMap<BodyHandle, SolverBody>,
    handle: BodyHandle,
    anchor: Vector,
    impulse: Vector,
) {
    let Some(body) = bodies.get_mut(&handle) else {
        return;
    };
    if !body.dynamic {
        return;
    }
    body.linear_velocity += impulse * body.inverse_mass;
    body.angular_velocity -= anchor.cross(impulse) * body.inverse_inertia;
}

fn effective_mass(
    body_a: SolverBody,
    body_b: SolverBody,
    anchor_a: Vector,
    anchor_b: Vector,
    direction: Vector,
) -> FloatNum {
    let anchor_a_cross = anchor_a.cross(direction);
    let anchor_b_cross = anchor_b.cross(direction);
    // Effective mass is the scalar inverse of "how hard is it to change
    // relative velocity along this row", including angular inertia through r x n.
    let denominator = body_a.inverse_mass
        + body_b.inverse_mass
        + body_a.inverse_inertia * anchor_a_cross * anchor_a_cross
        + body_b.inverse_inertia * anchor_b_cross * anchor_b_cross;
    if denominator.is_finite() && denominator > FloatNum::EPSILON {
        1.0 / denominator
    } else {
        0.0
    }
}

fn relative_contact_velocity(
    body_a: SolverBody,
    body_b: SolverBody,
    anchor_a: Vector,
    anchor_b: Vector,
) -> Vector {
    contact_point_velocity(body_a, anchor_a) - contact_point_velocity(body_b, anchor_b)
}

fn contact_point_velocity(body: SolverBody, anchor: Vector) -> Vector {
    body.linear_velocity + angular_point_velocity(body.angular_velocity, anchor)
}

fn angular_point_velocity(angular_velocity: FloatNum, anchor: Vector) -> Vector {
    Vector::new(
        angular_velocity * anchor.y(),
        -angular_velocity * anchor.x(),
    )
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

fn combine_materials(a: Material, b: Material) -> Material {
    Material {
        friction: (a.friction.max(0.0) * b.friction.max(0.0)).sqrt(),
        restitution: a.restitution.max(b.restitution).max(0.0),
    }
}
