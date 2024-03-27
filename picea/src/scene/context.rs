use std::{cell::RefCell, sync::atomic::AtomicBool};

use crate::math::{vector::Vector, FloatNum};

pub(crate) struct InnerGlobalContext {
    // indicate whether to update shape immediately or not
    pub(crate) merge_shape_transform: AtomicBool,
}

pub(crate) struct GlobalContext {
    // indicate whether to update shape immediately or not
    pub(crate) merge_shape_transform: bool,
}

static mut INNER_GLOBAL_CONTEXT: RefCell<InnerGlobalContext> = RefCell::new(InnerGlobalContext {
    merge_shape_transform: AtomicBool::new(false),
});

pub fn global_context() -> GlobalContext {
    unsafe {
        GlobalContext {
            merge_shape_transform: INNER_GLOBAL_CONTEXT
                .borrow()
                .merge_shape_transform
                .load(std::sync::atomic::Ordering::Relaxed),
        }
    }
}

pub(crate) fn global_context_mut() -> &'static mut InnerGlobalContext {
    unsafe { INNER_GLOBAL_CONTEXT.get_mut() }
}

#[derive(Debug)]
pub struct ConstraintParameters {
    // 位子修正的系数
    pub factor_position_bias: FloatNum,
    // 弹性系数  0 - 1 之间
    pub factor_restitution: FloatNum,
    // FIXME remove
    pub max_allow_permeate: FloatNum,
    pub factor_default_friction: FloatNum,
    // 允许碰撞深度是负值
    pub allow_permeate_negative: bool,

    pub skip_friction_constraints: bool,
    // more detail about this variable, see contact constraint
    pub max_allow_restrict_force_for_contact_solve: FloatNum,
    pub split_position_fix: bool,
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
