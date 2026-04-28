use std::collections::{BTreeMap, BTreeSet};

use crate::{
    body::Pose,
    collider::{CollisionFilter, Material, ShapeAabb, SharedShape},
    events::{
        CcdTrace, ContactEvent, ContactReductionReason, GenericConvexTrace, SleepTransitionReason,
        WarmStartCacheReason, WorldEvent,
    },
    handles::{BodyHandle, ColliderHandle},
    math::{point::Point, vector::Vector, FloatNum},
    pipeline::{
        broadphase::{BroadphaseStats, ColliderProxy},
        narrowphase::contact_from_shapes_with_cached_vertices,
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
pub(crate) struct ContactObservation {
    pub(crate) key: ContactKey,
    pub(crate) pair_key: ContactPairKey,
    pub(crate) body_a: BodyHandle,
    pub(crate) body_b: BodyHandle,
    pub(crate) collider_a: ColliderHandle,
    pub(crate) collider_b: ColliderHandle,
    pub(crate) anchor_a: Vector,
    pub(crate) anchor_b: Vector,
    pub(crate) point: Point,
    pub(crate) normal: Vector,
    pub(crate) depth: FloatNum,
    pub(crate) feature_id: crate::handles::ContactFeatureId,
    pub(crate) reduction_reason: ContactReductionReason,
    pub(crate) is_sensor: bool,
    pub(crate) material: Material,
    pub(crate) normal_impulse: FloatNum,
    pub(crate) tangent_impulse: FloatNum,
    pub(crate) warm_start_reason: WarmStartCacheReason,
    pub(crate) warm_start_normal_impulse: FloatNum,
    pub(crate) warm_start_tangent_impulse: FloatNum,
    pub(crate) normal_impulse_clamped: bool,
    pub(crate) tangent_impulse_clamped: bool,
    pub(crate) restitution_velocity_threshold: FloatNum,
    pub(crate) restitution_applied: bool,
    pub(crate) generic_convex_trace: Option<GenericConvexTrace>,
    pub(crate) ccd_trace: Option<CcdTrace>,
}

#[derive(Clone, Debug)]
struct ColliderSnapshot {
    handle: ColliderHandle,
    body: BodyHandle,
    shape: SharedShape,
    world_pose: Pose,
    aabb: ShapeAabb,
    convex_vertices: Option<Vec<Point>>,
    material: Material,
    filter: CollisionFilter,
    is_sensor: bool,
}

pub(crate) fn run_contact_phases(
    world: &mut World,
    config: &StepConfig,
    wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
    ccd_traces: &[CcdTrace],
) -> (
    Vec<WorldEvent>,
    usize,
    usize,
    BroadphaseStats,
    WarmStartStats,
) {
    let mut contacts = world.collect_contact_observations(ccd_traces);
    let broadphase_stats = contacts.broadphase_stats;
    let previous_contacts = world.take_active_contacts();
    world.prepare_contact_warm_start(&mut contacts.observations, &previous_contacts);
    crate::solver::contact::resolve_contacts(
        world,
        &mut contacts.observations,
        config,
        wake_reasons,
    );
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
    fn collect_contact_observations(
        &mut self,
        ccd_traces: &[CcdTrace],
    ) -> ContactPhaseObservations {
        let ccd_traces = ccd_trace_map(ccd_traces);
        let colliders = self.live_collider_snapshots();
        let proxies = colliders
            .iter()
            .map(|collider| ColliderProxy {
                handle: collider.handle,
                aabb: collider.aabb,
            })
            .collect::<Vec<_>>();
        let mut broadphase = self.update_broadphase(&proxies);
        let mut observations = Vec::with_capacity(broadphase.candidate_pairs.len());

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
            let Some(contact) = contact_from_shapes_with_cached_vertices(
                &collider_a.shape,
                collider_a.world_pose,
                collider_a.aabb,
                collider_a.convex_vertices.as_deref(),
                &collider_b.shape,
                collider_b.world_pose,
                collider_b.aabb,
                collider_b.convex_vertices.as_deref(),
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
                    ccd_trace: ccd_traces.get(&(ordered_a, ordered_b)).copied(),
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
                    ccd_trace: contact.ccd_trace,
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
                    ccd_trace: contact.ccd_trace,
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
                let geometry = record.derived_geometry(body.pose);
                Some(ColliderSnapshot {
                    handle,
                    body: record.body,
                    shape: record.shape.clone(),
                    world_pose,
                    aabb: geometry.aabb,
                    convex_vertices: geometry.convex_vertices,
                    material: record.material,
                    filter: record.filter,
                    is_sensor: record.is_sensor,
                })
            })
            .collect()
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

fn combine_materials(a: Material, b: Material) -> Material {
    Material {
        friction: (a.friction.max(0.0) * b.friction.max(0.0)).sqrt(),
        restitution: a.restitution.max(b.restitution).max(0.0),
    }
}

fn ccd_trace_map(traces: &[CcdTrace]) -> BTreeMap<(ColliderHandle, ColliderHandle), CcdTrace> {
    traces
        .iter()
        .copied()
        .map(|trace| {
            let pair = ordered_pair(trace.moving_collider, trace.static_collider);
            (pair, trace)
        })
        .collect()
}

fn ordered_pair(a: ColliderHandle, b: ColliderHandle) -> (ColliderHandle, ColliderHandle) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}
