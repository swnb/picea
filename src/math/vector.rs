use super::{point::Point, segment::Segment, CommonNum};
use std::{
    fmt::Display,
    ops::{Add, AddAssign, BitXor, Div, DivAssign, Mul, MulAssign, Neg, Not, Sub, SubAssign},
};

#[derive(Clone, Debug, Copy)]
pub struct Vector<T = CommonNum>
where
    T: Clone + Copy,
{
    pub(super) x: T,
    pub(super) y: T,
}

impl<T> Display for Vector<T>
where
    T: Display + Copy + Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{{ x: {}, y: {} }}", self.x, self.y))
    }
}

impl<T: Clone + Copy> Vector<T> {
    #[inline]
    pub const fn new(x: T, y: T) -> Self {
        Self { x, y }
    }

    #[inline]
    pub fn clone_from(&mut self, other: &Self) {
        self.x = other.x;
        self.y = other.y;
    }

    #[inline]
    pub fn x(&self) -> T {
        self.x
    }

    #[inline]
    pub fn set_x(&mut self, mut reducer: impl FnMut(T) -> T) {
        self.x = reducer(self.x);
    }

    #[inline]
    pub fn y(&self) -> T {
        self.y
    }

    #[inline]
    pub fn set_y(&mut self, mut reducer: impl FnMut(T) -> T) {
        self.y = reducer(self.y);
    }

    #[inline]
    pub fn to_point(&self) -> Point<T> {
        (self.x, self.y).into()
    }
}

macro_rules! impl_vector {
    ($($T:ident),*) => {
        $(
            impl PartialEq for Vector<$T> {
                fn eq(&self, other: &Self) -> bool {
                    (self.x() - other.x()).abs() < $T::EPSILON
                        && (self.y() - other.y()).abs() < $T::EPSILON
                }
            }

            impl Vector<$T> {
                #[inline]
                pub fn abs(&self) -> $T {
                    self.x.hypot(self.y)
                }

                pub fn normalize(&self) -> Vector<$T> {
                    let shrink = self.abs().recip();
                    (self.x() * shrink, self.y() * shrink).into()
                }

                #[inline]
                pub fn deg(&self, vector: &Vector<$T>) -> $T {
                    vector.y.atan2(vector.x) - self.y.atan2(self.x)
                }

                #[inline]
                pub fn affine_transformation_rotate(&self, deg: $T) -> Vector<$T> {
                    let c = deg.cos();
                    let s = deg.sin();
                    // clockwise
                    let new_x = self.y * s + self.x * c;
                    let new_y = self.y * c - self.x * s;
                    (new_x, new_y).into()
                }

                #[inline]
                pub fn affine_transformation_rotate_self(&mut self, deg: $T) {
                    let c = deg.cos();
                    let s = deg.sin();
                    let new_x = self.y * s + self.x * c;
                    let new_y = self.y * c - self.x * s;
                    self.x = new_x;
                    self.y = new_y;
                }

                #[inline]
                pub fn is_zero(&self) -> bool {
                    self.x == 0. && self.y == 0.
                }

                #[inline]
                pub fn set_zero(&mut self) {
                    self.x = 0.;
                    self.y = 0.;
                }
            }
        )*
    };
}

impl_vector![f32, f64];

impl<T> From<(T, T)> for Vector<T>
where
    T: Clone + Copy,
{
    fn from((x, y): (T, T)) -> Self {
        Self { x, y }
    }
}

impl<T: Clone + Copy> From<[T; 2]> for Vector<T> {
    fn from([x, y]: [T; 2]) -> Self {
        Self { x, y }
    }
}

impl<T: Clone + Copy> From<Segment<T>> for Vector<T>
where
    T: std::ops::Neg<Output = T> + std::ops::Sub<Output = T>,
{
    fn from(segment: Segment<T>) -> Self {
        (*segment.start_point(), *segment.end_point()).into()
    }
}

impl<T> From<(Point<T>, Point<T>)> for Vector<T>
where
    T: Clone + Copy + std::ops::Neg<Output = T> + std::ops::Sub<Output = T>,
{
    fn from((p1, p2): (Point<T>, Point<T>)) -> Self {
        let x = p2.x() - p1.x();
        let y = p2.y() - p1.y();
        (x, y).into()
    }
}

impl<T> From<(&Point<T>, &Point<T>)> for Vector<T>
where
    T: Clone + Copy + std::ops::Neg<Output = T> + std::ops::Sub<Output = T>,
{
    fn from((p1, p2): (&Point<T>, &Point<T>)) -> Self {
        let x = p2.x() - p1.x();
        let y = p2.y() - p1.y();
        (x, y).into()
    }
}

impl<T> Add<&Vector<T>> for Vector<T>
where
    T: Clone + Copy + Add<Output = T>,
{
    type Output = Self;
    fn add(self, rhs: &Vector<T>) -> Self::Output {
        let new_x = self.x + rhs.x;
        let new_y = self.y + rhs.y;
        (new_x, new_y).into()
    }
}

impl<T> Add<Vector<T>> for Vector<T>
where
    T: Clone + Copy + Add<Output = T>,
{
    type Output = Self;
    fn add(self, rhs: Vector<T>) -> Self::Output {
        let new_x = self.x + rhs.x;
        let new_y = self.y + rhs.y;
        (new_x, new_y).into()
    }
}

impl<T> AddAssign<&Vector<T>> for Vector<T>
where
    T: Clone + Copy + AddAssign<T>,
{
    fn add_assign(&mut self, rhs: &Vector<T>) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl<T> AddAssign<Vector<T>> for Vector<T>
where
    T: Clone + Copy + AddAssign<T>,
{
    fn add_assign(&mut self, rhs: Vector<T>) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl<T: Clone + Copy> Mul<Vector<T>> for Vector<T>
where
    T: Mul<Output = T> + Add<Output = T>,
{
    type Output = T;
    fn mul(self, rhs: Vector<T>) -> Self::Output {
        (self.x * rhs.x) + (self.y * rhs.y)
    }
}

impl<T> MulAssign<Vector<T>> for Vector<T>
where
    T: Clone + Copy + MulAssign<T>,
{
    fn mul_assign(&mut self, rhs: Vector<T>) {
        self.x *= rhs.x;
        self.y *= rhs.y;
    }
}

impl<T> Neg for Vector<T>
where
    T: Clone + Copy + Neg<Output = T>,
{
    type Output = Self;
    fn neg(self) -> Self::Output {
        (-self.x, -self.y).into()
    }
}

impl<T> Sub<&Vector<T>> for Vector<T>
where
    T: Clone + Copy + Sub<Output = T>,
{
    type Output = Self;
    fn sub(self, rhs: &Vector<T>) -> Self::Output {
        let new_x = self.x - rhs.x;
        let new_y = self.y - rhs.y;
        (new_x, new_y).into()
    }
}

impl<T> Sub<Vector<T>> for Vector<T>
where
    T: Clone + Copy + Sub<Output = T>,
{
    type Output = Self;
    fn sub(self, rhs: Vector<T>) -> Self::Output {
        let new_x = self.x - rhs.x;
        let new_y = self.y - rhs.y;
        (new_x, new_y).into()
    }
}

impl<T> SubAssign<&Vector<T>> for Vector<T>
where
    T: Clone + Copy + SubAssign<T>,
{
    fn sub_assign(&mut self, rhs: &Vector<T>) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl<T> SubAssign<Vector<T>> for Vector<T>
where
    T: Clone + Copy + SubAssign<T>,
{
    fn sub_assign(&mut self, rhs: Vector<T>) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl<T> Mul<T> for Vector<T>
where
    T: Mul<Output = T> + Clone + Copy,
{
    type Output = Vector<T>;
    fn mul(self, rhs: T) -> Self::Output {
        let new_x = self.x * rhs;
        let new_y = self.y * rhs;
        (new_x, new_y).into()
    }
}

impl<T> MulAssign<T> for Vector<T>
where
    T: Clone + Copy + MulAssign<T>,
{
    fn mul_assign(&mut self, rhs: T) {
        self.x *= rhs;
        self.y *= rhs;
    }
}

impl<T> Div<&T> for Vector<T>
where
    T: Div<Output = T> + Clone + Copy,
{
    type Output = Vector<T>;
    fn div(self, rhs: &T) -> Self::Output {
        let new_x = self.x / *rhs;
        let new_y = self.y / *rhs;
        (new_x, new_y).into()
    }
}

impl<T> Div<T> for Vector<T>
where
    T: Div<Output = T> + Clone + Copy,
{
    type Output = Vector<T>;
    fn div(self, rhs: T) -> Self::Output {
        let new_x = self.x / rhs;
        let new_y = self.y / rhs;
        (new_x, new_y).into()
    }
}

impl<T> DivAssign<&T> for Vector<T>
where
    T: Clone + Copy + DivAssign<T>,
{
    fn div_assign(&mut self, rhs: &T) {
        self.x /= *rhs;
        self.y /= *rhs;
    }
}

impl<T> DivAssign<T> for Vector<T>
where
    T: Clone + Copy + DivAssign<T>,
{
    fn div_assign(&mut self, rhs: T) {
        self.x /= rhs;
        self.y /= rhs;
    }
}

impl<T: Clone + Copy> Not for Vector<T>
where
    T: Neg<Output = T>,
{
    type Output = Self;
    fn not(self) -> Self::Output {
        Self {
            x: self.y,
            y: -self.x,
        }
    }
}

impl<T> From<Vector<T>> for (T, T)
where
    T: Clone + Copy,
{
    fn from(value: Vector<T>) -> Self {
        (value.x, value.y)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Vector3<T: Clone + Copy = CommonNum> {
    x: T,
    y: T,
    z: T,
}

impl<T: Clone + Copy> Vector3<T> {
    pub fn x(&self) -> T {
        self.x
    }

    pub fn y(&self) -> T {
        self.y
    }

    pub fn z(&self) -> T {
        self.z
    }
}

impl From<Vector> for Vector3<f32> {
    fn from(value: Vector) -> Self {
        Self {
            x: value.x,
            y: value.y,
            z: 0.,
        }
    }
}

impl From<Vector<f64>> for Vector3<f64> {
    fn from(value: Vector<f64>) -> Self {
        Self {
            x: value.x,
            y: value.y,
            z: 0.,
        }
    }
}

impl<T: Clone + Copy> From<(T, T, T)> for Vector3<T> {
    fn from((x, y, z): (T, T, T)) -> Self {
        Self { x, y, z }
    }
}

// 获取向量的叉乘大小, 只有 z 方向的
impl<T: Clone + Copy> BitXor<Self> for Vector<T>
where
    T: Mul<T, Output = T> + Sub<T, Output = T>,
    Self: Into<Vector3<T>>,
{
    type Output = T;
    fn bitxor(self, rhs: Self) -> Self::Output {
        let lhs: Vector3<T> = self.into();
        (lhs ^ rhs.into()).z()
    }
}

impl<T: Clone + Copy> BitXor<Self> for Vector3<T>
where
    T: Mul<T, Output = T> + Sub<T, Output = T>,
{
    type Output = Self;
    fn bitxor(self, rhs: Self) -> Self::Output {
        let Self {
            x: x1,
            y: y1,
            z: z1,
        } = self;

        let Self {
            x: x2,
            y: y2,
            z: z2,
        } = rhs;

        let x = y1 * z2 - z1 * y2;
        let y = z1 * x2 - x1 * z2;
        let z = x1 * y2 - y1 * x2;
        Self { x, y, z }
    }
}

impl<T: Clone + Copy> From<Vector3<T>> for Vector<T> {
    fn from(value: Vector3<T>) -> Self {
        Self {
            x: value.x,
            y: value.y,
        }
    }
}

impl<T: Clone + Copy> Mul for Vector3<T>
where
    T: Mul<T, Output = T> + Add<T, Output = T>,
{
    type Output = T;
    fn mul(self, rhs: Self) -> Self::Output {
        (self.x() * rhs.x()) + (self.y() * rhs.y()) + (self.z() * rhs.z())
    }
}

impl std::ops::Shr<Vector> for Vector {
    type Output = f32;
    fn shr(self, rhs: Vector) -> Self::Output {
        self * rhs * rhs.abs().recip()
    }
}
