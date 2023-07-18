use super::{vector::Vector, FloatNum};
use std::{
    fmt::Display,
    ops::{Add, AddAssign, Sub, SubAssign},
};

#[derive(Clone, Debug, Copy, Default)]
pub struct Point<T = FloatNum>
where
    T: Clone + Copy,
{
    pub(crate) x: T,
    pub(crate) y: T,
}

impl<T> Display for Point<T>
where
    T: Clone + Copy + Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{{ x: {}, y: {} }}", self.x, self.y))
    }
}

impl PartialEq for Point {
    fn eq(&self, other: &Self) -> bool {
        ((self.x() - other.x()).abs() < f32::EPSILON)
            && ((self.y() - other.y()).abs() < f32::EPSILON)
    }
}

impl<T> Point<T>
where
    T: Clone + Copy,
{
    #[inline]
    pub fn new(x: T, y: T) -> Self {
        Self { x, y }
    }

    #[inline]
    pub fn x(&self) -> T {
        self.x
    }

    #[inline]
    pub fn y(&self) -> T {
        self.y
    }

    #[inline]
    pub fn set_x(&mut self, x_reducer: impl FnOnce(T) -> T) {
        self.x = x_reducer(self.x)
    }

    #[inline]
    pub fn set_y(&mut self, y_reducer: impl FnOnce(T) -> T) {
        self.y = y_reducer(self.y)
    }

    #[inline]
    pub fn to_vector(self) -> Vector<T> {
        Vector {
            x: self.x,
            y: self.y,
        }
    }

    #[inline]
    pub fn clone_from(&mut self, other: &Self) {
        self.x = other.x;
        self.y = other.y;
    }
}

impl<T> From<(T, T)> for Point<T>
where
    T: Clone + Copy,
{
    fn from((x, y): (T, T)) -> Self {
        Point { x, y }
    }
}

impl<T> From<[T; 2]> for Point<T>
where
    T: Clone + Copy,
{
    fn from([x, y]: [T; 2]) -> Self {
        Point { x, y }
    }
}

impl<T> From<Point<T>> for (T, T)
where
    T: Clone + Copy,
{
    fn from(point: Point<T>) -> Self {
        (point.x, point.y)
    }
}

impl<T> Add<&Vector<T>> for Point<T>
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

impl<T> Add<Vector<T>> for Point<T>
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

impl<T> AddAssign<&Vector<T>> for Point<T>
where
    T: Clone + Copy + Add<Output = T> + AddAssign<T>,
{
    fn add_assign(&mut self, rhs: &Vector<T>) {
        self.set_x(|x| x + rhs.x);
        self.set_y(|y| y + rhs.y);
    }
}

impl<T> AddAssign<Vector<T>> for Point<T>
where
    T: Clone + Copy + Add<Output = T> + AddAssign<T>,
{
    fn add_assign(&mut self, rhs: Vector<T>) {
        self.set_x(|x| x + rhs.x);
        self.set_y(|y| y + rhs.y);
    }
}

impl<T> Sub<&Vector<T>> for Point<T>
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

impl<T> Sub<Vector<T>> for Point<T>
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

impl<T> Sub<Point<T>> for Point<T>
where
    T: Clone + Copy + Sub<Output = T>,
{
    type Output = Vector<T>;
    fn sub(self, rhs: Point<T>) -> Self::Output {
        let new_x = self.x - rhs.x;
        let new_y = self.y - rhs.y;
        (new_x, new_y).into()
    }
}

impl<T> Sub<&Point<T>> for Point<T>
where
    T: Clone + Copy + Sub<Output = T>,
{
    type Output = Vector<T>;
    fn sub(self, rhs: &Point<T>) -> Self::Output {
        let new_x = self.x - rhs.x;
        let new_y = self.y - rhs.y;
        (new_x, new_y).into()
    }
}

impl<T> SubAssign<&Vector<T>> for Point<T>
where
    T: Clone + Copy + Sub<Output = T> + SubAssign<T>,
{
    fn sub_assign(&mut self, rhs: &Vector<T>) {
        self.set_x(|x| x - rhs.x);
        self.set_y(|y| y - rhs.y);
    }
}

impl<T> SubAssign<Vector<T>> for Point<T>
where
    T: Clone + Copy + Sub<Output = T> + SubAssign<T>,
{
    fn sub_assign(&mut self, rhs: Vector<T>) {
        self.set_x(|x| x - rhs.x);
        self.set_y(|y| y - rhs.y);
    }
}
