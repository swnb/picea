use crate::math::{edge::Edge, point::Point, vector::Vector};

// use radial method to create vector from point to (infinite,point.y)
// if the size of edges which cross vector is odd, the point is inside shape
pub fn is_point_inside_shape(
    point: Point<f32>,
    edge_iter: &'_ mut dyn Iterator<Item = Edge<'_>>,
) -> bool {
    let mut cross_count: usize = 0;
    let offset_vector: Vector<f32> = ((0., 0.).into(), point).into();

    let is_point_cross_segment = |p1: Point<f32>, p2: Point<f32>| {
        let p1y_sub_p2y = p1.y() - p2.y();
        if p1y_sub_p2y <= f32::EPSILON {
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
            } => is_point_cross_segment(*start_point + offset_vector, *end_point + offset_vector),
            Edge::Circle {
                center_point,
                radius,
            } => center_point.to_vector().abs() <= radius,
            Edge::Line {
                start_point,
                end_point,
            } => is_point_cross_segment(*start_point + offset_vector, *end_point + offset_vector),
        };
        if is_cross {
            cross_count += 1;
        }
    }

    cross_count % 2 != 0
}
