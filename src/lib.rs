#![feature(generic_associated_types)]

pub mod algo;
pub mod element;
pub mod math;
pub mod meta;
pub mod renderer;
pub mod scene;
pub mod shape;
pub mod web;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
