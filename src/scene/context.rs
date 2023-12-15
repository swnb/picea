use crate::math::{vector::Vector, FloatNum};

#[derive(Debug)]
pub struct ConstraintParameters {
    // 位子修正的系数
    pub factor_position_bias: FloatNum,
    // 弹性系数  0 - 1 之间
    pub factor_elastic: FloatNum,
    // FIXME remove
    pub max_allow_permeate: FloatNum,
    pub factor_default_friction: FloatNum,
    // 允许碰撞深度是负值
    pub allow_permeate_negative: bool,

    pub skip_friction_constraints: bool,
}

impl Default for ConstraintParameters {
    fn default() -> Self {
        Self {
            factor_position_bias: 0.99,
            factor_elastic: 0.5,
            max_allow_permeate: 0.03,
            factor_default_friction: 0.2,
            allow_permeate_negative: true,
            skip_friction_constraints: false,
        }
    }
}

#[derive(Debug)]
/// define global config and state
pub struct Context {
    pub constraint_parameters: ConstraintParameters,
    // element will ignore sleep when when motion less than max_enter_sleep_motion for max_enter_sleep_frame times
    pub enable_sleep_mode: bool,
    pub max_enter_sleep_motion: FloatNum,
    pub max_enter_sleep_frame: u8,
    pub enable_gravity: bool,
    pub default_gravity: Vector,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            constraint_parameters: Default::default(),
            enable_sleep_mode: false,
            max_enter_sleep_frame: 40,
            max_enter_sleep_motion: 0.07,
            enable_gravity: true,
            default_gravity: (0., 9.8).into(),
        }
    }
}
