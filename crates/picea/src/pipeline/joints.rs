use std::collections::BTreeSet;

use crate::{
    events::NumericsWarningEvent,
    handles::BodyHandle,
    joint::JointDesc,
    math::{vector::Vector, FloatNum},
    world::World,
};

use super::integrate::is_finite_vector;

pub(crate) fn solve_joint_phase(
    world: &mut World,
    dt: FloatNum,
    awake_bodies: &mut BTreeSet<BodyHandle>,
    numeric_warnings: &mut Vec<NumericsWarningEvent>,
) {
    world.apply_joint_constraints(dt, awake_bodies, numeric_warnings);
}

impl World {
    pub(crate) fn apply_joint_constraints(
        &mut self,
        dt: FloatNum,
        awake_bodies: &mut BTreeSet<BodyHandle>,
        numeric_warnings: &mut Vec<NumericsWarningEvent>,
    ) {
        let joints = self
            .joint_records()
            .map(|(_, record)| record.desc.clone())
            .collect::<Vec<_>>();
        for desc in joints {
            match desc {
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
                        awake_bodies,
                    );
                }
                JointDesc::WorldAnchor(desc) => {
                    let pose = self
                        .body_record(desc.body)
                        .expect("joint endpoint must stay live during step")
                        .pose;
                    let anchor = pose.transform_point(desc.local_anchor);
                    let correction = (desc.world_anchor - anchor) * desc.stiffness.max(0.0) * dt;
                    if !is_finite_vector(correction) {
                        numeric_warnings.push(NumericsWarningEvent {
                            phase: "joint_solve".into(),
                            detail: "world_anchor_joint_correction".into(),
                        });
                        continue;
                    }
                    self.apply_single_body_correction(desc.body, correction, awake_bodies);
                }
            }
        }
    }
}

fn normalized_or_x_axis(vector: Vector) -> Vector {
    if vector.length() <= f32::EPSILON {
        Vector::new(1.0, 0.0)
    } else {
        vector.normalized_or_zero()
    }
}
