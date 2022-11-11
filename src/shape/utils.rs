use crate::math::{point::Point, vector::Vector};

/**
 * useful tool for polygon to transform
 */

pub fn compute_polygon_center_point<'a>(
    point_iter: impl Iterator<Item = &'a Point<f32>>,
    edge_count: f32,
) -> Point<f32> {
    let mut point_iter = point_iter.map(|p| p.to_vector());
    let first_point = point_iter.next().unwrap();
    let sum = point_iter.fold(first_point, |acc, p| acc + p);
    (sum * edge_count.recip()).to_point()
}

pub fn projection_polygon_on_vector<'a>(
    point_iter: impl Iterator<Item = &'a Point<f32>>,
    vector: Vector<f32>,
) -> (Point<f32>, Point<f32>) {
    let mut min = f32::MAX;
    let mut min_point = (0., 0.).into();
    let mut max = f32::MIN;
    let mut max_point = (0., 0.).into();
    point_iter.for_each(|&cur| {
        let size = cur >> vector;
        if size < min {
            min = size;
            min_point = cur;
        }
        if size > max {
            max = size;
            max_point = cur;
        }
    });
    (min_point, max_point)
}

pub fn translate_polygon<'a>(
    point_iter_mut: impl Iterator<Item = &'a mut Point<f32>>,
    vector: &Vector<f32>,
) {
    point_iter_mut.for_each(|p| *p += vector);
}

pub fn rotate_polygon<'a>(
    center_point: Point<f32>,
    point_iter_mut: impl Iterator<Item = &'a mut Point<f32>>,
    deg: f32,
) {
    point_iter_mut.for_each(|corner| {
        let mut corner_vector: Vector<f32> = (center_point, *corner).into();
        corner_vector.affine_transformation_rotate_self(deg);
        *corner = center_point + corner_vector;
    })
}

pub fn resize_by_vector<'a>(
    point_iter_mut: impl Iterator<Item = &'a mut Point<f32>>,
    vector: impl Into<Vector<f32>>,
    is_increase: bool,
) {
    let vector: Vector<f32> = vector.into();
    let (x, y) = vector.into();

    let mut half_x = (x * 0.5).abs();
    let mut half_y = (y * 0.5).abs();

    if !is_increase {
        half_x = -half_x;
        half_y = -half_y;
    }

    // TODO impl resize method
    unimplemented!()
}

/// It resizes the rectangle by a vector.
///
/// Arguments:
///
/// * `size`: the size of the vector to resize by
/// * `is_increase`: true if the rectangle is to be increased, false if it is to be decreased
pub fn resize_by_vector_size<'a>(
    point_iter_mut: impl Iterator<Item = &'a mut Point<f32>>,
    size: f32,
    is_increase: bool,
) {
    // TODO impl
    unimplemented!()

    // let size = size.abs();
    // self.compute_aspect();
    // let aspect: f32 = self.compute_aspect();
    // let y = size * aspect.hypot(1.).recip();
    // let x = aspect * y;
    // self.resize_by_vector((x, y), is_increase)
}

// TODO comment
pub fn indicate_increase_by_endpoint(
    end_point: impl Into<Point<f32>>,
    start_point: impl Into<Point<f32>>,
    center_point: impl Into<Point<f32>>,
) -> bool {
    let end_point = end_point.into();
    let start_point = start_point.into();
    let center_point = center_point.into();

    let start_vector: Vector<f32> = (center_point, start_point).into();
    let end_vector: Vector<f32> = (center_point, end_point).into();

    let start_vector_size = start_vector.abs();
    let end_vector_size = end_vector.abs();

    start_vector_size < end_vector_size
}
