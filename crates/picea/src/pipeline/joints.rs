use std::collections::BTreeMap;

use crate::{
    events::{NumericsWarningEvent, SleepTransitionReason},
    handles::BodyHandle,
    joint::{DistanceJointDesc, WorldAnchorJointDesc},
    math::{vector::Vector, FloatNum},
    pipeline::island::SolverStepStats,
    world::World,
};

use super::integrate::is_finite_vector;

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
}

pub(crate) fn solve_joint_phase(
    world: &mut World,
    dt: FloatNum,
    wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
    numeric_warnings: &mut Vec<NumericsWarningEvent>,
) -> SolverStepStats {
    let islands = crate::pipeline::sleep::build_active_solver_islands(
        world,
        std::iter::empty::<(BodyHandle, BodyHandle)>(),
        wake_reasons,
    );
    let (batches, stats) = joint_solve_batches(world, &islands);
    world.apply_joint_constraints(dt, batches, wake_reasons, numeric_warnings);
    stats
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
    let (batches, _) = joint_solve_batches(world, &islands);
    world.apply_joint_velocity_constraints(batches, wake_reasons);
}

impl World {
    fn apply_joint_constraints(
        &mut self,
        dt: FloatNum,
        batches: Vec<JointSolveBatch>,
        wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
        numeric_warnings: &mut Vec<NumericsWarningEvent>,
    ) {
        for batch in batches {
            for row in batch.rows {
                match row {
                    JointSolverRow::Distance {
                        desc,
                        body_a_slot,
                        body_b_slot,
                    } => {
                        let body_a = batch.body_slots[body_a_slot];
                        let body_b = batch.body_slots[body_b_slot];
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
                        let body = batch.body_slots[body_slot];
                        let pose = self
                            .body_record(body)
                            .expect("joint endpoint must stay live during step")
                            .pose;
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
                            let record = self
                                .body_record(body)
                                .expect("joint endpoint must stay live during step");
                            let damping_anchor = record.pose.transform_point(desc.local_anchor);
                            let axis = (desc.world_anchor - damping_anchor).normalized_or_zero();
                            // A coincident anchor has no physical constraint axis. Unlike the
                            // positional row's deterministic fallback, damping must skip this
                            // frame rather than introduce an arbitrary x-direction bias.
                            if axis.length() <= f32::EPSILON {
                                continue;
                            }
                            let world_center_of_mass = record
                                .pose
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
                }
            }
        }
    }

    fn apply_joint_velocity_constraints(
        &mut self,
        batches: Vec<JointSolveBatch>,
        wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
    ) {
        for batch in batches {
            for row in batch.rows {
                if let JointSolverRow::Distance {
                    desc,
                    body_a_slot,
                    body_b_slot,
                } = row
                {
                    if desc.stiffness.max(0.0) <= f32::EPSILON {
                        continue;
                    }
                    let body_a = batch.body_slots[body_a_slot];
                    let body_b = batch.body_slots[body_b_slot];
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
            }
        }
    }
}

fn joint_solve_batches(
    world: &World,
    islands: &[crate::pipeline::sleep::SolverIsland],
) -> (Vec<JointSolveBatch>, SolverStepStats) {
    let plan = crate::pipeline::island::build_island_solve_plan(
        islands,
        std::iter::empty(),
        world.joint_records().map(|(_, record)| record.desc.clone()),
    );
    let stats = SolverStepStats {
        body_slot_count: plan
            .islands
            .iter()
            .filter(|island| !island.joint_rows.is_empty())
            .map(|island| island.body_slots.len())
            .sum(),
        joint_row_count: plan
            .islands
            .iter()
            .map(|island| island.joint_rows.len())
            .sum(),
        ..SolverStepStats::default()
    };

    let batches = plan
        .islands
        .into_iter()
        .filter_map(|island| {
            let rows = island
                .joint_rows
                .into_iter()
                .map(|row| match row {
                    crate::pipeline::island::JointSolvePlanRow::Distance {
                        desc,
                        body_a_slot,
                        body_b_slot,
                    } => JointSolverRow::Distance {
                        desc,
                        body_a_slot,
                        body_b_slot,
                    },
                    crate::pipeline::island::JointSolvePlanRow::WorldAnchor { desc, body_slot } => {
                        JointSolverRow::WorldAnchor { desc, body_slot }
                    }
                })
                .collect::<Vec<_>>();
            (!rows.is_empty()).then_some(JointSolveBatch {
                body_slots: island.body_slots,
                rows,
            })
        })
        .collect();

    (batches, stats)
}

fn normalized_or_x_axis(vector: Vector) -> Vector {
    if vector.length() <= f32::EPSILON {
        Vector::new(1.0, 0.0)
    } else {
        vector.normalized_or_zero()
    }
}
