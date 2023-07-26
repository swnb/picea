use crate::math::FloatNum;

#[derive(Debug)]
pub(crate) struct ConstraintParameters {
    // 位子修正的系数
    pub(crate) factor_position_bias: FloatNum,
    // 弹性系数
    pub(crate) factor_elastic: FloatNum,
    // FIXME remove
    pub(crate) max_allow_permeate: FloatNum,
    pub(crate) factor_default_friction: FloatNum,
    // 允许碰撞深度是负值
    pub(crate) allow_permeate_negative: bool,

    pub(crate) skip_friction_constraints: bool,
}

impl Default for ConstraintParameters {
    fn default() -> Self {
        Self {
            factor_position_bias: 0.99,
            factor_elastic: 0.01,
            max_allow_permeate: 0.03,
            factor_default_friction: 0.2,
            allow_permeate_negative: true,
            skip_friction_constraints: false,
        }
    }
}

#[derive(Debug)]
/// define global config and state
pub(crate) struct Context {
    pub(crate) constraint_parameters: ConstraintParameters,
    // element will ignore sleep when when motion less than max_enter_sleep_motion for max_enter_sleep_frame times
    pub(crate) enable_sleep_mode: bool,
    pub(crate) max_enter_sleep_motion: FloatNum,
    pub(crate) max_enter_sleep_frame: u8,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            constraint_parameters: Default::default(),
            enable_sleep_mode: false,
            max_enter_sleep_frame: 40,
            max_enter_sleep_motion: 0.07,
        }
    }
}
