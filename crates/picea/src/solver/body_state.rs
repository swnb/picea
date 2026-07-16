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

    pub(crate) fn apply_single_body_point_impulse(
        &mut self,
        body: BodyHandle,
        anchor_from_center: Vector,
        impulse: Vector,
        wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
    ) {
        let Ok(record) = self.body_record(body) else {
            return;
        };
        let mass = record.mass_properties;
        // Picea's positive angular velocity is clockwise in screen space, so
        // the impulse torque uses the same negative cross-product sign as the
        // contact solver.
        let linear_delta = impulse * mass.inverse_mass;
        let angular_delta = -anchor_from_center.cross(impulse) * mass.inverse_inertia;
        self.apply_single_body_velocity_change(body, linear_delta, angular_delta, wake_reasons);
    }

    fn apply_single_body_velocity_delta(
        &mut self,
        body: BodyHandle,
        delta: Vector,
        wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
    ) {
        self.apply_single_body_velocity_change(body, delta, 0.0, wake_reasons);
    }

    fn apply_single_body_velocity_change(
        &mut self,
        body: BodyHandle,
        linear_delta: Vector,
        angular_delta: FloatNum,
        wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
    ) {
        if linear_delta.length() <= f32::EPSILON && angular_delta.abs() <= f32::EPSILON {
            return;
        }
        let record = self
            .body_record_mut(body)
            .expect("live body handles must resolve");
        if !record.body_type.is_dynamic() {
            return;
        }
        let was_sleeping = record.sleeping;
        record.linear_velocity += linear_delta;
        record.angular_velocity += angular_delta;
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

/// Commits both Revolute endpoint poses only after the complete pair has passed
/// finite/liveness checks. Keeping the preflight separate from mutation prevents
/// a bad second endpoint from leaving a half-applied constraint row behind.
pub(crate) fn apply_revolute_pose_pair_atomically(
    world: &mut World,
    body_a: BodyHandle,
    pose_a: Pose,
    body_b: BodyHandle,
    pose_b: Pose,
    wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
) -> bool {
    apply_revolute_pose_pair_atomically_with_wake(
        world,
        body_a,
        pose_a,
        false,
        body_b,
        pose_b,
        false,
        wake_reasons,
    )
}

// This boundary accepts both endpoint updates together so all liveness/finite
// checks complete before either body is mutated. Splitting or hiding the paired
// arguments would weaken that atomicity contract at call sites.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_revolute_pose_pair_atomically_with_wake(
    world: &mut World,
    body_a: BodyHandle,
    pose_a: Pose,
    force_wake_a: bool,
    body_b: BodyHandle,
    pose_b: Pose,
    force_wake_b: bool,
    wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
) -> bool {
    if body_a == body_b || !is_finite_pose(pose_a) || !is_finite_pose(pose_b) {
        return false;
    }

    let (current_a, dynamic_a, sleeping_a) = match world.body_record(body_a) {
        Ok(record) => (record.pose, record.body_type.is_dynamic(), record.sleeping),
        Err(_) => return false,
    };
    let (current_b, dynamic_b, sleeping_b) = match world.body_record(body_b) {
        Ok(record) => (record.pose, record.body_type.is_dynamic(), record.sleeping),
        Err(_) => return false,
    };
    let committed_a = if dynamic_a { pose_a } else { current_a };
    let committed_b = if dynamic_b { pose_b } else { current_b };
    if !is_finite_pose(committed_a) || !is_finite_pose(committed_b) {
        return false;
    }

    let changed_a = dynamic_a && pose_changed(current_a, committed_a);
    let changed_b = dynamic_b && pose_changed(current_b, committed_b);
    let wake_a = dynamic_a && (changed_a || force_wake_a);
    let wake_b = dynamic_b && (changed_b || force_wake_b);

    {
        let record = world
            .body_record_mut(body_a)
            .expect("preflighted Revolute endpoint A must remain live");
        record.pose = committed_a;
        if wake_a {
            record.sleeping = false;
            record.sleep_idle_time = 0.0;
        }
    }
    {
        let record = world
            .body_record_mut(body_b)
            .expect("preflighted Revolute endpoint B must remain live");
        record.pose = committed_b;
        if wake_b {
            record.sleeping = false;
            record.sleep_idle_time = 0.0;
        }
    }

    if wake_a && sleeping_a {
        crate::pipeline::sleep::record_wake_reason(
            wake_reasons,
            body_a,
            SleepTransitionReason::JointCorrection,
        );
    }
    if wake_b && sleeping_b {
        crate::pipeline::sleep::record_wake_reason(
            wake_reasons,
            body_b,
            SleepTransitionReason::JointCorrection,
        );
    }
    true
}

/// Applies a complete pair of point-constraint velocity deltas atomically.
/// Static and kinematic endpoints participate in effective mass as zero inverse
/// mass, but never receive solver write-back.
// As above, the explicit A/B deltas are part of the all-or-nothing write API.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_revolute_velocity_pair_atomically(
    world: &mut World,
    body_a: BodyHandle,
    linear_delta_a: Vector,
    angular_delta_a: FloatNum,
    body_b: BodyHandle,
    linear_delta_b: Vector,
    angular_delta_b: FloatNum,
    wake_reasons: &mut BTreeMap<BodyHandle, SleepTransitionReason>,
) -> bool {
    if body_a == body_b
        || !is_finite_vector(linear_delta_a)
        || !angular_delta_a.is_finite()
        || !is_finite_vector(linear_delta_b)
        || !angular_delta_b.is_finite()
    {
        return false;
    }

    let (linear_a, angular_a, dynamic_a, sleeping_a) = match world.body_record(body_a) {
        Ok(record) => (
            record.linear_velocity,
            record.angular_velocity,
            record.body_type.is_dynamic(),
            record.sleeping,
        ),
        Err(_) => return false,
    };
    let (linear_b, angular_b, dynamic_b, sleeping_b) = match world.body_record(body_b) {
        Ok(record) => (
            record.linear_velocity,
            record.angular_velocity,
            record.body_type.is_dynamic(),
            record.sleeping,
        ),
        Err(_) => return false,
    };
    let next_linear_a = if dynamic_a {
        linear_a + linear_delta_a
    } else {
        linear_a
    };
    let next_angular_a = if dynamic_a {
        angular_a + angular_delta_a
    } else {
        angular_a
    };
    let next_linear_b = if dynamic_b {
        linear_b + linear_delta_b
    } else {
        linear_b
    };
    let next_angular_b = if dynamic_b {
        angular_b + angular_delta_b
    } else {
        angular_b
    };
    if !is_finite_vector(next_linear_a)
        || !next_angular_a.is_finite()
        || !is_finite_vector(next_linear_b)
        || !next_angular_b.is_finite()
    {
        return false;
    }

    let changed_a =
        dynamic_a && velocity_changed(linear_a, angular_a, next_linear_a, next_angular_a);
    let changed_b =
        dynamic_b && velocity_changed(linear_b, angular_b, next_linear_b, next_angular_b);
    {
        let record = world
            .body_record_mut(body_a)
            .expect("preflighted Revolute endpoint A must remain live");
        record.linear_velocity = next_linear_a;
        record.angular_velocity = next_angular_a;
        if changed_a {
            record.sleeping = false;
            record.sleep_idle_time = 0.0;
        }
    }
    {
        let record = world
            .body_record_mut(body_b)
            .expect("preflighted Revolute endpoint B must remain live");
        record.linear_velocity = next_linear_b;
        record.angular_velocity = next_angular_b;
        if changed_b {
            record.sleeping = false;
            record.sleep_idle_time = 0.0;
        }
    }

    if changed_a && sleeping_a {
        crate::pipeline::sleep::record_wake_reason(
            wake_reasons,
            body_a,
            SleepTransitionReason::JointCorrection,
        );
    }
    if changed_b && sleeping_b {
        crate::pipeline::sleep::record_wake_reason(
            wake_reasons,
            body_b,
            SleepTransitionReason::JointCorrection,
        );
    }
    true
}

fn is_finite_pose(pose: Pose) -> bool {
    is_finite_vector(pose.translation()) && pose.angle().is_finite()
}

fn is_finite_vector(vector: Vector) -> bool {
    vector.x().is_finite() && vector.y().is_finite()
}

fn pose_changed(current: Pose, next: Pose) -> bool {
    current.translation().x() != next.translation().x()
        || current.translation().y() != next.translation().y()
        || current.angle() != next.angle()
}

fn velocity_changed(
    current_linear: Vector,
    current_angular: FloatNum,
    next_linear: Vector,
    next_angular: FloatNum,
) -> bool {
    current_linear.x() != next_linear.x()
        || current_linear.y() != next_linear.y()
        || current_angular != next_angular
}

pub(crate) fn translate_pose(pose: &mut Pose, translation: Vector, angle_delta: FloatNum) {
    *pose = crate::pipeline::integrate::translated_pose(*pose, translation, angle_delta);
}
