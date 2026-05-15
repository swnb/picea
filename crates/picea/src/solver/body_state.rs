use std::collections::BTreeMap;

use crate::{
    body::Pose,
    events::SleepTransitionReason,
    handles::BodyHandle,
    math::{vector::Vector, FloatNum},
    world::World,
};

impl World {
    pub(crate) fn apply_body_pair_position_correction_preserve_velocity(
        &mut self,
        body_a: BodyHandle,
        body_b: BodyHandle,
        correction_toward_a: Vector,
        wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
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
                self.apply_single_body_position_correction_preserve_velocity(
                    body_a,
                    correction_toward_a,
                    wake_reasons,
                );
                self.apply_single_body_position_correction_preserve_velocity(
                    body_b,
                    -correction_toward_a,
                    wake_reasons,
                );
            }
            (true, false) => {
                self.apply_single_body_position_correction_preserve_velocity(
                    body_a,
                    correction_toward_a * 2.0,
                    wake_reasons,
                );
            }
            (false, true) => {
                self.apply_single_body_position_correction_preserve_velocity(
                    body_b,
                    -correction_toward_a * 2.0,
                    wake_reasons,
                );
            }
            (false, false) => {}
        }
    }

    pub(crate) fn apply_single_body_position_correction_preserve_velocity(
        &mut self,
        body: BodyHandle,
        translation: Vector,
        wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
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
        let was_sleeping = record.sleeping;
        translate_pose(&mut record.pose, translation, 0.0);
        record.sleeping = false;
        record.sleep_idle_time = 0.0;
        if was_sleeping {
            crate::pipeline::sleep::record_wake_reason(
                wake_reasons,
                body,
                SleepTransitionReason::JointCorrection,
            );
        }
    }

    pub(crate) fn apply_body_pair_radial_velocity_constraint(
        &mut self,
        body_a: BodyHandle,
        body_b: BodyHandle,
        direction_from_a_to_b: Vector,
        wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
    ) {
        let Ok(record_a) = self.body_record(body_a) else {
            return;
        };
        let Ok(record_b) = self.body_record(body_b) else {
            return;
        };
        let inverse_mass_a = record_a.mass_properties.inverse_mass;
        let inverse_mass_b = record_b.mass_properties.inverse_mass;
        let total_inverse_mass = inverse_mass_a + inverse_mass_b;
        if total_inverse_mass <= f32::EPSILON {
            return;
        }

        let relative_radial_speed =
            (record_b.linear_velocity - record_a.linear_velocity).dot(direction_from_a_to_b);
        if relative_radial_speed.abs() <= f32::EPSILON {
            return;
        }
        let correction_per_inverse_mass =
            direction_from_a_to_b * (relative_radial_speed / total_inverse_mass);
        if inverse_mass_a > 0.0 {
            self.apply_single_body_velocity_delta(
                body_a,
                correction_per_inverse_mass * inverse_mass_a,
                wake_reasons,
            );
        }
        if inverse_mass_b > 0.0 {
            self.apply_single_body_velocity_delta(
                body_b,
                -correction_per_inverse_mass * inverse_mass_b,
                wake_reasons,
            );
        }
    }

    fn apply_single_body_velocity_delta(
        &mut self,
        body: BodyHandle,
        delta: Vector,
        wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
    ) {
        if delta.length() <= f32::EPSILON {
            return;
        }
        let record = self
            .body_record_mut(body)
            .expect("live body handles must resolve");
        if !record.body_type.is_dynamic() {
            return;
        }
        let was_sleeping = record.sleeping;
        record.linear_velocity += delta;
        record.sleeping = false;
        record.sleep_idle_time = 0.0;
        if was_sleeping {
            crate::pipeline::sleep::record_wake_reason(
                wake_reasons,
                body,
                SleepTransitionReason::JointCorrection,
            );
        }
    }
}

pub(crate) fn translate_pose(pose: &mut Pose, translation: Vector, angle_delta: FloatNum) {
    *pose = crate::pipeline::integrate::translated_pose(*pose, translation, angle_delta);
}
