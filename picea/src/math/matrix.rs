use std::ops::{Add, AddAssign};

pub struct Matrix<T: Clone + Copy, const R: usize, const C: usize> {
    data: [[T; C]; R],
}

impl<T: Clone + Copy, const R: usize, const C: usize> Matrix<T, R, C> {
    pub fn new(constructor: impl Fn() -> T) -> Self {
        let value = constructor();
        let data = [[value; C]; R];
        Self { data }
    }
}

impl<T: Clone + Copy, const R: usize, const C: usize> Add for Matrix<T, R, C>
where
    T: AddAssign,
{
    type Output = Self;
    fn add(mut self, rhs: Self) -> Self::Output {
        for i in 0..self.data.len() {
            for j in 0..self.data[i].len() {
                self.data[i][j] += rhs.data[i][j]
            }
        }
        self
    }
}
