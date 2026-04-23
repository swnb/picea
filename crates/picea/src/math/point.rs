use super::{vector::Vector, FloatNum};
use serde::{Deserialize, Serialize};
use std::{
    fmt::Display,
    ops::{Add, AddAssign, Sub, SubAssign},
};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Point {
    pub(crate) x: FloatNum,
    pub(crate) y: FloatNum,
}

impl Display for Point {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{{ x: {}, y: {} }}", self.x, self.y))
    }
}

impl PartialEq for Point {
    fn eq(&self, other: &Self) -> bool {
        (self.x() - other.x()).abs() < FloatNum::EPSILON
            && (self.y() - other.y()).abs() < FloatNum::EPSILON
    }
}

impl Point {
    #[inline]
    pub const fn new(x: FloatNum, y: FloatNum) -> Self {
        Self { x, y }
    }

    #[inline]
    pub fn x(&self) -> FloatNum {
        self.x
    }

    #[inline]
    pub fn y(&self) -> FloatNum {
        self.y
    }
}

impl From<(FloatNum, FloatNum)> for Point {
    fn from((x, y): (FloatNum, FloatNum)) -> Self {
        Self { x, y }
    }
}

impl From<[FloatNum; 2]> for Point {
    fn from([x, y]: [FloatNum; 2]) -> Self {
        Self { x, y }
    }
}

impl From<Point> for (FloatNum, FloatNum) {
    fn from(point: Point) -> Self {
        (point.x, point.y)
    }
}

impl From<Vector> for Point {
    fn from(vector: Vector) -> Self {
        Self::new(vector.x(), vector.y())
    }
}

impl From<&Vector> for Point {
    fn from(vector: &Vector) -> Self {
        Self::new(vector.x(), vector.y())
    }
}

impl Add<Vector> for Point {
    type Output = Self;

    fn add(self, rhs: Vector) -> Self::Output {
        Self::new(self.x + rhs.x(), self.y + rhs.y())
    }
}

impl Add<&Vector> for Point {
    type Output = Self;

    fn add(self, rhs: &Vector) -> Self::Output {
        self + *rhs
    }
}

impl Add<&Vector> for &Point {
    type Output = Point;

    fn add(self, rhs: &Vector) -> Self::Output {
        *self + *rhs
    }
}

impl AddAssign<Vector> for Point {
    fn add_assign(&mut self, rhs: Vector) {
        self.x += rhs.x();
        self.y += rhs.y();
    }
}

impl AddAssign<&Vector> for Point {
    fn add_assign(&mut self, rhs: &Vector) {
        *self += *rhs;
    }
}

impl Sub<Vector> for Point {
    type Output = Self;

    fn sub(self, rhs: Vector) -> Self::Output {
        Self::new(self.x - rhs.x(), self.y - rhs.y())
    }
}

impl Sub<&Vector> for Point {
    type Output = Self;

    fn sub(self, rhs: &Vector) -> Self::Output {
        self - *rhs
    }
}

impl SubAssign<Vector> for Point {
    fn sub_assign(&mut self, rhs: Vector) {
        self.x -= rhs.x();
        self.y -= rhs.y();
    }
}

impl SubAssign<&Vector> for Point {
    fn sub_assign(&mut self, rhs: &Vector) {
        *self -= *rhs;
    }
}

impl Sub<Point> for Point {
    type Output = Vector;

    fn sub(self, rhs: Point) -> Self::Output {
        Vector::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl Sub<&Point> for Point {
    type Output = Vector;

    fn sub(self, rhs: &Point) -> Self::Output {
        self - *rhs
    }
}
