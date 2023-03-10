pub mod collision;
pub mod force;

use std::{ops::Deref, rc::Rc};

use crate::{
    algo::collision::ContactInfo,
    math::vector::{Vector, Vector3},
};

use self::{
    collision::CollisionInfo,
    force::{Force, ForceGroup},
};

pub type Mass = f32;

pub type Angular = f32;

pub type Speed = Vector;

#[derive(Clone)]
pub struct Meta {
    force_group: ForceGroup,
    velocity: Speed,
    mass: ValueWithInv<Mass>,
    // mass: Mass,
    // inv_mass: Mass,
    moment_of_inertia: ValueWithInv<Mass>,
    // inv_moment_of_inertia: Mass,
    angular_velocity: f32,
    angular: f32,
    is_fixed: bool,
    collision_infos: Vec<ContactInfo>,
    // TODO 移除 collision
    is_collision: bool,
    is_transparent: bool,
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
        self.velocity
    }

    pub fn set_velocity(&mut self, mut reducer: impl FnMut(Speed) -> Speed) -> &mut Self {
        self.velocity = reducer(self.velocity);
        self
    }

    pub fn force_group(&self) -> &ForceGroup {
        &self.force_group
    }

    pub fn force_group_mut(&mut self) -> &mut ForceGroup {
        &mut self.force_group
    }

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

    pub fn angular_velocity(&self) -> f32 {
        self.angular_velocity
    }

    pub fn set_angular_velocity(&mut self, mut reducer: impl FnMut(f32) -> f32) -> &mut Self {
        self.angular_velocity = reducer(self.angular_velocity);
        self
    }

    pub fn angular(&self) -> f32 {
        self.angular
    }

    pub fn set_angular(&mut self, mut reducer: impl FnMut(f32) -> f32) -> &mut Self {
        self.angular = reducer(self.angular);
        self
    }

    pub fn collision_infos(&self) -> impl Iterator<Item = &ContactInfo> {
        self.collision_infos.iter()
    }

    pub fn set_collision_infos(
        &mut self,
        contact_infos: impl IntoIterator<Item = ContactInfo>,
    ) -> &mut Self {
        // TODO refactor
        self.collision_infos.clear();
        for contact_info in contact_infos {
            self.collision_infos.push(contact_info);
        }
        self
    }

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

    pub fn set_is_fixed(&mut self, is_fixed: bool) -> &mut Self {
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

    pub fn mark_transparent(&mut self, is_transparent: bool) -> &mut Self {
        self.is_transparent = is_transparent;
        self
    }

    pub fn compute_mass_eff(&self, normal: Vector, r: Vector) -> f32 {
        const C: f32 = 0.9;

        let r: Vector3<f32> = r.into();

        C * (self.inv_mass() + (r ^ normal.into()).z().powf(2.) * self.inv_moment_of_inertia())
            .recip()
    }

    pub fn compute_kinetic_energy(&self) -> f32 {
        let velocity = self.velocity();
        let velocity_square = velocity * velocity;

        let angular_velocity = self.angular_velocity();
        let angular_velocity_square = angular_velocity * angular_velocity;

        0.5 * (self.mass() * velocity_square + self.moment_of_inertia() * angular_velocity_square)
    }
}

#[derive(Clone)]
pub struct MetaBuilder {
    meta: Meta,
}

impl From<MetaBuilder> for Meta {
    fn from(builder: MetaBuilder) -> Self {
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
                velocity: (0., 0.).into(),
                mass: mass.into(),
                angular: 0.,
                angular_velocity: 0.,
                moment_of_inertia: (0.).into(),
                is_fixed: false,
                collision_infos: vec![],
                is_collision: false,
                is_transparent: false,
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
        self.meta.velocity = velocity.into();
        self
    }

    pub fn mass(mut self, mass: f32) -> Self {
        self.meta.mass.set_value(mass);
        self
    }

    pub fn angular_velocity(mut self, av: f32) -> Self {
        self.meta.angular_velocity = av;
        self
    }

    pub fn angular(mut self, angular: f32) -> Self {
        self.meta.angular = angular;
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
