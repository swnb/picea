use std::{
    cmp::Ordering,
    ops::{Range, RangeInclusive},
};

use super::FloatNum;

pub(crate) fn is_same_sign_f32(v1: f32, v2: f32) -> bool {
    (v1.is_sign_positive() && v2.is_sign_positive())
        || (v1.is_sign_negative() && v2.is_sign_negative())
}

pub(crate) fn is_same_sign_f64(v1: f64, v2: f64) -> bool {
    (v1.is_sign_positive() && v2.is_sign_positive())
        || (v1.is_sign_negative() && v2.is_sign_negative())
}

pub(crate) fn limit_at_range(value: FloatNum, range: RangeInclusive<FloatNum>) -> FloatNum {
    if &value < range.start() {
        *range.start()
    } else if &value > range.end() {
        *range.end()
    } else {
        value
    }
}
