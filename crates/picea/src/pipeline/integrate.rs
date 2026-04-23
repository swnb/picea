use crate::{
    body::BodyType,
    events::NumericsWarningEvent,
    math::{vector::Vector, FloatNum},
    pipeline::StepConfig,
    world::World,
};

pub(crate) fn run_integration_phase(
    world: &mut World,
    config: &StepConfig,
    numeric_warnings: &mut Vec<NumericsWarningEvent>,
) {
    world.integrate_body_motion(config, numeric_warnings);
}

impl World {
    pub(crate) fn integrate_body_motion(
        &mut self,
        config: &StepConfig,
        numeric_warnings: &mut Vec<NumericsWarningEvent>,
    ) {
        let body_handles = self.bodies().collect::<Vec<_>>();
        let enable_sleep = self.desc().enable_sleep;
        let gravity = self.desc().gravity;
        for handle in body_handles {
            let record = self
                .body_record_mut(handle)
                .expect("live body handles must resolve during step");
            match record.body_type {
                BodyType::Static => {
                    record.sleeping = false;
                }
                BodyType::Dynamic => {
                    if !(config.enable_sleep && enable_sleep) {
                        record.sleeping = false;
                    }
                    if record.sleeping {
                        continue;
                    }

                    let linear_velocity = (record.linear_velocity
                        + gravity * config.dt * record.gravity_scale)
                        * (1.0 - record.linear_damping * config.dt).max(0.0);
                    let angular_velocity = record.angular_velocity
                        * (1.0 - record.angular_damping * config.dt).max(0.0);
                    let pose = translated_pose(
                        record.pose,
                        linear_velocity * config.dt,
                        angular_velocity * config.dt,
                    );

                    if !is_finite_vector(linear_velocity)
                        || !angular_velocity.is_finite()
                        || !is_finite_pose(pose)
                    {
                        numeric_warnings.push(NumericsWarningEvent {
                            phase: "integrate".into(),
                            detail: "body_state".into(),
                        });
                        record.linear_velocity = Vector::default();
                        record.angular_velocity = 0.0;
                        continue;
                    }

                    record.linear_velocity = linear_velocity;
                    record.angular_velocity = angular_velocity;
                    record.pose = pose;
                }
                BodyType::Kinematic => {
                    record.sleeping = false;
                    let pose = translated_pose(
                        record.pose,
                        record.linear_velocity * config.dt,
                        record.angular_velocity * config.dt,
                    );
                    if !is_finite_pose(pose) {
                        numeric_warnings.push(NumericsWarningEvent {
                            phase: "integrate".into(),
                            detail: "kinematic_pose".into(),
                        });
                        continue;
                    }
                    record.pose = pose;
                }
            }
        }
    }
}

pub(crate) fn translated_pose(
    pose: crate::body::Pose,
    translation: Vector,
    angle_delta: FloatNum,
) -> crate::body::Pose {
    crate::body::Pose::from_xy_angle(
        pose.translation().x() + translation.x(),
        pose.translation().y() + translation.y(),
        pose.angle() + angle_delta,
    )
}

pub(crate) fn is_finite_vector(vector: Vector) -> bool {
    vector.x().is_finite() && vector.y().is_finite()
}

pub(crate) fn is_finite_pose(pose: crate::body::Pose) -> bool {
    is_finite_vector(pose.translation()) && pose.angle().is_finite()
}
