pub mod force;

use picea_macro_tools::{Builder, Deref, Fields};

use crate::math::{vector::Vector, FloatNum};

pub type Mass = f32;

pub type Angle = f32;

pub type Speed = Vector;

#[derive(Default, Clone, Fields, Debug)]
#[r]
#[w]
pub struct Transform {
    translation: Vector,
    rotation: FloatNum,
}

impl std::ops::AddAssign<&Transform> for Transform {
    fn add_assign(&mut self, rhs: &Self) {
        self.translation += rhs.translation;
        self.rotation += rhs.rotation;
    }
}

impl From<(Vector, FloatNum)> for Transform {
    fn from((translation, rotation): (Vector, FloatNum)) -> Self {
        Self {
            translation,
            rotation,
        }
    }
}

impl Transform {
    pub fn split(&self) -> (Vector, FloatNum) {
        (self.translation, self.rotation)
    }

    pub fn reset(&mut self) {
        self.translation = Default::default();
        self.rotation = 0.;
    }
}

#[derive(Clone, Fields, Builder)]
#[r]
pub struct Meta {
    #[w]
    velocity: Speed,
    #[shared(skip)]
    mass: ValueWithInv,
    #[shared(skip)]
    moment_of_inertia: ValueWithInv,
    #[w]
    angle_velocity: FloatNum,

    #[w(vis(pub(crate)))]
    #[r(vis(pub(crate)))]
    delta_transform: Transform,
    #[w(vis(pub(crate)))]
    total_transform: Transform,

    #[w]
    #[default = 0.2]
    factor_friction: FloatNum,
    #[w]
    #[default = 1.0]
    factor_restitution: FloatNum,

    #[w]
    is_fixed: bool,
    #[w]
    is_transparent: bool,
    #[w]
    is_ignore_gravity: bool,
    // if element is is_sleeping , skip constraint or collision
    #[builder(skip)]
    #[w]
    is_sleeping: bool,
    #[w]
    #[builder(skip)]
    contact_count: u16,
    #[builder(skip)]
    #[r(vis(pub(crate)))]
    #[w(vis(pub(crate)))]
    inactive_frame_count: u16,
}

#[derive(Deref, Clone, Fields, Builder)]
#[r]
struct ValueWithInv {
    #[deref]
    #[default = 1.0]
    value: FloatNum,
    #[default = 1.0]
    inv: FloatNum,
}

impl ValueWithInv {
    fn set_value(&mut self, new_value: FloatNum) {
        self.value = new_value;
        self.inv = new_value.recip();
    }
}

impl Meta {
    // TODO remove this because of fixed
    pub fn mass(&self) -> Mass {
        *self.mass
    }

    pub fn inv_mass(&self) -> Mass {
        self.mass.inv()
    }

    pub fn set_mass(&mut self, mass: FloatNum) -> &mut Self {
        self.mass.set_value(mass);
        self
    }

    pub(crate) fn sync_transform(&mut self) {
        self.total_transform += &self.delta_transform;
    }

    pub fn motion(&self) -> Vector {
        *self.velocity() * self.mass()
    }

    // TODO remove this because of fixed
    pub fn moment_of_inertia(&self) -> Mass {
        *self.moment_of_inertia
    }

    pub fn inv_moment_of_inertia(&self) -> Mass {
        self.moment_of_inertia.inv()
    }

    pub(crate) fn set_moment_of_inertia(
        &mut self,
        mut reducer: impl FnMut(Mass) -> Mass,
    ) -> &mut Self {
        self.moment_of_inertia
            .set_value(reducer(*self.moment_of_inertia));
        self
    }

    // r is vector from shape center_point to contact_point
    pub fn apply_impulse(&mut self, impulse: Vector, r: Vector) {
        // can't apply impulse to element when element fixed
        if self.is_fixed() {
            return;
        }

        let inv_mass = self.inv_mass();

        *self.velocity_mut() += impulse * inv_mass;

        let inv_moment_of_inertia = self.inv_moment_of_inertia();

        *self.angle_velocity_mut() += (r ^ impulse) * inv_moment_of_inertia
    }

    pub fn compute_kinetic_energy(&self) -> f32 {
        let velocity = self.velocity();
        let velocity_square = velocity * velocity;

        let angle_velocity = self.angle_velocity();
        let angle_velocity_square = angle_velocity * angle_velocity;

        0.5 * (self.mass() * velocity_square + self.moment_of_inertia() * angle_velocity_square)
    }

    pub fn compute_rough_energy(&self) -> [f32; 4] {
        let velocity = self.velocity();
        let velocity_square = velocity * velocity;

        let angle_velocity = self.angle_velocity();
        let angle_velocity_square = angle_velocity * angle_velocity;
        [
            velocity_square,
            angle_velocity_square,
            self.delta_transform().translation() * self.delta_transform().translation(),
            self.delta_transform().rotation().abs(),
        ]
    }

    pub(crate) fn silent(&mut self) {
        *self.angle_velocity_mut() = Default::default();
        *self.velocity_mut() = Default::default();
    }
}

impl MetaBuilder {
    pub fn mass(mut self, mass: FloatNum) -> Self {
        let mut v: ValueWithInv = ValueWithInvBuilder::default().into();
        v.set_value(mass);
        self.mass = v;
        self
    }
}
