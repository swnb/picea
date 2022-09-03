use std::ops::Not;

use super::vector::Vector;

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

impl From<AxisDirection> for Vector<f32> {
    fn from(axis: AxisDirection) -> Self {
        use AxisDirection::*;
        match axis {
            X => (1., 0.).into(),
            Y => (0., 1.).into(),
        }
    }
}

impl From<AxisDirection> for Vector {
    fn from(axis: AxisDirection) -> Self {
        use AxisDirection::*;
        match axis {
            X => (1., 0.).into(),
            Y => (0., 1.).into(),
        }
    }
}
