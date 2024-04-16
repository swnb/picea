use picea_macro_tools::Fields;

use crate::math::{vector::Vector, FloatNum};

#[derive(Debug, Fields)]
#[r(vis(pub(crate)))]
pub struct ConstraintParameters {
    // 位子修正的系数
    factor_position_bias: FloatNum,
    // 弹性系数  0 - 1 之间
    factor_restitution: FloatNum,
    // FIXME remove
    max_allow_permeate: FloatNum,
    factor_default_friction: FloatNum,
    // 允许碰撞深度是负值
    allow_permeate_negative: bool,

    skip_friction_constraints: bool,
    // more detail about this variable, see contact constraint
    max_allow_restrict_force_for_contact_solve: FloatNum,
    split_position_fix: bool,
}

impl Default for ConstraintParameters {
    fn default() -> Self {
        Self {
            factor_position_bias: 0.99,
            factor_restitution: 1.0,
            max_allow_permeate: 0.01,
            factor_default_friction: 1.0,
            allow_permeate_negative: true,
            skip_friction_constraints: false,
            // from matter.js
            max_allow_restrict_force_for_contact_solve: 2.0,
            split_position_fix: true,
        }
    }
}

#[derive(Debug, Fields)]
#[r]
/// define global config and state
pub struct Context {
    #[w]
    constraint_parameters: ConstraintParameters,
    // element will ignore sleep when when motion less than max_enter_sleep_motion for max_enter_sleep_frame times
    #[w]
    enable_sleep_mode: bool,
    max_enter_sleep_kinetic: FloatNum,
    max_enter_sleep_frame: u16,
    #[w]
    enable_gravity: bool,
    #[w]
    default_gravity: Vector,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            constraint_parameters: Default::default(),
            enable_sleep_mode: false,
            max_enter_sleep_frame: 300,
            max_enter_sleep_kinetic: 3.,
            enable_gravity: true,
            default_gravity: (0., 9.8).into(),
        }
    }
}
