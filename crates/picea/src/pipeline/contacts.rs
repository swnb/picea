use std::collections::{BTreeMap, BTreeSet};

use crate::{
    body::Pose,
    collider::{CollisionFilter, Material, ShapeAabb, SharedShape},
    events::{
        CcdTrace, ContactEvent, ContactLifecycleReason, ContactReductionReason, GenericConvexTrace,
        SleepTransitionReason, SourceRowContinuityReason, WarmStartCacheReason, WorldEvent,
    },
    handles::{BodyHandle, ColliderHandle, ContactId, ManifoldId},
    math::{point::Point, vector::Vector, FloatNum},
    pipeline::{
        broadphase::{BroadphaseStats, ColliderProxy},
        island::SolverStepStats,
        narrowphase::{
            contact_from_shapes_with_cached_vertices, decode_feature_id, ContactManifoldGeometry,
            ContactPointGeometry,
        },
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

#[cfg(test)]
std::thread_local! {
    static LIFECYCLE_CANDIDATE_EVALUATION_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn reset_lifecycle_candidate_evaluation_count() {
    LIFECYCLE_CANDIDATE_EVALUATION_COUNT.with(|count| count.set(0));
}

#[cfg(test)]
fn lifecycle_candidate_evaluation_count() -> usize {
    LIFECYCLE_CANDIDATE_EVALUATION_COUNT.with(std::cell::Cell::get)
}

#[derive(Clone, Debug)]
pub(crate) struct ContactObservation {
    pub(crate) key: ContactKey,
    pub(crate) pair_key: ContactPairKey,
    pub(crate) body_a: BodyHandle,
    pub(crate) body_b: BodyHandle,
    pub(crate) collider_a: ColliderHandle,
    pub(crate) collider_b: ColliderHandle,
    pub(crate) geometry_revision_a: u64,
    pub(crate) geometry_revision_b: u64,
    pub(crate) anchor_a: Vector,
    pub(crate) anchor_b: Vector,
    pub(crate) witness_a_local: Point,
    pub(crate) witness_b_local: Point,
    pub(crate) normal_a_local: Vector,
    pub(crate) normal_b_local: Vector,
    pub(crate) point: Point,
    pub(crate) normal: Vector,
    pub(crate) depth: FloatNum,
    pub(crate) signed_separation: FloatNum,
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
    pub(crate) source_row_continuity_reason: SourceRowContinuityReason,
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
    pub(crate) base_normal_solve_executed: bool,
    pub(crate) generic_convex_trace: Option<GenericConvexTrace>,
    pub(crate) ccd_trace: Option<CcdTrace>,
}

#[derive(Clone, Debug)]
struct ColliderSnapshot {
    handle: ColliderHandle,
    body: BodyHandle,
    geometry_revision: u64,
    shape: SharedShape,
    world_pose: Pose,
    aabb: ShapeAabb,
    convex_vertices: Option<Vec<Point>>,
    current_world_pose: Pose,
    current_aabb: ShapeAabb,
    current_convex_vertices: Option<Vec<Point>>,
    material: Material,
    filter: CollisionFilter,
    is_sensor: bool,
}

pub(crate) struct PendingContactPhases {
    observations: Vec<ContactObservation>,
    previous_contacts: BTreeMap<ContactKey, ContactRecord>,
    broadphase_stats: BroadphaseStats,
    solver_stats: SolverStepStats,
}

pub(crate) fn run_contact_solve_phase(
    world: &mut World,
    config: &StepConfig,
    wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
    ccd_traces: &[CcdTrace],
    predicted_body_poses: &BTreeMap<BodyHandle, Pose>,
) -> PendingContactPhases {
    let mut contacts = world.collect_contact_observations(ccd_traces, predicted_body_poses);
    let previous_contacts = world.take_active_contacts();
    world.prepare_contact_warm_start(&mut contacts.observations, &previous_contacts);
    let solver_stats = crate::solver::contact::resolve_contacts(
        world,
        &mut contacts.observations,
        config,
        wake_reasons,
    );
    PendingContactPhases {
        observations: contacts.observations,
        previous_contacts,
        broadphase_stats: contacts.broadphase_stats,
        solver_stats,
    }
}

pub(crate) fn finalize_contact_phases(
    world: &mut World,
    mut pending: PendingContactPhases,
) -> (
    Vec<WorldEvent>,
    usize,
    usize,
    BroadphaseStats,
    WarmStartStats,
    SolverStepStats,
) {
    world.finalize_contact_observations(&mut pending.observations);
    let (events, contact_count, manifold_count, warm_start_stats) =
        world.refresh_contact_events(pending.observations, pending.previous_contacts);
    (
        events,
        contact_count,
        manifold_count,
        pending.broadphase_stats,
        warm_start_stats,
        pending.solver_stats,
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
        predicted_body_poses: &BTreeMap<BodyHandle, Pose>,
    ) -> ContactPhaseObservations {
        let ccd_traces = ccd_trace_map(ccd_traces);
        let colliders = self.live_collider_snapshots(predicted_body_poses);
        let proxies = colliders
            .iter()
            .map(|collider| ColliderProxy {
                handle: collider.handle,
                // Preserve solver-start overlap while widening discovery just
                // enough to include the pose reached by this fixed step.
                aabb: union_aabb(collider.current_aabb, collider.aabb),
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
            let current_contact = contact_from_shapes_with_cached_vertices(
                &collider_a.shape,
                collider_a.current_world_pose,
                collider_a.current_aabb,
                collider_a.current_convex_vertices.as_deref(),
                &collider_b.shape,
                collider_b.current_world_pose,
                collider_b.current_aabb,
                collider_b.current_convex_vertices.as_deref(),
            );
            let predicted_contact = contact_from_shapes_with_cached_vertices(
                &collider_a.shape,
                collider_a.world_pose,
                collider_a.aabb,
                collider_a.convex_vertices.as_deref(),
                &collider_b.shape,
                collider_b.world_pose,
                collider_b.aabb,
                collider_b.convex_vertices.as_deref(),
            );
            if current_contact.is_none() && predicted_contact.is_none() {
                broadphase.stats.narrowphase_drop_count += 1;
                continue;
            }

            let (
                ordered_a,
                ordered_b,
                ordered_body_a,
                ordered_body_b,
                ordered_pose_a,
                ordered_pose_b,
                ordered_current_pose_a,
                ordered_current_pose_b,
                ordered_geometry_revision_a,
                ordered_geometry_revision_b,
            ) = if collider_a.handle <= collider_b.handle {
                (
                    collider_a.handle,
                    collider_b.handle,
                    collider_a.body,
                    collider_b.body,
                    collider_a.world_pose,
                    collider_b.world_pose,
                    collider_a.current_world_pose,
                    collider_b.current_world_pose,
                    collider_a.geometry_revision,
                    collider_b.geometry_revision,
                )
            } else {
                (
                    collider_b.handle,
                    collider_a.handle,
                    collider_b.body,
                    collider_a.body,
                    collider_b.world_pose,
                    collider_a.world_pose,
                    collider_b.current_world_pose,
                    collider_a.current_world_pose,
                    collider_b.geometry_revision,
                    collider_a.geometry_revision,
                )
            };

            let mut pair_observations = BTreeMap::new();
            for (contact, predicted) in current_contact
                .iter()
                .map(|contact| (contact, false))
                .chain(predicted_contact.iter().map(|contact| (contact, true)))
            {
                let normal = if collider_a.handle <= collider_b.handle {
                    contact.normal
                } else {
                    -contact.normal
                };
                for point in &contact.points {
                    let key = ContactKey::new(ordered_a, ordered_b, point.feature_id);
                    // A real solver-start feature owns the row. Predicted geometry
                    // only contributes features not already confirmed at that time.
                    if predicted && pair_observations.contains_key(&key) {
                        continue;
                    }
                    let current_depth = current_contact
                        .as_ref()
                        .and_then(|current| {
                            current
                                .points
                                .iter()
                                .find(|candidate| candidate.feature_id == point.feature_id)
                        })
                        .map(|point| point.depth)
                        .unwrap_or(0.0);
                    let observation = contact_observation_from_point(
                        ordered_a,
                        ordered_b,
                        ordered_body_a,
                        ordered_body_b,
                        ordered_geometry_revision_a,
                        ordered_geometry_revision_b,
                        if predicted {
                            ordered_pose_a
                        } else {
                            ordered_current_pose_a
                        },
                        if predicted {
                            ordered_pose_b
                        } else {
                            ordered_current_pose_b
                        },
                        ordered_current_pose_a,
                        ordered_current_pose_b,
                        point.point,
                        normal,
                        point.depth,
                        current_depth,
                        point.feature_id,
                        contact.reduction_reason,
                        collider_a.is_sensor || collider_b.is_sensor,
                        combine_materials(collider_a.material, collider_b.material),
                        contact.generic_convex_trace,
                        ccd_traces.get(&(ordered_a, ordered_b)).copied(),
                        ContactPairKey::new(ordered_a, ordered_b),
                    );
                    pair_observations.insert(key, observation);
                }
            }
            observations.extend(pair_observations.into_values());
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
        let warm_reservations =
            reserve_previous_matches(contacts, previous_contacts, ReservationPolicy::WarmStart);
        for (index, contact) in contacts.iter_mut().enumerate() {
            let reserved = warm_reservations
                .reserved
                .get(index)
                .and_then(Option::as_ref);
            let previous = reserved.and_then(|reserved| previous_contacts.get(&reserved.key));
            let had_previous_pair = warm_reservations
                .previous_pairs
                .contains_key(&contact.pair_key);
            let mut warm_start = warm_start_transfer(previous, contact, had_previous_pair);
            if reserved
                .is_some_and(|reserved| reserved.kind == PreviousMatchKind::PersistentEdgeSwap)
            {
                warm_start.tangent_impulse = 0.0;
            }
            contact.warm_start_reason = warm_start.reason;
            contact.warm_start_anchor_drift = warm_start.anchor_drift;
            contact.warm_start_normal_anchor_drift = warm_start.normal_anchor_drift;
            contact.warm_start_tangent_anchor_drift = warm_start.tangent_anchor_drift;
            contact.warm_start_normal_impulse = warm_start.normal_impulse;
            contact.warm_start_tangent_impulse = warm_start.tangent_impulse;
            contact.normal_impulse = warm_start.normal_impulse.max(0.0);
            contact.tangent_impulse = warm_start.tangent_impulse;
        }

        let source_reservations =
            reserve_previous_matches(contacts, previous_contacts, ReservationPolicy::SourceRow);
        for (index, contact) in contacts.iter_mut().enumerate() {
            let warm_match_kind = warm_reservations
                .reserved
                .get(index)
                .and_then(Option::as_ref)
                .map(|reserved| reserved.kind);
            let warm_hit_source_reason = (contact.warm_start_reason == WarmStartCacheReason::Hit)
                .then(|| match warm_match_kind {
                    Some(PreviousMatchKind::Exact | PreviousMatchKind::SameFeatureIndex) => {
                        Some(SourceRowContinuityReason::Unknown)
                    }
                    Some(PreviousMatchKind::PersistentEdgeSwap) => {
                        Some(SourceRowContinuityReason::EdgeSwap)
                    }
                    Some(PreviousMatchKind::SourceRowCandidate) | None => None,
                })
                .flatten();
            if let Some(reason) = warm_hit_source_reason {
                contact.source_row_continuity_candidate = false;
                contact.source_row_continuity_reason = reason;
                continue;
            }
            let reserved = source_reservations
                .reserved
                .get(index)
                .and_then(Option::as_ref);
            let is_candidate = reserved.is_some_and(|reserved| {
                reserved.kind == PreviousMatchKind::SourceRowCandidate
                    && contact.warm_start_reason == WarmStartCacheReason::MissFeatureId
            });
            contact.source_row_continuity_candidate = is_candidate;
            contact.source_row_continuity_reason = if is_candidate {
                SourceRowContinuityReason::Candidate
            } else {
                source_row_unreserved_reason(
                    previous_contacts,
                    source_reservations
                        .previous_pairs
                        .get(&contact.pair_key)
                        .map(Vec::as_slice),
                    contact,
                )
            };
        }
    }

    fn refresh_contact_events(
        &mut self,
        contacts: Vec<ContactObservation>,
        mut previous: BTreeMap<ContactKey, ContactRecord>,
    ) -> (Vec<WorldEvent>, usize, usize, WarmStartStats) {
        let lifecycle_reservations =
            reserve_previous_matches(&contacts, &previous, ReservationPolicy::Lifecycle);
        let mut pair_manifold_ids = BTreeMap::new();
        for (pair_key, current_indices) in &lifecycle_reservations.current_pairs {
            let Some(contact) = current_indices
                .first()
                .and_then(|index| contacts.get(*index))
            else {
                continue;
            };
            let Some(previous_keys) = lifecycle_reservations.previous_pairs.get(pair_key) else {
                continue;
            };
            if let Some(manifold_id) = previous_keys.iter().find_map(|key| {
                previous
                    .get(key)
                    .filter(|record| geometry_compatible(record, contact))
                    .map(|record| record.contact.manifold_id)
            }) {
                pair_manifold_ids.insert(*pair_key, manifold_id);
            }
        }
        let mut next = BTreeMap::new();
        let mut events = Vec::new();
        let mut warm_start_stats = WarmStartStats::default();

        for (index, contact) in contacts.into_iter().enumerate() {
            let existing = lifecycle_reservations
                .reserved
                .get(index)
                .and_then(Option::as_ref)
                .and_then(|reserved| {
                    let reason = match reserved.kind {
                        PreviousMatchKind::Exact => Some(ContactLifecycleReason::ExactFeature),
                        PreviousMatchKind::PersistentEdgeSwap => {
                            Some(ContactLifecycleReason::PersistentEdgeSwap)
                        }
                        PreviousMatchKind::SameFeatureIndex
                        | PreviousMatchKind::SourceRowCandidate => None,
                    }?;
                    previous
                        .remove(&reserved.key)
                        .map(|record| (record, reason))
                });
            let is_persisted = existing.is_some();
            warm_start_stats.record(contact.warm_start_reason);
            let event = if let Some((existing, lifecycle_reason)) = existing {
                contact_event(
                    existing.contact.contact_id,
                    existing.contact.manifold_id,
                    lifecycle_reason,
                    &contact,
                )
            } else {
                let manifold_id = *pair_manifold_ids
                    .entry(contact.pair_key)
                    .or_insert_with(|| self.alloc_next_manifold_id());
                contact_event(
                    self.alloc_next_contact_id(),
                    manifold_id,
                    ContactLifecycleReason::Started,
                    &contact,
                )
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
                    geometry_revision_a: contact.geometry_revision_a,
                    geometry_revision_b: contact.geometry_revision_b,
                    is_sensor: contact.is_sensor,
                    witness_a_local: contact.witness_a_local,
                    witness_b_local: contact.witness_b_local,
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

    fn finalize_contact_observations(&self, contacts: &mut Vec<ContactObservation>) {
        let mut pairs = BTreeMap::<ContactPairKey, Vec<ContactObservation>>::new();
        for contact in std::mem::take(contacts) {
            pairs.entry(contact.pair_key).or_default().push(contact);
        }

        for sources in pairs.values_mut() {
            sources.sort_by_key(|contact| contact.key);
            let Some(prototype) = sources.first().cloned() else {
                continue;
            };
            let Some((pose_a, pose_b)) = self.contact_collider_poses(&prototype) else {
                continue;
            };

            if let Some(manifold) = self.final_contact_manifold(&prototype) {
                let mut available_sources = sources
                    .drain(..)
                    .map(Some)
                    .collect::<Vec<Option<ContactObservation>>>();
                let final_points = manifold
                    .points
                    .into_iter()
                    .map(|point| (point.feature_id, point))
                    .collect::<BTreeMap<_, _>>()
                    .into_values()
                    .collect::<Vec<_>>();
                let matched_sources = reserve_final_sources(
                    &mut available_sources,
                    pose_a,
                    pose_b,
                    manifold.normal,
                    &final_points,
                );
                for (point, matched_source) in final_points.into_iter().zip(matched_sources) {
                    let source_trust_frame = matched_source
                        .as_ref()
                        .map(|source| (source.depth, source.anchor_a, source.anchor_b));
                    let mut contact = matched_source.unwrap_or_else(|| {
                        contact_observation_from_point(
                            prototype.collider_a,
                            prototype.collider_b,
                            prototype.body_a,
                            prototype.body_b,
                            prototype.geometry_revision_a,
                            prototype.geometry_revision_b,
                            pose_a,
                            pose_b,
                            pose_a,
                            pose_b,
                            point.point,
                            manifold.normal,
                            point.depth,
                            point.depth,
                            point.feature_id,
                            manifold.reduction_reason,
                            prototype.is_sensor,
                            prototype.material,
                            manifold.generic_convex_trace,
                            prototype.ccd_trace,
                            prototype.pair_key,
                        )
                    });
                    apply_final_manifold_geometry(
                        &mut contact,
                        pose_a,
                        pose_b,
                        manifold.normal,
                        point,
                        manifold.reduction_reason,
                        manifold.generic_convex_trace,
                    );
                    if let Some((depth, anchor_a, anchor_b)) = source_trust_frame {
                        // Position correction and warm-start trust are evaluated
                        // at solver start. Final geometry owns cache identity and
                        // local witnesses, but must not move the current row inputs.
                        contact.depth = depth;
                        contact.anchor_a = anchor_a;
                        contact.anchor_b = anchor_b;
                    }
                    contacts.push(contact);
                }
            } else {
                for mut contact in sources.drain(..) {
                    if !confirmed_solver_interaction(&contact) {
                        continue;
                    }
                    let witness_a = pose_a.transform_point(contact.witness_a_local);
                    let witness_b = pose_b.transform_point(contact.witness_b_local);
                    let signed_separation = (witness_a - witness_b).dot(contact.normal);
                    if !signed_separation.is_finite() {
                        continue;
                    }
                    contact.point = midpoint(witness_a, witness_b);
                    contact.signed_separation = signed_separation;
                    contact.depth = 0.0;
                    contact.normal_a_local = contact.normal.rotated(-pose_a.angle());
                    contact.normal_b_local = contact.normal.rotated(-pose_b.angle());
                    contacts.push(contact);
                }
            }
        }
        contacts.sort_by_key(|contact| contact.key);
    }

    fn contact_collider_poses(&self, contact: &ContactObservation) -> Option<(Pose, Pose)> {
        let collider_a = self.collider_record(contact.collider_a).ok()?;
        let collider_b = self.collider_record(contact.collider_b).ok()?;
        let body_a = self.body_record(collider_a.body).ok()?;
        let body_b = self.body_record(collider_b.body).ok()?;
        Some((
            collider_a.world_pose(body_a.pose),
            collider_b.world_pose(body_b.pose),
        ))
    }

    fn final_contact_manifold(
        &self,
        contact: &ContactObservation,
    ) -> Option<ContactManifoldGeometry> {
        let collider_a = self.collider_record(contact.collider_a).ok()?;
        let collider_b = self.collider_record(contact.collider_b).ok()?;
        let body_a = self.body_record(collider_a.body).ok()?;
        let body_b = self.body_record(collider_b.body).ok()?;
        let geometry_a = collider_a.derived_geometry(body_a.pose);
        let geometry_b = collider_b.derived_geometry(body_b.pose);
        contact_from_shapes_with_cached_vertices(
            &collider_a.shape,
            collider_a.world_pose(body_a.pose),
            geometry_a.aabb,
            geometry_a.convex_vertices.as_deref(),
            &collider_b.shape,
            collider_b.world_pose(body_b.pose),
            geometry_b.aabb,
            geometry_b.convex_vertices.as_deref(),
        )
    }

    fn live_collider_snapshots(
        &self,
        predicted_body_poses: &BTreeMap<BodyHandle, Pose>,
    ) -> Vec<ColliderSnapshot> {
        self.collider_records()
            .filter_map(|(handle, record)| {
                let body = self.body_record(record.body).ok()?;
                let predicted_body_pose = predicted_body_poses
                    .get(&record.body)
                    .copied()
                    .unwrap_or(body.pose);
                let world_pose = predicted_body_pose.compose(record.local_pose);
                let geometry = record.derived_geometry(predicted_body_pose);
                let current_world_pose = body.pose.compose(record.local_pose);
                let current_geometry = record.derived_geometry(body.pose);
                Some(ColliderSnapshot {
                    handle,
                    body: record.body,
                    geometry_revision: record.geometry_revision(),
                    shape: record.shape.clone(),
                    world_pose,
                    aabb: geometry.aabb,
                    convex_vertices: geometry.convex_vertices,
                    current_world_pose,
                    current_aabb: current_geometry.aabb,
                    current_convex_vertices: current_geometry.convex_vertices,
                    material: record.material,
                    filter: record.filter,
                    is_sensor: record.is_sensor,
                })
            })
            .collect()
    }
}

#[allow(clippy::too_many_arguments)]
fn contact_observation_from_point(
    ordered_a: ColliderHandle,
    ordered_b: ColliderHandle,
    ordered_body_a: BodyHandle,
    ordered_body_b: BodyHandle,
    geometry_revision_a: u64,
    geometry_revision_b: u64,
    source_pose_a: Pose,
    source_pose_b: Pose,
    current_pose_a: Pose,
    current_pose_b: Pose,
    point: Point,
    normal: Vector,
    source_depth: FloatNum,
    current_depth: FloatNum,
    feature_id: crate::handles::ContactFeatureId,
    reduction_reason: ContactReductionReason,
    is_sensor: bool,
    material: Material,
    generic_convex_trace: Option<GenericConvexTrace>,
    ccd_trace: Option<CcdTrace>,
    pair_key: ContactPairKey,
) -> ContactObservation {
    let half_depth = normal * (source_depth * 0.5);
    let witness_a_local = source_pose_a.inverse_transform_point(point - half_depth);
    let witness_b_local = source_pose_b.inverse_transform_point(point + half_depth);
    let witness_a = current_pose_a.transform_point(witness_a_local);
    let witness_b = current_pose_b.transform_point(witness_b_local);
    let current_point = midpoint(witness_a, witness_b);
    let signed_separation = (witness_a - witness_b).dot(normal);
    ContactObservation {
        key: ContactKey::new(ordered_a, ordered_b, feature_id),
        pair_key,
        body_a: ordered_body_a,
        body_b: ordered_body_b,
        collider_a: ordered_a,
        collider_b: ordered_b,
        geometry_revision_a,
        geometry_revision_b,
        anchor_a: current_point - current_pose_a.point(),
        anchor_b: current_point - current_pose_b.point(),
        witness_a_local,
        witness_b_local,
        normal_a_local: normal.rotated(-current_pose_a.angle()),
        normal_b_local: normal.rotated(-current_pose_b.angle()),
        point: current_point,
        normal,
        depth: current_depth,
        signed_separation,
        feature_id,
        reduction_reason,
        is_sensor,
        material,
        normal_impulse: 0.0,
        tangent_impulse: 0.0,
        warm_start_reason: WarmStartCacheReason::MissNoPrevious,
        warm_start_anchor_drift: 0.0,
        warm_start_normal_anchor_drift: 0.0,
        warm_start_tangent_anchor_drift: 0.0,
        warm_start_normal_impulse: 0.0,
        warm_start_tangent_impulse: 0.0,
        source_row_continuity_candidate: false,
        source_row_continuity_reason: SourceRowContinuityReason::Unknown,
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
        base_normal_solve_executed: false,
        generic_convex_trace,
        ccd_trace,
    }
}

fn midpoint(a: Point, b: Point) -> Point {
    Point::from((Vector::from(a) + Vector::from(b)) * 0.5)
}

fn confirmed_solver_interaction(contact: &ContactObservation) -> bool {
    !contact.is_sensor
        && contact.base_normal_solve_executed
        && contact.normal_impulse.is_finite()
        && contact.normal_impulse > FloatNum::EPSILON
}

fn final_source_match(
    sources: &[Option<ContactObservation>],
    pose_a: Pose,
    pose_b: Pose,
    final_normal: Vector,
    final_point: ContactPointGeometry,
) -> Option<usize> {
    if let Some((index, _)) = sources
        .iter()
        .enumerate()
        .filter_map(|(index, source)| source.as_ref().map(|source| (index, source)))
        .filter(|(_, source)| source.feature_id == final_point.feature_id)
        .min_by_key(|(_, source)| source.key)
    {
        return Some(index);
    }

    let final_normal = final_normal.normalized_or_zero();
    if final_normal.length() <= FloatNum::EPSILON {
        return None;
    }
    let half_depth = final_normal * (final_point.depth * 0.5);
    let final_witness_a = final_point.point - half_depth;
    let final_witness_b = final_point.point + half_depth;
    let final_witness_a_local = pose_a.inverse_transform_point(final_witness_a);
    let final_witness_b_local = pose_b.inverse_transform_point(final_witness_b);
    sources
        .iter()
        .enumerate()
        .filter_map(|(index, source)| source.as_ref().map(|source| (index, source)))
        .filter_map(|(index, source)| {
            let source_normal = source.normal.normalized_or_zero();
            if source_normal.length() <= FloatNum::EPSILON
                || source_normal.dot(final_normal) < WARM_START_NORMAL_DOT_THRESHOLD
            {
                return None;
            }
            let local_witness_drift = (source.witness_a_local - final_witness_a_local)
                .length()
                .max((source.witness_b_local - final_witness_b_local).length());
            if !local_witness_drift.is_finite()
                || local_witness_drift > PERSISTENT_MANIFOLD_LOCAL_ANCHOR_DRIFT_THRESHOLD
            {
                return None;
            }
            let source_witness_a = pose_a.transform_point(source.witness_a_local);
            let source_witness_b = pose_b.transform_point(source.witness_b_local);
            let source_point = midpoint(source_witness_a, source_witness_b);
            let distance = (source_point - final_point.point).length()
                + (source_witness_a - final_witness_a).length()
                + (source_witness_b - final_witness_b).length();
            distance
                .is_finite()
                .then_some((index, source.key, distance))
        })
        .min_by(|(_, lhs_key, lhs_distance), (_, rhs_key, rhs_distance)| {
            lhs_distance
                .total_cmp(rhs_distance)
                .then_with(|| lhs_key.cmp(rhs_key))
        })
        .map(|(index, _, _)| index)
}

fn reserve_final_sources(
    sources: &mut [Option<ContactObservation>],
    pose_a: Pose,
    pose_b: Pose,
    final_normal: Vector,
    final_points: &[ContactPointGeometry],
) -> Vec<Option<ContactObservation>> {
    let mut matched = (0..final_points.len()).map(|_| None).collect::<Vec<_>>();

    // Exact identities are reserved globally before any distance fallback so
    // an earlier feature cannot steal a source row owned by a later feature.
    for (point_index, point) in final_points.iter().enumerate() {
        let exact_index = sources
            .iter()
            .enumerate()
            .filter_map(|(index, source)| source.as_ref().map(|source| (index, source)))
            .filter(|(_, source)| source.feature_id == point.feature_id)
            .min_by_key(|(_, source)| source.key)
            .map(|(index, _)| index);
        if let Some(index) = exact_index {
            matched[point_index] = sources[index].take();
        }
    }

    for (point_index, point) in final_points.iter().enumerate() {
        if matched[point_index].is_some() {
            continue;
        }
        if let Some(index) = final_source_match(sources, pose_a, pose_b, final_normal, *point) {
            matched[point_index] = sources[index].take();
        }
    }
    matched
}

fn apply_final_manifold_geometry(
    contact: &mut ContactObservation,
    pose_a: Pose,
    pose_b: Pose,
    normal: Vector,
    point: ContactPointGeometry,
    reduction_reason: ContactReductionReason,
    generic_convex_trace: Option<GenericConvexTrace>,
) {
    let half_depth = normal * (point.depth * 0.5);
    let witness_a = point.point - half_depth;
    let witness_b = point.point + half_depth;
    contact.key = ContactKey::new(contact.collider_a, contact.collider_b, point.feature_id);
    contact.feature_id = point.feature_id;
    contact.normal = normal;
    contact.point = point.point;
    contact.depth = point.depth;
    contact.witness_a_local = pose_a.inverse_transform_point(witness_a);
    contact.witness_b_local = pose_b.inverse_transform_point(witness_b);
    contact.normal_a_local = normal.rotated(-pose_a.angle());
    contact.normal_b_local = normal.rotated(-pose_b.angle());
    contact.signed_separation = (witness_a - witness_b).dot(normal);
    contact.anchor_a = point.point - pose_a.point();
    contact.anchor_b = point.point - pose_b.point();
    contact.reduction_reason = reduction_reason;
    contact.generic_convex_trace = generic_convex_trace;
}

fn union_aabb(a: ShapeAabb, b: ShapeAabb) -> ShapeAabb {
    ShapeAabb {
        min: Point::new(a.min.x().min(b.min.x()), a.min.y().min(b.min.y())),
        max: Point::new(a.max.x().max(b.max.x()), a.max.y().max(b.max.y())),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum PreviousMatchKind {
    Exact,
    SameFeatureIndex,
    PersistentEdgeSwap,
    SourceRowCandidate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReservationPolicy {
    WarmStart,
    Lifecycle,
    SourceRow,
}

#[derive(Clone, Copy, Debug)]
struct ReservedPrevious {
    key: ContactKey,
    kind: PreviousMatchKind,
}

struct PairScopedReservations {
    reserved: Vec<Option<ReservedPrevious>>,
    current_pairs: BTreeMap<ContactPairKey, Vec<usize>>,
    previous_pairs: BTreeMap<ContactPairKey, Vec<ContactKey>>,
}

#[derive(Clone, Copy, Debug)]
struct CandidateEdge {
    previous_index: usize,
    previous_key: ContactKey,
    kind: PreviousMatchKind,
    drift: FloatNum,
}

fn reserve_previous_matches(
    contacts: &[ContactObservation],
    previous_contacts: &BTreeMap<ContactKey, ContactRecord>,
    policy: ReservationPolicy,
) -> PairScopedReservations {
    let mut current_pairs = BTreeMap::<ContactPairKey, Vec<usize>>::new();
    for (index, contact) in contacts.iter().enumerate() {
        if policy == ReservationPolicy::SourceRow
            && contact.warm_start_reason != WarmStartCacheReason::MissFeatureId
        {
            continue;
        }
        current_pairs
            .entry(contact.pair_key)
            .or_default()
            .push(index);
    }
    for indices in current_pairs.values_mut() {
        indices.sort_by_key(|index| (contacts.get(*index).map(|contact| contact.key), *index));
    }

    let mut previous_pairs = BTreeMap::<ContactPairKey, Vec<ContactKey>>::new();
    for key in previous_contacts.keys().copied() {
        previous_pairs.entry(key.pair).or_default().push(key);
    }

    let mut reserved = vec![None; contacts.len()];
    for (pair_key, current_indices) in &current_pairs {
        let Some(previous_keys) = previous_pairs.get(pair_key) else {
            continue;
        };
        reserve_pair_matches(
            contacts,
            previous_contacts,
            current_indices,
            previous_keys,
            policy,
            &mut reserved,
        );
    }

    PairScopedReservations {
        reserved,
        current_pairs,
        previous_pairs,
    }
}

fn reserve_pair_matches(
    contacts: &[ContactObservation],
    previous_contacts: &BTreeMap<ContactKey, ContactRecord>,
    current_indices: &[usize],
    previous_keys: &[ContactKey],
    policy: ReservationPolicy,
    reserved: &mut [Option<ReservedPrevious>],
) {
    let mut previous_used = vec![false; previous_keys.len()];
    let mut residual_current_indices = Vec::new();

    for current_index in current_indices.iter().copied() {
        let Some(contact) = contacts.get(current_index) else {
            continue;
        };
        let exact_previous_index = previous_keys
            .iter()
            .enumerate()
            .find(|(previous_index, key)| {
                !previous_used.get(*previous_index).copied().unwrap_or(true)
                    && **key == contact.key
                    && previous_contacts
                        .get(key)
                        .is_some_and(|record| geometry_compatible(record, contact))
            })
            .map(|(previous_index, _)| previous_index);
        if let Some(previous_index) = exact_previous_index {
            if let Some(used) = previous_used.get_mut(previous_index) {
                *used = true;
            }
            if let (Some(slot), Some(key)) = (
                reserved.get_mut(current_index),
                previous_keys.get(previous_index).copied(),
            ) {
                *slot = Some(ReservedPrevious {
                    key,
                    kind: PreviousMatchKind::Exact,
                });
            }
        } else {
            residual_current_indices.push(current_index);
        }
    }

    let mut candidate_graph = Vec::with_capacity(residual_current_indices.len());
    for current_index in residual_current_indices.iter().copied() {
        let mut edges = Vec::new();
        let Some(contact) = contacts.get(current_index) else {
            candidate_graph.push(edges);
            continue;
        };
        for (previous_index, previous_key) in previous_keys.iter().copied().enumerate() {
            if previous_used.get(previous_index).copied().unwrap_or(true) {
                continue;
            }
            let Some(previous) = previous_contacts.get(&previous_key) else {
                continue;
            };
            if let Some((kind, drift)) = reservation_candidate(policy, previous, contact) {
                edges.push(CandidateEdge {
                    previous_index,
                    previous_key,
                    kind,
                    drift,
                });
            }
        }
        edges.sort_by_key(|edge| (edge.previous_key, edge.kind));
        candidate_graph.push(edges);
    }

    let mut assignments = vec![None; residual_current_indices.len()];
    let mut best = None;
    enumerate_residual_matchings(
        0,
        &residual_current_indices,
        contacts,
        &candidate_graph,
        &mut previous_used,
        &mut assignments,
        policy,
        &mut best,
    );
    let Some(best) = best else {
        return;
    };
    for (current_index, edge) in residual_current_indices.iter().copied().zip(best) {
        let Some(edge) = edge else {
            continue;
        };
        if let Some(slot) = reserved.get_mut(current_index) {
            *slot = Some(ReservedPrevious {
                key: edge.previous_key,
                kind: edge.kind,
            });
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn enumerate_residual_matchings(
    position: usize,
    current_indices: &[usize],
    contacts: &[ContactObservation],
    candidate_graph: &[Vec<CandidateEdge>],
    previous_used: &mut [bool],
    assignments: &mut [Option<CandidateEdge>],
    policy: ReservationPolicy,
    best: &mut Option<Vec<Option<CandidateEdge>>>,
) {
    if position == current_indices.len() {
        if best.as_ref().is_none_or(|incumbent| {
            residual_matching_is_better(assignments, incumbent, current_indices, contacts, policy)
        }) {
            *best = Some(assignments.to_vec());
        }
        return;
    }

    if let Some(slot) = assignments.get_mut(position) {
        *slot = None;
    }
    enumerate_residual_matchings(
        position + 1,
        current_indices,
        contacts,
        candidate_graph,
        previous_used,
        assignments,
        policy,
        best,
    );

    let Some(edges) = candidate_graph.get(position) else {
        return;
    };
    for edge in edges.iter().copied() {
        if previous_used
            .get(edge.previous_index)
            .copied()
            .unwrap_or(true)
        {
            continue;
        }
        if let Some(used) = previous_used.get_mut(edge.previous_index) {
            *used = true;
        }
        if let Some(slot) = assignments.get_mut(position) {
            *slot = Some(edge);
        }
        enumerate_residual_matchings(
            position + 1,
            current_indices,
            contacts,
            candidate_graph,
            previous_used,
            assignments,
            policy,
            best,
        );
        if let Some(used) = previous_used.get_mut(edge.previous_index) {
            *used = false;
        }
    }
}

fn residual_matching_is_better(
    candidate: &[Option<CandidateEdge>],
    incumbent: &[Option<CandidateEdge>],
    current_indices: &[usize],
    contacts: &[ContactObservation],
    policy: ReservationPolicy,
) -> bool {
    let cardinality =
        |matching: &[Option<CandidateEdge>]| matching.iter().filter(|edge| edge.is_some()).count();
    let candidate_cardinality = cardinality(candidate);
    let incumbent_cardinality = cardinality(incumbent);
    if candidate_cardinality != incumbent_cardinality {
        return candidate_cardinality > incumbent_cardinality;
    }

    if policy == ReservationPolicy::WarmStart {
        let same_feature_count = |matching: &[Option<CandidateEdge>]| {
            matching
                .iter()
                .flatten()
                .filter(|edge| edge.kind == PreviousMatchKind::SameFeatureIndex)
                .count()
        };
        let candidate_same_feature_count = same_feature_count(candidate);
        let incumbent_same_feature_count = same_feature_count(incumbent);
        if candidate_same_feature_count != incumbent_same_feature_count {
            return candidate_same_feature_count > incumbent_same_feature_count;
        }
    }

    let drift_score = |matching: &[Option<CandidateEdge>]| {
        matching
            .iter()
            .flatten()
            .fold((0.0_f32, 0.0_f32), |(max_drift, total_drift), edge| {
                (max_drift.max(edge.drift), total_drift + edge.drift)
            })
    };
    let candidate_drift = drift_score(candidate);
    let incumbent_drift = drift_score(incumbent);
    let max_drift_order = candidate_drift.0.total_cmp(&incumbent_drift.0);
    if !max_drift_order.is_eq() {
        return max_drift_order.is_lt();
    }
    let total_drift_order = candidate_drift.1.total_cmp(&incumbent_drift.1);
    if !total_drift_order.is_eq() {
        return total_drift_order.is_lt();
    }

    let mapping = |matching: &[Option<CandidateEdge>]| {
        current_indices
            .iter()
            .copied()
            .zip(matching.iter().copied())
            .filter_map(|(current_index, edge)| {
                let edge = edge?;
                let current_key = contacts.get(current_index)?.key;
                Some((current_key, edge.previous_key, edge.kind))
            })
            .collect::<Vec<_>>()
    };
    mapping(candidate) < mapping(incumbent)
}

fn reservation_candidate(
    policy: ReservationPolicy,
    previous: &ContactRecord,
    contact: &ContactObservation,
) -> Option<(PreviousMatchKind, FloatNum)> {
    #[cfg(test)]
    if policy == ReservationPolicy::Lifecycle {
        LIFECYCLE_CANDIDATE_EVALUATION_COUNT.with(|count| count.set(count.get() + 1));
    }

    if !geometry_compatible(previous, contact) {
        return None;
    }
    let drift = contact_anchor_drift(previous, contact);
    if !drift.is_finite() {
        return None;
    }

    match policy {
        ReservationPolicy::WarmStart => {
            let transfer = warm_start_transfer(Some(previous), contact, true);
            if !transfer.reason.is_hit() {
                return None;
            }
            if same_point_identity_reduction(
                previous.contact.reduction_reason,
                contact.reduction_reason,
            ) && previous.contact.feature_id.index() == contact.feature_id.index()
            {
                Some((PreviousMatchKind::SameFeatureIndex, drift))
            } else if persistent_manifold_edge_compatible(previous, contact) {
                Some((PreviousMatchKind::PersistentEdgeSwap, drift))
            } else {
                None
            }
        }
        ReservationPolicy::Lifecycle => (!previous.is_sensor
            && !contact.is_sensor
            && persistent_manifold_edge_compatible(previous, contact))
        .then_some((PreviousMatchKind::PersistentEdgeSwap, drift)),
        ReservationPolicy::SourceRow => (contact.warm_start_reason
            == WarmStartCacheReason::MissFeatureId
            && !previous.is_sensor
            && source_row_continuity_record_reason(previous, contact)
                == SourceRowContinuityReason::Candidate)
            .then_some((PreviousMatchKind::SourceRowCandidate, drift)),
    }
}

fn geometry_compatible(previous: &ContactRecord, contact: &ContactObservation) -> bool {
    previous.contact.collider_a == contact.collider_a
        && previous.contact.collider_b == contact.collider_b
        && previous.geometry_revision_a == contact.geometry_revision_a
        && previous.geometry_revision_b == contact.geometry_revision_b
}

fn persistent_manifold_edge_compatible(
    previous: &ContactRecord,
    contact: &ContactObservation,
) -> bool {
    // A settling clipped manifold may pass through a single-point frame while
    // the reference face swaps, so point-count oscillation must not disqualify
    // an otherwise compatible edge-swap candidate.
    let clip_identity = |reason| {
        clipped_manifold_reduction(reason) || reason == ContactReductionReason::SinglePoint
    };
    if !clip_identity(previous.contact.reduction_reason)
        || !clip_identity(contact.reduction_reason)
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

    let drift_a = contact.witness_a_local - previous.witness_a_local;
    let drift_b = contact.witness_b_local - previous.witness_b_local;
    let local_anchor_drift = drift_a.length().max(drift_b.length());

    local_anchor_drift.is_finite()
        && local_anchor_drift <= PERSISTENT_MANIFOLD_LOCAL_ANCHOR_DRIFT_THRESHOLD
}

#[cfg(test)]
fn source_row_continuity_reason(
    previous_contacts: &BTreeMap<ContactKey, ContactRecord>,
    contact: &ContactObservation,
) -> SourceRowContinuityReason {
    if contact.is_sensor {
        return SourceRowContinuityReason::Sensor;
    }

    let mut rejection = None;
    for record in previous_contacts.values() {
        let reason = source_row_continuity_record_reason(record, contact);
        if matches!(reason, SourceRowContinuityReason::Candidate) {
            return reason;
        }
        rejection = source_row_continuity_rejection(rejection, reason);
    }

    rejection.unwrap_or(SourceRowContinuityReason::NoPreviousPair)
}

fn source_row_unreserved_reason(
    previous_contacts: &BTreeMap<ContactKey, ContactRecord>,
    previous_keys: Option<&[ContactKey]>,
    contact: &ContactObservation,
) -> SourceRowContinuityReason {
    if contact.is_sensor {
        return SourceRowContinuityReason::Sensor;
    }
    let Some(previous_keys) = previous_keys else {
        return if previous_contacts.is_empty() {
            SourceRowContinuityReason::NoPreviousPair
        } else {
            SourceRowContinuityReason::PairMismatch
        };
    };

    let mut rejection = None;
    let mut had_candidate = false;
    for key in previous_keys {
        let Some(record) = previous_contacts.get(key) else {
            continue;
        };
        let reason = source_row_continuity_record_reason(record, contact);
        if reason == SourceRowContinuityReason::Candidate {
            had_candidate = true;
        } else {
            rejection = source_row_continuity_rejection(rejection, reason);
        }
    }
    if had_candidate {
        SourceRowContinuityReason::Unknown
    } else {
        rejection.unwrap_or(SourceRowContinuityReason::PairMismatch)
    }
}

fn source_row_continuity_record_reason(
    previous: &ContactRecord,
    contact: &ContactObservation,
) -> SourceRowContinuityReason {
    if previous.contact.collider_a != contact.collider_a
        || previous.contact.collider_b != contact.collider_b
        || !geometry_compatible(previous, contact)
    {
        return SourceRowContinuityReason::PairMismatch;
    }
    if previous.is_sensor {
        return SourceRowContinuityReason::Sensor;
    }

    // Edge-swapped clipped manifolds may still represent one geometric pair,
    // but they are a separate lifecycle path from source-row continuity.
    if feature_id_edge_swap(previous.contact.feature_id, contact.feature_id) {
        return SourceRowContinuityReason::EdgeSwap;
    }

    let previous_normal = previous.contact.normal.normalized_or_zero();
    let current_normal = contact.normal.normalized_or_zero();
    if previous_normal.length() <= FloatNum::EPSILON
        || current_normal.length() <= FloatNum::EPSILON
        || previous_normal.dot(current_normal) < WARM_START_NORMAL_DOT_THRESHOLD
    {
        return SourceRowContinuityReason::NormalMismatch;
    }

    let local_anchor_drift = contact_anchor_drift(previous, contact);
    if !local_anchor_drift.is_finite()
        || local_anchor_drift > PERSISTENT_MANIFOLD_LOCAL_ANCHOR_DRIFT_THRESHOLD
    {
        return SourceRowContinuityReason::AnchorDrift;
    }

    SourceRowContinuityReason::Candidate
}

fn source_row_continuity_rejection(
    current: Option<SourceRowContinuityReason>,
    next: SourceRowContinuityReason,
) -> Option<SourceRowContinuityReason> {
    if matches!(
        next,
        SourceRowContinuityReason::Candidate | SourceRowContinuityReason::NoPreviousPair
    ) {
        return current;
    }

    let Some(current) = current else {
        return Some(next);
    };

    if source_row_continuity_rejection_rank(next) < source_row_continuity_rejection_rank(current) {
        Some(next)
    } else {
        Some(current)
    }
}

fn source_row_continuity_rejection_rank(classification: SourceRowContinuityReason) -> usize {
    match classification {
        SourceRowContinuityReason::EdgeSwap => 0,
        SourceRowContinuityReason::NormalMismatch => 1,
        SourceRowContinuityReason::AnchorDrift => 2,
        SourceRowContinuityReason::Sensor => 3,
        SourceRowContinuityReason::Unknown
        | SourceRowContinuityReason::NoPreviousPair
        | SourceRowContinuityReason::PairMismatch
        | SourceRowContinuityReason::Candidate => 4,
    }
}

fn contact_event(
    contact_id: ContactId,
    manifold_id: ManifoldId,
    lifecycle_reason: ContactLifecycleReason,
    contact: &ContactObservation,
) -> ContactEvent {
    ContactEvent {
        contact_id,
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
        source_row_continuity_reason: contact.source_row_continuity_reason,
        lifecycle_reason,
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
}

fn contact_anchor_drift(previous: &ContactRecord, contact: &ContactObservation) -> FloatNum {
    let drift_a = contact.witness_a_local - previous.witness_a_local;
    let drift_b = contact.witness_b_local - previous.witness_b_local;
    let drift_a = drift_a.length();
    let drift_b = drift_b.length();
    if drift_a.is_finite() && drift_b.is_finite() {
        drift_a.max(drift_b)
    } else {
        FloatNum::NAN
    }
}

fn clipped_manifold_reduction(reason: ContactReductionReason) -> bool {
    matches!(
        reason,
        ContactReductionReason::Clipped | ContactReductionReason::DuplicateReduced
    )
}

/// Whether two reduction reasons describe the same clip-path point identity.
/// A settling clipped manifold oscillates between two points and one
/// (`Clipped`/`DuplicateReduced` <-> `SinglePoint`); the reduction label tracks
/// how the manifold was produced that frame, not which physical point survived,
/// so it must not break same-feature-index warm-start continuity on its own.
fn same_point_identity_reduction(a: ContactReductionReason, b: ContactReductionReason) -> bool {
    let clip_family = |reason| {
        matches!(
            reason,
            ContactReductionReason::SinglePoint
                | ContactReductionReason::Clipped
                | ContactReductionReason::DuplicateReduced
        )
    };
    a == b || (clip_family(a) && clip_family(b))
}

fn feature_id_edge_swap(
    previous: crate::handles::ContactFeatureId,
    current: crate::handles::ContactFeatureId,
) -> bool {
    if previous.index() == current.index() {
        return false;
    }
    let Some(previous) = decode_feature_id(previous) else {
        return false;
    };
    let Some(current) = decode_feature_id(current) else {
        return false;
    };

    previous.kind == current.kind
        && previous.reference_edge != previous.incident_edge
        && current.reference_edge != current.incident_edge
        && previous.reference_edge == current.incident_edge
        && previous.incident_edge == current.reference_edge
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

    if previous.is_sensor {
        return warm_start_transfer_result(WarmStartCacheReason::MissPreviousSensor, 0.0, 0.0);
    }

    if !previous.normal_impulse.is_finite() || !previous.tangent_impulse.is_finite() {
        return warm_start_transfer_result(WarmStartCacheReason::DroppedInvalidImpulse, 0.0, 0.0);
    }

    let previous_normal = previous.contact.normal.normalized_or_zero();
    let current_normal = contact.normal.normalized_or_zero();
    let current_normal_a_local = contact.normal_a_local.normalized_or_zero();
    let current_normal_b_local = contact.normal_b_local.normalized_or_zero();
    // Normal mismatch means the old impulse would push along the wrong
    // constraint row. Feature ids alone are not enough after a normal flip.
    if previous_normal.length() <= FloatNum::EPSILON
        || current_normal.length() <= FloatNum::EPSILON
        || previous_normal.dot(current_normal) < WARM_START_NORMAL_DOT_THRESHOLD
        || current_normal_a_local.length() <= FloatNum::EPSILON
        || current_normal_b_local.length() <= FloatNum::EPSILON
    {
        return warm_start_transfer_result(WarmStartCacheReason::DroppedNormalMismatch, 0.0, 0.0);
    }

    // Feature ids are local geometric names, not raw world-space guarantees.
    // Compare contact anchors relative to both colliders so a pair translating
    // together keeps its cache, while contact movement on either shape drops it.
    let drift_a = contact.witness_a_local - previous.witness_a_local;
    let drift_b = contact.witness_b_local - previous.witness_b_local;
    let drift_a_length = drift_a.length();
    let drift_b_length = drift_b.length();
    if !drift_a.x().is_finite()
        || !drift_a.y().is_finite()
        || !drift_b.x().is_finite()
        || !drift_b.y().is_finite()
        || !drift_a_length.is_finite()
        || !drift_b_length.is_finite()
    {
        return warm_start_transfer_result(WarmStartCacheReason::DroppedPointDrift, 0.0, 0.0);
    }
    let normal_drift = drift_a
        .dot(current_normal_a_local)
        .abs()
        .max(drift_b.dot(current_normal_b_local).abs());
    let tangent_a_local = current_normal_a_local.perp().normalized_or_zero();
    let tangent_b_local = current_normal_b_local.perp().normalized_or_zero();
    let tangential_drift = drift_a
        .dot(tangent_a_local)
        .abs()
        .max(drift_b.dot(tangent_b_local).abs());
    let drift = drift_a_length.max(drift_b_length);
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
        body::{BodyDesc, BodyType},
        collider::{ColliderDesc, ColliderPatch},
        events::{ContactLifecycleReason, SourceRowContinuityReason},
        handles::{ContactFeatureId, ContactId, ManifoldId},
        pipeline::SimulationPipeline,
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
            source_row_continuity_reason: SourceRowContinuityReason::Unknown,
            lifecycle_reason: ContactLifecycleReason::Unknown,
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
            geometry_revision_a: 0,
            geometry_revision_b: 0,
            anchor_a: Vector::new(0.25, 0.0),
            anchor_b: Vector::new(-0.25, 0.0),
            witness_a_local: Point::new(0.25, 0.0),
            witness_b_local: Point::new(-0.25, 0.0),
            normal_a_local: Vector::new(1.0, 0.0),
            normal_b_local: Vector::new(1.0, 0.0),
            point: Point::new(0.0, 0.0),
            normal: Vector::new(1.0, 0.0),
            depth: 0.01,
            signed_separation: -0.01,
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
            source_row_continuity_reason: SourceRowContinuityReason::Unknown,
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
            base_normal_solve_executed: false,
            generic_convex_trace: None,
            ccd_trace: None,
        }
    }

    fn test_contact_event_with_ids(
        feature_id: ContactFeatureId,
        contact_raw: u32,
        manifold_raw: u32,
    ) -> ContactEvent {
        let mut event = test_contact_event(feature_id);
        event.contact_id = ContactId::from_raw_parts(contact_raw, 0);
        event.manifold_id = ManifoldId::from_raw_parts(manifold_raw, 0);
        event
    }

    fn test_contact_record(
        feature_id: ContactFeatureId,
        contact_raw: u32,
        manifold_raw: u32,
        witness_x: FloatNum,
        normal_impulse: FloatNum,
        tangent_impulse: FloatNum,
    ) -> ContactRecord {
        ContactRecord {
            contact: test_contact_event_with_ids(feature_id, contact_raw, manifold_raw),
            geometry_revision_a: 0,
            geometry_revision_b: 0,
            is_sensor: false,
            witness_a_local: Point::new(witness_x, 0.0),
            witness_b_local: Point::new(witness_x, 0.0),
            normal_impulse,
            tangent_impulse,
        }
    }

    fn set_observation_witness(contact: &mut ContactObservation, witness_x: FloatNum) {
        contact.witness_a_local = Point::new(witness_x, 0.0);
        contact.witness_b_local = Point::new(witness_x, 0.0);
    }

    fn test_normal(degrees: FloatNum) -> Vector {
        let radians = degrees.to_radians();
        Vector::new(radians.cos(), radians.sin())
    }

    fn active_event_for_feature(
        events: &[WorldEvent],
        feature_id: ContactFeatureId,
    ) -> (&ContactEvent, bool) {
        events
            .iter()
            .find_map(|event| match event {
                WorldEvent::ContactStarted(contact) if contact.feature_id == feature_id => {
                    Some((contact, false))
                }
                WorldEvent::ContactPersisted(contact) if contact.feature_id == feature_id => {
                    Some((contact, true))
                }
                _ => None,
            })
            .expect("active contact event should exist for the feature")
    }

    #[test]
    fn lifecycle_reservation_keeps_later_exact_match() {
        let previous_feature = test_feature(0x0100_1003, 1);
        let fallback_feature = test_feature(0x0100_3001, 0);
        let previous_record = test_contact_record(previous_feature, 41, 7, 0.0, 1.0, 0.25);
        let previous_contact_id = previous_record.contact.contact_id;
        let mut previous_contacts = BTreeMap::new();
        previous_contacts.insert(
            ContactKey::new(test_collider(1), test_collider(2), previous_feature),
            previous_record,
        );
        let mut fallback = test_contact_observation(fallback_feature);
        set_observation_witness(&mut fallback, 0.01);
        let exact = test_contact_observation(previous_feature);
        let mut contacts = vec![exact, fallback];
        contacts.sort_by_key(|contact| contact.key);
        assert_eq!(contacts[0].feature_id, fallback_feature);
        assert_eq!(contacts[1].feature_id, previous_feature);
        let mut world = World::new(WorldDesc::default());

        let (events, _, _, _) = world.refresh_contact_events(contacts, previous_contacts);

        let (fallback_event, fallback_persisted) =
            active_event_for_feature(&events, fallback_feature);
        let (exact_event, exact_persisted) = active_event_for_feature(&events, previous_feature);
        assert!(
            !fallback_persisted,
            "fallback may start but must not steal an exact reservation"
        );
        assert_ne!(fallback_event.contact_id, previous_contact_id);
        assert!(
            exact_persisted,
            "the later full exact current point must stay persisted"
        );
        assert_eq!(exact_event.contact_id, previous_contact_id);
        assert_eq!(
            exact_event.lifecycle_reason,
            ContactLifecycleReason::ExactFeature
        );
    }

    #[test]
    fn lifecycle_reservation_maximizes_edge_swap_cardinality() {
        let previous_index = 0x0100_1003;
        let current_index = 0x0100_3001;
        let previous_features = [
            test_feature(previous_index, 0),
            test_feature(previous_index, 1),
        ];
        let current_features = [
            test_feature(current_index, 2),
            test_feature(current_index, 3),
        ];
        let mut p0 = test_contact_record(previous_features[0], 51, 8, 0.0, 1.0, 0.1);
        let mut p1 = test_contact_record(previous_features[1], 52, 8, 0.03, 2.0, 0.2);
        p0.contact.normal = test_normal(0.0);
        p1.contact.normal = test_normal(10.0);
        let mut previous_contacts = BTreeMap::new();
        previous_contacts.insert(
            ContactKey::new(test_collider(1), test_collider(2), previous_features[0]),
            p0,
        );
        previous_contacts.insert(
            ContactKey::new(test_collider(1), test_collider(2), previous_features[1]),
            p1,
        );
        let mut c0 = test_contact_observation(current_features[0]);
        let mut c1 = test_contact_observation(current_features[1]);
        set_observation_witness(&mut c0, 0.01);
        set_observation_witness(&mut c1, 0.03);
        c0.normal = test_normal(5.0);
        c1.normal = test_normal(-10.0);
        let mut world = World::new(WorldDesc::default());

        let (events, _, _, _) = world.refresh_contact_events(vec![c0, c1], previous_contacts);

        let (c0_event, c0_persisted) = active_event_for_feature(&events, current_features[0]);
        let (c1_event, c1_persisted) = active_event_for_feature(&events, current_features[1]);
        assert!(
            c0_persisted && c1_persisted,
            "the eligible edge graph has cardinality two"
        );
        assert_eq!(c0_event.contact_id, ContactId::from_raw_parts(52, 0));
        assert_eq!(c1_event.contact_id, ContactId::from_raw_parts(51, 0));
    }

    #[test]
    fn warm_start_reservation_keeps_distinct_impulses_with_local_witnesses() {
        let previous_index = 0x0100_1003;
        let current_index = 0x0100_3001;
        let previous_features = [
            test_feature(previous_index, 0),
            test_feature(previous_index, 1),
        ];
        let current_features = [
            test_feature(current_index, 2),
            test_feature(current_index, 3),
        ];
        let previous_records = [
            test_contact_record(previous_features[0], 61, 9, 0.0, 1.0, 0.3),
            test_contact_record(previous_features[1], 62, 9, 0.04, 2.0, 0.6),
        ];
        let mut previous_contacts = BTreeMap::new();
        for (feature, record) in previous_features.into_iter().zip(previous_records) {
            previous_contacts.insert(
                ContactKey::new(test_collider(1), test_collider(2), feature),
                record,
            );
        }
        let mut c0 = test_contact_observation(current_features[0]);
        let mut c1 = test_contact_observation(current_features[1]);
        set_observation_witness(&mut c0, 0.01);
        set_observation_witness(&mut c1, 0.015);
        let mut contacts = vec![c0, c1];
        let world = World::new(WorldDesc::default());

        world.prepare_contact_warm_start(&mut contacts, &previous_contacts);

        assert!(contacts
            .iter()
            .all(|contact| contact.warm_start_reason == WarmStartCacheReason::Hit));
        assert_eq!(contacts[0].warm_start_normal_impulse, 1.0);
        assert_eq!(contacts[1].warm_start_normal_impulse, 2.0);
        assert_ne!(
            contacts[0].warm_start_normal_impulse, contacts[1].warm_start_normal_impulse,
            "one previous point must not feed both current rows"
        );
        assert!(contacts
            .iter()
            .all(|contact| contact.warm_start_tangent_impulse == 0.0));
    }

    #[test]
    fn warm_start_reservation_maximizes_residual_cardinality_after_exact_matches() {
        let exact_feature = test_feature(0x0200_2004, 0);
        let exact_fallback_feature = test_feature(0x0200_4002, 1);
        let exact_record = test_contact_record(exact_feature, 71, 10, 0.0, 30.0, 3.0);
        let mut exact_previous = BTreeMap::new();
        exact_previous.insert(
            ContactKey::new(test_collider(1), test_collider(2), exact_feature),
            exact_record,
        );
        let mut exact_fallback = test_contact_observation(exact_fallback_feature);
        set_observation_witness(&mut exact_fallback, 0.01);
        let mut exact_current = test_contact_observation(exact_feature);
        set_observation_witness(&mut exact_current, 0.0);
        let mut exact_contacts = vec![exact_fallback, exact_current];
        let world = World::new(WorldDesc::default());
        world.prepare_contact_warm_start(&mut exact_contacts, &exact_previous);
        let exact_hard_ok = exact_contacts[0].warm_start_reason != WarmStartCacheReason::Hit
            && exact_contacts[1].warm_start_reason == WarmStartCacheReason::Hit
            && exact_contacts[1].warm_start_normal_impulse == 30.0;

        let previous_index = 0x0100_1003;
        let swapped_index = 0x0100_3001;
        let p0_feature = test_feature(previous_index, 0);
        let p1_feature = test_feature(swapped_index, 0);
        let mut p0 = test_contact_record(p0_feature, 72, 10, 0.0, 10.0, 1.0);
        let mut p1 = test_contact_record(p1_feature, 73, 10, 0.03, 20.0, 2.0);
        p0.contact.normal = test_normal(0.0);
        p1.contact.normal = test_normal(10.0);
        let mut residual_previous = BTreeMap::new();
        residual_previous.insert(
            ContactKey::new(test_collider(1), test_collider(2), exact_feature),
            test_contact_record(exact_feature, 71, 10, 0.0, 30.0, 3.0),
        );
        residual_previous.insert(
            ContactKey::new(test_collider(1), test_collider(2), p0_feature),
            p0,
        );
        residual_previous.insert(
            ContactKey::new(test_collider(1), test_collider(2), p1_feature),
            p1,
        );
        let mut c0 = test_contact_observation(test_feature(previous_index, 1));
        let mut c1 = test_contact_observation(test_feature(swapped_index, 1));
        set_observation_witness(&mut c0, 0.01);
        set_observation_witness(&mut c1, 0.03);
        c0.normal = test_normal(5.0);
        c1.normal = test_normal(-10.0);
        let mut residual_exact = test_contact_observation(exact_feature);
        set_observation_witness(&mut residual_exact, 0.0);
        let mut residual_contacts = vec![c0, c1, residual_exact];
        world.prepare_contact_warm_start(&mut residual_contacts, &residual_previous);
        let residual_ok = residual_contacts[0].warm_start_reason == WarmStartCacheReason::Hit
            && residual_contacts[1].warm_start_reason == WarmStartCacheReason::Hit
            && residual_contacts[0].warm_start_normal_impulse == 20.0
            && residual_contacts[1].warm_start_normal_impulse == 10.0
            && residual_contacts[0].warm_start_tangent_impulse == 0.0
            && residual_contacts[1].warm_start_tangent_impulse == 0.0
            && residual_contacts[2].warm_start_normal_impulse == 30.0;

        assert!(
            exact_hard_ok && residual_ok,
            "exact must be reserved before residual matching, then residual cardinality must beat same-index preference; exact={:?}, residual={:?}",
            exact_contacts
                .iter()
                .map(|contact| (
                    contact.warm_start_reason,
                    contact.warm_start_normal_impulse,
                    contact.warm_start_tangent_impulse,
                ))
                .collect::<Vec<_>>(),
            residual_contacts
                .iter()
                .map(|contact| (
                    contact.warm_start_reason,
                    contact.warm_start_normal_impulse,
                    contact.warm_start_tangent_impulse,
                ))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn residual_matching_ranks_kind_then_max_and_total_drift() {
        fn best_matching(
            contacts: &[ContactObservation],
            current_indices: &[usize],
            candidate_graph: &[Vec<CandidateEdge>],
            previous_count: usize,
            policy: ReservationPolicy,
        ) -> Vec<Option<CandidateEdge>> {
            let mut previous_used = vec![false; previous_count];
            let mut assignments = vec![None; current_indices.len()];
            let mut best = None;
            enumerate_residual_matchings(
                0,
                current_indices,
                contacts,
                candidate_graph,
                &mut previous_used,
                &mut assignments,
                policy,
                &mut best,
            );
            best.expect("finite candidate graph should produce a best matching")
        }

        let contacts = vec![
            test_contact_observation(test_feature(0x0100_3001, 0)),
            test_contact_observation(test_feature(0x0100_3001, 1)),
        ];
        let previous_keys = [
            ContactKey::new(
                test_collider(1),
                test_collider(2),
                test_feature(0x0100_1003, 0),
            ),
            ContactKey::new(
                test_collider(1),
                test_collider(2),
                test_feature(0x0100_1003, 1),
            ),
        ];
        let edge = |previous_index, kind, drift| CandidateEdge {
            previous_index,
            previous_key: previous_keys[previous_index],
            kind,
            drift,
        };

        let kind_preference = best_matching(
            &contacts,
            &[0],
            &[vec![
                edge(0, PreviousMatchKind::SameFeatureIndex, 0.20),
                edge(1, PreviousMatchKind::PersistentEdgeSwap, 0.01),
            ]],
            2,
            ReservationPolicy::WarmStart,
        );
        assert_eq!(kind_preference[0].map(|edge| edge.previous_index), Some(0));

        let max_drift = best_matching(
            &contacts,
            &[0, 1],
            &[
                vec![
                    edge(0, PreviousMatchKind::PersistentEdgeSwap, 0.20),
                    edge(1, PreviousMatchKind::PersistentEdgeSwap, 0.10),
                ],
                vec![
                    edge(1, PreviousMatchKind::PersistentEdgeSwap, 0.20),
                    edge(0, PreviousMatchKind::PersistentEdgeSwap, 0.25),
                ],
            ],
            2,
            ReservationPolicy::Lifecycle,
        );
        assert_eq!(
            max_drift
                .iter()
                .map(|edge| edge.map(|edge| edge.previous_index))
                .collect::<Vec<_>>(),
            vec![Some(0), Some(1)]
        );

        let total_drift = best_matching(
            &contacts,
            &[0, 1],
            &[
                vec![
                    edge(0, PreviousMatchKind::PersistentEdgeSwap, 0.10),
                    edge(1, PreviousMatchKind::PersistentEdgeSwap, 0.20),
                ],
                vec![
                    edge(1, PreviousMatchKind::PersistentEdgeSwap, 0.20),
                    edge(0, PreviousMatchKind::PersistentEdgeSwap, 0.20),
                ],
            ],
            2,
            ReservationPolicy::Lifecycle,
        );
        assert_eq!(
            total_drift
                .iter()
                .map(|edge| edge.map(|edge| edge.previous_index))
                .collect::<Vec<_>>(),
            vec![Some(0), Some(1)]
        );
    }

    #[test]
    fn residual_matching_lexicographic_tie_break_is_input_order_independent() {
        fn run(
            contacts: Vec<ContactObservation>,
            previous_contacts: &BTreeMap<ContactKey, ContactRecord>,
        ) -> BTreeMap<ContactKey, ContactKey> {
            let reservations = reserve_previous_matches(
                &contacts,
                previous_contacts,
                ReservationPolicy::Lifecycle,
            );
            contacts
                .iter()
                .enumerate()
                .filter_map(|(index, contact)| {
                    reservations
                        .reserved
                        .get(index)
                        .and_then(Option::as_ref)
                        .map(|reserved| (contact.key, reserved.key))
                })
                .collect()
        }

        let previous_features = [test_feature(0x0100_1003, 0), test_feature(0x0100_1003, 1)];
        let mut previous_contacts = BTreeMap::new();
        for (slot, feature) in previous_features.into_iter().enumerate() {
            previous_contacts.insert(
                ContactKey::new(test_collider(1), test_collider(2), feature),
                test_contact_record(feature, 90 + slot as u32, 15, 0.0, 1.0, 0.1),
            );
        }
        let mut first = test_contact_observation(test_feature(0x0100_3001, 2));
        let mut second = test_contact_observation(test_feature(0x0100_3001, 3));
        set_observation_witness(&mut first, 0.0);
        set_observation_witness(&mut second, 0.0);

        let forward = run(vec![first.clone(), second.clone()], &previous_contacts);
        let reversed = run(vec![second, first], &previous_contacts);

        assert_eq!(forward, reversed);
        assert_eq!(forward.len(), 2);
        assert_eq!(
            forward.into_iter().collect::<Vec<_>>(),
            vec![
                (
                    ContactKey::new(
                        test_collider(1),
                        test_collider(2),
                        test_feature(0x0100_3001, 2),
                    ),
                    ContactKey::new(
                        test_collider(1),
                        test_collider(2),
                        test_feature(0x0100_1003, 0),
                    ),
                ),
                (
                    ContactKey::new(
                        test_collider(1),
                        test_collider(2),
                        test_feature(0x0100_3001, 3),
                    ),
                    ContactKey::new(
                        test_collider(1),
                        test_collider(2),
                        test_feature(0x0100_1003, 1),
                    ),
                ),
            ]
        );
    }

    #[test]
    fn reservation_rejects_nonfinite_edges_and_handles_five_by_five() {
        let previous_feature = test_feature(0x0100_1003, 0);
        let mut nonfinite_previous = BTreeMap::new();
        nonfinite_previous.insert(
            ContactKey::new(test_collider(1), test_collider(2), previous_feature),
            test_contact_record(previous_feature, 95, 16, 0.0, 1.0, 0.1),
        );
        let mut nonfinite = test_contact_observation(test_feature(0x0100_3001, 0));
        set_observation_witness(&mut nonfinite, 0.0);
        nonfinite.witness_a_local = Point::new(FloatNum::NAN, 0.0);
        let nonfinite_result = reserve_previous_matches(
            &[nonfinite],
            &nonfinite_previous,
            ReservationPolicy::Lifecycle,
        );
        assert!(nonfinite_result.reserved[0].is_none());

        let mut previous_contacts = BTreeMap::new();
        let mut contacts = Vec::new();
        for slot in 0..5_u32 {
            let previous_feature = test_feature(0x0100_1003, slot);
            previous_contacts.insert(
                ContactKey::new(test_collider(1), test_collider(2), previous_feature),
                test_contact_record(previous_feature, 100 + slot, 17, 0.0, 1.0, 0.1),
            );
            let mut current = test_contact_observation(test_feature(0x0100_3001, 10 + slot));
            set_observation_witness(&mut current, 0.0);
            contacts.push(current);
        }
        let five_by_five =
            reserve_previous_matches(&contacts, &previous_contacts, ReservationPolicy::Lifecycle);
        let reserved_keys = five_by_five
            .reserved
            .iter()
            .flatten()
            .map(|reserved| reserved.key)
            .collect::<BTreeSet<_>>();
        assert_eq!(five_by_five.reserved.iter().flatten().count(), 5);
        assert_eq!(reserved_keys.len(), 5);
    }

    #[test]
    fn source_row_reservation_does_not_reuse_previous_point() {
        let previous_feature = test_feature(0x0100_1003, 0);
        let previous_record = test_contact_record(previous_feature, 81, 11, 0.0, 1.0, 0.1);
        let mut previous_contacts = BTreeMap::new();
        previous_contacts.insert(
            ContactKey::new(test_collider(1), test_collider(2), previous_feature),
            previous_record,
        );
        let mut c0 = test_contact_observation(test_feature(0x0100_1004, 0));
        let mut c1 = test_contact_observation(test_feature(0x0100_1005, 0));
        set_observation_witness(&mut c0, 0.01);
        set_observation_witness(&mut c1, 0.02);
        let mut contacts = vec![c0, c1];
        let world = World::new(WorldDesc::default());

        world.prepare_contact_warm_start(&mut contacts, &previous_contacts);

        assert_eq!(
            contacts
                .iter()
                .filter(|contact| contact.source_row_continuity_candidate)
                .count(),
            1,
            "one previous point can authorize at most one current source row"
        );
        assert!(contacts[0].source_row_continuity_candidate);
        assert!(!contacts[1].source_row_continuity_candidate);
    }

    #[test]
    fn source_row_contention_keeps_unreserved_reason_unknown() {
        let previous_feature = test_feature(0x0100_1003, 0);
        let previous_record = test_contact_record(previous_feature, 82, 11, 0.0, 1.0, 0.1);
        let mut previous_contacts = BTreeMap::new();
        previous_contacts.insert(
            ContactKey::new(test_collider(1), test_collider(2), previous_feature),
            previous_record,
        );
        let mut first = test_contact_observation(test_feature(0x0100_1004, 0));
        let mut second = test_contact_observation(test_feature(0x0100_1005, 0));
        set_observation_witness(&mut first, 0.01);
        set_observation_witness(&mut second, 0.02);
        let mut contacts = vec![first, second];
        let world = World::new(WorldDesc::default());

        world.prepare_contact_warm_start(&mut contacts, &previous_contacts);

        assert_eq!(
            contacts[0].source_row_continuity_reason,
            SourceRowContinuityReason::Candidate
        );
        assert!(contacts[0].source_row_continuity_candidate);
        assert!(!contacts[1].source_row_continuity_candidate);
        assert_eq!(
            contacts[1].source_row_continuity_reason,
            SourceRowContinuityReason::Unknown
        );
    }

    #[test]
    fn source_row_revision_is_rejected_before_solver_rows_are_built() {
        let mut world = World::new(WorldDesc {
            gravity: Vector::default(),
            ..WorldDesc::default()
        });
        let body_a = world
            .create_body(BodyDesc {
                body_type: BodyType::Static,
                pose: Pose::default(),
                ..BodyDesc::default()
            })
            .expect("first body should be created");
        let body_b = world
            .create_body(BodyDesc {
                body_type: BodyType::Dynamic,
                pose: Pose::from_xy_angle(1.5, 0.0, 0.0),
                can_sleep: false,
                ..BodyDesc::default()
            })
            .expect("second body should be created");
        world
            .create_collider(
                body_a,
                ColliderDesc {
                    shape: SharedShape::rect(2.0, 2.0),
                    ..ColliderDesc::default()
                },
            )
            .expect("first collider should be created");
        let collider_b = world
            .create_collider(
                body_b,
                ColliderDesc {
                    shape: SharedShape::rect(2.0, 2.0),
                    ..ColliderDesc::default()
                },
            )
            .expect("second collider should be created");
        let mut pipeline = SimulationPipeline::new(StepConfig {
            position_iterations: 0,
            ..StepConfig::default()
        });
        let first = pipeline.step(&mut world);
        assert_eq!(first.stats.contact_count, 2);
        assert!(
            first.stats.contact_row_count > 0,
            "dynamic/static fixture must build at least one solver row"
        );
        world
            .apply_collider_patch(
                collider_b,
                ColliderPatch {
                    shape: Some(SharedShape::rect(2.02, 2.0)),
                    ..ColliderPatch::default()
                },
            )
            .expect("geometry patch should succeed");

        let predicted_body_poses = BTreeMap::new();
        let mut contacts = world
            .collect_contact_observations(&[], &predicted_body_poses)
            .observations;
        let previous_contacts = world.take_active_contacts();
        world.prepare_contact_warm_start(&mut contacts, &previous_contacts);

        assert_eq!(contacts.len(), 2);
        assert!(
            contacts.iter().all(|contact| {
                contact.warm_start_reason == WarmStartCacheReason::MissFeatureId
                    && contact.warm_start_normal_impulse == 0.0
                    && contact.warm_start_tangent_impulse == 0.0
                    && contact.normal_impulse == 0.0
                    && contact.tangent_impulse == 0.0
                    && !contact.source_row_continuity_candidate
                    && contact.source_row_continuity_reason != SourceRowContinuityReason::Candidate
            }),
            "geometry generation mismatch must be rejected in pre-solver contact facts; facts={:?}",
            contacts
                .iter()
                .map(|contact| (
                    contact.warm_start_reason,
                    contact.source_row_continuity_candidate,
                    contact.source_row_continuity_reason,
                ))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn source_row_rejects_previous_sensor_feature_miss() {
        let previous_feature = test_feature(0x0100_1003, 0);
        let current_feature = test_feature(0x0100_1004, 0);
        let mut previous_record = test_contact_record(previous_feature, 84, 14, 0.0, 0.0, 0.0);
        previous_record.is_sensor = true;
        let mut previous_contacts = BTreeMap::new();
        previous_contacts.insert(
            ContactKey::new(test_collider(1), test_collider(2), previous_feature),
            previous_record,
        );
        let mut current = test_contact_observation(current_feature);
        set_observation_witness(&mut current, 0.01);
        let mut contacts = vec![current];
        let world = World::new(WorldDesc::default());

        world.prepare_contact_warm_start(&mut contacts, &previous_contacts);

        assert_eq!(
            contacts[0].warm_start_reason,
            WarmStartCacheReason::MissFeatureId
        );
        assert!(!contacts[0].source_row_continuity_candidate);
        assert_eq!(
            contacts[0].source_row_continuity_reason,
            SourceRowContinuityReason::Sensor
        );
    }

    #[test]
    fn source_row_keeps_unknown_for_exact_and_same_index_warm_hits() {
        fn run(previous_feature: ContactFeatureId, current_feature: ContactFeatureId) {
            let previous_record = test_contact_record(previous_feature, 85, 14, 0.0, 1.0, 0.2);
            let mut previous_contacts = BTreeMap::new();
            previous_contacts.insert(
                ContactKey::new(test_collider(1), test_collider(2), previous_feature),
                previous_record,
            );
            let mut current = test_contact_observation(current_feature);
            set_observation_witness(&mut current, 0.0);
            let mut contacts = vec![current];
            let world = World::new(WorldDesc::default());

            world.prepare_contact_warm_start(&mut contacts, &previous_contacts);

            assert_eq!(contacts[0].warm_start_reason, WarmStartCacheReason::Hit);
            assert!(!contacts[0].source_row_continuity_candidate);
            assert_eq!(
                contacts[0].source_row_continuity_reason,
                SourceRowContinuityReason::Unknown
            );
        }

        let previous_feature = test_feature(0x0100_1003, 0);
        run(previous_feature, previous_feature);
        run(previous_feature, test_feature(0x0100_1003, 1));
    }

    #[test]
    fn sensor_transition_same_index_does_not_persist_lifecycle() {
        fn run(previous_sensor: bool, current_sensor: bool) -> ContactEvent {
            let previous_feature = test_feature(0x0100_1003, 0);
            let current_feature = test_feature(0x0100_1003, 1);
            let mut previous_record = test_contact_record(previous_feature, 85, 14, 0.0, 1.0, 0.2);
            previous_record.contact.warm_start_reason = if previous_sensor {
                WarmStartCacheReason::SkippedSensor
            } else {
                WarmStartCacheReason::Hit
            };
            previous_record.is_sensor = previous_sensor;
            previous_record.contact.warm_start_normal_impulse = 0.0;
            previous_record.contact.warm_start_tangent_impulse = 0.0;
            let previous_contact_id = previous_record.contact.contact_id;
            let previous_manifold_id = previous_record.contact.manifold_id;
            let mut previous_contacts = BTreeMap::new();
            previous_contacts.insert(
                ContactKey::new(test_collider(1), test_collider(2), previous_feature),
                previous_record,
            );
            let mut current = test_contact_observation(current_feature);
            set_observation_witness(&mut current, 0.01);
            current.is_sensor = current_sensor;
            let mut contacts = vec![current];
            let mut world = World::new(WorldDesc::default());

            world.prepare_contact_warm_start(&mut contacts, &previous_contacts);
            let expected_warm_reason = if current_sensor {
                WarmStartCacheReason::SkippedSensor
            } else {
                WarmStartCacheReason::MissFeatureId
            };
            assert_eq!(contacts[0].warm_start_reason, expected_warm_reason);
            assert_eq!(contacts[0].warm_start_normal_impulse, 0.0);
            assert_eq!(contacts[0].warm_start_tangent_impulse, 0.0);
            let (events, _, _, _) = world.refresh_contact_events(contacts, previous_contacts);
            let (event, persisted) = active_event_for_feature(&events, current_feature);
            assert!(!persisted);
            assert_eq!(event.lifecycle_reason, ContactLifecycleReason::Started);
            assert_ne!(
                event.lifecycle_reason,
                ContactLifecycleReason::PersistentEdgeSwap
            );
            assert_ne!(event.lifecycle_reason, ContactLifecycleReason::ExactFeature);
            assert_ne!(event.contact_id, previous_contact_id);
            assert_eq!(event.manifold_id, previous_manifold_id);
            *event
        }

        let solid_to_sensor = run(false, true);
        let sensor_to_sensor = run(true, true);
        let sensor_to_solid = run(true, false);
        assert_eq!(
            solid_to_sensor.warm_start_reason,
            WarmStartCacheReason::SkippedSensor
        );
        assert_eq!(
            sensor_to_sensor.warm_start_reason,
            WarmStartCacheReason::SkippedSensor
        );
        assert_eq!(
            sensor_to_solid.warm_start_reason,
            WarmStartCacheReason::MissFeatureId
        );
    }

    #[test]
    fn manifold_persistence_two_to_one_to_two_preserves_only_surviving_point() {
        let surviving_feature = test_feature(0x0100_1003, 0);
        let returning_feature = test_feature(0x0100_1004, 0);
        let mut surviving = test_contact_observation(surviving_feature);
        let mut returning = test_contact_observation(returning_feature);
        set_observation_witness(&mut surviving, -0.1);
        set_observation_witness(&mut returning, 0.1);
        surviving.normal_impulse = 1.0;
        surviving.tangent_impulse = 0.1;
        returning.normal_impulse = 2.0;
        returning.tangent_impulse = 0.2;
        let mut world = World::new(WorldDesc::default());

        let (first_events, _, _, _) = world
            .refresh_contact_events(vec![surviving.clone(), returning.clone()], BTreeMap::new());
        let first_surviving = *active_event_for_feature(&first_events, surviving_feature).0;
        let first_returning = *active_event_for_feature(&first_events, returning_feature).0;
        let first_previous = world.take_active_contacts();

        let mut second_contacts = vec![surviving.clone()];
        world.prepare_contact_warm_start(&mut second_contacts, &first_previous);
        let (second_events, _, _, _) =
            world.refresh_contact_events(second_contacts, first_previous);
        let second_surviving = *active_event_for_feature(&second_events, surviving_feature).0;
        assert_eq!(second_surviving.contact_id, first_surviving.contact_id);
        assert!(second_events.iter().any(|event| matches!(
            event,
            WorldEvent::ContactEnded(contact) if contact.contact_id == first_returning.contact_id
        )));
        let second_previous = world.take_active_contacts();
        let surviving_record = second_previous
            .get(&ContactKey::new(
                test_collider(1),
                test_collider(2),
                surviving_feature,
            ))
            .expect("surviving local witness should remain active");
        assert_eq!(surviving_record.witness_a_local, Point::new(-0.1, 0.0));
        assert_eq!(surviving_record.witness_b_local, Point::new(-0.1, 0.0));

        let mut third_contacts = vec![surviving, returning];
        world.prepare_contact_warm_start(&mut third_contacts, &second_previous);
        let returning_warm_start = third_contacts
            .iter()
            .find(|contact| contact.feature_id == returning_feature)
            .expect("returning point should be present before lifecycle refresh");
        assert_ne!(
            returning_warm_start.warm_start_reason,
            WarmStartCacheReason::Hit
        );
        assert_eq!(returning_warm_start.warm_start_normal_impulse, 0.0);
        assert_eq!(returning_warm_start.warm_start_tangent_impulse, 0.0);
        assert_eq!(returning_warm_start.normal_impulse, 0.0);
        assert_eq!(returning_warm_start.tangent_impulse, 0.0);
        let (third_events, _, _, _) = world.refresh_contact_events(third_contacts, second_previous);
        let third_surviving = *active_event_for_feature(&third_events, surviving_feature).0;
        let (third_returning, returning_persisted) =
            active_event_for_feature(&third_events, returning_feature);

        assert_eq!(third_surviving.contact_id, first_surviving.contact_id);
        assert_eq!(third_surviving.manifold_id, first_surviving.manifold_id);
        assert!(!returning_persisted);
        assert_ne!(third_returning.contact_id, first_returning.contact_id);
        assert_eq!(third_returning.manifold_id, first_surviving.manifold_id);
        let third_active = world.take_active_contacts();
        let returning_record = third_active
            .get(&ContactKey::new(
                test_collider(1),
                test_collider(2),
                returning_feature,
            ))
            .expect("returning point should become active with a fresh record");
        assert_eq!(returning_record.witness_a_local, Point::new(0.1, 0.0));
        assert_eq!(returning_record.witness_b_local, Point::new(0.1, 0.0));
        assert_eq!(returning_record.normal_impulse, 0.0);
        assert_eq!(returning_record.tangent_impulse, 0.0);
    }

    #[test]
    fn manifold_persistence_history_only_separation_does_not_fabricate_contact() {
        let mut base_world = World::new(WorldDesc {
            gravity: Vector::default(),
            ..WorldDesc::default()
        });
        let body_a = base_world
            .create_body(BodyDesc {
                body_type: BodyType::Static,
                pose: Pose::default(),
                ..BodyDesc::default()
            })
            .expect("first body should be created");
        let body_b = base_world
            .create_body(BodyDesc {
                body_type: BodyType::Static,
                pose: Pose::from_xy_angle(5.0, 0.0, 0.0),
                ..BodyDesc::default()
            })
            .expect("second body should be created");
        let collider_a = base_world
            .create_collider(
                body_a,
                ColliderDesc {
                    shape: SharedShape::rect(1.0, 1.0),
                    ..ColliderDesc::default()
                },
            )
            .expect("first collider should be created");
        let collider_b = base_world
            .create_collider(
                body_b,
                ColliderDesc {
                    shape: SharedShape::rect(1.0, 1.0),
                    ..ColliderDesc::default()
                },
            )
            .expect("second collider should be created");
        let feature = test_feature(0x0100_1003, 0);
        let mut source = test_contact_observation(feature);
        source.body_a = body_a;
        source.body_b = body_b;
        source.collider_a = collider_a;
        source.collider_b = collider_b;
        source.pair_key = ContactPairKey::new(collider_a, collider_b);
        source.key = ContactKey::new(collider_a, collider_b, feature);
        let previous_record = ContactRecord {
            contact: ContactEvent {
                body_a,
                body_b,
                collider_a,
                collider_b,
                ..test_contact_event_with_ids(feature, 91, 12)
            },
            geometry_revision_a: 0,
            geometry_revision_b: 0,
            is_sensor: false,
            witness_a_local: source.witness_a_local,
            witness_b_local: source.witness_b_local,
            normal_impulse: 1.0,
            tangent_impulse: 0.0,
        };
        let mut previous = BTreeMap::new();
        previous.insert(source.key, previous_record);

        let mut separated_world = base_world.clone();
        let mut unconfirmed = vec![source.clone()];
        separated_world.finalize_contact_observations(&mut unconfirmed);
        assert!(unconfirmed.is_empty());
        let (ended_events, contact_count, _, _) =
            separated_world.refresh_contact_events(unconfirmed, previous.clone());
        assert_eq!(contact_count, 0);
        assert!(matches!(
            ended_events.as_slice(),
            [WorldEvent::ContactEnded(_)]
        ));

        let mut confirmed_world = base_world;
        let mut confirmed = source;
        confirmed.base_normal_solve_executed = true;
        confirmed.normal_impulse = 0.5;
        let mut confirmed_contacts = vec![confirmed];
        confirmed_world.finalize_contact_observations(&mut confirmed_contacts);
        assert_eq!(confirmed_contacts.len(), 1);
        assert_eq!(confirmed_contacts[0].depth, 0.0);
        let (confirmed_events, confirmed_count, _, _) =
            confirmed_world.refresh_contact_events(confirmed_contacts, previous);
        assert_eq!(confirmed_count, 1);
        assert!(matches!(
            confirmed_events.as_slice(),
            [WorldEvent::ContactPersisted(contact)] if contact.depth == 0.0
        ));
    }

    #[test]
    fn reservation_stays_pair_scoped_for_four_by_four_inputs_and_unrelated_pairs() {
        fn run(
            include_unrelated: bool,
        ) -> (
            Vec<(ContactFeatureId, ContactId, ContactLifecycleReason)>,
            usize,
        ) {
            let previous_index = 0x0100_1003;
            let current_index = 0x0100_3001;
            let previous_features = [
                test_feature(previous_index, 0),
                test_feature(previous_index, 1),
                test_feature(0x0200_2004, 0),
                test_feature(0x0300_3005, 0),
            ];
            let current_features = [
                test_feature(current_index, 2),
                test_feature(current_index, 3),
                previous_features[2],
                previous_features[3],
            ];
            let mut p0 = test_contact_record(previous_features[0], 101, 13, 0.0, 1.0, 0.1);
            let mut p1 = test_contact_record(previous_features[1], 102, 13, 0.03, 2.0, 0.2);
            p0.contact.normal = test_normal(0.0);
            p1.contact.normal = test_normal(10.0);
            let mut previous = BTreeMap::new();
            for (feature, record) in [
                (previous_features[0], p0),
                (previous_features[1], p1),
                (
                    previous_features[2],
                    test_contact_record(previous_features[2], 103, 13, 0.1, 3.0, 0.3),
                ),
                (
                    previous_features[3],
                    test_contact_record(previous_features[3], 104, 13, 0.2, 4.0, 0.4),
                ),
            ] {
                previous.insert(
                    ContactKey::new(test_collider(1), test_collider(2), feature),
                    record,
                );
            }
            if include_unrelated {
                for index in 0..32_u32 {
                    let collider_a = test_collider(100 + index * 2);
                    let collider_b = test_collider(101 + index * 2);
                    let feature = test_feature(0x0400_4006 + index, 0);
                    let mut record =
                        test_contact_record(feature, 1000 + index, 100 + index, 0.0, 0.0, 0.0);
                    record.contact.collider_a = collider_a;
                    record.contact.collider_b = collider_b;
                    previous.insert(ContactKey::new(collider_a, collider_b, feature), record);
                }
            }
            let mut c0 = test_contact_observation(current_features[0]);
            let mut c1 = test_contact_observation(current_features[1]);
            let mut c2 = test_contact_observation(current_features[2]);
            let mut c3 = test_contact_observation(current_features[3]);
            set_observation_witness(&mut c0, 0.01);
            set_observation_witness(&mut c1, 0.03);
            set_observation_witness(&mut c2, 0.1);
            set_observation_witness(&mut c3, 0.2);
            c0.normal = test_normal(5.0);
            c1.normal = test_normal(-10.0);
            let mut world = World::new(WorldDesc::default());
            reset_lifecycle_candidate_evaluation_count();
            let (events, _, _, _) = world.refresh_contact_events(vec![c0, c1, c2, c3], previous);
            let candidate_evaluation_count = lifecycle_candidate_evaluation_count();
            let contacts = events
                .into_iter()
                .filter_map(|event| match event {
                    WorldEvent::ContactStarted(contact) | WorldEvent::ContactPersisted(contact)
                        if contact.collider_a == test_collider(1)
                            && contact.collider_b == test_collider(2) =>
                    {
                        Some((
                            contact.feature_id,
                            contact.contact_id,
                            contact.lifecycle_reason,
                        ))
                    }
                    _ => None,
                })
                .collect();
            (contacts, candidate_evaluation_count)
        }

        let (pair_only, base_candidate_evaluations) = run(false);
        let (with_unrelated, unrelated_candidate_evaluations) = run(true);
        eprintln!(
            "A15 candidate evaluations: base={base_candidate_evaluations}, with_unrelated={unrelated_candidate_evaluations}"
        );
        assert!(
            base_candidate_evaluations > 0 && base_candidate_evaluations <= 16,
            "4x4 candidate-edge instrumentation must be non-vacuous and bounded; base_candidate_evaluations={base_candidate_evaluations}"
        );
        assert!(
            unrelated_candidate_evaluations > 0,
            "unrelated-pair run must exercise the same candidate-edge predicate"
        );
        let observable_same = with_unrelated == pair_only;
        let four_distinct_persisted = pair_only.len() == 4
            && pair_only
                .iter()
                .all(|(_, _, reason)| *reason != ContactLifecycleReason::Started)
            && pair_only
                .iter()
                .map(|(_, contact_id, _)| *contact_id)
                .collect::<BTreeSet<_>>()
                .len()
                == 4;
        assert!(
            observable_same
                && four_distinct_persisted
                && base_candidate_evaluations == unrelated_candidate_evaluations,
            "reservation must stay pair-scoped and one-to-one; observable_same={observable_same}, four_distinct_persisted={four_distinct_persisted}, base_candidate_evaluations={base_candidate_evaluations}, with_unrelated_candidate_evaluations={unrelated_candidate_evaluations}, pair_only={pair_only:?}, with_unrelated={with_unrelated:?}"
        );
    }

    #[test]
    fn warm_start_fallback_transfers_between_same_feature_index_point_slots() {
        let previous_feature = test_feature(0x0100_1003, 0);
        let current_feature = test_feature(0x0100_1003, 1);
        let previous_record = ContactRecord {
            contact: test_contact_event(previous_feature),
            geometry_revision_a: 0,
            geometry_revision_b: 0,
            is_sensor: false,
            witness_a_local: Point::new(0.25, 0.0),
            witness_b_local: Point::new(-0.25, 0.0),
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
    fn same_feature_index_warm_start_uses_local_witnesses_across_rigid_rotation() {
        let previous_feature = test_feature(0x0100_1003, 0);
        let current_feature = test_feature(0x0100_1003, 1);
        let previous_record = ContactRecord {
            contact: test_contact_event(previous_feature),
            geometry_revision_a: 0,
            geometry_revision_b: 0,
            is_sensor: false,
            witness_a_local: Point::new(2.0, 0.0),
            witness_b_local: Point::new(-2.0, 0.0),
            normal_impulse: 0.25,
            tangent_impulse: 0.05,
        };
        let mut previous_contacts = BTreeMap::new();
        previous_contacts.insert(
            ContactKey::new(test_collider(1), test_collider(2), previous_feature),
            previous_record,
        );
        let mut contact = test_contact_observation(current_feature);
        let angle: FloatNum = 0.1;
        contact.normal = Vector::new(angle.cos(), angle.sin());
        contact.anchor_a = Vector::new(2.0, 0.0).rotated(angle);
        contact.anchor_b = Vector::new(-2.0, 0.0).rotated(angle);
        contact.witness_a_local = Point::new(2.0, 0.0);
        contact.witness_b_local = Point::new(-2.0, 0.0);
        let mut contacts = vec![contact];
        let world = World::new(WorldDesc::default());

        world.prepare_contact_warm_start(&mut contacts, &previous_contacts);

        assert_eq!(contacts[0].warm_start_reason, WarmStartCacheReason::Hit);
        assert_eq!(contacts[0].warm_start_normal_impulse, 0.25);
        assert_eq!(contacts[0].warm_start_tangent_impulse, 0.05);
    }

    #[test]
    fn warm_start_projects_local_witness_drift_in_each_collider_basis() {
        let previous_feature = test_feature(0x0100_1003, 0);
        let current_feature = test_feature(0x0100_1003, 1);
        let previous_record = ContactRecord {
            contact: test_contact_event(previous_feature),
            geometry_revision_a: 0,
            geometry_revision_b: 0,
            is_sensor: false,
            witness_a_local: Point::new(0.25, 0.0),
            witness_b_local: Point::new(-0.25, 0.0),
            normal_impulse: 0.25,
            tangent_impulse: 0.05,
        };
        let mut previous_contacts = BTreeMap::new();
        previous_contacts.insert(
            ContactKey::new(test_collider(1), test_collider(2), previous_feature),
            previous_record,
        );
        let local_delta = Vector::new(0.09, 0.04);
        let mut contact = test_contact_observation(current_feature);
        contact.witness_a_local += local_delta;
        contact.witness_b_local += local_delta;
        contact.normal_a_local = Vector::new(0.0, 1.0);
        contact.normal_b_local = Vector::new(0.0, 1.0);
        let mut contacts = vec![contact];
        let world = World::new(WorldDesc::default());

        world.prepare_contact_warm_start(&mut contacts, &previous_contacts);

        assert_eq!(contacts[0].warm_start_reason, WarmStartCacheReason::Hit);
    }

    #[test]
    fn exact_warm_start_rejects_single_sided_nonfinite_witness() {
        let feature = test_feature(0x0100_1003, 0);
        let previous_record = test_contact_record(feature, 111, 18, 0.0, 1.5, 0.5);
        let mut previous_contacts = BTreeMap::new();
        previous_contacts.insert(
            ContactKey::new(test_collider(1), test_collider(2), feature),
            previous_record,
        );
        let mut current = test_contact_observation(feature);
        set_observation_witness(&mut current, 0.0);
        current.witness_a_local = Point::new(FloatNum::NAN, 0.0);
        let mut contacts = vec![current];
        let world = World::new(WorldDesc::default());

        world.prepare_contact_warm_start(&mut contacts, &previous_contacts);

        assert_eq!(
            contacts[0].warm_start_reason,
            WarmStartCacheReason::DroppedPointDrift
        );
        assert_eq!(contacts[0].warm_start_normal_impulse, 0.0);
        assert_eq!(contacts[0].warm_start_tangent_impulse, 0.0);
        assert_eq!(contacts[0].normal_impulse, 0.0);
        assert_eq!(contacts[0].tangent_impulse, 0.0);
    }

    #[test]
    fn confirmed_solver_interaction_requires_finite_positive_non_sensor_impulse() {
        let mut contact = test_contact_observation(test_feature(0x0100_1003, 0));
        contact.normal_impulse = 0.25;
        assert!(!confirmed_solver_interaction(&contact));

        contact.base_normal_solve_executed = true;
        assert!(confirmed_solver_interaction(&contact));

        contact.normal_impulse = 0.0;
        assert!(!confirmed_solver_interaction(&contact));

        let zero_impulse_from_any_discovery_origin = contact.clone();
        assert!(!confirmed_solver_interaction(
            &zero_impulse_from_any_discovery_origin
        ));

        contact.normal_impulse = FloatNum::NAN;
        assert!(!confirmed_solver_interaction(&contact));

        contact.normal_impulse = 0.25;
        contact.is_sensor = true;
        assert!(!confirmed_solver_interaction(&contact));
    }

    #[test]
    fn final_source_match_rejects_non_exact_distant_local_witnesses() {
        let mut source = test_contact_observation(test_feature(0x0100_1003, 0));
        source.witness_a_local = Point::new(1.0, 0.0);
        source.witness_b_local = Point::new(-1.0, 0.0);
        let final_point = ContactPointGeometry {
            point: Point::new(0.0, 0.0),
            depth: 0.01,
            feature_id: test_feature(0x0100_1004, 0),
        };

        let matched = final_source_match(
            &[Some(source)],
            Pose::default(),
            Pose::default(),
            Vector::new(1.0, 0.0),
            final_point,
        );

        assert_eq!(matched, None);
    }

    #[test]
    fn final_source_reservation_preserves_later_exact_match() {
        let small_feature = test_feature(0x0100_1001, 0);
        let fallback_feature = test_feature(0x0100_1002, 0);
        let large_feature = test_feature(0x0100_1003, 0);
        let mut exact_large = test_contact_observation(large_feature);
        exact_large.witness_a_local = Point::new(0.005, 0.0);
        exact_large.witness_b_local = Point::new(0.015, 0.0);
        exact_large.normal_impulse = 2.0;
        let mut fallback_small = test_contact_observation(fallback_feature);
        fallback_small.witness_a_local = Point::new(0.045, 0.0);
        fallback_small.witness_b_local = Point::new(0.055, 0.0);
        fallback_small.normal_impulse = 1.0;
        let mut sources = vec![Some(fallback_small), Some(exact_large)];
        sources.sort_by_key(|source| source.as_ref().map(|source| source.key));
        let final_points = [
            ContactPointGeometry {
                point: Point::new(0.0, 0.0),
                depth: 0.01,
                feature_id: small_feature,
            },
            ContactPointGeometry {
                point: Point::new(0.2, 0.0),
                depth: 0.01,
                feature_id: large_feature,
            },
        ];

        let matched = reserve_final_sources(
            &mut sources,
            Pose::default(),
            Pose::default(),
            Vector::new(1.0, 0.0),
            &final_points,
        );

        assert_eq!(
            matched[0].as_ref().map(|source| source.normal_impulse),
            Some(1.0)
        );
        assert_eq!(
            matched[1].as_ref().map(|source| source.normal_impulse),
            Some(2.0)
        );
        assert!(sources.iter().all(Option::is_none));
    }

    #[test]
    fn warm_start_fallback_survives_clip_manifold_point_count_oscillation() {
        // A settling two-point clipped manifold can drop to one point for a
        // frame (reduction reason Clipped -> SinglePoint) and come back. The
        // surviving point keeps its pair, feature index, anchors, and normal;
        // the reduction label alone must not drop its warm-start impulses.
        let previous_feature = test_feature(0x0100_1003, 1);
        let current_feature = test_feature(0x0100_1003, 0);
        let previous_record = ContactRecord {
            contact: test_contact_event(previous_feature),
            geometry_revision_a: 0,
            geometry_revision_b: 0,
            is_sensor: false,
            witness_a_local: Point::new(0.25, 0.0),
            witness_b_local: Point::new(-0.25, 0.0),
            normal_impulse: 0.25,
            tangent_impulse: 0.05,
        };
        let mut previous_contacts = BTreeMap::new();
        previous_contacts.insert(
            ContactKey::new(test_collider(1), test_collider(2), previous_feature),
            previous_record,
        );
        let mut contacts = vec![test_contact_observation(current_feature)];
        contacts[0].reduction_reason = ContactReductionReason::SinglePoint;
        let world = World::new(WorldDesc::default());

        world.prepare_contact_warm_start(&mut contacts, &previous_contacts);

        assert_eq!(contacts[0].warm_start_reason, WarmStartCacheReason::Hit);
        assert_eq!(contacts[0].warm_start_normal_impulse, 0.25);
        assert_eq!(contacts[0].warm_start_tangent_impulse, 0.05);
    }

    #[test]
    fn source_row_continuity_candidate_marks_non_edge_swap_feature_miss() {
        let previous_feature = test_feature(0x0100_1003, 0);
        let current_feature = test_feature(0x0100_1004, 0);
        let previous_record = ContactRecord {
            contact: test_contact_event(previous_feature),
            geometry_revision_a: 0,
            geometry_revision_b: 0,
            is_sensor: false,
            witness_a_local: Point::new(0.25, 0.0),
            witness_b_local: Point::new(-0.25, 0.0),
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

        assert_eq!(
            source_row_continuity_reason(&previous_contacts, &contacts[0]),
            SourceRowContinuityReason::Candidate
        );

        world.prepare_contact_warm_start(&mut contacts, &previous_contacts);

        assert_eq!(
            contacts[0].warm_start_reason,
            WarmStartCacheReason::MissFeatureId
        );
        assert!(contacts[0].source_row_continuity_candidate);
        assert_eq!(
            contacts[0].source_row_continuity_reason,
            SourceRowContinuityReason::Candidate
        );
        assert_eq!(contacts[0].warm_start_normal_impulse, 0.0);
        assert_eq!(contacts[0].warm_start_tangent_impulse, 0.0);
        assert_eq!(contacts[0].normal_impulse, 0.0);
        assert_eq!(contacts[0].tangent_impulse, 0.0);
    }

    #[test]
    fn edge_swap_manifold_can_warm_start_without_source_row_continuity() {
        let previous_feature = test_feature(0x0100_1003, 0);
        let current_feature = test_feature(0x0100_3001, 0);
        let previous_record = ContactRecord {
            contact: test_contact_event(previous_feature),
            geometry_revision_a: 0,
            geometry_revision_b: 0,
            is_sensor: false,
            witness_a_local: Point::new(0.25, 0.0),
            witness_b_local: Point::new(-0.25, 0.0),
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

        assert_eq!(
            source_row_continuity_reason(&previous_contacts, &contacts[0]),
            SourceRowContinuityReason::EdgeSwap
        );

        world.prepare_contact_warm_start(&mut contacts, &previous_contacts);

        assert_eq!(contacts[0].warm_start_reason, WarmStartCacheReason::Hit);
        assert!(!contacts[0].source_row_continuity_candidate);
        assert_eq!(
            contacts[0].source_row_continuity_reason,
            SourceRowContinuityReason::EdgeSwap
        );
        assert_eq!(contacts[0].warm_start_normal_impulse, 0.25);
        assert_eq!(contacts[0].warm_start_tangent_impulse, 0.0);
        assert_eq!(contacts[0].normal_impulse, 0.25);
        assert_eq!(contacts[0].tangent_impulse, 0.0);
    }

    #[test]
    fn edge_swap_warm_hit_ignores_source_compatible_distractor() {
        let edge_feature = test_feature(0x0100_1003, 0);
        let distractor_feature = test_feature(0x0100_1004, 0);
        let current_feature = test_feature(0x0100_3001, 0);
        let edge_record = test_contact_record(edge_feature, 112, 19, 0.0, 1.0, 0.2);
        let distractor_record = test_contact_record(distractor_feature, 113, 19, 0.01, 2.0, 0.3);
        let mut previous_contacts = BTreeMap::new();
        previous_contacts.insert(
            ContactKey::new(test_collider(1), test_collider(2), edge_feature),
            edge_record,
        );
        previous_contacts.insert(
            ContactKey::new(test_collider(1), test_collider(2), distractor_feature),
            distractor_record,
        );
        let mut current = test_contact_observation(current_feature);
        set_observation_witness(&mut current, 0.0);
        let mut contacts = vec![current];
        let world = World::new(WorldDesc::default());

        world.prepare_contact_warm_start(&mut contacts, &previous_contacts);

        assert_eq!(contacts[0].warm_start_reason, WarmStartCacheReason::Hit);
        assert!(!contacts[0].source_row_continuity_candidate);
        assert_eq!(
            contacts[0].source_row_continuity_reason,
            SourceRowContinuityReason::EdgeSwap
        );
    }

    #[test]
    fn edge_swap_warm_start_survives_clip_manifold_point_count_oscillation() {
        // SAT reference-face swaps on a settling manifold often coincide with
        // the manifold dropping to a single point for that frame. The reduction
        // label change (Clipped -> SinglePoint) must not disqualify the
        // persistent edge-swap warm-start path when pair, swapped feature
        // edges, normal, and local anchors all stay compatible.
        let previous_feature = test_feature(0x0100_1003, 0);
        let current_feature = test_feature(0x0100_3001, 0);
        let previous_record = ContactRecord {
            contact: test_contact_event(previous_feature),
            geometry_revision_a: 0,
            geometry_revision_b: 0,
            is_sensor: false,
            witness_a_local: Point::new(0.25, 0.0),
            witness_b_local: Point::new(-0.25, 0.0),
            normal_impulse: 0.25,
            tangent_impulse: 0.05,
        };
        let mut previous_contacts = BTreeMap::new();
        previous_contacts.insert(
            ContactKey::new(test_collider(1), test_collider(2), previous_feature),
            previous_record,
        );
        let mut contacts = vec![test_contact_observation(current_feature)];
        contacts[0].reduction_reason = ContactReductionReason::SinglePoint;
        let world = World::new(WorldDesc::default());

        world.prepare_contact_warm_start(&mut contacts, &previous_contacts);

        assert_eq!(contacts[0].warm_start_reason, WarmStartCacheReason::Hit);
        assert_eq!(contacts[0].warm_start_normal_impulse, 0.25);
        assert_eq!(
            contacts[0].warm_start_tangent_impulse, 0.0,
            "edge-swap transfers stay conservative about the tangent basis"
        );
    }

    #[test]
    fn refresh_contact_events_reports_edge_swap_lifecycle_without_warm_start_transfer() {
        let previous_feature = test_feature(0x0100_1003, 0);
        let current_feature = test_feature(0x0100_3001, 0);
        let previous_event = test_contact_event(previous_feature);
        let previous_contact_id = previous_event.contact_id;
        let previous_manifold_id = previous_event.manifold_id;
        let previous_record = ContactRecord {
            contact: previous_event,
            geometry_revision_a: 0,
            geometry_revision_b: 0,
            is_sensor: false,
            witness_a_local: Point::new(0.25, 0.0),
            witness_b_local: Point::new(-0.25, 0.0),
            normal_impulse: 0.25,
            tangent_impulse: 0.05,
        };
        let mut previous_contacts = BTreeMap::new();
        previous_contacts.insert(
            ContactKey::new(test_collider(1), test_collider(2), previous_feature),
            previous_record,
        );
        let mut current_contact = test_contact_observation(current_feature);
        current_contact.warm_start_reason = WarmStartCacheReason::MissFeatureId;
        let mut world = World::new(WorldDesc::default());

        let (events, contact_count, manifold_count, warm_start_stats) =
            world.refresh_contact_events(vec![current_contact], previous_contacts);

        assert_eq!(contact_count, 1);
        assert_eq!(manifold_count, 1);
        assert_eq!(warm_start_stats.hit_count, 0);
        assert_eq!(warm_start_stats.miss_count, 1);
        let WorldEvent::ContactPersisted(event) = &events[0] else {
            panic!("edge-swap lifecycle should keep contact persisted");
        };
        assert_eq!(event.contact_id, previous_contact_id);
        assert_eq!(event.manifold_id, previous_manifold_id);
        assert_eq!(
            event.lifecycle_reason,
            ContactLifecycleReason::PersistentEdgeSwap
        );
        assert_eq!(event.warm_start_reason, WarmStartCacheReason::MissFeatureId);
        assert_eq!(event.warm_start_normal_impulse, 0.0);
        assert_eq!(event.warm_start_tangent_impulse, 0.0);
    }

    #[test]
    fn source_row_continuity_candidate_excludes_anchor_drift() {
        let previous_feature = test_feature(0x0100_1003, 0);
        let current_feature = test_feature(0x0100_1004, 0);
        let previous_record = ContactRecord {
            contact: test_contact_event(previous_feature),
            geometry_revision_a: 0,
            geometry_revision_b: 0,
            is_sensor: false,
            witness_a_local: Point::new(0.25, 0.0),
            witness_b_local: Point::new(-0.25, 0.0),
            normal_impulse: 0.25,
            tangent_impulse: 0.05,
        };
        let mut previous_contacts = BTreeMap::new();
        previous_contacts.insert(
            ContactKey::new(test_collider(1), test_collider(2), previous_feature),
            previous_record,
        );
        let mut contact = test_contact_observation(current_feature);
        contact.witness_a_local = Point::new(0.75, 0.0);
        let mut contacts = vec![contact];
        let world = World::new(WorldDesc::default());

        assert_eq!(
            source_row_continuity_reason(&previous_contacts, &contacts[0]),
            SourceRowContinuityReason::AnchorDrift
        );

        world.prepare_contact_warm_start(&mut contacts, &previous_contacts);

        assert_eq!(
            contacts[0].warm_start_reason,
            WarmStartCacheReason::MissFeatureId
        );
        assert!(!contacts[0].source_row_continuity_candidate);
        assert_eq!(
            contacts[0].source_row_continuity_reason,
            SourceRowContinuityReason::AnchorDrift
        );
        assert_eq!(contacts[0].warm_start_normal_impulse, 0.0);
        assert_eq!(contacts[0].warm_start_tangent_impulse, 0.0);
    }

    #[test]
    fn source_row_continuity_reason_distinguishes_pair_mismatch() {
        let previous_feature = test_feature(0x0100_1003, 0);
        let current_feature = test_feature(0x0100_1004, 0);
        let mut previous_event = test_contact_event(previous_feature);
        previous_event.collider_a = test_collider(3);
        previous_event.collider_b = test_collider(4);
        let previous_record = ContactRecord {
            contact: previous_event,
            geometry_revision_a: 0,
            geometry_revision_b: 0,
            is_sensor: false,
            witness_a_local: Point::new(0.25, 0.0),
            witness_b_local: Point::new(-0.25, 0.0),
            normal_impulse: 0.25,
            tangent_impulse: 0.05,
        };
        let mut previous_contacts = BTreeMap::new();
        previous_contacts.insert(
            ContactKey::new(test_collider(3), test_collider(4), previous_feature),
            previous_record,
        );
        let contact = test_contact_observation(current_feature);
        let mut contacts = vec![test_contact_observation(current_feature)];
        let mut no_previous_contacts = vec![test_contact_observation(current_feature)];
        let world = World::new(WorldDesc::default());

        assert_eq!(
            source_row_continuity_reason(&previous_contacts, &contact),
            SourceRowContinuityReason::PairMismatch
        );
        assert_eq!(
            source_row_continuity_reason(&BTreeMap::new(), &contact),
            SourceRowContinuityReason::NoPreviousPair
        );

        world.prepare_contact_warm_start(&mut contacts, &previous_contacts);
        assert_eq!(
            contacts[0].source_row_continuity_reason,
            SourceRowContinuityReason::PairMismatch
        );
        assert!(!contacts[0].source_row_continuity_candidate);

        world.prepare_contact_warm_start(&mut no_previous_contacts, &BTreeMap::new());
        assert_eq!(
            no_previous_contacts[0].source_row_continuity_reason,
            SourceRowContinuityReason::NoPreviousPair
        );
        assert!(!no_previous_contacts[0].source_row_continuity_candidate);
    }
}
