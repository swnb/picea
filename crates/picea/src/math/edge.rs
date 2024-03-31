use super::point::Point;

#[derive(Debug, Clone, Copy)]
pub enum Edge<'a> {
    Arc {
        start_point: &'a Point,
        support_point: &'a Point,
        end_point: &'a Point,
    },
    Circle {
        center_point: Point,
        radius: f32,
    },
    Line {
        start_point: &'a Point,
        end_point: &'a Point,
    },
}

impl<'a> From<(&'a Point, &'a Point, &'a Point)> for Edge<'a> {
    fn from((start_point, support_point, end_point): (&'a Point, &'a Point, &'a Point)) -> Self {
        Self::new_arc(start_point, support_point, end_point)
    }
}

impl<'a> From<(&'a Point, &'a Point)> for Edge<'a> {
    fn from((start_point, end_point): (&'a Point, &'a Point)) -> Self {
        Self::new_line(start_point, end_point)
    }
}

impl<'a> Edge<'a> {
    pub fn new_arc(start_point: &'a Point, support_point: &'a Point, end_point: &'a Point) -> Self {
        Self::Arc {
            start_point,
            support_point,
            end_point,
        }
    }

    pub fn new_line(start_point: &'a Point, end_point: &'a Point) -> Self {
        Self::Line {
            start_point,
            end_point,
        }
    }
}
