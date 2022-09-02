use crate::{
    math::{
        axis::AxisDirection,
        point::Point,
        vector::{Vector, Vector3},
    },
    meta::collision::{CollisionInfo, ContactType},
};
use std::{
    cmp::Ordering,
    ops::{ControlFlow, Index, IndexMut},
};

// define element trait
pub trait Element {
    fn id(&self) -> u32;

    fn projection_on_axis(&self, axis: AxisDirection) -> (f32, f32);

    fn projection_on_vector(&self, vector: &Vector<f32>) -> (Point<f32>, Point<f32>);

    fn center_point(&self) -> Point<f32>;
}

// define collection of elements
pub trait ElementCollection {
    type Element: Element;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn get(&self, index: usize) -> &Self::Element;

    fn get_mut(&mut self, index: usize) -> &mut Self::Element;

    fn sort(&mut self, compare: impl Fn(&Self::Element, &Self::Element) -> Ordering + Copy);
}

// new type for ElementCollection , aim to add method for it
struct ElementCollectionWrapper<T>(T)
where
    T: ElementCollection;

impl<T> Index<usize> for ElementCollectionWrapper<T>
where
    T: ElementCollection,
{
    type Output = T::Element;
    fn index(&self, index: usize) -> &Self::Output {
        self.0.get(index)
    }
}

impl<T> IndexMut<usize> for ElementCollectionWrapper<T>
where
    T: ElementCollection,
{
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.0.get_mut(index)
    }
}

impl<T> ElementCollectionWrapper<T>
where
    T: ElementCollection,
{
    fn len(&self) -> usize {
        self.0.len()
    }

    fn sort<F>(&mut self, compare: F)
    where
        F: Fn(&T::Element, &T::Element) -> Ordering + Copy,
    {
        self.0.sort(compare);
    }
}

// entry of collision check, if element is collision, handler will call
pub fn detect_collision<T>(
    elements: T,
    mut handler: impl FnMut(&mut T::Element, &mut T::Element, CollisionInfo),
) where
    T: ElementCollection,
{
    // let time = std::time::Instant::now();

    // TODO
    let axis = AxisDirection::X;

    let elements = ElementCollectionWrapper(elements);

    sweep_and_prune_collision_detection(elements, axis, |a, b| {
        // TODO special collision algo for circle and circle
        if let Some(collision_info) = special_collision_detection(a, b) {
            handler(a, b, collision_info);
        }
    });

    // dbg!(time.elapsed());
}

pub fn special_collision_detection<E: Element>(a: &mut E, b: &mut E) -> Option<CollisionInfo> {
    let center_point_a = a.center_point();
    let center_point_b = b.center_point();
    let first_approximation_vector: Vector<f32> = (center_point_a, center_point_b).into();

    let compute_support_point = |reference_vector| {
        let (_, max_point_a) = a.projection_on_vector(&reference_vector);
        let (_, max_point_b) = b.projection_on_vector(&-reference_vector);
        (max_point_b, max_point_a).into()
    };

    gjk_collision_detective(first_approximation_vector, compute_support_point).map(|simplex| {
        let collision_edge = epa_compute_collision_edge(simplex, compute_support_point);

        let g_a = &collision_edge.a;
        let g_b = &collision_edge.b;
        let b1 = g_a.start_point;
        let a1 = g_a.end_point;
        let b2 = g_b.start_point;
        let a2 = g_b.end_point;

        fn compute_collision_contact_type(p1: Point<f32>, p2: Point<f32>) -> ContactType {
            use ContactType::*;
            if p1 == p2 {
                Point(p1)
            } else {
                Edge([p1, p2])
            }
        }

        let contact_a = compute_collision_contact_type(a1, a2);
        let contact_b = compute_collision_contact_type(b1, b2);

        CollisionInfo {
            collision_element_id_pair: (a.id(), b.id()),
            contact_a,
            contact_b,
            normal: collision_edge.normal,
            depth: collision_edge.depth,
        }
    })
}

/**
 * 粗检测
 * find the elements that maybe collision
 */
fn sweep_and_prune_collision_detection<T, Z>(
    mut elements: ElementCollectionWrapper<T>,
    axis: AxisDirection,
    mut handler: Z,
) where
    T: ElementCollection,
    Z: FnMut(&mut T::Element, &mut T::Element),
{
    elements.sort(|a, b| {
        let (ref min_a_x, _) = a.projection_on_axis(axis);
        let (ref min_b_x, _) = b.projection_on_axis(axis);
        min_a_x.partial_cmp(min_b_x).unwrap()
    });

    for i in 1..elements.len() {
        let cur = &elements[i];
        let (min_x, _) = cur.projection_on_axis(axis);
        for j in (0..i).rev() {
            let is_collision_on_x = elements[j].projection_on_axis(axis).1 >= min_x;

            if is_collision_on_x {
                let (a_min_y, a_max_y) = elements[i].projection_on_axis(!axis);
                let (b_min_y, b_max_y) = elements[j].projection_on_axis(!axis);

                if !(a_max_y < b_min_y || b_max_y < a_min_y) {
                    // detective precise collision
                    let a: *mut _ = &mut elements[i];
                    let b: *mut _ = &mut elements[j];
                    unsafe {
                        handler(&mut *b, &mut *a);
                    };
                }
            } else {
                // no element is collision
                break;
            }
        }
    }
}

// gjk 两个多边形形成的差集, 衍生的点
#[derive(Clone, Debug)]
struct GJKDifferencePoint {
    start_point: Point<f32>,
    end_point: Point<f32>,
    vector: Vector<f32>,
}

impl PartialEq for GJKDifferencePoint {
    fn eq(&self, other: &Self) -> bool {
        self.vector == other.vector
    }
}

impl From<(Point<f32>, Point<f32>)> for GJKDifferencePoint {
    fn from((s, e): (Point<f32>, Point<f32>)) -> Self {
        Self {
            start_point: s,
            end_point: e,
            vector: (s, e).into(),
        }
    }
}

type Triangle = [GJKDifferencePoint; 3];

// for rectangle , avg compare is 1 to 2 time;
// https://youtu.be/ajv46BSqcK4 gjk algo explain
fn gjk_collision_detective(
    first_approximation_vector: Vector<f32>,
    compute_support_point: impl Fn(Vector<f32>) -> GJKDifferencePoint,
) -> Option<Triangle> {
    let approximation_vector = first_approximation_vector;

    let mut a = compute_support_point(approximation_vector);

    let compute_support_point = |reference_vector: Vector<f32>| {
        let result = compute_support_point(reference_vector);
        // FIXME this is wrong? <= 0
        if (result.vector * reference_vector) < 0. {
            None
        } else {
            Some(result)
        }
    };

    let approximation_vector = -a.vector;
    let mut b = compute_support_point(approximation_vector)?;
    if a == b {
        return None;
    }

    fn compute_third_reference_vector(a: Vector<f32>, b: Vector<f32>) -> Vector<f32> {
        let inv_b = -b;
        let base_vector: Vector<f32> = a + inv_b;
        let base_vector: Vector3<f32> = base_vector.into();
        (base_vector ^ inv_b.into() ^ base_vector).into()
    }

    let approximation_vector = compute_third_reference_vector(a.vector, b.vector);

    let mut c = compute_support_point(approximation_vector)?;

    if c == a || c == b {
        return None;
    }

    enum Res {
        Success,
        Failure,
    }

    // image triangle with point a, b, c, keep c as the updated point
    let mut is_origin_inside_triangle = || -> Option<ControlFlow<(), Res>> {
        use Res::*;

        let inv_c = -c.vector;

        let ca: Vector3<_> = (a.vector + inv_c).into();
        let cb: Vector3<_> = (b.vector + inv_c).into();
        let cb_normal = (cb ^ (cb ^ ca)).into();

        if inv_c * cb_normal > f32::EPSILON {
            // refactor

            let tmp = compute_support_point(cb_normal)?;

            if tmp == c || tmp == b {
                return Some(ControlFlow::Break(()));
            }

            // update point
            a = c.clone();
            c = tmp;

            return Some(ControlFlow::Continue(Failure));
        }

        let ca_normal: Vector<f32> = (cb ^ ca ^ ca).into();

        if inv_c * ca_normal > f32::EPSILON {
            let tmp = compute_support_point(ca_normal)?;

            if tmp == c || tmp == a {
                return Some(ControlFlow::Break(()));
            }

            // update point
            b = c.clone();
            c = tmp;
            return Some(ControlFlow::Continue(Failure));
        }

        Some(ControlFlow::Continue(Success))
    };

    loop {
        use ControlFlow::*;
        use Res::*;

        return match is_origin_inside_triangle()? {
            Break(_) => None,
            Continue(Success) => Some([a, b, c]),
            Continue(Failure) => continue,
        };
    }
}

#[derive(Clone, Debug)]
struct ClosestGJKDifferenceEdge {
    a: GJKDifferencePoint,
    b: GJKDifferencePoint,
    normal: Vector<f32>,
    depth: f32,
}

// https://dyn4j.org/2010/05/epa-expanding-polytope-algorithm/ epa algo explain
fn epa_compute_collision_edge(
    triangle: Triangle,
    compute_support_point: impl Fn(Vector<f32>) -> GJKDifferencePoint,
) -> ClosestGJKDifferenceEdge {
    fn compute_edge_info_away_from_edge(
        a: &GJKDifferencePoint,
        b: &GJKDifferencePoint,
    ) -> (f32, Vector<f32>) {
        let ab = (b.vector - a.vector).into();
        let ao: Vector3<_> = (-a.vector).into();
        let mut normal: Vector<_> = (ao ^ ab ^ ab).into();
        normal = normal.normalize();
        let depth = a.vector * normal;
        (depth, normal)
    }

    // init simplex
    let mut simplex = Vec::with_capacity(3 * 3);
    for i in 0..3 {
        let j = (i + 1) % 3;
        let a = &triangle[i];
        let b = &triangle[j];
        let (depth, normal) = compute_edge_info_away_from_edge(a, b);
        simplex.push(ClosestGJKDifferenceEdge {
            a: a.clone(),
            b: b.clone(),
            depth,
            normal,
        });
    }

    loop {
        let mut origin_closest_edge_anchor = (f32::MAX, 0);
        for (i, edge) in simplex.iter().enumerate() {
            if edge.depth < origin_closest_edge_anchor.0 {
                origin_closest_edge_anchor.0 = edge.depth;
                origin_closest_edge_anchor.1 = i;
            }
        }

        let origin_closest_edge_index = origin_closest_edge_anchor.1;
        let i = origin_closest_edge_index;

        let origin_closest_edge = &simplex[i];

        let expand_point = compute_support_point(origin_closest_edge.normal);

        if (expand_point.vector - origin_closest_edge.a.vector) * origin_closest_edge.normal
            < f32::EPSILON
        {
            // can't expand
            return origin_closest_edge.clone();
        }

        let a = origin_closest_edge.a.clone();
        let b = expand_point.clone();
        let (depth, normal) = compute_edge_info_away_from_edge(&a, &b);

        if (depth - origin_closest_edge.depth).abs() < f32::EPSILON {
            return origin_closest_edge.clone();
        }

        let left = ClosestGJKDifferenceEdge {
            a,
            b,
            depth,
            normal,
        };

        let a = expand_point;
        let b = origin_closest_edge.b.clone();
        let (depth, normal) = compute_edge_info_away_from_edge(&a, &b);

        if (depth - origin_closest_edge.depth).abs() < f32::EPSILON {
            return origin_closest_edge.clone();
        }

        let right = ClosestGJKDifferenceEdge {
            a,
            b,
            depth,
            normal,
        };

        simplex.splice(i..(i + 1), [left, right]);
    }
}

// fn sat_collision_detective<T>(a: &T::Element, b: &T::Element) -> Option<Vector<f32>>
// where
//     T: ElementCollection,
// {
//     let shape_a = a.shape();
//     let shape_b = b.shape();

//     use ElementShape::*;
//     let (shape_a, shape_b) = match shape_a {
//         Rect(shape_a) => match shape_b {
//             Rect(shape_b) => (shape_a, shape_b),
//             // TODO impl
//             _ => return None,
//         },
//         // TODO impl
//         _ => return None,
//     };

//     let edge_iter = shape_a.edge_iter().chain(shape_b.edge_iter());

//     let mut collision_normal: (f32, Option<Vector<f32>>) = (f32::MAX, None);

//     fn projection(shape: &ElementShape, axis: Vector<f32>) -> (f32, f32) {
//         use ElementShape::*;
//         match shape {
//             Rect(shape) => shape
//                 .corner_iter()
//                 .fold((f32::MAX, f32::MIN), |mut pre, &corner| {
//                     let size = corner >> axis;
//                     if size < pre.0 {
//                         pre.0 = size
//                     }
//                     if size > pre.1 {
//                         pre.1 = size
//                     }
//                     pre
//                 }),
//             Circle(shape) => {
//                 // TODO 实现圆的投影逻辑
//                 unimplemented!()
//             }
//         }
//     }

//     for edge in edge_iter {
//         let normal = !edge;
//         let (a_min, a_max) = projection(a.shape(), normal);
//         let (b_min, b_max) = projection(b.shape(), normal);

//         if a_min < b_min {
//             if b_min > a_max {
//                 return None;
//             } else {
//                 let cross_size = b_max.min(a_max) - b_min;
//                 if collision_normal.0 > cross_size {
//                     collision_normal.0 = cross_size;
//                     collision_normal.1 = Some(normal)
//                 }
//             }
//         } else if a_min > b_max {
//             return None;
//         } else {
//             let cross_size = b_max.min(a_max) - a_min;
//             if collision_normal.0 > cross_size {
//                 collision_normal.0 = cross_size;
//                 collision_normal.1 = Some(normal)
//             }
//         }
//     }
//     collision_normal.1
// }

fn v_clip_collision_pointer_detective<T>(a: &T::Element, b: &T::Element, normal: Vector<f32>)
where
    T: ElementCollection,
{
}
