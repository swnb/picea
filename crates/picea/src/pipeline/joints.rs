use std::collections::{BTreeMap, BTreeSet};

use crate::{
    body::{BodyType, MassProperties, Pose},
    events::{NumericsWarningEvent, SleepTransitionReason},
    handles::BodyHandle,
    joint::{DistanceJointDesc, RevoluteJointDesc, WorldAnchorJointDesc},
    math::{point::Point, vector::Vector, FloatNum},
    pipeline::island::SolverStepStats,
    solver::body_state::{
        apply_revolute_pose_pair_atomically, apply_revolute_pose_pair_atomically_with_wake,
        apply_revolute_velocity_pair_atomically,
    },
    world::World,
};

use super::integrate::{is_finite_pose, is_finite_vector};

struct JointSolveBatch {
    body_slots: Vec<BodyHandle>,
    rows: Vec<JointSolverRow>,
}

enum JointSolverRow {
    Distance {
        desc: DistanceJointDesc,
        body_a_slot: usize,
        body_b_slot: usize,
    },
    WorldAnchor {
        desc: WorldAnchorJointDesc,
        body_slot: usize,
    },
    Revolute {
        desc: RevoluteJointDesc,
        body_a_slot: usize,
        body_b_slot: usize,
    },
}

struct RevolutePostBatch {
    body_slots: Vec<BodyHandle>,
    rows: Vec<RevolutePostRow>,
}

struct RevolutePostRow {
    desc: RevoluteJointDesc,
    body_a_slot: usize,
    body_b_slot: usize,
}

/// The mandatory phase is the sole owner of joint stats. After its position
/// rows run, only Revolute carriers survive for the post-contact pose pass;
/// optional velocity projection deliberately rebuilds its own live plan.
pub(crate) struct MandatoryJointPlan {
    revolute_post_batches: Vec<RevolutePostBatch>,
    stats: SolverStepStats,
}

impl MandatoryJointPlan {
    pub(crate) fn stats(&self) -> SolverStepStats {
        self.stats
    }

    fn from_solved_batches(batches: Vec<JointSolveBatch>, stats: SolverStepStats) -> Self {
        let revolute_post_batches = batches
            .into_iter()
            .filter_map(|batch| {
                let rows = batch
                    .rows
                    .into_iter()
                    .filter_map(|row| match row {
                        JointSolverRow::Revolute {
                            desc,
                            body_a_slot,
                            body_b_slot,
                        } => Some(RevolutePostRow {
                            desc,
                            body_a_slot,
                            body_b_slot,
                        }),
                        JointSolverRow::Distance { .. } | JointSolverRow::WorldAnchor { .. } => {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                (!rows.is_empty()).then_some(RevolutePostBatch {
                    body_slots: batch.body_slots,
                    rows,
                })
            })
            .collect();
        Self {
            revolute_post_batches,
            stats,
        }
    }
}

pub(crate) fn solve_joint_phase(
    world: &mut World,
    dt: FloatNum,
    ccd_clamped_bodies: &BTreeSet<BodyHandle>,
    wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
    numeric_warnings: &mut Vec<NumericsWarningEvent>,
) -> MandatoryJointPlan {
    let islands = crate::pipeline::sleep::build_active_solver_islands(
        world,
        std::iter::empty::<(BodyHandle, BodyHandle)>(),
        wake_reasons,
    );
    let batches = joint_solve_batches(world, &islands);
    let stats = joint_solver_stats(&batches);
    world.apply_joint_constraints(
        dt,
        ccd_clamped_bodies,
        &batches,
        wake_reasons,
        numeric_warnings,
    );
    MandatoryJointPlan::from_solved_batches(batches, stats)
}

pub(crate) fn solve_joint_velocity_phase(
    world: &mut World,
    wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
) {
    let islands = crate::pipeline::sleep::build_active_solver_islands(
        world,
        std::iter::empty::<(BodyHandle, BodyHandle)>(),
        wake_reasons,
    );
    let velocity_batches = joint_solve_batches(world, &islands);
    world.apply_joint_velocity_constraints(&velocity_batches, wake_reasons);
}

pub(crate) fn reconcile_revolute_post_contact_phase(
    world: &mut World,
    dt: FloatNum,
    ccd_clamped_bodies: &BTreeSet<BodyHandle>,
    plan: &MandatoryJointPlan,
    wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
) {
    world.reconcile_revolute_rows(dt, ccd_clamped_bodies, plan, wake_reasons);
}

impl World {
    fn apply_joint_constraints(
        &mut self,
        dt: FloatNum,
        ccd_clamped_bodies: &BTreeSet<BodyHandle>,
        batches: &[JointSolveBatch],
        wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
        numeric_warnings: &mut Vec<NumericsWarningEvent>,
    ) {
        for batch in batches {
            for row in &batch.rows {
                match row {
                    JointSolverRow::Distance {
                        desc,
                        body_a_slot,
                        body_b_slot,
                    } => {
                        let body_a = batch.body_slots[*body_a_slot];
                        let body_b = batch.body_slots[*body_b_slot];
                        let pose_a = self
                            .body_record(body_a)
                            .expect("joint endpoints must stay live during step")
                            .pose;
                        let pose_b = self
                            .body_record(body_b)
                            .expect("joint endpoints must stay live during step")
                            .pose;
                        let anchor_a = pose_a.transform_point(desc.local_anchor_a);
                        let anchor_b = pose_b.transform_point(desc.local_anchor_b);
                        let delta = anchor_b - anchor_a;
                        let distance = delta.length();
                        let direction = normalized_or_x_axis(delta);
                        let error = distance - desc.rest_length;
                        let stiffness = desc.stiffness.max(0.0);
                        if stiffness <= f32::EPSILON {
                            continue;
                        }
                        let correction = direction * error * stiffness * dt;
                        if !is_finite_vector(correction) {
                            numeric_warnings.push(NumericsWarningEvent {
                                phase: "joint_solve".into(),
                                detail: "distance_joint_correction".into(),
                            });
                            continue;
                        }
                        if error.abs() > f32::EPSILON {
                            self.apply_body_pair_position_correction_preserve_velocity(
                                body_a,
                                body_b,
                                correction,
                                wake_reasons,
                            );
                        }
                    }
                    JointSolverRow::WorldAnchor { desc, body_slot } => {
                        let body = batch.body_slots[*body_slot];
                        let pose = self.world_anchor_solver_pose(
                            body,
                            dt,
                            ccd_clamped_bodies,
                            numeric_warnings,
                        );
                        let anchor = pose.transform_point(desc.local_anchor);
                        let axis_to_world_anchor = desc.world_anchor - anchor;
                        let correction = axis_to_world_anchor * desc.stiffness.max(0.0) * dt;
                        if !is_finite_vector(correction) {
                            numeric_warnings.push(NumericsWarningEvent {
                                phase: "joint_solve".into(),
                                detail: "world_anchor_joint_correction".into(),
                            });
                            continue;
                        }
                        self.apply_single_body_position_correction_preserve_velocity(
                            body,
                            correction,
                            wake_reasons,
                        );

                        let damping_strength = (desc.damping * dt).clamp(0.0, 1.0);
                        if damping_strength > 0.0 {
                            // Position correction changes the authoritative pose, so rebuild
                            // the post-velocity view before evaluating damping. CCD-clamped
                            // bodies already sit at their time of impact and must not be
                            // advanced a second time.
                            let damping_pose = self.world_anchor_solver_pose(
                                body,
                                dt,
                                ccd_clamped_bodies,
                                numeric_warnings,
                            );
                            let record = self
                                .body_record(body)
                                .expect("joint endpoint must stay live during step");
                            let damping_anchor = damping_pose.transform_point(desc.local_anchor);
                            let axis = (desc.world_anchor - damping_anchor).normalized_or_zero();
                            // A coincident anchor has no physical constraint axis. Unlike the
                            // positional row's deterministic fallback, damping must skip this
                            // frame rather than introduce an arbitrary x-direction bias.
                            if axis.length() <= f32::EPSILON {
                                continue;
                            }
                            let world_center_of_mass = damping_pose
                                .transform_point(record.mass_properties.local_center_of_mass);
                            let anchor_from_center = damping_anchor - world_center_of_mass;
                            // Positive angular velocity is clockwise in Picea screen space, so
                            // the point velocity contribution is omega * (r.y, -r.x).
                            let point_velocity = record.linear_velocity
                                + Vector::new(
                                    record.angular_velocity * anchor_from_center.y(),
                                    -record.angular_velocity * anchor_from_center.x(),
                                );
                            let radial_speed = point_velocity.dot(axis);
                            let angular_leverage = anchor_from_center.cross(axis);
                            let effective_inverse_mass = record.mass_properties.inverse_mass
                                + record.mass_properties.inverse_inertia
                                    * angular_leverage
                                    * angular_leverage;
                            if !effective_inverse_mass.is_finite()
                                || effective_inverse_mass <= f32::EPSILON
                            {
                                continue;
                            }
                            // World-anchor damping belongs to this mandatory joint row rather
                            // than the optional distance-joint projection phase. Clamping the
                            // per-step strength prevents overdamping from reversing the radial
                            // velocity and injecting energy when damping * dt exceeds one.
                            let target_radial_delta = -radial_speed * damping_strength;
                            let impulse = axis * (target_radial_delta / effective_inverse_mass);
                            if !is_finite_vector(impulse) {
                                numeric_warnings.push(NumericsWarningEvent {
                                    phase: "joint_solve".into(),
                                    detail: "world_anchor_joint_damping".into(),
                                });
                                continue;
                            }
                            self.apply_single_body_point_impulse(
                                body,
                                anchor_from_center,
                                impulse,
                                wake_reasons,
                            );
                        }
                    }
                    JointSolverRow::Revolute {
                        desc,
                        body_a_slot,
                        body_b_slot,
                    } => {
                        let body_a = batch.body_slots[*body_a_slot];
                        let body_b = batch.body_slots[*body_b_slot];
                        let Some(solution) = self.revolute_pose_solution(
                            desc,
                            body_a,
                            body_b,
                            dt,
                            ccd_clamped_bodies,
                            false,
                            false,
                        ) else {
                            continue;
                        };
                        apply_revolute_pose_solution(
                            self,
                            body_a,
                            body_b,
                            solution,
                            false,
                            false,
                            wake_reasons,
                        );
                    }
                }
            }
        }
    }

    fn world_anchor_solver_pose(
        &self,
        body: BodyHandle,
        dt: FloatNum,
        ccd_clamped_bodies: &BTreeSet<BodyHandle>,
        numeric_warnings: &mut Vec<NumericsWarningEvent>,
    ) -> Pose {
        let record = self
            .body_record(body)
            .expect("joint endpoint must stay live during step");
        let current = record.pose;
        if ccd_clamped_bodies.contains(&body) {
            return current;
        }

        // WorldAnchor solves before final position integration in the
        // velocity-first pipeline. Evaluate its geometry at the endpoint that
        // this body's current velocities would reach, matching the later pose
        // integration without mutating authoritative state early.
        let predicted = crate::pipeline::integrate::translated_pose(
            current,
            record.linear_velocity * dt,
            record.angular_velocity * dt,
        );
        if is_finite_pose(predicted) {
            predicted
        } else {
            numeric_warnings.push(NumericsWarningEvent {
                phase: "joint_solve".into(),
                detail: "world_anchor_joint_prediction".into(),
            });
            current
        }
    }

    fn apply_joint_velocity_constraints(
        &mut self,
        batches: &[JointSolveBatch],
        wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
    ) {
        for batch in batches {
            for row in &batch.rows {
                match row {
                    JointSolverRow::Distance {
                        desc,
                        body_a_slot,
                        body_b_slot,
                    } => {
                        if desc.stiffness.max(0.0) <= f32::EPSILON {
                            continue;
                        }
                        let body_a = batch.body_slots[*body_a_slot];
                        let body_b = batch.body_slots[*body_b_slot];
                        let pose_a = self
                            .body_record(body_a)
                            .expect("joint endpoints must stay live during step")
                            .pose;
                        let pose_b = self
                            .body_record(body_b)
                            .expect("joint endpoints must stay live during step")
                            .pose;
                        let anchor_a = pose_a.transform_point(desc.local_anchor_a);
                        let anchor_b = pose_b.transform_point(desc.local_anchor_b);
                        let direction = normalized_or_x_axis(anchor_b - anchor_a);
                        self.apply_body_pair_radial_velocity_constraint(
                            body_a,
                            body_b,
                            direction,
                            wake_reasons,
                        );
                    }
                    JointSolverRow::WorldAnchor { .. } => {}
                    JointSolverRow::Revolute {
                        desc,
                        body_a_slot,
                        body_b_slot,
                    } => {
                        let body_a = batch.body_slots[*body_a_slot];
                        let body_b = batch.body_slots[*body_b_slot];
                        self.apply_revolute_velocity_constraint(desc, body_a, body_b, wake_reasons);
                    }
                }
            }
        }
    }

    fn apply_revolute_velocity_constraint(
        &mut self,
        desc: &RevoluteJointDesc,
        body_a: BodyHandle,
        body_b: BodyHandle,
        wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
    ) {
        let (pose_a, mass_a, linear_a, angular_a) = match self.body_record(body_a) {
            Ok(record) => (
                record.pose,
                record.mass_properties,
                record.linear_velocity,
                record.angular_velocity,
            ),
            Err(_) => return,
        };
        let (pose_b, mass_b, linear_b, angular_b) = match self.body_record(body_b) {
            Ok(record) => (
                record.pose,
                record.mass_properties,
                record.linear_velocity,
                record.angular_velocity,
            ),
            Err(_) => return,
        };
        let Some(geometry) = revolute_geometry(
            pose_a,
            mass_a,
            desc.local_anchor_a,
            pose_b,
            mass_b,
            desc.local_anchor_b,
        ) else {
            return;
        };
        if !is_finite_vector(linear_a)
            || !angular_a.is_finite()
            || !is_finite_vector(linear_b)
            || !angular_b.is_finite()
        {
            return;
        }
        let point_velocity_a = linear_a + point_angular_velocity(angular_a, geometry.lever_a);
        let point_velocity_b = linear_b + point_angular_velocity(angular_b, geometry.lever_b);
        let relative_point_velocity = point_velocity_b - point_velocity_a;
        let Some(lambda) =
            inverse_multiply_negated(geometry.inverse_effective_mass, relative_point_velocity)
        else {
            return;
        };
        let Some(deltas) = revolute_position_deltas(
            mass_a.inverse_mass,
            mass_a.inverse_inertia,
            geometry.lever_a,
            mass_b.inverse_mass,
            mass_b.inverse_inertia,
            geometry.lever_b,
            lambda,
        ) else {
            return;
        };
        apply_revolute_velocity_pair_atomically(
            self,
            body_a,
            deltas.translation_a,
            deltas.angle_a,
            body_b,
            deltas.translation_b,
            deltas.angle_b,
            wake_reasons,
        );
    }

    fn reconcile_revolute_rows(
        &mut self,
        dt: FloatNum,
        ccd_clamped_bodies: &BTreeSet<BodyHandle>,
        plan: &MandatoryJointPlan,
        wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
    ) {
        for batch in &plan.revolute_post_batches {
            for row in &batch.rows {
                let body_a = batch.body_slots[row.body_a_slot];
                let body_b = batch.body_slots[row.body_b_slot];
                let Some(probe) = self.revolute_pose_solution(
                    &row.desc,
                    body_a,
                    body_b,
                    dt,
                    ccd_clamped_bodies,
                    false,
                    false,
                ) else {
                    continue;
                };
                let would_wake_a = probe.endpoint_a.sleeping
                    && probe.endpoint_a.body_type.is_dynamic()
                    && correction_is_nonzero(probe.deltas.translation_a, probe.deltas.angle_a);
                let would_wake_b = probe.endpoint_b.sleeping
                    && probe.endpoint_b.body_type.is_dynamic()
                    && correction_is_nonzero(probe.deltas.translation_b, probe.deltas.angle_b);

                // A sleeping endpoint is first probed at current pose. Once that
                // finite demand establishes wake eligibility, rebuild the whole
                // row using the velocity advance final integration will consume.
                let solution = if would_wake_a || would_wake_b {
                    let Some(recomputed) = self.revolute_pose_solution(
                        &row.desc,
                        body_a,
                        body_b,
                        dt,
                        ccd_clamped_bodies,
                        would_wake_a,
                        would_wake_b,
                    ) else {
                        continue;
                    };
                    recomputed
                } else {
                    probe
                };
                apply_revolute_pose_solution(
                    self,
                    body_a,
                    body_b,
                    solution,
                    would_wake_a,
                    would_wake_b,
                    wake_reasons,
                );
            }
        }
    }

    // The row is intentionally evaluated as one atomic two-endpoint operation.
    // Keeping both handles and both wake decisions explicit makes it harder to
    // accidentally apply a partial constraint or reuse one endpoint's state.
    #[allow(clippy::too_many_arguments)]
    fn revolute_pose_solution(
        &self,
        desc: &RevoluteJointDesc,
        body_a: BodyHandle,
        body_b: BodyHandle,
        dt: FloatNum,
        ccd_clamped_bodies: &BTreeSet<BodyHandle>,
        advance_sleeping_a: bool,
        advance_sleeping_b: bool,
    ) -> Option<RevolutePoseSolution> {
        let endpoint_a =
            self.sample_revolute_endpoint(body_a, dt, ccd_clamped_bodies, advance_sleeping_a)?;
        let endpoint_b =
            self.sample_revolute_endpoint(body_b, dt, ccd_clamped_bodies, advance_sleeping_b)?;
        let geometry = revolute_geometry(
            endpoint_a.eval_pose,
            endpoint_a.mass,
            desc.local_anchor_a,
            endpoint_b.eval_pose,
            endpoint_b.mass,
            desc.local_anchor_b,
        )?;
        let constraint_error = geometry.anchor_b - geometry.anchor_a;
        let lambda = inverse_multiply_negated(geometry.inverse_effective_mass, constraint_error)?;
        let deltas = revolute_position_deltas(
            endpoint_a.mass.inverse_mass,
            endpoint_a.mass.inverse_inertia,
            geometry.lever_a,
            endpoint_b.mass.inverse_mass,
            endpoint_b.mass.inverse_inertia,
            geometry.lever_b,
            lambda,
        )?;
        Some(RevolutePoseSolution {
            endpoint_a,
            endpoint_b,
            deltas,
        })
    }

    fn sample_revolute_endpoint(
        &self,
        body: BodyHandle,
        dt: FloatNum,
        ccd_clamped_bodies: &BTreeSet<BodyHandle>,
        advance_sleeping: bool,
    ) -> Option<RevoluteEndpointSample> {
        let record = self.body_record(body).ok()?;
        let current_pose = record.pose;
        let mass = record.mass_properties;
        if !is_finite_pose(current_pose) || !mass.is_finite_non_negative() || !dt.is_finite() {
            return None;
        }

        let predicts_motion = match record.body_type {
            BodyType::Static => false,
            BodyType::Dynamic => !record.sleeping || advance_sleeping,
            BodyType::Kinematic => true,
        };
        let (sampled_translation_advance, sampled_angle_advance) = if predicts_motion {
            let translation = if record.body_type.is_dynamic() && ccd_clamped_bodies.contains(&body)
            {
                Vector::default()
            } else {
                record.linear_velocity * dt
            };
            let angle = record.angular_velocity * dt;
            if !is_finite_vector(translation) || !angle.is_finite() {
                return None;
            }
            (translation, angle)
        } else {
            (Vector::default(), 0.0)
        };
        let eval_pose = crate::pipeline::integrate::translated_pose(
            current_pose,
            sampled_translation_advance,
            sampled_angle_advance,
        );
        if !is_finite_pose(eval_pose) {
            return None;
        }
        Some(RevoluteEndpointSample {
            body_type: record.body_type,
            sleeping: record.sleeping,
            current_pose,
            eval_pose,
            mass,
            sampled_translation_advance,
            sampled_angle_advance,
        })
    }
}

fn joint_solve_batches(
    world: &World,
    islands: &[crate::pipeline::sleep::SolverIsland],
) -> Vec<JointSolveBatch> {
    let plan = crate::pipeline::island::build_island_solve_plan(
        islands,
        std::iter::empty(),
        world.joint_records().map(|(_, record)| record.desc.clone()),
    );
    plan.islands
        .into_iter()
        .filter_map(|island| {
            let body_slots = island.body_slots;
            let rows = island
                .joint_rows
                .into_iter()
                .filter_map(|row| match row {
                    crate::pipeline::island::JointSolvePlanRow::Distance {
                        desc,
                        body_a_slot,
                        body_b_slot,
                    } => Some(JointSolverRow::Distance {
                        desc,
                        body_a_slot,
                        body_b_slot,
                    }),
                    crate::pipeline::island::JointSolvePlanRow::WorldAnchor { desc, body_slot } => {
                        Some(JointSolverRow::WorldAnchor { desc, body_slot })
                    }
                    crate::pipeline::island::JointSolvePlanRow::Revolute {
                        desc,
                        body_a_slot,
                        body_b_slot,
                    } => {
                        let body_a = body_slots[body_a_slot];
                        let body_b = body_slots[body_b_slot];
                        // A singular point constraint cannot produce a logical solver row.
                        // Filter it before stats/post carriers are built so fail-closed rows
                        // neither mutate bodies nor claim successful solver work.
                        revolute_row_is_solvable(world, &desc, body_a, body_b).then_some(
                            JointSolverRow::Revolute {
                                desc,
                                body_a_slot,
                                body_b_slot,
                            },
                        )
                    }
                })
                .collect::<Vec<_>>();
            (!rows.is_empty()).then_some(JointSolveBatch { body_slots, rows })
        })
        .collect()
}

fn revolute_row_is_solvable(
    world: &World,
    desc: &RevoluteJointDesc,
    body_a: BodyHandle,
    body_b: BodyHandle,
) -> bool {
    let (Ok(endpoint_a), Ok(endpoint_b)) = (world.body_record(body_a), world.body_record(body_b))
    else {
        return false;
    };
    revolute_geometry(
        endpoint_a.pose,
        endpoint_a.mass_properties,
        desc.local_anchor_a,
        endpoint_b.pose,
        endpoint_b.mass_properties,
        desc.local_anchor_b,
    )
    .is_some()
}

fn joint_solver_stats(batches: &[JointSolveBatch]) -> SolverStepStats {
    SolverStepStats {
        body_slot_count: batches.iter().map(|batch| batch.body_slots.len()).sum(),
        joint_row_count: batches.iter().map(|batch| batch.rows.len()).sum(),
        ..SolverStepStats::default()
    }
}

#[derive(Clone, Copy)]
struct RevoluteEndpointSample {
    body_type: BodyType,
    sleeping: bool,
    current_pose: Pose,
    eval_pose: Pose,
    mass: MassProperties,
    sampled_translation_advance: Vector,
    sampled_angle_advance: FloatNum,
}

#[derive(Clone, Copy)]
struct RevolutePoseSolution {
    endpoint_a: RevoluteEndpointSample,
    endpoint_b: RevoluteEndpointSample,
    deltas: RevolutePositionDeltas,
}

#[derive(Clone, Copy)]
struct RevoluteGeometry {
    anchor_a: Point,
    anchor_b: Point,
    lever_a: Vector,
    lever_b: Vector,
    inverse_effective_mass: [[FloatNum; 2]; 2],
}

fn revolute_geometry(
    pose_a: Pose,
    mass_a: MassProperties,
    local_anchor_a: Point,
    pose_b: Pose,
    mass_b: MassProperties,
    local_anchor_b: Point,
) -> Option<RevoluteGeometry> {
    if !is_finite_pose(pose_a)
        || !is_finite_pose(pose_b)
        || !mass_a.is_finite_non_negative()
        || !mass_b.is_finite_non_negative()
        || !is_finite_point(local_anchor_a)
        || !is_finite_point(local_anchor_b)
    {
        return None;
    }
    let anchor_a = pose_a.transform_point(local_anchor_a);
    let anchor_b = pose_b.transform_point(local_anchor_b);
    let center_a = pose_a.transform_point(mass_a.local_center_of_mass);
    let center_b = pose_b.transform_point(mass_b.local_center_of_mass);
    let lever_a = anchor_a - center_a;
    let lever_b = anchor_b - center_b;
    if !is_finite_point(anchor_a)
        || !is_finite_point(anchor_b)
        || !is_finite_point(center_a)
        || !is_finite_point(center_b)
        || !is_finite_vector(lever_a)
        || !is_finite_vector(lever_b)
    {
        return None;
    }

    let k11 = mass_a.inverse_mass
        + mass_b.inverse_mass
        + mass_a.inverse_inertia * lever_a.y() * lever_a.y()
        + mass_b.inverse_inertia * lever_b.y() * lever_b.y();
    let k12 = -mass_a.inverse_inertia * lever_a.x() * lever_a.y()
        - mass_b.inverse_inertia * lever_b.x() * lever_b.y();
    let k22 = mass_a.inverse_mass
        + mass_b.inverse_mass
        + mass_a.inverse_inertia * lever_a.x() * lever_a.x()
        + mass_b.inverse_inertia * lever_b.x() * lever_b.x();
    let inverse_effective_mass = invert_revolute_effective_mass(k11, k12, k22)?;
    Some(RevoluteGeometry {
        anchor_a,
        anchor_b,
        lever_a,
        lever_b,
        inverse_effective_mass,
    })
}

fn apply_revolute_pose_solution(
    world: &mut World,
    body_a: BodyHandle,
    body_b: BodyHandle,
    solution: RevolutePoseSolution,
    force_wake_a: bool,
    force_wake_b: bool,
    wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
) -> bool {
    let Some(pose_a) = corrected_revolute_current_pose(
        solution.endpoint_a,
        solution.deltas.translation_a,
        solution.deltas.angle_a,
    ) else {
        return false;
    };
    let Some(pose_b) = corrected_revolute_current_pose(
        solution.endpoint_b,
        solution.deltas.translation_b,
        solution.deltas.angle_b,
    ) else {
        return false;
    };
    if force_wake_a || force_wake_b {
        apply_revolute_pose_pair_atomically_with_wake(
            world,
            body_a,
            pose_a,
            force_wake_a,
            body_b,
            pose_b,
            force_wake_b,
            wake_reasons,
        )
    } else {
        apply_revolute_pose_pair_atomically(world, body_a, pose_a, body_b, pose_b, wake_reasons)
    }
}

fn corrected_revolute_current_pose(
    endpoint: RevoluteEndpointSample,
    center_translation: Vector,
    angle_delta: FloatNum,
) -> Option<Pose> {
    if !endpoint.body_type.is_dynamic() {
        return Some(endpoint.current_pose);
    }
    let eval_world_com = endpoint
        .eval_pose
        .transform_point(endpoint.mass.local_center_of_mass);
    let corrected_world_com = eval_world_com + center_translation;
    let corrected_angle = endpoint.eval_pose.angle() + angle_delta;
    rebuild_revolute_current_pose_from_world_com(
        endpoint.eval_pose,
        endpoint.mass.local_center_of_mass,
        corrected_world_com,
        corrected_angle,
        endpoint.sampled_translation_advance,
        endpoint.sampled_angle_advance,
    )
}

fn inverse_multiply_negated(inverse: [[FloatNum; 2]; 2], vector: Vector) -> Option<Vector> {
    if !is_finite_vector(vector) {
        return None;
    }
    let result = Vector::new(
        -(inverse[0][0] * vector.x() + inverse[0][1] * vector.y()),
        -(inverse[1][0] * vector.x() + inverse[1][1] * vector.y()),
    );
    is_finite_vector(result).then_some(result)
}

fn point_angular_velocity(angular_velocity: FloatNum, lever: Vector) -> Vector {
    Vector::new(angular_velocity * lever.y(), -angular_velocity * lever.x())
}

fn is_finite_point(point: Point) -> bool {
    point.x().is_finite() && point.y().is_finite()
}

fn correction_is_nonzero(translation: Vector, angle: FloatNum) -> bool {
    is_finite_vector(translation)
        && angle.is_finite()
        && (translation.x() != 0.0 || translation.y() != 0.0 || angle != 0.0)
}

fn normalized_or_x_axis(vector: Vector) -> Vector {
    if vector.length() <= f32::EPSILON {
        Vector::new(1.0, 0.0)
    } else {
        vector.normalized_or_zero()
    }
}

fn invert_revolute_effective_mass(
    k11: FloatNum,
    k12: FloatNum,
    k22: FloatNum,
) -> Option<[[FloatNum; 2]; 2]> {
    if !k11.is_finite() || !k12.is_finite() || !k22.is_finite() {
        return None;
    }
    let determinant = k11 * k22 - k12 * k12;
    if !determinant.is_finite() || determinant <= FloatNum::EPSILON {
        return None;
    }
    let inverse_determinant = determinant.recip();
    let inverse = [
        [k22 * inverse_determinant, -k12 * inverse_determinant],
        [-k12 * inverse_determinant, k11 * inverse_determinant],
    ];
    inverse
        .iter()
        .flatten()
        .all(|component| component.is_finite())
        .then_some(inverse)
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct RevolutePositionDeltas {
    translation_a: Vector,
    translation_b: Vector,
    angle_a: FloatNum,
    angle_b: FloatNum,
}

fn revolute_position_deltas(
    inverse_mass_a: FloatNum,
    inverse_inertia_a: FloatNum,
    r_a: Vector,
    inverse_mass_b: FloatNum,
    inverse_inertia_b: FloatNum,
    r_b: Vector,
    lambda: Vector,
) -> Option<RevolutePositionDeltas> {
    if !inverse_mass_a.is_finite()
        || inverse_mass_a < 0.0
        || !inverse_inertia_a.is_finite()
        || inverse_inertia_a < 0.0
        || !inverse_mass_b.is_finite()
        || inverse_mass_b < 0.0
        || !inverse_inertia_b.is_finite()
        || inverse_inertia_b < 0.0
        || !is_finite_vector(r_a)
        || !is_finite_vector(r_b)
        || !is_finite_vector(lambda)
    {
        return None;
    }
    let deltas = RevolutePositionDeltas {
        translation_a: -lambda * inverse_mass_a,
        translation_b: lambda * inverse_mass_b,
        angle_a: inverse_inertia_a * r_a.cross(lambda),
        angle_b: -inverse_inertia_b * r_b.cross(lambda),
    };
    (is_finite_vector(deltas.translation_a)
        && is_finite_vector(deltas.translation_b)
        && deltas.angle_a.is_finite()
        && deltas.angle_b.is_finite())
    .then_some(deltas)
}

fn rebuild_revolute_current_pose_from_world_com(
    eval_pose: Pose,
    local_center_of_mass: Point,
    corrected_world_com: Point,
    corrected_angle: FloatNum,
    sampled_translation_advance: Vector,
    sampled_angle_advance: FloatNum,
) -> Option<Pose> {
    if !is_finite_pose(eval_pose)
        || !is_finite_point(local_center_of_mass)
        || !is_finite_point(corrected_world_com)
        || !corrected_angle.is_finite()
        || !is_finite_vector(sampled_translation_advance)
        || !sampled_angle_advance.is_finite()
    {
        return None;
    }
    let rotated_local_com =
        Pose::from_xy_angle(0.0, 0.0, corrected_angle).transform_point(local_center_of_mass);
    let corrected_origin = corrected_world_com - rotated_local_com;
    let current_translation = corrected_origin - sampled_translation_advance;
    let current_angle = corrected_angle - sampled_angle_advance;
    let current_pose = Pose::from_xy_angle(
        current_translation.x(),
        current_translation.y(),
        current_angle,
    );
    is_finite_pose(current_pose).then_some(current_pose)
}

#[cfg(test)]
mod tests;
