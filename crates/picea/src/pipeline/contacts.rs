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
        island::SolverStepStats,
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
const WARM_START_TANGENTIAL_DRIFT_THRESHOLD: FloatNum = 0.10;
const PERSISTENT_MANIFOLD_LOCAL_ANCHOR_DRIFT_THRESHOLD: FloatNum = 0.25;

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
    pub(crate) warm_start_anchor_drift: FloatNum,
    pub(crate) warm_start_normal_anchor_drift: FloatNum,
    pub(crate) warm_start_tangent_anchor_drift: FloatNum,
    pub(crate) warm_start_normal_impulse: FloatNum,
    pub(crate) warm_start_tangent_impulse: FloatNum,
    pub(crate) source_row_continuity_candidate: bool,
    pub(crate) solver_initial_normal_speed: FloatNum,
    pub(crate) solver_initial_tangent_speed: FloatNum,
    pub(crate) solver_final_normal_speed: FloatNum,
    pub(crate) solver_final_tangent_speed: FloatNum,
    pub(crate) solver_position_bias: FloatNum,
    pub(crate) solver_restitution_bias: FloatNum,
    pub(crate) solver_support_friction_impulse: FloatNum,
    pub(crate) solver_position_correction_depth: FloatNum,
    pub(crate) solver_position_correction_body_a_translation: FloatNum,
    pub(crate) solver_position_correction_body_b_translation: FloatNum,
    pub(crate) solver_normal_impulse_delta: FloatNum,
    pub(crate) solver_tangent_impulse_delta: FloatNum,
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
    SolverStepStats,
) {
    let mut contacts = world.collect_contact_observations(ccd_traces);
    let broadphase_stats = contacts.broadphase_stats;
    let previous_contacts = world.take_active_contacts();
    world.prepare_contact_warm_start(&mut contacts.observations, &previous_contacts);
    let solver_stats = crate::solver::contact::resolve_contacts(
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
        solver_stats,
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
                    warm_start_anchor_drift: 0.0,
                    warm_start_normal_anchor_drift: 0.0,
                    warm_start_tangent_anchor_drift: 0.0,
                    warm_start_normal_impulse: 0.0,
                    warm_start_tangent_impulse: 0.0,
                    source_row_continuity_candidate: false,
                    solver_initial_normal_speed: 0.0,
                    solver_initial_tangent_speed: 0.0,
                    solver_final_normal_speed: 0.0,
                    solver_final_tangent_speed: 0.0,
                    solver_position_bias: 0.0,
                    solver_restitution_bias: 0.0,
                    solver_support_friction_impulse: 0.0,
                    solver_position_correction_depth: 0.0,
                    solver_position_correction_body_a_translation: 0.0,
                    solver_position_correction_body_b_translation: 0.0,
                    solver_normal_impulse_delta: 0.0,
                    solver_tangent_impulse_delta: 0.0,
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
            let exact_previous = previous_contacts.get(&contact.key);
            let fallback_previous = exact_previous
                .is_none()
                .then(|| same_feature_index_warm_start_candidate(previous_contacts, contact))
                .flatten();
            let warm_start = if let Some(previous) = exact_previous.or(fallback_previous) {
                warm_start_transfer(Some(previous), contact, true)
            } else {
                warm_start_transfer(None, contact, previous_pairs.contains(&contact.pair_key))
            };
            contact.warm_start_reason = warm_start.reason;
            contact.warm_start_anchor_drift = warm_start.anchor_drift;
            contact.warm_start_normal_anchor_drift = warm_start.normal_anchor_drift;
            contact.warm_start_tangent_anchor_drift = warm_start.tangent_anchor_drift;
            contact.warm_start_normal_impulse = warm_start.normal_impulse;
            contact.warm_start_tangent_impulse = warm_start.tangent_impulse;
            contact.source_row_continuity_candidate =
                matches!(
                    contact.warm_start_reason,
                    WarmStartCacheReason::MissFeatureId
                ) && source_row_continuity_candidate(&previous_contacts, contact);
            contact.normal_impulse = warm_start.normal_impulse.max(0.0);
            contact.tangent_impulse = warm_start.tangent_impulse;
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
            let existing = previous.remove(&contact.key).or_else(|| {
                persistent_manifold_lifecycle_candidate_key(&previous, &contact)
                    .and_then(|key| previous.remove(&key))
            });
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
                    warm_start_anchor_drift: contact.warm_start_anchor_drift,
                    warm_start_normal_anchor_drift: contact.warm_start_normal_anchor_drift,
                    warm_start_tangent_anchor_drift: contact.warm_start_tangent_anchor_drift,
                    warm_start_normal_impulse: contact.warm_start_normal_impulse,
                    warm_start_tangent_impulse: contact.warm_start_tangent_impulse,
                    source_row_continuity_candidate: contact.source_row_continuity_candidate,
                    solver_normal_impulse: contact.normal_impulse,
                    solver_tangent_impulse: contact.tangent_impulse,
                    solver_initial_normal_speed: contact.solver_initial_normal_speed,
                    solver_initial_tangent_speed: contact.solver_initial_tangent_speed,
                    solver_final_normal_speed: contact.solver_final_normal_speed,
                    solver_final_tangent_speed: contact.solver_final_tangent_speed,
                    solver_position_bias: contact.solver_position_bias,
                    solver_restitution_bias: contact.solver_restitution_bias,
                    solver_support_friction_impulse: contact.solver_support_friction_impulse,
                    solver_position_correction_depth: contact.solver_position_correction_depth,
                    solver_position_correction_body_a_translation: contact
                        .solver_position_correction_body_a_translation,
                    solver_position_correction_body_b_translation: contact
                        .solver_position_correction_body_b_translation,
                    solver_normal_impulse_delta: contact.solver_normal_impulse_delta,
                    solver_tangent_impulse_delta: contact.solver_tangent_impulse_delta,
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
                    warm_start_anchor_drift: contact.warm_start_anchor_drift,
                    warm_start_normal_anchor_drift: contact.warm_start_normal_anchor_drift,
                    warm_start_tangent_anchor_drift: contact.warm_start_tangent_anchor_drift,
                    warm_start_normal_impulse: contact.warm_start_normal_impulse,
                    warm_start_tangent_impulse: contact.warm_start_tangent_impulse,
                    source_row_continuity_candidate: contact.source_row_continuity_candidate,
                    solver_normal_impulse: contact.normal_impulse,
                    solver_tangent_impulse: contact.tangent_impulse,
                    solver_initial_normal_speed: contact.solver_initial_normal_speed,
                    solver_initial_tangent_speed: contact.solver_initial_tangent_speed,
                    solver_final_normal_speed: contact.solver_final_normal_speed,
                    solver_final_tangent_speed: contact.solver_final_tangent_speed,
                    solver_position_bias: contact.solver_position_bias,
                    solver_restitution_bias: contact.solver_restitution_bias,
                    solver_support_friction_impulse: contact.solver_support_friction_impulse,
                    solver_position_correction_depth: contact.solver_position_correction_depth,
                    solver_position_correction_body_a_translation: contact
                        .solver_position_correction_body_a_translation,
                    solver_position_correction_body_b_translation: contact
                        .solver_position_correction_body_b_translation,
                    solver_normal_impulse_delta: contact.solver_normal_impulse_delta,
                    solver_tangent_impulse_delta: contact.solver_tangent_impulse_delta,
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

fn same_feature_index_warm_start_candidate<'a>(
    previous_contacts: &'a BTreeMap<ContactKey, ContactRecord>,
    contact: &ContactObservation,
) -> Option<&'a ContactRecord> {
    previous_contacts
        .values()
        .filter(|record| {
            record.contact.collider_a == contact.collider_a
                && record.contact.collider_b == contact.collider_b
                && record.contact.reduction_reason == contact.reduction_reason
                && record.contact.feature_id.index() == contact.feature_id.index()
                && warm_start_transfer(Some(record), contact, true)
                    .reason
                    .is_hit()
        })
        .min_by(|lhs, rhs| {
            let lhs_drift = contact_anchor_drift(lhs, contact);
            let rhs_drift = contact_anchor_drift(rhs, contact);
            lhs_drift.total_cmp(&rhs_drift)
        })
}

fn persistent_manifold_lifecycle_candidate_key(
    previous_contacts: &BTreeMap<ContactKey, ContactRecord>,
    contact: &ContactObservation,
) -> Option<ContactKey> {
    previous_contacts
        .iter()
        .filter(|(_, record)| persistent_manifold_lifecycle_candidate(record, contact))
        .min_by(|(_, lhs), (_, rhs)| {
            let lhs_drift = contact_anchor_drift(lhs, contact);
            let rhs_drift = contact_anchor_drift(rhs, contact);
            lhs_drift.total_cmp(&rhs_drift)
        })
        .map(|(key, _)| *key)
}

fn persistent_manifold_lifecycle_candidate(
    previous: &ContactRecord,
    contact: &ContactObservation,
) -> bool {
    if contact.is_sensor
        || previous.contact.collider_a != contact.collider_a
        || previous.contact.collider_b != contact.collider_b
        || !clipped_manifold_reduction(previous.contact.reduction_reason)
        || !clipped_manifold_reduction(contact.reduction_reason)
        || !feature_id_edge_swap(previous.contact.feature_id, contact.feature_id)
    {
        return false;
    }

    let previous_normal = previous.contact.normal.normalized_or_zero();
    let current_normal = contact.normal.normalized_or_zero();
    if previous_normal.length() <= FloatNum::EPSILON
        || current_normal.length() <= FloatNum::EPSILON
        || previous_normal.dot(current_normal) < WARM_START_NORMAL_DOT_THRESHOLD
    {
        return false;
    }

    let drift_a = contact.anchor_a - previous.anchor_a;
    let drift_b = contact.anchor_b - previous.anchor_b;
    let local_anchor_drift = drift_a.length().max(drift_b.length());

    local_anchor_drift.is_finite()
        && local_anchor_drift <= PERSISTENT_MANIFOLD_LOCAL_ANCHOR_DRIFT_THRESHOLD
}

fn source_row_continuity_candidate(
    previous_contacts: &BTreeMap<ContactKey, ContactRecord>,
    contact: &ContactObservation,
) -> bool {
    if contact.is_sensor {
        return false;
    }

    previous_contacts
        .values()
        .any(|record| source_row_continuity_candidate_record(record, contact))
}

fn source_row_continuity_candidate_record(
    previous: &ContactRecord,
    contact: &ContactObservation,
) -> bool {
    if previous.contact.collider_a != contact.collider_a
        || previous.contact.collider_b != contact.collider_b
        || feature_id_edge_swap(previous.contact.feature_id, contact.feature_id)
    {
        return false;
    }

    let previous_normal = previous.contact.normal.normalized_or_zero();
    let current_normal = contact.normal.normalized_or_zero();
    if previous_normal.length() <= FloatNum::EPSILON
        || current_normal.length() <= FloatNum::EPSILON
        || previous_normal.dot(current_normal) < WARM_START_NORMAL_DOT_THRESHOLD
    {
        return false;
    }

    let local_anchor_drift = contact_anchor_drift(previous, contact);
    local_anchor_drift.is_finite()
        && local_anchor_drift <= PERSISTENT_MANIFOLD_LOCAL_ANCHOR_DRIFT_THRESHOLD
}

fn contact_anchor_drift(previous: &ContactRecord, contact: &ContactObservation) -> FloatNum {
    let drift_a = contact.anchor_a - previous.anchor_a;
    let drift_b = contact.anchor_b - previous.anchor_b;
    drift_a.length().max(drift_b.length())
}

fn clipped_manifold_reduction(reason: ContactReductionReason) -> bool {
    matches!(
        reason,
        ContactReductionReason::Clipped | ContactReductionReason::DuplicateReduced
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DecodedFeatureId {
    kind: usize,
    reference_edge: usize,
    incident_edge: usize,
}

fn feature_id_edge_swap(
    previous: crate::handles::ContactFeatureId,
    current: crate::handles::ContactFeatureId,
) -> bool {
    let Some(previous) = decode_feature_id(previous) else {
        return false;
    };
    let Some(current) = decode_feature_id(current) else {
        return false;
    };

    previous.kind == current.kind
        && previous.reference_edge == current.incident_edge
        && previous.incident_edge == current.reference_edge
}

fn decode_feature_id(feature_id: crate::handles::ContactFeatureId) -> Option<DecodedFeatureId> {
    let raw = feature_id.index()?;
    Some(DecodedFeatureId {
        kind: (raw >> 24) & 0xff,
        reference_edge: (raw >> 12) & 0xfff,
        incident_edge: raw & 0xfff,
    })
}

#[derive(Clone, Copy, Debug, Default)]
struct WarmStartTransfer {
    reason: WarmStartCacheReason,
    anchor_drift: FloatNum,
    normal_anchor_drift: FloatNum,
    tangent_anchor_drift: FloatNum,
    normal_impulse: FloatNum,
    tangent_impulse: FloatNum,
}

fn warm_start_transfer(
    previous: Option<&ContactRecord>,
    contact: &ContactObservation,
    had_previous_pair: bool,
) -> WarmStartTransfer {
    if contact.is_sensor {
        return warm_start_transfer_result(WarmStartCacheReason::SkippedSensor, 0.0, 0.0);
    }

    let Some(previous) = previous else {
        return if had_previous_pair {
            warm_start_transfer_result(WarmStartCacheReason::MissFeatureId, 0.0, 0.0)
        } else {
            warm_start_transfer_result(WarmStartCacheReason::MissNoPrevious, 0.0, 0.0)
        };
    };

    if previous.contact.warm_start_reason == WarmStartCacheReason::SkippedSensor {
        return warm_start_transfer_result(WarmStartCacheReason::MissPreviousSensor, 0.0, 0.0);
    }

    if !previous.normal_impulse.is_finite() || !previous.tangent_impulse.is_finite() {
        return warm_start_transfer_result(WarmStartCacheReason::DroppedInvalidImpulse, 0.0, 0.0);
    }

    let previous_normal = previous.contact.normal.normalized_or_zero();
    let current_normal = contact.normal.normalized_or_zero();
    // Normal mismatch means the old impulse would push along the wrong
    // constraint row. Feature ids alone are not enough after a normal flip.
    if previous_normal.length() <= FloatNum::EPSILON
        || current_normal.length() <= FloatNum::EPSILON
        || previous_normal.dot(current_normal) < WARM_START_NORMAL_DOT_THRESHOLD
    {
        return warm_start_transfer_result(WarmStartCacheReason::DroppedNormalMismatch, 0.0, 0.0);
    }

    // Feature ids are local geometric names, not raw world-space guarantees.
    // Compare contact anchors relative to both colliders so a pair translating
    // together keeps its cache, while contact movement on either shape drops it.
    let drift_a = contact.anchor_a - previous.anchor_a;
    let drift_b = contact.anchor_b - previous.anchor_b;
    let normal_drift = drift_a
        .dot(current_normal)
        .abs()
        .max(drift_b.dot(current_normal).abs());
    let tangent = current_normal.perp().normalized_or_zero();
    let tangential_drift = drift_a.dot(tangent).abs().max(drift_b.dot(tangent).abs());
    let drift = drift_a.length().max(drift_b.length());
    let anchor_drift = WarmStartTransfer {
        anchor_drift: drift,
        normal_anchor_drift: normal_drift,
        tangent_anchor_drift: tangential_drift,
        ..WarmStartTransfer::default()
    };
    if !drift.is_finite()
        || !normal_drift.is_finite()
        || !tangential_drift.is_finite()
        || normal_drift > WARM_START_POINT_DRIFT_THRESHOLD
    {
        return WarmStartTransfer {
            reason: WarmStartCacheReason::DroppedPointDrift,
            ..anchor_drift
        };
    }
    if tangential_drift > WARM_START_TANGENTIAL_DRIFT_THRESHOLD {
        return WarmStartTransfer {
            reason: WarmStartCacheReason::DroppedPointDrift,
            ..anchor_drift
        };
    }

    WarmStartTransfer {
        reason: WarmStartCacheReason::Hit,
        normal_impulse: previous.normal_impulse,
        tangent_impulse: previous.tangent_impulse,
        ..anchor_drift
    }
}

fn warm_start_transfer_result(
    reason: WarmStartCacheReason,
    normal_impulse: FloatNum,
    tangent_impulse: FloatNum,
) -> WarmStartTransfer {
    WarmStartTransfer {
        reason,
        normal_impulse,
        tangent_impulse,
        ..WarmStartTransfer::default()
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        handles::{ContactFeatureId, ContactId, ManifoldId},
        world::WorldDesc,
    };

    fn test_body(raw: u32) -> BodyHandle {
        BodyHandle::from_raw_parts(raw, 0)
    }

    fn test_collider(raw: u32) -> ColliderHandle {
        ColliderHandle::from_raw_parts(raw, 0)
    }

    fn test_feature(index: u32, point_slot: u32) -> ContactFeatureId {
        ContactFeatureId::from_raw_parts(index, point_slot)
    }

    fn test_contact_event(feature_id: ContactFeatureId) -> ContactEvent {
        ContactEvent {
            contact_id: ContactId::from_raw_parts(1, 0),
            manifold_id: ManifoldId::from_raw_parts(1, 0),
            body_a: test_body(1),
            body_b: test_body(2),
            collider_a: test_collider(1),
            collider_b: test_collider(2),
            feature_id,
            point: Point::new(0.0, 0.0),
            normal: Vector::new(1.0, 0.0),
            depth: 0.01,
            reduction_reason: ContactReductionReason::Clipped,
            warm_start_reason: WarmStartCacheReason::Hit,
            warm_start_anchor_drift: 0.0,
            warm_start_normal_anchor_drift: 0.0,
            warm_start_tangent_anchor_drift: 0.0,
            warm_start_normal_impulse: 0.25,
            warm_start_tangent_impulse: 0.05,
            source_row_continuity_candidate: false,
            solver_normal_impulse: 0.25,
            solver_tangent_impulse: 0.05,
            solver_initial_normal_speed: 0.0,
            solver_initial_tangent_speed: 0.0,
            solver_final_normal_speed: 0.0,
            solver_final_tangent_speed: 0.0,
            solver_position_bias: 0.0,
            solver_restitution_bias: 0.0,
            solver_support_friction_impulse: 0.0,
            solver_position_correction_depth: 0.0,
            solver_position_correction_body_a_translation: 0.0,
            solver_position_correction_body_b_translation: 0.0,
            solver_normal_impulse_delta: 0.0,
            solver_tangent_impulse_delta: 0.0,
            normal_impulse_clamped: false,
            tangent_impulse_clamped: false,
            restitution_velocity_threshold: 0.0,
            restitution_applied: false,
            generic_convex_trace: None,
            ccd_trace: None,
        }
    }

    fn test_contact_observation(feature_id: ContactFeatureId) -> ContactObservation {
        let collider_a = test_collider(1);
        let collider_b = test_collider(2);
        ContactObservation {
            key: ContactKey::new(collider_a, collider_b, feature_id),
            pair_key: ContactPairKey::new(collider_a, collider_b),
            body_a: test_body(1),
            body_b: test_body(2),
            collider_a,
            collider_b,
            anchor_a: Vector::new(0.25, 0.0),
            anchor_b: Vector::new(-0.25, 0.0),
            point: Point::new(0.0, 0.0),
            normal: Vector::new(1.0, 0.0),
            depth: 0.01,
            feature_id,
            reduction_reason: ContactReductionReason::Clipped,
            is_sensor: false,
            material: Material::default(),
            normal_impulse: 0.0,
            tangent_impulse: 0.0,
            warm_start_reason: WarmStartCacheReason::MissNoPrevious,
            warm_start_anchor_drift: 0.0,
            warm_start_normal_anchor_drift: 0.0,
            warm_start_tangent_anchor_drift: 0.0,
            warm_start_normal_impulse: 0.0,
            warm_start_tangent_impulse: 0.0,
            source_row_continuity_candidate: false,
            solver_initial_normal_speed: 0.0,
            solver_initial_tangent_speed: 0.0,
            solver_final_normal_speed: 0.0,
            solver_final_tangent_speed: 0.0,
            solver_position_bias: 0.0,
            solver_restitution_bias: 0.0,
            solver_support_friction_impulse: 0.0,
            solver_position_correction_depth: 0.0,
            solver_position_correction_body_a_translation: 0.0,
            solver_position_correction_body_b_translation: 0.0,
            solver_normal_impulse_delta: 0.0,
            solver_tangent_impulse_delta: 0.0,
            normal_impulse_clamped: false,
            tangent_impulse_clamped: false,
            restitution_velocity_threshold: 0.0,
            restitution_applied: false,
            generic_convex_trace: None,
            ccd_trace: None,
        }
    }

    #[test]
    fn warm_start_fallback_transfers_between_same_feature_index_point_slots() {
        let previous_feature = test_feature(0x0100_1003, 0);
        let current_feature = test_feature(0x0100_1003, 1);
        let previous_record = ContactRecord {
            contact: test_contact_event(previous_feature),
            anchor_a: Vector::new(0.25, 0.0),
            anchor_b: Vector::new(-0.25, 0.0),
            normal_impulse: 0.25,
            tangent_impulse: 0.05,
        };
        let mut previous_contacts = BTreeMap::new();
        previous_contacts.insert(
            ContactKey::new(test_collider(1), test_collider(2), previous_feature),
            previous_record,
        );
        let mut contacts = vec![test_contact_observation(current_feature)];
        let world = World::new(WorldDesc::default());

        world.prepare_contact_warm_start(&mut contacts, &previous_contacts);

        assert_eq!(contacts[0].warm_start_reason, WarmStartCacheReason::Hit);
        assert_eq!(contacts[0].warm_start_normal_impulse, 0.25);
        assert_eq!(contacts[0].warm_start_tangent_impulse, 0.05);
        assert!(!contacts[0].source_row_continuity_candidate);
    }

    #[test]
    fn source_row_continuity_candidate_marks_non_edge_swap_feature_miss() {
        let previous_feature = test_feature(0x0100_1003, 0);
        let current_feature = test_feature(0x0100_1004, 0);
        let previous_record = ContactRecord {
            contact: test_contact_event(previous_feature),
            anchor_a: Vector::new(0.25, 0.0),
            anchor_b: Vector::new(-0.25, 0.0),
            normal_impulse: 0.25,
            tangent_impulse: 0.05,
        };
        let mut previous_contacts = BTreeMap::new();
        previous_contacts.insert(
            ContactKey::new(test_collider(1), test_collider(2), previous_feature),
            previous_record,
        );
        let mut contacts = vec![test_contact_observation(current_feature)];
        let world = World::new(WorldDesc::default());

        world.prepare_contact_warm_start(&mut contacts, &previous_contacts);

        assert_eq!(
            contacts[0].warm_start_reason,
            WarmStartCacheReason::MissFeatureId
        );
        assert!(contacts[0].source_row_continuity_candidate);
    }

    #[test]
    fn source_row_continuity_candidate_excludes_edge_swap_lifecycle_path() {
        let previous_feature = test_feature(0x0100_1003, 0);
        let current_feature = test_feature(0x0100_3001, 0);
        let previous_record = ContactRecord {
            contact: test_contact_event(previous_feature),
            anchor_a: Vector::new(0.25, 0.0),
            anchor_b: Vector::new(-0.25, 0.0),
            normal_impulse: 0.25,
            tangent_impulse: 0.05,
        };
        let mut previous_contacts = BTreeMap::new();
        previous_contacts.insert(
            ContactKey::new(test_collider(1), test_collider(2), previous_feature),
            previous_record,
        );
        let mut contacts = vec![test_contact_observation(current_feature)];
        let world = World::new(WorldDesc::default());

        world.prepare_contact_warm_start(&mut contacts, &previous_contacts);

        assert_eq!(
            contacts[0].warm_start_reason,
            WarmStartCacheReason::MissFeatureId
        );
        assert!(!contacts[0].source_row_continuity_candidate);
    }
}
