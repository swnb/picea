pub mod axis;
pub mod edge;
pub mod matrix;
pub mod point;
pub mod segment;
pub mod vector;

pub(crate) mod num;
// TODO rename
pub type FloatNum = f32;

pub const fn pi() -> FloatNum {
    std::f32::consts::PI
}

pub const fn tau() -> FloatNum {
    std::f32::consts::TAU
}
