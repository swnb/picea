use std::ops::RangeInclusive;

use super::FloatNum;

pub(crate) fn is_same_sign(v1: FloatNum, v2: FloatNum) -> bool {
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
