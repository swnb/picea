use std::{borrow::Cow, ops::Deref};

use crate::{
    math::{
        edge::Edge, num::is_same_sign, point::Point, segment::Segment, vector::Vector, FloatNum,
    },
    meta::Mass,
};

use super::EdgeIterable;

/**
 * useful tool for polygon to transform
 */

/**
 * this function simply return the avg point of vertexes, it doesn't suit for all convex polygon
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

/**
 * split convex polygon into triangles , use the rate of area sum all the center point of triangle
 */
pub fn compute_convex_center_point(points: &[Point]) -> Point {
    let triangles = split_convex_polygon_to_triangles(points);

    let total_area = triangles
        .iter()
        .fold(0., |acc, triangle| acc + compute_area_of_triangle(triangle));

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
pub fn compute_area_of_convex(vertexes: &[Point]) -> FloatNum {
    let triangles = split_convex_polygon_to_triangles(vertexes);
    triangles.into_iter().fold(0., |acc, triangle| {
        acc + compute_area_of_triangle(&triangle)
    })
}

pub fn compute_moment_of_inertia_of_triangle(vertexes: &[Point; 3], m: Mass) -> FloatNum {
    let mut sum = 0.;
    for i in 0..3usize {
        let edge: Vector = (vertexes[i], vertexes[(i + 1) % 3]).into();
        sum += edge * edge;
    }
    (1. / 36.) * sum * m
}

/**
 * a,b,c is three vertex of triangle
 * s = 1/2 * (ab X ac);
 */
pub fn compute_area_of_triangle(vertexes: &[Point; 3]) -> FloatNum {
    let [a, b, c] = *vertexes;
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
    let vector = vector.normalize();
    let mut min = f32::MAX;
    let mut min_point = (0., 0.).into();
    let mut max = f32::MIN;
    let mut max_point = (0., 0.).into();
    point_iter.for_each(|&cur| {
        let size = cur.to_vector() * vector;
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
    vertexes: impl Iterator<Item = &'a mut Point>,
    center_point: &Point,
    from: &Point,
    to: &Point,
) {
    let hold_point = from;
    let resize_vector: &Vector = &(from, to).into();

    let hold_vector: Vector = (center_point, hold_point).into();
    let project_size = resize_vector >> &hold_vector;

    vertexes.for_each(|point| {
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

struct VertexesHelper<'a>(&'a [Point]);

impl<'a> Deref for VertexesHelper<'a> {
    type Target = &'a [Point];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl VertexesHelper<'_> {
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

pub(crate) struct VertexesToEdgeIter<'a> {
    index: usize,
    vertexes: &'a [Point],
}

impl<'a> VertexesToEdgeIter<'a> {
    pub fn new(vertexes: &'a [Point]) -> Self {
        Self { index: 0, vertexes }
    }
}

impl<'a> Iterator for VertexesToEdgeIter<'a> {
    type Item = Edge<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let len = self.vertexes.len();
        if self.index >= len {
            return None;
        }

        let edge = Edge::Line {
            start_point: &self.vertexes[self.index],
            end_point: &self.vertexes[(self.index + 1) % len],
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
pub fn check_is_polygon_clockwise(vertexes: &[Point]) -> bool {
    let mut area = 0.;
    let vertexes_len = vertexes.len();
    for i in 0..vertexes_len {
        let a = vertexes[i];
        let b = vertexes[(i + 1) % vertexes_len];
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
pub fn check_is_concave(vertexes: &[Point]) -> bool {
    let vertexes_len = vertexes.len();
    if vertexes_len <= 2 {
        return false;
    }

    let mut pre_edge: Vector = (vertexes[0], vertexes[1]).into();

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

    for i in 1..vertexes_len {
        let start_point = vertexes[i];
        let end_point = vertexes[(i + 1) % vertexes_len];

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
 * NOTE we must ensure polygon vertexes is clockwise
 * if polygon vertexes is counter clockwise , this algo is failure
 * try use check_is_polygon_clockwise to check is polygon clockwise
 * reference https://zhuanlan.zhihu.com/p/350994427
 */
pub fn split_clockwise_concave_polygon_to_two_convex_polygon(
    vertexes: &[Point],
) -> Option<[Vec<Point>; 2]> {
    let helper = VertexesHelper(vertexes);

    let vertexes_len = helper.len();
    if vertexes_len < 3 {
        return None;
    }

    for i in 0..vertexes_len {
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
        let mut min_cut_edge_index = vertexes_len;
        let mut cut_point = Point::new(0., 0.);
        // NOTE this can't be negative
        let min_projection_size_on_cut_edge = FloatNum::MAX;

        let mut cut_point_at_end_point = false;

        for j in 0..vertexes_len {
            // j can't index adjoin edge
            if j == i || (i + 1) % vertexes_len == j || (j + 1) % vertexes_len == i {
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
            }
        }

        if min_cut_edge_index == vertexes_len {
            unreachable!("cant' found the cut edge , something is wrong");
        }

        let z = min_cut_edge_index.max(i);
        let e = min_cut_edge_index.min(i);

        let mut polygon_one = Vec::with_capacity(vertexes_len - z + e + 1);
        polygon_one.extend(&vertexes[0..=e]);
        polygon_one.push(cut_point);
        if z + 1 < vertexes_len {
            polygon_one.extend(&vertexes[(z + 1)..]);
        }

        debug_assert_eq!(polygon_one.len(), vertexes_len - z + e + 1);

        let mut polygon_two = Vec::with_capacity(z - e + 1);
        polygon_two.extend(&vertexes[(e + 1)..=z]);
        polygon_two.push(cut_point);

        debug_assert_eq!(polygon_two.len(), z - e + 1);

        let remove_same_cut_point = |vertexes: &mut Vec<Point>| {
            let mut i = 0;
            while i < vertexes.len() {
                if &vertexes[i] == &cut_point {
                    let j = i + 1;
                    if vertexes.len() > j {
                        while &vertexes[j] == &cut_point {
                            vertexes.remove(j);
                        }
                    }
                    break;
                }
                i += 1;
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

pub fn split_concave_polygon_to_convex_polygons(vertexes: &[Point]) -> Vec<Vec<Point>> {
    if !check_is_concave(vertexes) {
        return vec![vertexes.into()];
    }

    let vertexes_cow = if check_is_polygon_clockwise(vertexes) {
        Cow::Borrowed(vertexes)
    } else {
        let mut vertexes = vertexes.to_owned();
        vertexes.reverse();
        Cow::Owned(vertexes)
    };

    let vertexes = &vertexes_cow[..];

    let mut result = vec![];

    let mut stack = vec![];

    if let Some(two_polygon) = split_clockwise_concave_polygon_to_two_convex_polygon(vertexes) {
        stack.extend(two_polygon);
    } else {
        let vertexes = match vertexes_cow {
            Cow::Borrowed(v) => v.to_owned(),
            Cow::Owned(v) => v,
        };
        result.push(vertexes);
    }

    while let Some(polygon) = stack.pop() {
        if let Some(two_polygon) = split_clockwise_concave_polygon_to_two_convex_polygon(&polygon) {
            stack.extend(two_polygon);
        } else {
            result.push(polygon);
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

mod tests {

    #[test]
    fn test_split_concave_polygon() {
        use super::split_clockwise_concave_polygon_to_two_convex_polygon;
        use crate::math::point::Point;
        use crate::math::FloatNum;
        use crate::shape::utils::check_is_polygon_clockwise;

        let vertexes = vec![(-1, 1), (0, 0), (1, 1), (1, -1), (-1, -1)];

        let vertexes = &vertexes
            .iter()
            .map(|&(x, y)| (x as FloatNum, y as FloatNum))
            .map(|v| v.into())
            .collect::<Vec<Point>>();

        assert!(!check_is_polygon_clockwise(
            &vertexes.iter().copied().rev().collect::<Vec<Point>>()
        ));

        assert!(check_is_polygon_clockwise(vertexes));

        let result = split_clockwise_concave_polygon_to_two_convex_polygon(vertexes).unwrap();

        // dbg!(result);

        let vertexes = &vec![
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

        let result = split_clockwise_concave_polygon_to_two_convex_polygon(vertexes).unwrap();

        dbg!(result);
    }
}
