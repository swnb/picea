use super::point::Point;

pub type Axis<T> = Segment<T>;

#[derive(Clone)]
pub struct Segment<T: Clone + Copy> {
    start_point: Point<T>,
    end_point: Point<T>,
}

impl<T: Clone + Copy> Segment<T> {
    pub fn new(start_point: Point<T>, end_point: Point<T>) -> Self {
        Self {
            start_point,
            end_point,
        }
    }

    pub fn get_start_point(&self) -> &Point<T> {
        &self.start_point
    }

    pub fn get_end_point(&self) -> &Point<T> {
        &self.end_point
    }
}

impl<T: Clone + Copy> From<(Point<T>, Point<T>)> for Segment<T> {
    fn from((start_point, end_point): (Point<T>, Point<T>)) -> Self {
        Segment {
            start_point,
            end_point,
        }
    }
}
