pub fn is_same_sign_f32(v1: f32, v2: f32) -> bool {
    (v1.is_sign_positive() && v2.is_sign_positive())
        || (v1.is_sign_negative() && v2.is_sign_negative())
}

pub fn is_same_sign_f64(v1: f64, v2: f64) -> bool {
    (v1.is_sign_positive() && v2.is_sign_positive())
        || (v1.is_sign_negative() && v2.is_sign_negative())
}
