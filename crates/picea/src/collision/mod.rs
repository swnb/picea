use picea_macro_tools::{Deref, Fields};

use crate::{
    math::{
        axis::AxisDirection,
        num::is_same_sign,
        pi,
        point::Point,
        segment::Segment,
        vector::{Vector, Vector3},
        FloatNum,
    },
    shape::{CenterPoint, NearestPoint},
};
use std::{
    cmp::Ordering,
    fmt::{Display, Write},
    ops::{ControlFlow, Deref, DerefMut, IndexMut},
};

pub trait Projector {
    fn projection_on_vector(&self, vector: &Vector) -> (Point, Point);

    fn projection_on_axis(&self, axis: AxisDirection) -> (f32, f32) {
        use AxisDirection::*;
        let (p1, p2) = self.projection_on_vector(&axis.into());
        match axis {
            X => (p1.x(), p2.x()),
            Y => (p1.y(), p2.y()),
        }
    }
}

pub trait SubCollider: Projector + CenterPoint + NearestPoint {}

impl<T> SubCollider for T where T: Projector + CenterPoint + NearestPoint {}

pub trait Collider: Projector + CenterPoint + NearestPoint {
    fn sub_colliders(&'_ self) -> Option<Box<dyn Iterator<Item = &'_ dyn SubCollider> + '_>> {
        None
    }

    // measure whether should accept sub collider's concat point as parent collider's contact point
    fn measure_sub_collider_concat_point(&self, _contact_point: &Point) -> bool {
        true
    }
}

// define collection of elements
pub trait CollisionalCollection: IndexMut<usize, Output = Self::Collider> {
    type Collider: Collider;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn sort(&mut self, compare: impl Fn(&Self::Collider, &Self::Collider) -> Ordering);
}

// new type for ColliderCollection , aim to add method for it
struct CollisionalCollectionWrapper<T>(T)
where
    T: CollisionalCollection;

impl<T> Deref for CollisionalCollectionWrapper<T>
where
    T: CollisionalCollection,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: CollisionalCollection> DerefMut for CollisionalCollectionWrapper<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// entry of collision check, if element is collision, handler will call
pub fn detect_collision<T, H, F>(elements: T, handler: H, skip: F)
where
    T: CollisionalCollection,
    // TODO use Iterator instead Vec
    H: Fn(&T::Collider, &T::Collider, Vec<ContactPointPair>),
    F: Fn(&T::Collider, &T::Collider) -> bool,
{
    // let time = std::time::Instant::now();
    rough_collision_detection(elements, |collider_a, collider_b| {
        if skip(collider_a, collider_b) {
            return;
        }
        prepare_accurate_collision_detection(
            collider_a,
            collider_b,
            |sub_collider_a, sub_collider_b| {
                if let Some(contact_point_pairs) =
                    accurate_collision_detection_for_sub_collider(sub_collider_a, sub_collider_b)
                {
                    handler(collider_a, collider_b, contact_point_pairs);
                }
            },
        )
    })
    // dbg!(time.elapsed());
}

/**
 * Rough collision detection, get two colliders that may collide
 */
pub fn rough_collision_detection<T, H>(elements: T, mut handler: H)
where
    T: CollisionalCollection,
    // TODO use Iterator instead Vec
    H: FnMut(&T::Collider, &T::Collider),
{
    // let time = std::time::Instant::now();

    // TODO
    let axis = AxisDirection::X;

    let elements = CollisionalCollectionWrapper(elements);

    sweep_and_prune_collision_detection(elements, axis, |a, b| {
        // TODO special collision algo for circle and circle
        handler(a, b);
    });

    // dbg!(time.elapsed());
}

/**
 * Accurate collision detection, extracting subparts that should collide
 */
pub fn prepare_accurate_collision_detection<C, H>(collider_a: &C, collider_b: &C, mut handler: H)
where
    C: Collider,
    H: FnMut(&dyn SubCollider, &dyn SubCollider),
{
    let sub_colliders_a = collider_a.sub_colliders();
    let sub_colliders_b = collider_b.sub_colliders();

    match (sub_colliders_a, sub_colliders_b) {
        // TODO
        (Some(sub_colliders_a), Some(_)) => {
            for collider_a in sub_colliders_a {
                let sub_colliders_b = collider_b.sub_colliders().unwrap();
                for collider_b in sub_colliders_b {
                    handler(collider_a, collider_b);
                }
            }
        }
        (Some(sub_colliders_a), None) => {
            for collider_a in sub_colliders_a {
                handler(collider_a, collider_b);
            }
        }
        (None, Some(sub_colliders_b)) => {
            for collider_b in sub_colliders_b {
                handler(collider_a, collider_b);
            }
        }
        (None, None) => {
            handler(collider_a, collider_b);
        }
    }
}

/**
 * Accurate collision detection, to obtain information such as collision points
 */
pub fn accurate_collision_detection_for_sub_collider(
    a: &dyn SubCollider,
    b: &dyn SubCollider,
) -> Option<Vec<ContactPointPair>> {
    let center_point_a = a.center_point();
    let center_point_b = b.center_point();
    let first_approximation_vector: Vector = (center_point_a, center_point_b).into();

    let compute_support_point = |reference_vector| {
        let (_, max_point_a) = a.projection_on_vector(&reference_vector);
        let (_, max_point_b) = b.projection_on_vector(&-reference_vector);
        (max_point_a, max_point_b).into()
    };

    let simplex = gjk_collision_detective(first_approximation_vector, compute_support_point)?;

    // REVIEW move this into gjk
    const MAX_TOLERABLE_CONTACT_DEPTH: FloatNum = 0.01;

    if simplex
        .iter()
        .any(|p| p.abs() < MAX_TOLERABLE_CONTACT_DEPTH)
    {
        return None;
    }

    let minkowski_edge: MinkowskiEdge = epa_compute_collision_edge(simplex, compute_support_point);

    let contact_infos: Vec<ContactPointPair> = minkowski_edge.get_contact_info(a, b);

    contact_infos.into()
}

/**
 * 粗检测
 * find the elements that maybe collision
 */
fn sweep_and_prune_collision_detection<T, Z>(
    mut elements: CollisionalCollectionWrapper<T>,
    axis: AxisDirection,
    mut handler: Z,
) where
    T: CollisionalCollection,
    Z: FnMut(&T::Collider, &T::Collider),
{
    if elements.is_empty() {
        return;
    }

    elements.sort(|a, b| {
        let (min_a_x, _) = a.projection_on_axis(axis);
        let (min_b_x, _) = b.projection_on_axis(axis);
        min_a_x.partial_cmp(&min_b_x).unwrap()
    });

    let len = elements.len();

    for i in 0..(len - 1) {
        let cur = &elements[i];
        let (_, max_x) = cur.projection_on_axis(axis);
        for j in (i + 1)..len {
            let is_collision_on_x = elements[j].projection_on_axis(axis).0 <= max_x;
            if is_collision_on_x {
                let (a_min_y, a_max_y) = elements[i].projection_on_axis(!axis);
                let (b_min_y, b_max_y) = elements[j].projection_on_axis(!axis);

                if !(a_max_y < b_min_y || b_max_y < a_min_y) {
                    // detective precise collision
                    let a: *const _ = &elements[i];
                    let b: *const _ = &elements[j];
                    unsafe {
                        handler(&*a, &*b);
                    };
                }
            } else {
                break;
            }
        }
    }
}

// TODO object is too large , we need shrink this struct in the future， rm start_point and end_point
// gjk 两个多边形形成的差集, 衍生的点
#[derive(Clone, Debug, Deref)]
pub(crate) struct MinkowskiDifferencePoint {
    pub(crate) start_point_from_a: Point,
    pub(crate) end_point_from_b: Point,
    #[deref]
    pub(crate) value: Vector,
}

impl Display for MinkowskiDifferencePoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{},", self.value))
    }
}

impl PartialEq for MinkowskiDifferencePoint {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl From<(Point, Point)> for MinkowskiDifferencePoint {
    fn from((s, e): (Point, Point)) -> Self {
        Self {
            start_point_from_a: s,
            end_point_from_b: e,
            value: (e, s).into(),
        }
    }
}

type Simplex = [MinkowskiDifferencePoint; 3];

// for rectangle , avg compare is 1 to 2 time;
// https://youtu.be/ajv46BSqcK4 gjk algo explain
pub(crate) fn gjk_collision_detective(
    first_approximation_vector: Vector,
    compute_support_point: impl Fn(Vector) -> MinkowskiDifferencePoint,
) -> Option<Simplex> {
    let approximation_vector = first_approximation_vector;

    let mut a = compute_support_point(approximation_vector);

    if a.value == (0., 0.).into() {
        return None;
    }

    let compute_support_point = |reference_vector: Vector| {
        let result = compute_support_point(reference_vector);
        // dbg!(&result, reference_vector);
        // FIXME this is wrong? <= 0
        if (result.value * reference_vector) < 0. {
            None
        } else {
            Some(result)
        }
    };

    let approximation_vector = -a.value;
    let mut b = compute_support_point(approximation_vector)?;

    if a == b {
        return None;
    }

    fn compute_third_reference_vector(a: Vector, b: Vector) -> Vector {
        let inv_b = -b;
        let vector_ba: Vector = a + inv_b;
        let base_vector: Vector3<f32> = vector_ba.into();
        let tmp_vector = base_vector ^ inv_b.into();
        if tmp_vector.z().abs() < FloatNum::EPSILON {
            !vector_ba
        } else {
            (tmp_vector ^ base_vector).into()
        }
    }

    let approximation_vector = compute_third_reference_vector(a.value, b.value);

    let mut c = compute_support_point(approximation_vector)?;

    if c == a || c == b {
        return None;
    }

    enum Res {
        Success,
        Failure,
    }

    // image simplex with point a, b, c, keep c as the updated point
    let mut is_origin_inside_simplex = || -> Option<ControlFlow<(), Res>> {
        use Res::*;

        let inv_c = -c.value;

        let ca: Vector3<_> = (a.value + inv_c).into();
        let cb: Vector3<_> = (b.value + inv_c).into();
        let cb_normal = (cb ^ (cb ^ ca)).into();

        if inv_c * cb_normal > f32::EPSILON {
            let tmp = compute_support_point(cb_normal)?;

            if tmp == c || tmp == b {
                return Some(ControlFlow::Break(()));
            }

            // update point
            a = c.clone();
            c = tmp;

            return Some(ControlFlow::Continue(Failure));
        }

        let ca_normal: Vector = (cb ^ ca ^ ca).into();

        if inv_c * ca_normal > f32::EPSILON {
            let tmp = compute_support_point(ca_normal)?;

            if tmp == c || tmp == a {
                return Some(ControlFlow::Break(()));
            }

            // update point
            b = c.clone();
            c = tmp;
            return ControlFlow::Continue(Failure).into();
        }

        ControlFlow::Continue(Success).into()
    };

    loop {
        use ControlFlow::*;
        use Res::*;

        return match is_origin_inside_simplex()? {
            Break(_) => None,
            Continue(Success) => Some([a, b, c]),
            Continue(Failure) => continue,
        };
    }
}

pub(crate) fn compute_minkowski<F>(compute_support_point: F) -> Vec<MinkowskiDifferencePoint>
where
    F: Fn(Vector) -> MinkowskiDifferencePoint,
{
    const SAMPLE_SIZE: usize = 100;

    let mut vector: Vector = (0., 1.).into();
    let mut result = Vec::with_capacity(SAMPLE_SIZE);
    let rad = pi() * 2. * (SAMPLE_SIZE as FloatNum).recip();
    for _ in 0..SAMPLE_SIZE {
        vector.affine_transformation_rotate_self(rad);
        let p = compute_support_point(vector);
        result.push(p);
    }
    result
}

// MinkowskiEdge means this edge maybe the Minkowski's edge
// it depends where it can or not expand any more
// if edge can't expand , and it's is closest edge to the origin, it is the edge we need
// the edge must inside the minkowski
#[derive(Clone, Debug)]
pub(crate) struct MinkowskiEdge {
    pub(crate) start_different_point: MinkowskiDifferencePoint,
    pub(crate) end_different_point: MinkowskiDifferencePoint,
    pub(crate) normal: Vector,
    pub(crate) depth: f32,
}

impl Display for MinkowskiEdge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_char('[')?;
        let start_point = &self.start_different_point;
        f.write_str(&format!("{},", start_point.value))?;
        let end_point = &self.end_different_point;
        f.write_str(&format!("{}", end_point.value))?;
        f.write_char(']')
    }
}

impl From<(MinkowskiDifferencePoint, MinkowskiDifferencePoint)> for MinkowskiEdge {
    fn from(
        (start_point, end_point): (MinkowskiDifferencePoint, MinkowskiDifferencePoint),
    ) -> Self {
        let a = start_point;
        let b = end_point;
        let ab = (b.value - a.value).into();
        let ao: Vector3<_> = (-a.value).into();

        let ao_x_ab = ao ^ ab;

        //  NOTE maybe z < EPSILON
        // ao_x_ab.z() == 0 means ab pass origin, follow compute will get NaN
        if (ao_x_ab).z().abs() < 1e-7 {
            let ab: Vector = ab.into();
            //  NOTE current normal direction need to be corrected when we need to know the orientation
            let normal = !ab;

            Self {
                start_different_point: a,
                end_different_point: b,
                normal: normal.normalize(),
                depth: 0.,
            }
        } else {
            let normal: Vector<_> = (ao_x_ab ^ ab).into();
            let depth = a.value >> normal;

            debug_assert!(depth.is_sign_positive());

            Self {
                start_different_point: a,
                end_different_point: b,
                normal: normal.normalize(),
                depth,
            }
        }
    }
}

impl MinkowskiEdge {
    pub(crate) fn expand<F, C>(
        &self,
        compute_support_point: F,
        compute_minkowski_center_point: C,
    ) -> Option<[MinkowskiEdge; 2]>
    where
        F: Fn(Vector) -> MinkowskiDifferencePoint,
        C: Fn() -> Point,
    {
        let mut normal = self.normal;

        if self.depth == 0. {
            let center_point = compute_minkowski_center_point();
            let normal_toward_origin = -center_point.to_vector();
            if !(normal * normal_toward_origin).is_sign_positive() {
                normal = -normal;
            }
        }

        let different_point = compute_support_point(normal);
        let new_point = different_point.value;

        // consider this const variable is same as zero
        const MAX_TOLERABLE_ERROR: f32 = 1e-4;

        if (new_point * normal) <= MAX_TOLERABLE_ERROR {
            return None;
        }

        if (different_point.value - self.start_different_point.value).abs() < MAX_TOLERABLE_ERROR
            || (different_point.value - self.end_different_point.value).abs() < MAX_TOLERABLE_ERROR
        {
            return None;
        }

        if ((new_point - self.start_different_point.value) * normal) <= MAX_TOLERABLE_ERROR {
            return None;
        }

        if ((new_point - self.end_different_point.value) * normal) <= MAX_TOLERABLE_ERROR {
            return None;
        }

        let result = [
            (self.start_different_point.clone(), different_point.clone()).into(),
            (different_point, self.end_different_point.clone()).into(),
        ];

        result.into()
    }

    pub(crate) fn get_contact_info(
        &self,
        sub_collider_a: &dyn SubCollider,
        sub_collider_b: &dyn SubCollider,
    ) -> Vec<ContactPointPair> {
        let gen_more_contact_points_with_nearest_point = sub_collider_a
            .support_find_nearest_point()
            && sub_collider_b.support_find_nearest_point();

        if gen_more_contact_points_with_nearest_point {
            return self.get_contact_info_with_nearest_point(sub_collider_a, sub_collider_b);
        }

        let &Self {
            normal,
            depth,
            ref start_different_point,
            ref end_different_point,
        } = self;

        let center_point_a = sub_collider_a.center_point();
        let center_point_b = sub_collider_b.center_point();

        let a1 = start_different_point.start_point_from_a;
        let a2 = end_different_point.start_point_from_a;

        let b1 = start_different_point.end_point_from_b;
        let b2 = end_different_point.end_point_from_b;

        // TODO use v_clip for all situation
        if a1 == a2 && b1 == b2 {
            let contact_point_a = a1;

            let tmp_vector: Vector<_> = (contact_point_a, center_point_a).into();

            // TODO 判断或许有误
            // FIXME 这里的处理必须要对 Line 做特殊处理
            let normal_toward_a = if (tmp_vector * normal).is_sign_negative() {
                -normal
            } else {
                normal
            };

            let contact_point_pair = ContactPointPair {
                point_a: a1,
                point_b: b1,
                normal_toward_a,
                depth,
                ..Default::default()
            };

            vec![contact_point_pair]
        } else if a1 == a2 {
            let contact_point_a = a1;

            let tmp_vector: Vector<_> = (contact_point_a, center_point_a).into();
            // TODO 判断或许有误
            let normal_toward_a = if (tmp_vector * normal).is_sign_negative() {
                -normal
            } else {
                normal
            };

            let contact_point_b = a1 + (normal_toward_a * depth);

            let contact_point_pair =
                ContactPointPair::new(contact_point_a, contact_point_b, normal_toward_a, depth);

            vec![contact_point_pair]
        } else if b1 == b2 {
            let contact_point_b = b1;

            let tmp_vector: Vector<_> = (contact_point_b, center_point_b).into();
            // TODO 判断或许有误
            let normal_toward_b = if (tmp_vector * normal).is_sign_negative() {
                -normal
            } else {
                normal
            };

            let normal_toward_a = -normal_toward_b;

            let contact_point_a = b1 + (normal_toward_b * depth);

            let contact_point_pair =
                ContactPointPair::new(contact_point_a, contact_point_b, normal_toward_a, depth);

            vec![contact_point_pair]
        } else {
            // pick one rand

            let contact_point_b = b1;

            let tmp_vector: Vector<_> = (contact_point_b, center_point_b).into();
            // TODO 判断或许有误
            let normal_toward_b = if (tmp_vector * normal).is_sign_negative() {
                -normal
            } else {
                normal
            };

            let normal_toward_a = -normal_toward_b;

            let contact_point_a = b1 + (normal_toward_b * depth);

            let contact_point_pair =
                ContactPointPair::new(contact_point_a, contact_point_b, normal_toward_a, depth);

            vec![contact_point_pair]

            // let edge_a: Segment<_> = (a1, a2).into();
            // let edge_b: Segment<_> = (b1, b2).into();

            // v_clip(edge_a, edge_b, normal, center_point_a, center_point_b)
        }
    }

    fn get_contact_info_with_nearest_point(
        &self,
        sub_collider_a: &dyn SubCollider,
        sub_collider_b: &dyn SubCollider,
    ) -> Vec<ContactPointPair> {
        let Self {
            normal,
            start_different_point,
            end_different_point,
            ..
        } = self;

        let center_point_a = sub_collider_a.center_point();
        let center_point_b = sub_collider_b.center_point();

        let a1 = start_different_point.start_point_from_a;
        let a2 = end_different_point.start_point_from_a;

        let b1 = start_different_point.end_point_from_b;
        let b2 = end_different_point.end_point_from_b;

        let (edge_a, edge_b) = match (a1 == a2, b1 == b2) {
            (true, true) => {
                let a2 = sub_collider_a.nearest_point(&a1, normal);
                let b2 = sub_collider_b.nearest_point(&b1, normal);
                let edge_a: Segment<_> = (a1, a2).into();
                let edge_b: Segment<_> = (b1, b2).into();
                (edge_a, edge_b)
            }
            (true, false) => {
                let a2 = sub_collider_a.nearest_point(&a1, normal);
                let edge_a: Segment<_> = (a1, a2).into();
                let edge_b: Segment<_> = (b1, b2).into();
                (edge_a, edge_b)
            }
            (false, true) => {
                let b2 = sub_collider_b.nearest_point(&b1, normal);
                let edge_a: Segment<_> = (a1, a2).into();
                let edge_b: Segment<_> = (b1, b2).into();
                (edge_a, edge_b)
            }
            (false, false) => {
                let edge_a: Segment<_> = (a1, a2).into();
                let edge_b: Segment<_> = (b1, b2).into();
                (edge_a, edge_b)
            }
        };

        v_clip(edge_a, edge_b, *normal, center_point_a, center_point_b)
    }
}

#[derive(Deref)]
struct Minkowski {
    #[deref]
    edges: Vec<MinkowskiEdge>,
}

impl Minkowski {
    pub(crate) fn new(simplex: Simplex) -> Self {
        // expect two iter to find the close edge
        let mut edges: Vec<MinkowskiEdge> = Vec::with_capacity(3 + 2);
        for i in 0..3 {
            let j = (i + 1) % 3;
            let a = simplex[i].clone();
            let b = simplex[j].clone();
            let edge = (a, b).into();
            edges.push(edge);
        }

        Self { edges }
    }

    pub(crate) fn compute_center_point(&self) -> Point {
        let sum_result: Vector = self.edges.iter().fold(Default::default(), |mut acc, cur| {
            acc += cur.start_different_point.value;
            acc
        });

        (sum_result * (self.len() as FloatNum).recip()).to_point()
    }

    // expand the minkowski, find the min
    pub(crate) fn expand<F>(&mut self, compute_support_point: F) -> Result<(), ()>
    where
        F: Fn(Vector) -> MinkowskiDifferencePoint,
    {
        let min_index = self.find_min_edge_index();

        self[min_index]
            .expand(&compute_support_point, || self.compute_center_point())
            .map(|new_edges| {
                self.edges.splice(min_index..min_index + 1, new_edges);
            })
            .ok_or(())
    }

    pub(crate) fn find_min_edge(&self) -> MinkowskiEdge {
        let min_index = self.find_min_edge_index();

        self[min_index].clone()
    }

    fn find_min_edge_index(&self) -> usize {
        let mut min_depth = f32::MAX;
        let mut min_index = 0;
        for (i, edge) in self.iter().enumerate() {
            if edge.depth < min_depth {
                min_index = i;
                min_depth = edge.depth;
            }
        }
        min_index
    }
}

// https://dyn4j.org/2010/05/epa-expanding-polytope-algorithm/ epa algo explain
pub(crate) fn epa_compute_collision_edge<F>(
    simplex: Simplex,
    compute_support_point: F,
) -> MinkowskiEdge
where
    F: Fn(Vector) -> MinkowskiDifferencePoint,
{
    let mut minkowski = Minkowski::new(simplex);

    const MAX_ITER_COUNT: usize = 10;
    // avoid dead loop
    let mut current_iter_count = 0;

    while minkowski.expand(&compute_support_point).is_ok() && current_iter_count <= MAX_ITER_COUNT {
        current_iter_count += 1;
    }

    minkowski.find_min_edge()
}

/**
 * ContactInfo contain the collider collision info
 * contact_point is where the collision happen
 * normal toward the shape of collider
 * depth is how deep the collision happen
 */
#[derive(Clone, Debug, Default, Fields)]
#[r]
pub struct ContactPointPair {
    point: Point,
    point_a: Point,
    point_b: Point,
    #[r(copy)]
    normal_toward_a: Vector,
    depth: f32,
}

impl ContactPointPair {
    pub(crate) fn new(point_a: Point, point_b: Point, normal: Vector, depth: FloatNum) -> Self {
        let point = ((point_a.to_vector() + point_b.to_vector()) * 0.5).to_point();

        ContactPointPair {
            point_a,
            point_b,
            point,
            normal_toward_a: normal,
            depth,
        }
    }
}

// want more detail about v_clip, visit
// https://dyn4j.org/2011/11/contact-points-using-clipping/
fn v_clip(
    edge_a: Segment<f32>,
    edge_b: Segment<f32>,
    normal: Vector,
    center_point_a: Point,
    center_point_b: Point,
) -> Vec<ContactPointPair> {
    // which collider is reference , A or B
    enum Collider {
        A,
        B,
    }

    let get_reference_normal = |reference_center_point: Point, incident_center_point: Point| {
        let tmp_vector: Vector = (incident_center_point, reference_center_point).into();

        // normal direction must point to reference poly
        if (tmp_vector * normal).is_sign_negative() {
            -normal
        } else {
            normal
        }
    };

    let (reference_edge, incident_edge, reference_normal, reference_collider) = {
        if (edge_a.to_vector() * normal).abs() < (edge_b.to_vector() * normal).abs() {
            let reference_normal = get_reference_normal(center_point_a, center_point_b);
            (edge_a, edge_b, reference_normal, Collider::A)
        } else {
            let reference_normal = get_reference_normal(center_point_b, center_point_a);
            (edge_b, edge_a, reference_normal, Collider::B)
        }
    };

    // first and second clip
    let contact_points = clip(&reference_edge, &incident_edge);

    // last clip
    let reference_point = reference_edge.start_point();

    let reference_projection_size = reference_point.to_vector() * reference_normal;

    let compute_contact_pair = |contact_point: Point| {
        let depth = contact_point.to_vector() * reference_normal - reference_projection_size;
        if depth.is_sign_negative() {
            return None;
        }

        let contact_point_1 = contact_point + (-reference_normal * depth);
        let contact_point_2 = contact_point;

        let normal_1 = reference_normal;
        let normal_2 = -normal_1;

        let contact_pair = match reference_collider {
            Collider::A => ContactPointPair::new(contact_point_1, contact_point_2, normal_1, depth),
            Collider::B => ContactPointPair::new(contact_point_2, contact_point_1, normal_2, depth),
        };
        contact_pair.into()
    };

    contact_points
        .into_iter()
        .filter_map(compute_contact_pair)
        .collect()
}

fn clip(reference_edge: &Segment<f32>, incident_edge: &Segment<f32>) -> Vec<Point> {
    let mut contact_points = Vec::with_capacity(4);

    let reference_vector = reference_edge.to_vector().normalize();
    let incident_v1 = incident_edge.start_point();
    let incident_v2 = incident_edge.end_point();

    // vector must normalize
    let compute_incident_point_project_size = |reference_point: &Point, vector: Vector| {
        let reference_project_size = reference_point.to_vector() * vector;

        let incident_v1_projection_size = incident_v1.to_vector() * vector - reference_project_size;
        let incident_v2_projection_size = incident_v2.to_vector() * vector - reference_project_size;
        (incident_v1_projection_size, incident_v2_projection_size)
    };

    let reference_v1 = reference_edge.start_point();
    let (s1_by_reference_v1, s2_by_reference_v1) =
        compute_incident_point_project_size(reference_v1, reference_vector);
    let reference_v2 = reference_edge.end_point();
    let (s1_by_reference_v2, s2_by_reference_v2) =
        compute_incident_point_project_size(reference_v2, -reference_vector);

    if s1_by_reference_v1.is_sign_positive() && s1_by_reference_v2.is_sign_positive() {
        contact_points.push(*incident_v1);
    }

    if s2_by_reference_v1.is_sign_positive() && s2_by_reference_v2.is_sign_positive() {
        contact_points.push(*incident_v2);
    }

    if !is_same_sign(s1_by_reference_v1, s2_by_reference_v1) {
        // 尝试切割
        let s1_abs = s1_by_reference_v1.abs();
        let s2_abs = s2_by_reference_v1.abs();

        let incident_vector = incident_edge.to_vector();
        let ratio = s1_abs * (s1_abs + s2_abs).recip();
        let contact_point = *incident_v1 + incident_vector * ratio;

        contact_points.push(contact_point);
    }

    if !is_same_sign(s1_by_reference_v2, s2_by_reference_v2) {
        // 尝试切割
        let s1_abs = s1_by_reference_v2.abs();
        let s2_abs = s2_by_reference_v2.abs();

        let incident_vector = incident_edge.to_vector();
        let ratio = s1_abs * (s1_abs + s2_abs).recip();
        let contact_point = *incident_v1 + incident_vector * ratio;

        contact_points.push(contact_point);
    }

    contact_points
}

// fn sat_collision_detective<T>(a: &T::Element, b: &T::Element) -> Option<Vector>
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

//     let mut collision_normal: (f32, Option<Vector>) = (f32::MAX, None);

//     fn projection(shape: &ElementShape, axis: Vector) -> (f32, f32) {
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

mod tests {

    #[test]
    fn test_minkowski_point() {
        use super::{MinkowskiDifferencePoint, MinkowskiEdge};
        use crate::math::point::Point;
        let start_different_point = MinkowskiDifferencePoint {
            start_point_from_a: Point { x: 50.0, y: 70.0 },
            end_point_from_b: Point {
                x: 50.0,
                y: -9.469273,
            },
            value: (0.0, 79.46927).into(),
        };
        let end_different_point = MinkowskiDifferencePoint {
            start_point_from_a: Point { x: 50.0, y: -30.0 },
            end_point_from_b: Point {
                x: 50.0,
                y: -9.469273,
            },
            value: (0.0, -20.530727).into(),
        };

        let edge: MinkowskiEdge = (start_different_point, end_different_point).into();

        assert!(edge.depth == 0.);
    }
}
