pub mod collision;
pub mod force;

use std::rc::Rc;

use crate::math::vector::{Vector, Vector3};

use self::{
    collision::CollisionInfo,
    force::{Force, ForceGroup},
};

pub type Mass = f32;

pub type Angular = f32;

pub type Speed = Vector<f32>;

#[derive(Clone)]
pub struct Meta {
    force_group: ForceGroup,
    velocity: Speed,
    mass: Mass,
    inv_mass: Mass,
    moment_of_inertia: Mass,
    inv_moment_of_inertia: Mass,
    angular_velocity: f32,
    angular: f32,
    is_fixed: bool,
    collision_infos: Vec<Rc<CollisionInfo>>,
    // TODO 移除 collision
    is_collision: bool,
}

impl Meta {
    pub fn velocity(&self) -> Speed {
        self.velocity
    }

    pub fn set_velocity(&mut self, mut reducer: impl FnMut(Speed) -> Speed) -> &mut Self {
        self.velocity = reducer(self.velocity);
        self
    }

    pub fn force(&self) -> &ForceGroup {
        &self.force_group
    }

    pub fn force_group_mut(&mut self) -> &mut ForceGroup {
        &mut self.force_group
    }

    pub fn mass(&self) -> Mass {
        self.mass
    }

    pub fn inv_mass(&self) -> Mass {
        self.inv_mass
    }

    pub fn set_mass(&mut self, mut reducer: impl FnMut(Mass) -> Mass) -> &mut Self {
        self.mass = reducer(self.mass);
        self.inv_mass = self.mass.recip();
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

    pub fn collision_infos(&self) -> impl Iterator<Item = &CollisionInfo> {
        self.collision_infos.iter().map(|info| &**info)
    }

    pub fn set_collision_infos(&mut self, info: Rc<CollisionInfo>) -> &mut Self {
        // TODO refactor
        self.collision_infos.clear();
        self.collision_infos.push(info);
        self
    }

    pub fn moment_of_inertia(&self) -> Mass {
        self.moment_of_inertia
    }

    pub fn inv_moment_of_inertia(&self) -> Mass {
        self.inv_moment_of_inertia
    }

    pub fn set_moment_of_inertia(&mut self, mut reducer: impl FnMut(Mass) -> Mass) -> &mut Self {
        self.moment_of_inertia = reducer(self.moment_of_inertia);
        self.inv_moment_of_inertia = self.moment_of_inertia.recip();
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

    pub fn compute_mass_eff(&self, normal: Vector<f32>, r: Vector<f32>) -> f32 {
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

        let inv_mass = mass.recip();

        Self {
            meta: Meta {
                force_group: ForceGroup::new(),
                velocity: (0., 0.).into(),
                mass,
                inv_mass,
                angular: 0.,
                angular_velocity: 0.,
                moment_of_inertia: 0.,
                inv_moment_of_inertia: 0.,
                is_fixed: false,
                collision_infos: vec![],
                is_collision: false,
            },
        }
    }

    pub fn force(mut self, force_id: &str, force: impl Into<Vector<f32>>) -> Self {
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
        self.meta.mass = mass;
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
}
