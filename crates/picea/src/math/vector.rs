use super::{point::Point, segment::Segment, FloatNum};
use serde::{Deserialize, Serialize};
use std::{
    fmt::Display,
    ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign},
};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Vector {
    pub(super) x: FloatNum,
    pub(super) y: FloatNum,
}

impl Display for Vector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{{ x: {}, y: {} }}", self.x, self.y))
    }
}

impl PartialEq for Vector {
    fn eq(&self, other: &Self) -> bool {
        (self.x() - other.x()).abs() < FloatNum::EPSILON
            && (self.y() - other.y()).abs() < FloatNum::EPSILON
    }
}

impl Vector {
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

    #[inline]
    pub fn length(&self) -> FloatNum {
        self.x.hypot(self.y)
    }

    #[inline]
    pub fn length_squared(&self) -> FloatNum {
        self.dot(*self)
    }

    #[inline]
    pub fn dot(&self, other: Vector) -> FloatNum {
        self.x * other.x + self.y * other.y
    }

    #[inline]
    pub fn cross(&self, other: Vector) -> FloatNum {
        self.x * other.y - self.y * other.x
    }

    #[inline]
    pub fn perp(&self) -> Self {
        Self::new(self.y, -self.x)
    }

    #[inline]
    pub fn normalized(&self) -> Self {
        self.normalized_or_zero()
    }

    pub fn normalized_or_zero(&self) -> Self {
        let length = self.length();
        // Degenerate vectors have no stable direction; keep them finite by
        // collapsing zero, near-zero, and non-finite inputs to zero.
        if !length.is_finite() || length <= FloatNum::EPSILON {
            return Self::default();
        }

        *self / length
    }

    #[inline]
    pub fn project_onto(&self, axis: Vector) -> FloatNum {
        let axis_length = axis.length();
        if !axis_length.is_finite() || axis_length <= FloatNum::EPSILON {
            0.0
        } else {
            self.dot(axis) / axis_length
        }
    }

    #[inline]
    pub fn rotated(&self, radians: FloatNum) -> Self {
        let cos = radians.cos();
        let sin = radians.sin();
        let new_x = self.y * sin + self.x * cos;
        let new_y = self.y * cos - self.x * sin;
        Self::new(new_x, new_y)
    }
}

impl From<(FloatNum, FloatNum)> for Vector {
    fn from((x, y): (FloatNum, FloatNum)) -> Self {
        Self::new(x, y)
    }
}

impl From<[FloatNum; 2]> for Vector {
    fn from([x, y]: [FloatNum; 2]) -> Self {
        Self::new(x, y)
    }
}

impl From<Point> for Vector {
    fn from(point: Point) -> Self {
        Self::new(point.x(), point.y())
    }
}

impl From<&Point> for Vector {
    fn from(point: &Point) -> Self {
        Self::new(point.x(), point.y())
    }
}

impl From<Segment> for Vector {
    fn from(segment: Segment) -> Self {
        segment.direction()
    }
}

impl From<&Segment> for Vector {
    fn from(segment: &Segment) -> Self {
        segment.direction()
    }
}

impl From<(Point, Point)> for Vector {
    fn from((p1, p2): (Point, Point)) -> Self {
        Self::new(p2.x() - p1.x(), p2.y() - p1.y())
    }
}

impl From<(&Point, &Point)> for Vector {
    fn from((p1, p2): (&Point, &Point)) -> Self {
        Self::new(p2.x() - p1.x(), p2.y() - p1.y())
    }
}

impl From<Vector> for (FloatNum, FloatNum) {
    fn from(value: Vector) -> Self {
        (value.x, value.y)
    }
}

impl Add<Vector> for Vector {
    type Output = Self;

    fn add(self, rhs: Vector) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl Add<&Vector> for Vector {
    type Output = Self;

    fn add(self, rhs: &Vector) -> Self::Output {
        self + *rhs
    }
}

impl AddAssign<Vector> for Vector {
    fn add_assign(&mut self, rhs: Vector) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl AddAssign<&Vector> for Vector {
    fn add_assign(&mut self, rhs: &Vector) {
        *self += *rhs;
    }
}

impl Sub<Vector> for Vector {
    type Output = Self;

    fn sub(self, rhs: Vector) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl Sub<&Vector> for Vector {
    type Output = Self;

    fn sub(self, rhs: &Vector) -> Self::Output {
        self - *rhs
    }
}

impl SubAssign<Vector> for Vector {
    fn sub_assign(&mut self, rhs: Vector) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl SubAssign<&Vector> for Vector {
    fn sub_assign(&mut self, rhs: &Vector) {
        *self -= *rhs;
    }
}

impl Mul<FloatNum> for Vector {
    type Output = Self;

    fn mul(self, rhs: FloatNum) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl Div<FloatNum> for Vector {
    type Output = Self;

    fn div(self, rhs: FloatNum) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

impl Neg for Vector {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y)
    }
}

#[cfg(test)]
mod tests {
    use super::Vector;

    #[test]
    fn normalized_or_zero_returns_zero_for_degenerate_vectors() {
        let vectors = [
            Vector::new(0.0, 0.0),
            Vector::new(f32::MIN_POSITIVE * f32::EPSILON, 0.0),
        ];

        for vector in vectors {
            let normalized = vector.normalized_or_zero();
            assert!(normalized.x().is_finite());
            assert!(normalized.y().is_finite());
            assert_eq!(normalized, Vector::new(0.0, 0.0));
        }
    }

    #[test]
    fn normalized_preserves_regular_vectors() {
        let normalized = Vector::new(3.0, 4.0).normalized();

        assert!((normalized.length() - 1.0).abs() <= f32::EPSILON);
        assert_eq!(normalized, Vector::new(0.6, 0.8));
    }

    #[test]
    fn projection_onto_degenerate_vector_returns_zero() {
        let vector = Vector::new(3.0, 4.0);
        let zero = Vector::new(0.0, 0.0);
        let tiny = Vector::new(f32::MIN_POSITIVE * f32::EPSILON, 0.0);

        assert_eq!(vector.project_onto(zero), 0.0);
        assert_eq!(vector.project_onto(tiny), 0.0);
    }
}
