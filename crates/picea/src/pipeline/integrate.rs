use std::collections::{BTreeMap, BTreeSet};

use crate::{
    body::{BodyType, Pose},
    events::NumericsWarningEvent,
    handles::BodyHandle,
    math::{vector::Vector, FloatNum},
    pipeline::StepConfig,
    world::World,
};

pub(crate) fn run_velocity_integration_phase(
    world: &mut World,
    config: &StepConfig,
    numeric_warnings: &mut Vec<NumericsWarningEvent>,
) {
    world.integrate_body_velocities(config, numeric_warnings);
}

pub(crate) fn run_position_integration_phase(
    world: &mut World,
    config: &StepConfig,
    numeric_warnings: &mut Vec<NumericsWarningEvent>,
    skip_bodies: &BTreeSet<BodyHandle>,
) {
    world.integrate_body_positions(config, numeric_warnings, skip_bodies);
}

pub(crate) fn preintegrated_body_poses(
    world: &World,
    dt: FloatNum,
    ccd_clamped_bodies: &BTreeSet<BodyHandle>,
) -> BTreeMap<BodyHandle, Pose> {
    world
        .bodies()
        .filter_map(|handle| {
            let record = world.body_record(handle).ok()?;
            let current = record.pose;
            let should_predict = !ccd_clamped_bodies.contains(&handle)
                && match record.body_type {
                    BodyType::Static => false,
                    BodyType::Dynamic => !record.sleeping,
                    BodyType::Kinematic => true,
                };
            let predicted = should_predict.then(|| {
                translated_pose(
                    current,
                    record.linear_velocity * dt,
                    record.angular_velocity * dt,
                )
            });
            // Contact discovery is advisory: invalid motion must fall back to the
            // authoritative pose instead of injecting non-finite broadphase bounds.
            Some((
                handle,
                predicted
                    .filter(|pose| is_finite_pose(*pose))
                    .unwrap_or(current),
            ))
        })
        .collect()
}

impl World {
    // Spike variant A: gravity and damping land in velocities before the
    // contact solver runs, so resting contacts can absorb the gravity impulse
    // instead of converting it into penetration + position correction.
    pub(crate) fn integrate_body_velocities(
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
                BodyType::Static | BodyType::Kinematic => {
                    record.sleeping = false;
                    record.sleep_idle_time = 0.0;
                }
                BodyType::Dynamic => {
                    if !(config.enable_sleep && enable_sleep) {
                        record.sleeping = false;
                        record.sleep_idle_time = 0.0;
                    }
                    if record.sleeping {
                        continue;
                    }

                    let linear_velocity = (record.linear_velocity
                        + gravity * config.dt * record.gravity_scale)
                        * (1.0 - record.linear_damping * config.dt).max(0.0);
                    let angular_velocity = record.angular_velocity
                        * (1.0 - record.angular_damping * config.dt).max(0.0);

                    if !is_finite_vector(linear_velocity) || !angular_velocity.is_finite() {
                        numeric_warnings.push(NumericsWarningEvent {
                            phase: "integrate".into(),
                            detail: "body_state".into(),
                        });
                        record.linear_velocity = Vector::default();
                        record.angular_velocity = 0.0;
                        record.sleep_idle_time = 0.0;
                        continue;
                    }

                    record.linear_velocity = linear_velocity;
                    record.angular_velocity = angular_velocity;
                }
            }
        }
    }

    pub(crate) fn integrate_body_positions(
        &mut self,
        config: &StepConfig,
        numeric_warnings: &mut Vec<NumericsWarningEvent>,
        skip_bodies: &BTreeSet<BodyHandle>,
    ) {
        let body_handles = self.bodies().collect::<Vec<_>>();
        for handle in body_handles {
            // Predictive CCD already advanced these bodies to their impact plane
            // this step; integrating them again would push them past it.
            if skip_bodies.contains(&handle) {
                continue;
            }
            let record = self
                .body_record_mut(handle)
                .expect("live body handles must resolve during step");
            match record.body_type {
                BodyType::Static => {}
                BodyType::Dynamic => {
                    if record.sleeping {
                        continue;
                    }
                    let pose = translated_pose(
                        record.pose,
                        record.linear_velocity * config.dt,
                        record.angular_velocity * config.dt,
                    );
                    if !is_finite_pose(pose) {
                        numeric_warnings.push(NumericsWarningEvent {
                            phase: "integrate".into(),
                            detail: "body_state".into(),
                        });
                        record.linear_velocity = Vector::default();
                        record.angular_velocity = 0.0;
                        record.sleep_idle_time = 0.0;
                        continue;
                    }
                    record.pose = pose;
                }
                BodyType::Kinematic => {
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
