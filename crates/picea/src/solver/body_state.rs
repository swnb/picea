use std::collections::BTreeSet;

use crate::{
    body::Pose,
    handles::BodyHandle,
    math::{vector::Vector, FloatNum},
    world::World,
};

impl World {
    pub(crate) fn apply_body_pair_correction(
        &mut self,
        body_a: BodyHandle,
        body_b: BodyHandle,
        correction_toward_a: Vector,
        awake_bodies: &mut BTreeSet<BodyHandle>,
    ) {
        let body_a_dynamic = self
            .body_record(body_a)
            .expect("live body handles must resolve")
            .body_type
            .is_dynamic();
        let body_b_dynamic = self
            .body_record(body_b)
            .expect("live body handles must resolve")
            .body_type
            .is_dynamic();

        match (body_a_dynamic, body_b_dynamic) {
            (true, true) => {
                self.apply_single_body_correction(body_a, correction_toward_a, awake_bodies);
                self.apply_single_body_correction(body_b, -correction_toward_a, awake_bodies);
            }
            (true, false) => {
                self.apply_single_body_correction(body_a, correction_toward_a * 2.0, awake_bodies);
            }
            (false, true) => {
                self.apply_single_body_correction(body_b, -correction_toward_a * 2.0, awake_bodies);
            }
            (false, false) => {}
        }
    }

    pub(crate) fn apply_single_body_correction(
        &mut self,
        body: BodyHandle,
        translation: Vector,
        awake_bodies: &mut BTreeSet<BodyHandle>,
    ) {
        if translation.length() <= f32::EPSILON {
            return;
        }
        let record = self
            .body_record_mut(body)
            .expect("live body handles must resolve");
        if !record.body_type.is_dynamic() {
            return;
        }
        translate_pose(&mut record.pose, translation, 0.0);
        record.linear_velocity = Vector::default();
        record.angular_velocity = 0.0;
        record.sleeping = false;
        awake_bodies.insert(body);
    }
}

pub(crate) fn translate_pose(pose: &mut Pose, translation: Vector, angle_delta: FloatNum) {
    *pose = crate::pipeline::integrate::translated_pose(*pose, translation, angle_delta);
}
