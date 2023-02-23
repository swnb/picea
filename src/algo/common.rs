use crate::math::{edge::Edge, point::Point, vector::Vector};

// use radial method to create vector from point to (infinite,point.y)
// if the size of edges which cross vector is odd, the point is inside shape
pub fn is_point_inside_shape(
    point: Point<f32>,
    edge_iter: &'_ mut dyn Iterator<Item = Edge<'_>>,
) -> bool {
    let mut cross_count: usize = 0;
    let offset_vector: Vector<f32> = (point, (0., 0.).into()).into();

    let is_point_cross_segment = |p1: Point<f32>, p2: Point<f32>| {
        if (p1.y() * p2.y()).is_sign_positive() {
            return false;
        }

        let p1y_sub_p2y = p1.y() - p2.y();
        if p1y_sub_p2y.abs() <= f32::EPSILON {
            // parallel
            false
        } else {
            let cross_point_x = p1.x() + (p1.y() * (p2.x() - p1.x()) * p1y_sub_p2y.recip());
            cross_point_x.is_sign_positive()
        }
    };

    for edge in edge_iter {
        let is_cross = match edge {
            Edge::Arc {
                start_point,
                end_point,
                ..
            } => unimplemented!(),
            Edge::Circle {
                center_point,
                radius,
            } => (center_point.to_vector() + offset_vector).abs() <= radius,
            Edge::Line {
                start_point,
                end_point,
            } => {
                let result = is_point_cross_segment(
                    *start_point + offset_vector,
                    *end_point + offset_vector,
                );

                result
            }
        };
        if is_cross {
            cross_count += 1;
        }
    }

    cross_count % 2 != 0
}

pub fn is_point_inside_shape_debug<'a>(
    point: Point<f32>,
    edge_iter: &'a mut dyn Iterator<Item = Edge<'_>>,
) -> Vec<Edge<'a>> {
    vec![]
}

mod test {
    use crate::math::{point::Point, vector::Vector};

    use super::is_point_inside_shape;

    #[test]
    fn test_is_point_inside_shape() {
        let is_point_cross_segment = |p1: Point<f32>, p2: Point<f32>| {
            if (p1.y() * p2.y()).is_sign_positive() {
                return false;
            }
            let p1y_sub_p2y = p1.y() - p2.y();
            if p1y_sub_p2y.abs() <= f32::EPSILON {
                // parallel
                false
            } else {
                let cross_point_x = p1.x() + (p1.y() * (p2.x() - p1.x()) * p1y_sub_p2y.recip());
                dbg!("{}", cross_point_x);
                cross_point_x.is_sign_positive()
            }
        };
        let p1: Point<f32> = (-31.55, 142.13).into();
        let p2: Point<f32> = (-46.091, 227.683).into();
        let offset_vector: Vector<f32> = (82.6, -175.48).into();
        assert!(is_point_cross_segment(
            p1 + offset_vector,
            p2 + offset_vector
        ));
    }
}
