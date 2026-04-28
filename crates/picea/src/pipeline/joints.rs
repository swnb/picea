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
                        if error.abs() <= f32::EPSILON {
                            continue;
                        }
                        let correction = direction * error * desc.stiffness.max(0.0) * dt;
                        if !is_finite_vector(correction) {
                            numeric_warnings.push(NumericsWarningEvent {
                                phase: "joint_solve".into(),
                                detail: "distance_joint_correction".into(),
                            });
                            continue;
                        }
                        self.apply_body_pair_correction(body_a, body_b, correction, wake_reasons);
                    }
                    JointSolverRow::WorldAnchor { desc, body_slot } => {
                        let body = batch.body_slots[body_slot];
                        let pose = self
                            .body_record(body)
                            .expect("joint endpoint must stay live during step")
                            .pose;
                        let anchor = pose.transform_point(desc.local_anchor);
                        let correction =
                            (desc.world_anchor - anchor) * desc.stiffness.max(0.0) * dt;
                        if !is_finite_vector(correction) {
                            numeric_warnings.push(NumericsWarningEvent {
                                phase: "joint_solve".into(),
                                detail: "world_anchor_joint_correction".into(),
                            });
                            continue;
                        }
                        self.apply_single_body_correction(body, correction, wake_reasons);
                    }
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
