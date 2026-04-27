use std::collections::{BTreeMap, BTreeSet};

use crate::{
    events::{NumericsWarningEvent, SleepTransitionReason},
    handles::BodyHandle,
    joint::JointDesc,
    math::{vector::Vector, FloatNum},
    world::World,
};

use super::integrate::is_finite_vector;

struct JointSolveBatch {
    rows: Vec<JointSolverRow>,
}

struct JointSolverRow {
    desc: JointDesc,
}

pub(crate) fn solve_joint_phase(
    world: &mut World,
    dt: FloatNum,
    wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
    numeric_warnings: &mut Vec<NumericsWarningEvent>,
) {
    let islands = crate::pipeline::sleep::build_active_solver_islands(
        world,
        std::iter::empty::<(BodyHandle, BodyHandle)>(),
        wake_reasons,
    );
    let batches = joint_solve_batches(world, &islands);
    world.apply_joint_constraints(dt, batches, wake_reasons, numeric_warnings);
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
                match row.desc {
                    JointDesc::Distance(desc) => {
                        let pose_a = self
                            .body_record(desc.body_a)
                            .expect("joint endpoints must stay live during step")
                            .pose;
                        let pose_b = self
                            .body_record(desc.body_b)
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
                        self.apply_body_pair_correction(
                            desc.body_a,
                            desc.body_b,
                            correction,
                            wake_reasons,
                        );
                    }
                    JointDesc::WorldAnchor(desc) => {
                        let pose = self
                            .body_record(desc.body)
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
                        self.apply_single_body_correction(desc.body, correction, wake_reasons);
                    }
                }
            }
        }
    }
}

fn joint_solve_batches(
    world: &World,
    islands: &[crate::pipeline::sleep::SolverIsland],
) -> Vec<JointSolveBatch> {
    let body_islands = islands
        .iter()
        .flat_map(|island| island.bodies.iter().map(|body| (*body, island.id)))
        .collect::<BTreeMap<_, _>>();
    let active_islands = islands
        .iter()
        .filter(|island| island.active)
        .map(|island| island.id)
        .collect::<BTreeSet<_>>();
    let mut batches = BTreeMap::<u32, Vec<JointSolverRow>>::new();

    for (_, record) in world.joint_records() {
        let desc = record.desc.clone();
        if let Some(island) =
            joint_island(&desc, &body_islands).filter(|island| active_islands.contains(island))
        {
            batches
                .entry(island)
                .or_default()
                .push(JointSolverRow { desc });
        }
    }

    batches
        .into_values()
        .map(|rows| JointSolveBatch { rows })
        .collect()
}

fn joint_island(desc: &JointDesc, body_islands: &BTreeMap<BodyHandle, u32>) -> Option<u32> {
    match desc {
        JointDesc::Distance(desc) => match (
            body_islands.get(&desc.body_a).copied(),
            body_islands.get(&desc.body_b).copied(),
        ) {
            (Some(a), Some(b)) if a == b => Some(a),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            _ => None,
        },
        JointDesc::WorldAnchor(desc) => body_islands.get(&desc.body).copied(),
    }
}

fn normalized_or_x_axis(vector: Vector) -> Vector {
    if vector.length() <= f32::EPSILON {
        Vector::new(1.0, 0.0)
    } else {
        vector.normalized_or_zero()
    }
}
