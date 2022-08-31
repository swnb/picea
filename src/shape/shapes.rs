use super::{
    circle::CircleShape, rect::RectShape, ComputeMomentOfInertia, ProjectionOnAxis, Shape,
};
use crate::{
    math::{axis::AxisDirection, point::Point, vector::Vector},
    meta::Mass,
};

#[derive(Clone, Debug)]
pub enum ShapeUnion {
    Rect(RectShape),
    Circle(CircleShape),
}

impl ShapeUnion {
    fn get_inner_shape(&self) -> &dyn Shape {
        use ShapeUnion::*;
        match self {
            Rect(s) => s,
            Circle(s) => s,
        }
    }

    fn get_inner_shape_mut(&mut self) -> &mut dyn Shape {
        use ShapeUnion::*;
        match self {
            Rect(s) => s,
            Circle(s) => s,
        }
    }
}

impl Shape for ShapeUnion {
    fn compute_center_point(&self) -> Point<f32> {
        self.get_inner_shape().compute_center_point()
    }

    fn projection_on_vector(&self, vector: Vector<f32>) -> (Point<f32>, Point<f32>) {
        self.get_inner_shape().projection_on_vector(vector)
    }

    fn translate(&mut self, vector: &Vector<f32>) {
        self.get_inner_shape_mut().translate(vector)
    }

    fn rotate(&mut self, deg: f32) {
        self.get_inner_shape_mut().rotate(deg)
    }
}

impl ProjectionOnAxis for ShapeUnion {
    fn projection_on_axis(&self, axis_direction: AxisDirection) -> (f32, f32) {
        use AxisDirection::*;
        use ShapeUnion::*;
        match self {
            Rect(shape) => match axis_direction {
                X => shape.projection_on_x_axis(),
                Y => shape.projection_on_y_axis(),
            },
            Circle(shape) => {
                let center_point = shape.get_center_point();
                let (center_x, center_y): (f32, f32) = center_point.into();
                let radius = shape.radius();
                match axis_direction {
                    X => (center_x - radius, center_x + radius),
                    Y => (center_y - radius, center_y + radius),
                }
            }
        }
    }
}

impl ComputeMomentOfInertia for ShapeUnion {
    // compute moment of inertia;
    fn compute_moment_of_inertia(&self, m: Mass) -> f32 {
        use ShapeUnion::*;
        match self {
            Rect(shape) => {
                // m(x^2+y^2)/12;
                let (width, height) = shape.compute_bounding();
                (width.powf(2.) + height.powf(2.)) * m * 12f32.recip()
            }
            Circle(shape) => m * shape.radius().powf(2.) * 0.5,
        }
    }
}
