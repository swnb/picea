pub mod force;
pub(crate) mod nail;

use std::ops::Deref;

use crate::math::{vector::Vector, FloatNum};

use self::force::{Force, ForceGroup};

pub type Mass = f32;

pub type Angle = f32;

pub type Speed = Vector;

#[derive(Clone)]
pub struct Meta {
    force_group: ForceGroup,
    pre_velocity: Speed,
    stashed_velocity: Speed,
    mass: ValueWithInv<Mass>,
    moment_of_inertia: ValueWithInv<Mass>,
    pre_angle_velocity: FloatNum,
    stashed_angle_velocity: FloatNum,
    angle: FloatNum,
    is_fixed: bool,
    // TODO 移除 collision
    is_collision: bool,
    is_transparent: bool,
    // if element is is_sleeping , skip constraint or collision
    is_sleeping: bool,
    motionless_frame_counter: u8,
}

struct ValueWithInv<T> {
    value: T,
    inv_value: T,
}

impl<T> Deref for ValueWithInv<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T: Clone> Clone for ValueWithInv<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            inv_value: self.inv_value.clone(),
        }
    }
}

macro_rules! impl_value_with_inv {
    ($($t:ty),*) => {
        $(
            impl From<$t> for ValueWithInv<$t> {
                fn from(value: $t) -> Self {
                    Self {
                        value,
                        inv_value: value.recip(),
                    }
                }
            }

            impl ValueWithInv<$t> {
                fn value(&self) -> $t {
                    self.value
                }

                fn inv(&self) -> $t {
                    self.inv_value
                }

                fn set_value(&mut self, new_value: $t) {
                    self.value = new_value;
                    self.inv_value = new_value.recip();
                }
            }
        )*
    };
}

impl_value_with_inv!(f32, f64);

impl Meta {
    pub fn velocity(&self) -> Speed {
        self.stashed_velocity
    }

    pub fn set_velocity(&mut self, mut reducer: impl FnMut(Speed) -> Speed) -> &mut Self {
        self.stashed_velocity = reducer(self.stashed_velocity);
        self
    }

    pub fn force_group(&self) -> &ForceGroup {
        &self.force_group
    }

    pub fn force_group_mut(&mut self) -> &mut ForceGroup {
        &mut self.force_group
    }

    // TODO remove this because of fixed
    pub fn mass(&self) -> Mass {
        *self.mass
    }

    pub fn inv_mass(&self) -> Mass {
        self.mass.inv()
    }

    pub fn set_mass(&mut self, mut reducer: impl FnMut(Mass) -> Mass) -> &mut Self {
        self.mass.set_value(reducer(*self.mass));
        self
    }

    pub fn angle_velocity(&self) -> f32 {
        self.stashed_angle_velocity
    }

    pub fn set_angle_velocity(&mut self, mut reducer: impl FnMut(f32) -> f32) -> &mut Self {
        self.stashed_angle_velocity = reducer(self.stashed_angle_velocity);
        self
    }

    pub fn angle(&self) -> f32 {
        self.angle
    }

    pub fn set_angle(&mut self, mut reducer: impl FnMut(f32) -> f32) -> &mut Self {
        self.angle = reducer(self.angle);
        self
    }

    // TODO remove this because of fixed
    pub fn moment_of_inertia(&self) -> Mass {
        *self.moment_of_inertia
    }

    pub fn inv_moment_of_inertia(&self) -> Mass {
        self.moment_of_inertia.inv()
    }

    pub fn set_moment_of_inertia(&mut self, mut reducer: impl FnMut(Mass) -> Mass) -> &mut Self {
        self.moment_of_inertia
            .set_value(reducer(*self.moment_of_inertia));
        self
    }

    pub fn is_fixed(&self) -> bool {
        self.is_fixed
    }

    pub fn mark_is_fixed(&mut self, is_fixed: bool) -> &mut Self {
        self.is_fixed = is_fixed;
        self
    }

    // TODO  refactor, remove
    pub fn mark_collision(&mut self, is_collision: bool) -> &mut Self {
        self.is_collision = is_collision;
        self
    }

    pub fn is_collision(&self) -> bool {
        self.is_collision
    }

    pub fn is_transparent(&self) -> bool {
        self.is_transparent
    }

    pub fn mark_is_transparent(&mut self, is_transparent: bool) -> &mut Self {
        self.is_transparent = is_transparent;
        self
    }

    pub fn mark_motionless(&mut self) {
        self.motionless_frame_counter += 1;
    }

    pub fn motionless_frame_counter(&self) -> u8 {
        self.motionless_frame_counter
    }

    pub fn reset_motionless_frame_counter(&mut self) {
        self.motionless_frame_counter = 0;
    }

    pub fn mark_is_sleeping(&mut self, is_sleeping: bool) {
        if is_sleeping {
            self.set_velocity(|_| (0., 0.).into());
            self.set_angle_velocity(|_| 0.);
        }
        self.is_sleeping = is_sleeping;
    }

    pub fn is_sleeping(&self) -> bool {
        self.is_sleeping
    }

    // r is vector from shape center_point to contact_point
    pub fn apply_impulse(&mut self, impulse: Vector, r: Vector) {
        // can't apply impulse to element when element fixed
        if self.is_fixed() {
            return;
        }

        let inv_mass = self.inv_mass();

        self.set_velocity(|pre_velocity| pre_velocity + impulse * inv_mass);

        let inv_moment_of_inertia = self.inv_moment_of_inertia();

        self.set_angle_velocity(|pre_angle_velocity| {
            // TODO (r ^ (normal * lambda))
            pre_angle_velocity + (r ^ impulse) * inv_moment_of_inertia
        });
    }

    pub fn compute_kinetic_energy(&self) -> f32 {
        let velocity = self.velocity();
        let velocity_square = velocity * velocity;

        let angle_velocity = self.angle_velocity();
        let angle_velocity_square = angle_velocity * angle_velocity;

        0.5 * (self.mass() * velocity_square + self.moment_of_inertia() * angle_velocity_square)
    }
}

#[derive(Clone)]
pub struct MetaBuilder {
    meta: Meta,
}

impl From<MetaBuilder> for Meta {
    fn from(mut builder: MetaBuilder) -> Self {
        builder.meta.pre_velocity = builder.meta.stashed_velocity;
        builder.meta.pre_angle_velocity = builder.meta.stashed_angle_velocity;
        builder.meta
    }
}

impl MetaBuilder {
    pub fn new(mass: f32) -> Self {
        if mass.is_normal() && mass.is_sign_positive() {
            // TODO
        }

        Self {
            meta: Meta {
                force_group: ForceGroup::new(),
                pre_velocity: (0., 0.).into(),
                stashed_velocity: (0., 0.).into(),
                mass: mass.into(),
                angle: 0.,
                pre_angle_velocity: 0.,
                stashed_angle_velocity: 0.,
                moment_of_inertia: (0.).into(),
                is_fixed: false,
                is_collision: false,
                is_transparent: false,
                is_sleeping: false,
                motionless_frame_counter: 0,
            },
        }
    }

    pub fn force(mut self, force_id: &str, force: impl Into<Vector>) -> Self {
        self.meta
            .force_group_mut()
            .add_force(Force::new(force_id, force.into()));
        self
    }

    pub fn velocity(mut self, velocity: impl Into<Speed>) -> Self {
        self.meta.stashed_velocity = velocity.into();
        self
    }

    pub fn mass(mut self, mass: f32) -> Self {
        self.meta.mass.set_value(mass);
        self
    }

    pub fn angle_velocity(mut self, av: f32) -> Self {
        self.meta.stashed_angle_velocity = av;
        self
    }

    pub fn angle(mut self, angle: f32) -> Self {
        self.meta.angle = angle;
        self
    }

    pub fn is_fixed(mut self, is_fixed: bool) -> Self {
        self.meta.is_fixed = is_fixed;
        self
    }

    pub fn is_transparent(mut self, is_transparent: bool) -> Self {
        self.meta.is_transparent = is_transparent;
        self
    }
}
