pub mod force;

use macro_tools::{Builder, Deref, Fields};

use crate::math::{point::Point, vector::Vector, FloatNum};

pub type Mass = f32;

pub type Angle = f32;

pub type Speed = Vector;

#[derive(Clone, Fields, Builder)]
#[field(r)]
pub struct Meta {
    #[field(r, w, reducer)]
    velocity: Speed,
    #[shared(skip)]
    mass: ValueWithInv,
    #[shared(skip)]
    moment_of_inertia: ValueWithInv,
    #[field(r, w, reducer)]
    angle_velocity: FloatNum,
    #[shared(skip)]
    pre_angle: FloatNum,
    #[field(r, w, reducer)]
    angle: FloatNum,
    #[builder(skip)]
    pre_position: Point,
    #[builder(skip)]
    position: Point,
    #[shared(skip)]
    position_translate: Vector,

    #[field(r, w)]
    factor_friction: FloatNum,
    #[field(r, w)]
    factor_restitution: FloatNum,

    #[field(r, w)]
    is_fixed: bool,
    #[field(r, w)]
    is_transparent: bool,
    #[field(r, w)]
    is_ignore_gravity: bool,
    // if element is is_sleeping , skip constraint or collision
    #[builder(skip)]
    is_sleeping: bool,
    #[builder(skip)]
    #[field(r, w)]
    contact_count: u16,
    #[shared(skip)]
    motionless_frame_counter: u8,
}

#[derive(Deref, Clone, Fields, Builder)]
#[field(r)]
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

    pub fn set_mass(&mut self, mut reducer: impl FnMut(Mass) -> Mass) -> &mut Self {
        self.mass.set_value(reducer(*self.mass));
        self
    }

    pub fn init_position(&mut self, position: Point) {
        self.position = position;
    }

    pub fn translate_position(&mut self, translate: &Vector) {
        self.position += translate;
        self.position_translate += translate;
    }

    pub fn position_translate(&self) -> &Vector {
        &self.position_translate
    }

    pub fn sync_position(&mut self) -> &mut Self {
        self.pre_position = self.position;
        self
    }

    pub fn sync_angle(&mut self) -> &mut Self {
        self.pre_angle = self.angle;
        self
    }

    pub fn get_delta_position(&self) -> Vector {
        self.position - self.pre_position
    }

    pub fn get_delta_angle(&self) -> FloatNum {
        self.angle - self.pre_angle
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

impl MetaBuilder {
    pub fn mass(mut self, mass: FloatNum) -> Self {
        let mut v: ValueWithInv = ValueWithInvBuilder::default().into();
        v.set_value(mass);
        self.mass = v;
        self
    }
}
