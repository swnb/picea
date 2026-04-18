use std::{borrow::Cow, ops::Deref};

use crate::{
    collision::Projector,
    math::{
        axis::AxisDirection, edge::Edge, num::is_same_sign, point::Point, segment::Segment,
        vector::Vector, FloatNum,
    },
    meta::Mass,
};

use super::{CenterPoint, EdgeIterable, NearestPoint};

/**
 * useful tool for polygon to transform
 */

/**
 * this function simply return the avg point of vertices, it doesn't suit for all convex polygon
 */
pub fn compute_polygon_approximate_center_point<'a>(
    point_iter: impl Iterator<Item = &'a Point>,
    edge_count: f32,
) -> Point {
    let mut point_iter = point_iter.map(|p| p.to_vector());
    let first_point = point_iter.next().unwrap();
    let sum = point_iter.fold(first_point, |acc, p| acc + p);
    (sum * edge_count.recip()).to_point()
}

fn is_finite_point(point: &Point) -> bool {
    point.x().is_finite() && point.y().is_finite()
}

fn finite_points(vertices: &[Point]) -> Vec<Point> {
    vertices.iter().copied().filter(is_finite_point).collect()
}

fn compute_finite_average_point(points: &[Point]) -> Point {
    let mut count = 0;
    let sum = points.iter().fold(Vector::default(), |acc, point| {
        if is_finite_point(point) {
            count += 1;
            acc + point.to_vector()
        } else {
            acc
        }
    });

    if count == 0 {
        Point::default()
    } else {
        (sum * (count as FloatNum).recip()).to_point()
    }
}

/**
 * split convex polygon into triangles , use the rate of area sum all the center point of triangle
 * Degenerate or non-finite inputs fall back to a finite vertex average/default point.
 */
pub fn compute_convex_center_point(points: &[Point]) -> Point {
    let finite_points = finite_points(points);
    if finite_points.is_empty() {
        return Point::default();
    }
    let points = &finite_points[..];

    let triangles = split_convex_polygon_to_triangles(points);

    let total_area = triangles
        .iter()
        .fold(0., |acc, triangle| acc + compute_area_of_triangle(triangle));

    if !total_area.is_finite() || total_area <= FloatNum::EPSILON {
        return compute_finite_average_point(points);
    }

    let total_area_inv = total_area.recip();

    let center_point: Vector = triangles.iter().fold(Default::default(), |acc, triangle| {
        let center_point = compute_polygon_approximate_center_point(triangle.iter(), 3.);
        let rate = compute_area_of_triangle(triangle) * total_area_inv;
        acc + (center_point.to_vector() * rate)
    });

    center_point.to_point()
}

/**
 * split convex polygon into triangles , compute the sum of all triangle area
 */
pub fn compute_area_of_convex(vertices: &[Point]) -> FloatNum {
    let triangles = split_convex_polygon_to_triangles(vertices);
    triangles.into_iter().fold(0., |acc, triangle| {
        acc + compute_area_of_triangle(&triangle)
    })
}

pub fn compute_moment_of_inertia_of_triangle(vertices: &[Point; 3], m: Mass) -> FloatNum {
    let mut sum = 0.;
    for i in 0..3usize {
        let edge: Vector = (vertices[i], vertices[(i + 1) % 3]).into();
        sum += edge * edge;
    }
    (1. / 36.) * sum * m
}

/**
 * a,b,c is three vertex of triangle
 * s = 1/2 * (ab X ac);
 */
pub fn compute_area_of_triangle(vertices: &[Point; 3]) -> FloatNum {
    let [a, b, c] = *vertices;
    let ab: Vector = (a, b).into();
    let ac = (a, c).into();
    (ab ^ ac).abs() * 0.5
}

// split convex polygon into many triangles
pub fn split_convex_polygon_to_triangles(points: &[Point]) -> Vec<[Point; 3]> {
    let points_len = points.len();

    if points_len < 3 {
        return vec![];
    }

    let mut result = Vec::with_capacity(points_len - 2);

    // a , b, c is the three point of triangles
    let a = points[0];

    for i in 1..(points_len - 1) {
        let b = points[i];
        let c = points[i + 1];

        result.push([a, b, c]);
    }

    result
}

pub fn projection_polygon_on_vector<'a>(
    point_iter: impl Iterator<Item = &'a Point>,
    vector: Vector,
) -> (Point, Point) {
    // A zero or near-zero direction collapses every projection to the first
    // finite vertex; non-finite vertices are ignored and all-invalid input
    // returns the finite default point.
    let vector = vector.normalize();
    let mut min: Option<(FloatNum, Point)> = None;
    let mut max: Option<(FloatNum, Point)> = None;
    point_iter.for_each(|&cur| {
        if !is_finite_point(&cur) {
            return;
        }

        let size = cur.to_vector() * vector;
        if !size.is_finite() {
            return;
        }

        if min.is_none_or(|(pre, _)| size < pre) {
            min = Some((size, cur));
        }
        if max.is_none_or(|(pre, _)| size > pre) {
            max = Some((size, cur));
        }
    });

    (
        min.map(|(_, point)| point).unwrap_or_default(),
        max.map(|(_, point)| point).unwrap_or_default(),
    )
}

pub fn translate_polygon<'a>(point_iter_mut: impl Iterator<Item = &'a mut Point>, vector: &Vector) {
    point_iter_mut.for_each(|p| *p += vector);
}

pub fn rotate_polygon<'a>(
    center_point: Point,
    point_iter_mut: impl Iterator<Item = &'a mut Point>,
    rad: f32,
) {
    point_iter_mut.for_each(|corner| {
        let mut corner_vector: Vector = (center_point, *corner).into();
        corner_vector.affine_transformation_rotate_self(rad);
        *corner = center_point + corner_vector;
    })
}

pub fn resize_by_vector<'a>(
    vertices: impl Iterator<Item = &'a mut Point>,
    center_point: &Point,
    from: &Point,
    to: &Point,
) {
    let hold_point = from;
    let resize_vector: &Vector = &(from, to).into();

    let hold_vector: Vector = (center_point, hold_point).into();
    let project_size = resize_vector >> &hold_vector;

    vertices.for_each(|point| {
        let v: Vector = (center_point, &*point).into();
        let abs_vector = v.abs();
        let resized_vector = &(v.normalize() * (abs_vector + project_size));
        *point = *center_point + resized_vector
    })
}

// TODO comment
pub fn indicate_increase_by_endpoint(
    end_point: impl Into<Point>,
    start_point: impl Into<Point>,
    center_point: impl Into<Point>,
) -> bool {
    let end_point = end_point.into();
    let start_point = start_point.into();
    let center_point = center_point.into();

    let start_vector: Vector = (center_point, start_point).into();
    let end_vector: Vector = (center_point, end_point).into();

    let start_vector_size = start_vector.abs();
    let end_vector_size = end_vector.abs();

    start_vector_size < end_vector_size
}

struct VerticesHelper<'a>(&'a [Point]);

impl<'a> Deref for VerticesHelper<'a> {
    type Target = &'a [Point];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl VerticesHelper<'_> {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn index_edge(&self, index: usize) -> Segment {
        let len = self.len();
        let a = self[index % len];
        let b = self[(index + 1) % len];
        (a, b).into()
    }
}

pub(crate) struct VerticesToEdgeIter<'a> {
    index: usize,
    vertices: &'a [Point],
}

impl<'a> VerticesToEdgeIter<'a> {
    pub fn new(vertices: &'a [Point]) -> Self {
        Self { index: 0, vertices }
    }
}

impl<'a> Iterator for VerticesToEdgeIter<'a> {
    type Item = Edge<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let len = self.vertices.len();
        if self.index >= len {
            return None;
        }

        let edge = Edge::Line {
            start_point: &self.vertices[self.index],
            end_point: &self.vertices[(self.index + 1) % len],
        };

        self.index += 1;

        edge.into()
    }
}

/**
 * cross method, use the area of polygon
 * function isClockwise(polygon):
 *   area = 0
 *  for i from 0 to n-1:
 *        j = (i + 1) % n
 *        area += polygon[i].x * polygon[j].y
 *        area -= polygon[j].x * polygon[i].y
 *    if area < 0:
 *        return true
 *    else:
 *        return false
 */
pub fn check_is_polygon_clockwise(vertices: &[Point]) -> bool {
    let mut area = 0.;
    let vertices_len = vertices.len();
    for i in 0..vertices_len {
        let a = vertices[i];
        let b = vertices[(i + 1) % vertices_len];
        area += a.to_vector() ^ b.to_vector();
    }

    area.is_sign_negative()
}

/**
 * segment_a has start point 'a' and end point 'b'
 * segment_b has start point 'c' and end point 'd'
 * if ab cross cd
 * then two condition must satisfy
 * 1. ac X ab is not same sign with ad X ab
 * 2. ca X cd is not same sign with cb X cd
 */
pub fn check_is_segment_cross(segment_a: &Segment, segment_b: &Segment) -> bool {
    let a = segment_a.start_point();
    let b = segment_a.end_point();

    let c = segment_b.start_point();
    let d = segment_b.end_point();

    let ac: Vector = (a, c).into();
    let ab: Vector = (a, b).into();
    let ad: Vector = (a, d).into();

    let ca: Vector = -ac;
    let cb: Vector = (c, b).into();
    let cd: Vector = (c, d).into();

    // FIXME what if cross result zero
    !is_same_sign(ac ^ ab, ad ^ ab) && !is_same_sign(ca ^ cd, cb ^ cd)
}

/**
 * ray segment_a has start point 'a' and end point 'b'
 * segment_b has start point 'c' and end point 'd'
 */
fn check_is_ray_cross_segment(ray_segment_a: &Segment, segment_b: &Segment) -> bool {
    let a = ray_segment_a.start_point();
    let vector_ab: Vector = ray_segment_a.into();
    let c = segment_b.start_point();
    let d = segment_b.end_point();

    let ac: Vector = (a, c).into();
    let ad: Vector = (a, d).into();

    !is_same_sign(ac ^ vector_ab, ad ^ vector_ab)
}

/**
 * assume segment_a did cross segment_b
* segment_a has start point 'a' and end point 'b'
* segment_b has start point 'c' and end point 'd'
* get cd normal vector 'cd_normal'
* get project_size for 'a', 'b', 'c' in  'cd_normal'
* get rate for ac and cb, compute the point inside ab with rate of ac / (ac+cb)
* NOTE segment_b can be ray vector
*/
pub fn compute_cross_point_between_two_segment(segment_a: &Segment, segment_b: &Segment) -> Point {
    let a = segment_a.start_point();
    let b = segment_a.end_point();

    let c = segment_b.start_point();

    let cd: Vector = segment_b.into();
    let cd_normal = !cd;

    let a_projection_size = cd_normal * a.to_vector();
    let c_projection_size = cd_normal * c.to_vector();
    let b_project_size = cd_normal * b.to_vector();

    let ac_size = c_projection_size - a_projection_size;
    let cb_size = b_project_size - c_projection_size;

    // REVIEW ac+cb maybe zero ?

    let ab: Vector = (a, b).into();
    let rate = ac_size * (ac_size + cb_size).recip();
    *a + ab * rate
}

// check is shape concave or not
pub fn check_is_concave(vertices: &[Point]) -> bool {
    let vertices_len = vertices.len();
    if vertices_len <= 2 {
        return false;
    }

    let mut pre_edge: Vector = (vertices[0], vertices[1]).into();

    enum CrossResultSign {
        Negative,
        Positive,
        Undefine,
    }

    impl From<FloatNum> for CrossResultSign {
        fn from(value: FloatNum) -> Self {
            if value.is_sign_positive() {
                CrossResultSign::Positive
            } else {
                CrossResultSign::Negative
            }
        }
    }

    let mut pre_cross_result: CrossResultSign = CrossResultSign::Undefine;

    for i in 1..vertices_len {
        let start_point = vertices[i];
        let end_point = vertices[(i + 1) % vertices_len];

        let current_edge: Vector = (start_point, end_point).into();

        let current_cross_result: CrossResultSign = (pre_edge ^ current_edge).into();

        use CrossResultSign::*;
        match pre_cross_result {
            Negative => {
                if let Positive = current_cross_result {
                    return true;
                }
            }
            Positive => {
                if let Negative = current_cross_result {
                    return true;
                }
            }
            _ => {}
        };

        pre_edge = current_edge;
        pre_cross_result = current_cross_result;
    }

    false
}

/**
 * NOTE we must ensure polygon vertices is clockwise
 * if polygon vertices is counter clockwise , this algo is failure
 * try use check_is_polygon_clockwise to check is polygon clockwise
 * reference https://zhuanlan.zhihu.com/p/350994427
 */
pub fn split_clockwise_concave_polygon_to_two_convex_polygon(
    vertices: &[Point],
) -> Option<[Vec<Point>; 2]> {
    let helper = VerticesHelper(vertices);

    let vertices_len = helper.len();
    if vertices_len <= 3 {
        return None;
    }

    for i in 0..vertices_len {
        let edge_a = helper.index_edge(i);
        let edge_b = helper.index_edge(i + 1);

        if (edge_a.to_vector() ^ edge_b.to_vector()).is_sign_negative() {
            continue;
        }
        // find the concave polygon's concave point 'b'

        // use edge_a as reference edge, compute the cross_point in cut edge use ray reference edge
        let reference_edge = edge_a;

        let reference_vector = reference_edge.to_vector();

        // find the minimum distance toward cut_edge
        let mut min_cut_edge_index = vertices_len;
        let mut cut_point = Point::new(0., 0.);
        // NOTE this can't be negative
        let mut min_projection_size_on_cut_edge = FloatNum::MAX;

        let mut cut_point_at_end_point = false;

        for j in 0..vertices_len {
            // j can't index adjoin edge
            if j == i || (i + 1) % vertices_len == j || (j + 1) % vertices_len == i {
                continue;
            }
            let cut_edge = helper.index_edge(j);
            if !check_is_ray_cross_segment(&reference_edge, &cut_edge) {
                continue;
            }
            // TODO what is cross_point equal one of cut_edge's point;
            let cross_point = compute_cross_point_between_two_segment(&cut_edge, &reference_edge);

            if &cross_point == cut_edge.start_point() || &cross_point == cut_edge.end_point() {
                cut_point_at_end_point = true;
            }

            let ray: Vector = (reference_edge.start_point(), &cross_point).into();
            let projection_size = ray * reference_vector;
            if projection_size.is_sign_negative() {
                // can't be negative;
                continue;
            }

            if projection_size < min_projection_size_on_cut_edge {
                min_cut_edge_index = j;
                cut_point = cross_point;
                min_projection_size_on_cut_edge = projection_size;
            }
        }

        if min_cut_edge_index == vertices_len {
            // Degenerate inputs can look concave but have no valid internal cut.
            // Try another candidate; the caller will use a conservative fallback
            // if no cut is found at all.
            continue;
        }

        let z = min_cut_edge_index.max(i);
        let e = min_cut_edge_index.min(i);

        let mut polygon_one = Vec::with_capacity(vertices_len - z + e + 1);
        polygon_one.extend(&vertices[0..=e]);
        polygon_one.push(cut_point);
        if z + 1 < vertices_len {
            polygon_one.extend(&vertices[(z + 1)..]);
        }

        debug_assert_eq!(polygon_one.len(), vertices_len - z + e + 1);

        let mut polygon_two = Vec::with_capacity(z - e + 1);
        polygon_two.extend(&vertices[(e + 1)..=z]);
        polygon_two.push(cut_point);

        debug_assert_eq!(polygon_two.len(), z - e + 1);

        let remove_same_cut_point = |vertices: &mut Vec<Point>| {
            let mut index = 0;
            while index + 1 < vertices.len() {
                if vertices[index] == cut_point && vertices[index + 1] == cut_point {
                    vertices.remove(index + 1);
                } else {
                    index += 1;
                }
            }

            // Endpoint cuts can put the same cut point at both ends of the
            // polygon slice. Remove the wrapped duplicate before recursion.
            if vertices.len() > 1
                && vertices[0] == cut_point
                && vertices[vertices.len() - 1] == cut_point
            {
                vertices.pop();
            }
        };

        if cut_point_at_end_point {
            remove_same_cut_point(&mut polygon_one);
            remove_same_cut_point(&mut polygon_two);
        }

        return [polygon_one, polygon_two].into();
    }

    None
}

fn fallback_polygon_for_unsplittable_input(vertices: &[Point]) -> Vec<Point> {
    let mut result = Vec::with_capacity(vertices.len());

    for &point in vertices {
        if !is_finite_point(&point) {
            continue;
        }
        if result.last().is_some_and(|last| *last == point) {
            continue;
        }
        result.push(point);
    }

    if result.len() > 1 && result.first() == result.last() {
        result.pop();
    }

    let mut index = 0;
    while result.len() > 3 && index < result.len() {
        let len = result.len();
        let prev = result[(index + len - 1) % len];
        let current = result[index];
        let next = result[(index + 1) % len];

        let incoming: Vector = (prev, current).into();
        let outgoing: Vector = (current, next).into();

        if incoming.abs() <= FloatNum::EPSILON
            || outgoing.abs() <= FloatNum::EPSILON
            || (incoming ^ outgoing).abs() <= FloatNum::EPSILON
        {
            result.remove(index);
        } else {
            index += 1;
        }
    }

    // If cleanup cannot produce a valid finite polygon, return a tiny
    // conservative triangle so downstream shape constructors still see a
    // finite, non-concave polygon with at least three points.
    if is_valid_fallback_polygon(&result) {
        result
    } else {
        conservative_finite_triangle()
    }
}

fn is_valid_fallback_polygon(vertices: &[Point]) -> bool {
    vertices.len() >= 3
        && vertices.first() != vertices.last()
        && vertices.iter().all(is_finite_point)
        && !check_is_concave(vertices)
}

fn conservative_finite_triangle() -> Vec<Point> {
    vec![(0., 0.).into(), (1., 0.).into(), (0., 1.).into()]
}

pub fn split_concave_polygon_to_convex_polygons(vertices: &[Point]) -> Vec<Vec<Point>> {
    if !check_is_concave(vertices) {
        return vec![vertices.into()];
    }

    let vertices_cow = if check_is_polygon_clockwise(vertices) {
        Cow::Borrowed(vertices)
    } else {
        let mut vertices = vertices.to_owned();
        vertices.reverse();
        Cow::Owned(vertices)
    };

    let vertices = &vertices_cow[..];

    let mut result = vec![];

    let mut stack = vec![];

    if let Some(two_polygon) = split_clockwise_concave_polygon_to_two_convex_polygon(vertices) {
        stack.extend(two_polygon);
    } else {
        let vertices = match vertices_cow {
            Cow::Borrowed(v) => v.to_owned(),
            Cow::Owned(v) => v,
        };
        result.push(fallback_polygon_for_unsplittable_input(&vertices));
    }

    while let Some(polygon) = stack.pop() {
        if let Some(two_polygon) = split_clockwise_concave_polygon_to_two_convex_polygon(&polygon) {
            stack.extend(two_polygon);
        } else {
            result.push(fallback_polygon_for_unsplittable_input(&polygon));
        }
    }

    result
}

pub fn rotate_point(point: &Point, origin_point: &Point, rad: FloatNum) -> Point {
    let mut tmp_vector: Vector = (origin_point, point).into();
    tmp_vector.affine_transformation_rotate_self(rad);
    *origin_point + tmp_vector
}

pub fn find_nearest_point<T: EdgeIterable + ?Sized>(
    shape: &T,
    reference_point: &Point,
    &direction: &Vector,
) -> Point {
    let mut closest_point_to_reference_point = *reference_point;
    let mut min_project_size_to_reference_point = FloatNum::MAX;

    let reference_project_size = reference_point.to_vector() * direction;

    let mut hit_count = 0;

    for edge in shape.edge_iter() {
        match edge {
            Edge::Arc {
                start_point,
                support_point,
                end_point,
            } => {
                // TODO
                unimplemented!()
            }
            Edge::Circle {
                center_point,
                radius,
            } => {
                unimplemented!()
            }
            Edge::Line {
                start_point,
                end_point,
            } => {
                if start_point == reference_point {
                    let project_size = end_point.to_vector() * direction;
                    let project_size_to_reference_point =
                        (project_size - reference_project_size).abs();
                    if project_size_to_reference_point < min_project_size_to_reference_point {
                        min_project_size_to_reference_point = project_size_to_reference_point;
                        closest_point_to_reference_point = *end_point;
                    }
                    hit_count += 1;
                } else if end_point == reference_point {
                    let project_size = start_point.to_vector() * direction;
                    let project_size_to_reference_point =
                        (project_size - reference_project_size).abs();
                    if project_size_to_reference_point < min_project_size_to_reference_point {
                        min_project_size_to_reference_point = project_size_to_reference_point;
                        closest_point_to_reference_point = *start_point;
                    }
                    hit_count += 1;
                }

                if hit_count >= 2 {
                    break;
                }
            }
        }
    }

    closest_point_to_reference_point
}

// use radial method to create vector from point to (infinite,point.y)
// if the size of edges which cross vector is odd, the point is inside shape
pub fn is_point_inside_shape(
    point: impl Into<Point>,
    edge_iter: &'_ mut dyn Iterator<Item = Edge<'_>>,
) -> bool {
    let mut cross_count: usize = 0;
    let offset_vector: Vector = (point.into(), (0., 0.).into()).into();

    let is_segment_cross_axis_x = |p1: Point, p2: Point| {
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
            } => is_segment_cross_axis_x(*start_point + offset_vector, *end_point + offset_vector),
        };
        if is_cross {
            cross_count += 1;
        }
    }

    cross_count % 2 != 0
}

mod test {

    #[test]
    fn test_is_point_inside_shape() {
        use crate::math::{point::Point, vector::Vector};

        let is_point_cross_segment = |p1: Point, p2: Point| {
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
        let p1: Point = (-31.55, 142.13).into();
        let p2: Point = (-46.091, 227.683).into();
        let offset_vector: Vector = (82.6, -175.48).into();
        assert!(is_point_cross_segment(
            p1 + offset_vector,
            p2 + offset_vector
        ));
    }
}

pub trait VerticesIter {
    fn vertices_iter(&self) -> impl Iterator<Item = &Point>;

    fn vertices_iter_mut(&mut self) -> impl Iterator<Item = &mut Point>;
}

impl<T> Projector for T
where
    T: VerticesIter,
{
    fn projection_on_vector(&self, &vector: &Vector) -> (Point, Point) {
        projection_polygon_on_vector(self.vertices_iter(), vector)
    }

    #[inline]
    fn projection_on_axis(&self, axis: AxisDirection) -> (f32, f32) {
        use AxisDirection::*;
        let point_iter = self.vertices_iter();
        type Reducer<T> = fn((T, T), &Point<T>) -> (T, T);
        let reducer: Reducer<f32> = match axis {
            X => |mut pre, v| {
                pre.0 = v.x().min(pre.0);
                pre.1 = v.x().max(pre.1);
                pre
            },
            Y => |mut pre, v| {
                pre.0 = v.y().min(pre.0);
                pre.1 = v.y().max(pre.1);
                pre
            },
        };
        point_iter.fold((f32::MAX, f32::MIN), reducer)
    }
}

pub trait CenterPointHelper: CenterPoint {
    fn center_point(&self) -> Point {
        CenterPoint::center_point(self)
    }

    fn center_point_mut(&mut self) -> &mut Point;
}

impl<T> NearestPoint for T
where
    T: VerticesIter + EdgeIterable,
{
    fn support_find_nearest_point(&self) -> bool {
        true
    }

    fn nearest_point(&self, reference_point: &Point, direction: &Vector) -> Point {
        find_nearest_point(self, reference_point, direction)
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! impl_shape_traits_use_deref {
    ($struct_name:ty, $($variants:tt)*) => {
        impl<$($variants)*> $crate::shape::utils::VerticesIter for $struct_name {
            fn vertices_iter(&self) -> impl Iterator<Item = &Point> {
                self.deref().vertices_iter()
            }

            fn vertices_iter_mut(&mut self) -> impl Iterator<Item = &mut Point> {
                self.deref_mut().vertices_iter_mut()
            }
        }

        impl<$($variants)*> $crate::shape::GeometryTransformer for $struct_name {
            fn sync_transform(&mut self, transform: &$crate::meta::Transform) {
                self.deref_mut().sync_transform(transform)
            }
        }

        impl<$($variants)*> $crate::shape::EdgeIterable for $struct_name {
            fn edge_iter(&self) -> Box<dyn Iterator<Item = $crate::shape::Edge<'_>> + '_> {
                self.deref().edge_iter()
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::math::segment::Segment;
    use crate::math::{point::Point, vector::Vector, FloatNum};

    use super::rotate_polygon;

    fn points(raw: &[(FloatNum, FloatNum)]) -> Vec<Point> {
        raw.iter().copied().map(Point::from).collect()
    }

    fn assert_finite_points(vertices: &[Point]) {
        assert!(vertices
            .iter()
            .all(|point| point.x().is_finite() && point.y().is_finite()));
    }

    fn polygon_area(vertices: &[Point]) -> FloatNum {
        if vertices.len() < 3 {
            return 0.;
        }

        let double_area = vertices.iter().enumerate().fold(0., |acc, (index, point)| {
            let next = vertices[(index + 1) % vertices.len()];
            acc + (point.to_vector() ^ next.to_vector())
        });

        double_area.abs() * 0.5
    }

    fn assert_convex_partition(source: &[Point], result: &[Vec<Point>]) {
        assert!(!result.is_empty());

        for vertices in result {
            assert_valid_output_polygon(vertices);
        }

        let source_area = polygon_area(source);
        let result_area: FloatNum = result.iter().map(|vertices| polygon_area(vertices)).sum();
        let tolerance = source_area.max(1.) * 0.0001;

        assert!((source_area - result_area).abs() <= tolerance);
    }

    fn assert_valid_output_polygon(vertices: &[Point]) {
        assert!(vertices.len() >= 3);
        assert_ne!(vertices.first(), vertices.last());
        assert_finite_points(vertices);
        assert!(!super::check_is_concave(vertices));
    }

    #[test]
    fn projection_on_zero_vector_returns_finite_collapsed_projection() {
        let vertices = points(&[(2., 3.), (4., 6.), (5., 9.)]);

        let (min_point, max_point) =
            super::projection_polygon_on_vector(vertices.iter(), Vector::new(0., 0.));

        assert_eq!(min_point, vertices[0]);
        assert_eq!(max_point, vertices[0]);
        assert_finite_points(&[min_point, max_point]);
    }

    #[test]
    fn degenerate_polygon_inputs_use_conservative_finite_fallbacks() {
        let cases = [
            (
                "collinear_zero_area",
                points(&[(0., 0.), (1., 0.), (2., 0.), (3., 0.)]),
            ),
            (
                "repeated_point",
                points(&[(0., 0.), (1., 0.), (1., 0.), (1., 1.), (0., 1.)]),
            ),
            (
                "tiny_edge",
                points(&[
                    (0., 0.),
                    (f32::EPSILON * 0.25, 0.),
                    (1., 0.),
                    (1., 1.),
                    (0., 1.),
                ]),
            ),
        ];

        for (name, vertices) in cases {
            assert_finite_points(&vertices);
            assert!(!super::check_is_concave(&vertices), "{name}");
            assert!(!super::check_is_polygon_clockwise(&vertices) || polygon_area(&vertices) > 0.);

            let result = super::split_concave_polygon_to_convex_polygons(&vertices);
            assert_eq!(result.len(), 1, "{name}");
            assert_eq!(result[0], vertices, "{name}");
            assert_finite_points(&result[0]);
        }
    }

    #[test]
    fn convex_center_point_uses_average_for_zero_area_inputs() {
        let vertices = points(&[(0., 0.), (1., 0.), (2., 0.), (3., 0.)]);

        let center_point = super::compute_convex_center_point(&vertices);

        assert_finite_points(&[center_point]);
        assert_eq!(center_point, (1.5, 0.).into());
    }

    #[test]
    fn segment_cross_handles_collinear_and_zero_length_segments() {
        let cases: [(Segment, Segment); 3] = [
            (
                ((0., 0.).into(), (1., 0.).into()).into(),
                ((2., 0.).into(), (3., 0.).into()).into(),
            ),
            (
                ((0., 0.).into(), (1., 0.).into()).into(),
                ((1., 0.).into(), (1., 0.).into()).into(),
            ),
            (
                ((0., 0.).into(), (0., 0.).into()).into(),
                ((0., 1.).into(), (1., 1.).into()).into(),
            ),
        ];

        for (segment_a, segment_b) in cases {
            assert!(!super::check_is_segment_cross(&segment_a, &segment_b));
        }
    }

    #[test]
    fn concave_split_accepts_clockwise_and_counter_clockwise_input() {
        let clockwise = points(&[(-1., 1.), (0., 0.), (1., 1.), (1., -1.), (-1., -1.)]);
        let mut counter_clockwise = clockwise.clone();
        counter_clockwise.reverse();

        let cases = [
            ("clockwise", clockwise),
            ("counter_clockwise", counter_clockwise),
        ];

        for (name, vertices) in cases {
            let result = super::split_concave_polygon_to_convex_polygons(&vertices);
            assert_convex_partition(&vertices, &result);
            assert!(
                result
                    .iter()
                    .all(|vertices| !super::check_is_concave(vertices)),
                "{name}"
            );
        }
    }

    #[test]
    fn concave_split_falls_back_when_degenerate_input_has_no_cut_edge() {
        let vertices = points(&[(-1., -1.), (-1., 0.), (-1., 1.), (0., -1.)]);

        assert!(super::check_is_concave(&vertices));

        let result = super::split_concave_polygon_to_convex_polygons(&vertices);

        assert_convex_partition(&vertices, &result);
    }

    #[test]
    fn concave_split_fallback_never_returns_too_few_vertices() {
        let vertices = points(&[(-1., -1.), (-1., -1.), (-1., 0.)]);

        assert!(super::check_is_concave(&vertices));

        let result = super::split_concave_polygon_to_convex_polygons(&vertices);

        assert!(!result.is_empty());
        for vertices in result {
            assert_valid_output_polygon(&vertices);
        }
    }

    #[test]
    fn convex_center_point_ignores_non_finite_vertices() {
        let vertices = [
            Point::new(FloatNum::NAN, 0.),
            Point::new(FloatNum::INFINITY, 1.),
            Point::new(0., 0.),
            Point::new(2., 0.),
        ];

        let center_point = super::compute_convex_center_point(&vertices);

        assert_finite_points(&[center_point]);
        assert_eq!(center_point, (1., 0.).into());
    }

    #[test]
    fn polygon_projection_ignores_non_finite_vertices() {
        let vertices = [
            Point::new(FloatNum::NAN, 0.),
            Point::new(FloatNum::INFINITY, 1.),
            Point::new(2., 0.),
            Point::new(4., 0.),
        ];

        let (min_point, max_point) =
            super::projection_polygon_on_vector(vertices.iter(), Vector::new(1., 0.));

        assert_finite_points(&[min_point, max_point]);
        assert_eq!(min_point, (2., 0.).into());
        assert_eq!(max_point, (4., 0.).into());
    }

    #[test]
    fn finite_fallbacks_use_default_when_no_finite_vertices_exist() {
        let vertices = [
            Point::new(FloatNum::NAN, 0.),
            Point::new(FloatNum::INFINITY, FloatNum::NEG_INFINITY),
        ];

        let center_point = super::compute_convex_center_point(&vertices);
        let (min_point, max_point) =
            super::projection_polygon_on_vector(vertices.iter(), Vector::new(1., 0.));

        assert_eq!(center_point, Point::default());
        assert_eq!(min_point, Point::default());
        assert_eq!(max_point, Point::default());
    }

    #[test]
    fn test_split_concave_polygon() {
        use super::split_clockwise_concave_polygon_to_two_convex_polygon;
        use crate::math::point::Point;
        use crate::math::FloatNum;
        use crate::shape::utils::check_is_polygon_clockwise;

        let vertices = vec![(-1, 1), (0, 0), (1, 1), (1, -1), (-1, -1)];

        let vertices = &vertices
            .iter()
            .map(|&(x, y)| (x as FloatNum, y as FloatNum))
            .map(|v| v.into())
            .collect::<Vec<Point>>();

        assert!(!check_is_polygon_clockwise(
            &vertices.iter().copied().rev().collect::<Vec<Point>>()
        ));

        assert!(check_is_polygon_clockwise(vertices));

        let result = split_clockwise_concave_polygon_to_two_convex_polygon(vertices).unwrap();

        // dbg!(result);

        let vertices = &vec![
            (1.0, 1.0),
            (1.0, -1.0),
            (-1.0, -1.0),
            (-1.0, 1.0),
            (0.0, 0.0),
        ]
        .iter()
        .map(|&(x, y)| (x as FloatNum, y as FloatNum))
        .map(|v| v.into())
        .collect::<Vec<Point>>();

        let result = split_clockwise_concave_polygon_to_two_convex_polygon(vertices).unwrap();

        dbg!(result);
    }

    #[test]
    fn test_split_concave_polygon1() {
        use crate::math::point::Point;

        let vertices = vec![
            Point { x: 15.0, y: 55.0 },
            Point { x: 20.0, y: 60.0 },
            Point { x: 25.0, y: 58.0 },
            Point { x: 30.0, y: 63.0 },
            Point { x: 35.0, y: 61.0 },
            Point { x: 40.0, y: 66.0 },
            Point { x: 45.0, y: 64.0 },
            Point { x: 50.0, y: 69.0 },
            Point { x: 55.0, y: 67.0 },
            Point { x: 60.0, y: 72.0 },
            Point { x: 65.0, y: 70.0 },
            Point { x: 70.0, y: 75.0 },
            Point { x: 75.0, y: 73.0 },
            Point { x: 80.0, y: 78.0 },
            Point { x: 85.0, y: 76.0 },
            Point { x: 90.0, y: 81.0 },
            Point { x: 95.0, y: 79.0 },
            Point { x: 100.0, y: 84.0 },
            Point { x: 105.0, y: 79.0 },
            Point { x: 110.0, y: 81.0 },
            Point { x: 115.0, y: 76.0 },
            Point { x: 120.0, y: 78.0 },
            Point { x: 125.0, y: 73.0 },
            Point { x: 130.0, y: 75.0 },
            Point { x: 135.0, y: 70.0 },
            Point { x: 140.0, y: 72.0 },
            Point { x: 145.0, y: 67.0 },
            Point { x: 150.0, y: 69.0 },
            Point { x: 155.0, y: 64.0 },
            Point { x: 160.0, y: 66.0 },
            Point { x: 165.0, y: 61.0 },
            Point { x: 170.0, y: 63.0 },
            Point { x: 175.0, y: 58.0 },
            Point { x: 180.0, y: 60.0 },
            Point { x: 180.0, y: -30.0 },
            Point { x: 181.0, y: 110.0 },
            Point { x: 0.0, y: 110.0 },
            Point { x: 10.0, y: -30.0 },
        ];

        let result = super::split_concave_polygon_to_convex_polygons(&vertices);

        assert_convex_partition(&vertices, &result);
    }

    #[test]
    fn rotate_test() {
        let mut current_position: Vec<Point> = vec![
            (95.673, 130.119).into(),
            (104.680, 130.146).into(),
            (105.934, 140.100).into(),
            (96.927, 140.073).into(),
        ];
        let position_translate = Vector::<FloatNum>::new(1.3052475, 39.899548);

        current_position.iter_mut().for_each(|position| {
            *position -= position_translate;
        });

        let mut points = [
            Point { x: 95.0, y: 90.0 },
            Point { x: 104.0, y: 90.0 },
            Point { x: 104.0, y: 100.0 },
            Point { x: 95.0, y: 100.0 },
        ];
        rotate_polygon(Point { x: 99.5, y: 95.0 }, points.iter_mut(), 0.10710551);
        dbg!(points);
        dbg!(current_position);
    }
}
