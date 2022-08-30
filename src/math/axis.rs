use std::ops::Not;

#[derive(Clone, Copy)]
pub enum AxisDirection {
    X,
    Y,
}

impl Not for AxisDirection {
    type Output = Self;
    fn not(self) -> Self::Output {
        use AxisDirection::*;
        match self {
            X => Y,
            Y => X,
        }
    }
}
