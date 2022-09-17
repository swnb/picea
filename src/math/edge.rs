use super::point::Point;

#[derive(Debug, Clone, Copy)]
pub enum Edge<'a> {
    Arc {
        start_point: &'a Point<f32>,
        support_point: &'a Point<f32>,
        end_point: &'a Point<f32>,
    },
    Circle {
        center_point: Point<f32>,
        radius: f32,
    },
    Line {
        start_point: &'a Point<f32>,
        end_point: &'a Point<f32>,
    },
}

impl<'a> From<(&'a Point<f32>, &'a Point<f32>, &'a Point<f32>)> for Edge<'a> {
    fn from(
        (start_point, support_point, end_point): (&'a Point<f32>, &'a Point<f32>, &'a Point<f32>),
    ) -> Self {
        Self::new_arc(start_point, support_point, end_point)
    }
}

impl<'a> From<(&'a Point<f32>, &'a Point<f32>)> for Edge<'a> {
    fn from((start_point, end_point): (&'a Point<f32>, &'a Point<f32>)) -> Self {
        Self::new_line(start_point, end_point)
    }
}

impl<'a> Edge<'a> {
    pub fn new_arc(
        start_point: &'a Point<f32>,
        support_point: &'a Point<f32>,
        end_point: &'a Point<f32>,
    ) -> Self {
        Self::Arc {
            start_point,
            support_point,
            end_point,
        }
    }

    pub fn new_line(start_point: &'a Point<f32>, end_point: &'a Point<f32>) -> Self {
        Self::Line {
            start_point,
            end_point,
        }
    }
}
