use std::cmp::Ordering;

use crate::{
    body::Pose,
    collider::SharedShape,
    events::{
        EpaTerminationReason, GenericConvexFallbackReason, GenericConvexTrace, GjkTerminationReason,
    },
    math::{point::Point, vector::Vector, FloatNum},
};

const GJK_MAX_ITERATIONS: usize = 32;
const EPA_MAX_ITERATIONS: usize = 32;
const GJK_EPSILON: FloatNum = 1.0e-5;
const EPA_EPSILON: FloatNum = 1.0e-4;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GjkResult {
    pub(crate) intersects: bool,
    pub(crate) distance: FloatNum,
    pub(crate) termination: GjkTerminationReason,
    pub(crate) iterations: usize,
    pub(crate) simplex_len: usize,
    simplex: Vec<SupportPoint>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EpaPenetration {
    pub(crate) normal: Vector,
    pub(crate) depth: FloatNum,
    pub(crate) contact_point: Point,
    pub(crate) termination: EpaTerminationReason,
    pub(crate) iterations: usize,
    pub(crate) gjk: GjkResult,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EpaFailure {
    pub(crate) termination: EpaTerminationReason,
    pub(crate) iterations: usize,
    pub(crate) gjk_termination: GjkTerminationReason,
    pub(crate) gjk_iterations: usize,
    pub(crate) simplex_len: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GenericConvexContact {
    pub(crate) normal: Vector,
    pub(crate) depth: FloatNum,
    pub(crate) point: Point,
    pub(crate) trace: GenericConvexTrace,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct SupportPoint {
    v: Vector,
    point_a: Point,
    point_b: Point,
}

pub(crate) fn gjk_distance(
    shape_a: &SharedShape,
    pose_a: Pose,
    shape_b: &SharedShape,
    pose_b: Pose,
) -> GjkResult {
    let mut direction = initial_direction(pose_a, pose_b);
    let Some(first) = support(shape_a, pose_a, shape_b, pose_b, direction) else {
        return gjk_failure(GjkTerminationReason::InvalidSupport, 0, Vec::new());
    };
    let mut simplex = vec![first];
    direction = -first.v;
    if direction.length() <= GJK_EPSILON {
        return gjk_done(false, 0.0, GjkTerminationReason::Touching, 1, simplex);
    }

    for iteration in 1..=GJK_MAX_ITERATIONS {
        let Some(next) = support(shape_a, pose_a, shape_b, pose_b, direction) else {
            return gjk_failure(GjkTerminationReason::InvalidSupport, iteration, simplex);
        };
        let progress = next.v.dot(direction);
        simplex.push(next);
        if progress < -GJK_EPSILON {
            let distance = closest_distance_to_origin(&simplex);
            return gjk_done(
                false,
                distance,
                GjkTerminationReason::Separated,
                iteration,
                simplex,
            );
        }
        if update_simplex(&mut simplex, &mut direction) {
            return gjk_done(
                true,
                0.0,
                GjkTerminationReason::Intersect,
                iteration,
                simplex,
            );
        }
        if progress.abs() <= GJK_EPSILON || next.v.length() <= GJK_EPSILON {
            return gjk_done(
                false,
                0.0,
                GjkTerminationReason::Touching,
                iteration,
                simplex,
            );
        }
        if direction.length() <= GJK_EPSILON {
            return gjk_done(
                false,
                0.0,
                GjkTerminationReason::DegenerateDirection,
                iteration,
                simplex,
            );
        }
    }

    gjk_done(
        false,
        closest_distance_to_origin(&simplex),
        GjkTerminationReason::MaxIterations,
        GJK_MAX_ITERATIONS,
        simplex,
    )
}

pub(crate) fn epa_penetration(
    shape_a: &SharedShape,
    pose_a: Pose,
    shape_b: &SharedShape,
    pose_b: Pose,
) -> Result<EpaPenetration, EpaFailure> {
    let gjk = gjk_distance(shape_a, pose_a, shape_b, pose_b);
    if !gjk.intersects {
        return Err(EpaFailure {
            termination: EpaTerminationReason::GjkDidNotIntersect,
            iterations: 0,
            gjk_termination: gjk.termination,
            gjk_iterations: gjk.iterations,
            simplex_len: gjk.simplex_len,
        });
    }
    let mut polytope = gjk.simplex.clone();
    if polytope.len() < 3 || signed_area(&polytope).abs() <= EPA_EPSILON {
        return Err(EpaFailure {
            termination: EpaTerminationReason::DegenerateEdge,
            iterations: 0,
            gjk_termination: gjk.termination,
            gjk_iterations: gjk.iterations,
            simplex_len: gjk.simplex_len,
        });
    }
    if signed_area(&polytope) < 0.0 {
        polytope.reverse();
    }

    for iteration in 1..=EPA_MAX_ITERATIONS {
        let Some(edge) = closest_edge(&polytope) else {
            return Err(epa_failure(
                EpaTerminationReason::DegenerateEdge,
                iteration,
                &gjk,
            ));
        };
        let Some(candidate) = support(shape_a, pose_a, shape_b, pose_b, edge.normal) else {
            return Err(epa_failure(
                EpaTerminationReason::InvalidSupport,
                iteration,
                &gjk,
            ));
        };
        let support_distance = candidate.v.dot(edge.normal);
        if !support_distance.is_finite() {
            return Err(epa_failure(
                EpaTerminationReason::InvalidSupport,
                iteration,
                &gjk,
            ));
        }
        if support_distance - edge.distance <= EPA_EPSILON {
            let mut normal = edge.normal.normalized_or_zero();
            let center_delta = pose_a.point() - pose_b.point();
            if normal.dot(center_delta) < 0.0 {
                normal = -normal;
            }
            if normal.length() <= GJK_EPSILON {
                return Err(epa_failure(
                    EpaTerminationReason::DegenerateEdge,
                    iteration,
                    &gjk,
                ));
            }
            let depth = edge.distance.max(0.0);
            let point_a = shape_a
                .support_point(pose_a, -normal)
                .unwrap_or(edge.a.point_a);
            let point_b = shape_b
                .support_point(pose_b, normal)
                .unwrap_or(edge.a.point_b);
            let contact_point = Point::from((Vector::from(point_a) + Vector::from(point_b)) * 0.5);
            return Ok(EpaPenetration {
                normal,
                depth,
                contact_point,
                termination: EpaTerminationReason::Converged,
                iterations: iteration,
                gjk,
            });
        }
        if polytope
            .iter()
            .any(|point| (point.v - candidate.v).length() <= EPA_EPSILON)
        {
            return Err(epa_failure(
                EpaTerminationReason::DegenerateEdge,
                iteration,
                &gjk,
            ));
        }
        polytope.insert(edge.insert_index, candidate);
    }

    Err(epa_failure(
        EpaTerminationReason::MaxIterations,
        EPA_MAX_ITERATIONS,
        &gjk,
    ))
}

pub(crate) fn generic_convex_contact(
    shape_a: &SharedShape,
    pose_a: Pose,
    shape_b: &SharedShape,
    pose_b: Pose,
) -> Option<GenericConvexContact> {
    match epa_penetration(shape_a, pose_a, shape_b, pose_b) {
        Ok(penetration) => Some(GenericConvexContact {
            normal: penetration.normal,
            depth: penetration.depth,
            point: penetration.contact_point,
            trace: GenericConvexTrace {
                fallback_reason: GenericConvexFallbackReason::GenericConvexFallback,
                gjk_termination: penetration.gjk.termination,
                epa_termination: penetration.termination,
                gjk_iterations: penetration.gjk.iterations,
                epa_iterations: penetration.iterations,
                simplex_len: penetration.gjk.simplex_len,
            },
        }),
        Err(failure) if failure.gjk_termination == GjkTerminationReason::Intersect => {
            contained_epa_contact(shape_a, pose_a, shape_b, pose_b, failure)
        }
        Err(_) => None,
    }
}

fn contained_epa_contact(
    shape_a: &SharedShape,
    pose_a: Pose,
    shape_b: &SharedShape,
    pose_b: Pose,
    failure: EpaFailure,
) -> Option<GenericConvexContact> {
    let normal = contained_fallback_normal(pose_a, pose_b);
    let point_a = shape_a.support_point(pose_a, -normal)?;
    let point_b = shape_b.support_point(pose_b, normal)?;
    let point = Point::from((Vector::from(point_a) + Vector::from(point_b)) * 0.5);
    let depth = (point_a - point_b).dot(normal).max(0.0);
    (normal.x().is_finite()
        && normal.y().is_finite()
        && point.x().is_finite()
        && point.y().is_finite()
        && depth.is_finite())
    .then_some(GenericConvexContact {
        normal,
        depth,
        point,
        trace: GenericConvexTrace {
            fallback_reason: GenericConvexFallbackReason::EpaFailureContained,
            gjk_termination: failure.gjk_termination,
            epa_termination: failure.termination,
            gjk_iterations: failure.gjk_iterations,
            epa_iterations: failure.iterations,
            simplex_len: failure.simplex_len,
        },
    })
}

fn contained_fallback_normal(pose_a: Pose, pose_b: Pose) -> Vector {
    let center_delta = pose_a.point() - pose_b.point();
    if center_delta.length() > GJK_EPSILON
        && center_delta.x().is_finite()
        && center_delta.y().is_finite()
    {
        center_delta.normalized_or_zero()
    } else {
        Vector::new(-1.0, 0.0)
    }
}

fn support(
    shape_a: &SharedShape,
    pose_a: Pose,
    shape_b: &SharedShape,
    pose_b: Pose,
    direction: Vector,
) -> Option<SupportPoint> {
    let point_a = shape_a.support_point(pose_a, direction)?;
    let point_b = shape_b.support_point(pose_b, -direction)?;
    let v = point_a - point_b;
    (v.x().is_finite() && v.y().is_finite()).then_some(SupportPoint {
        v,
        point_a,
        point_b,
    })
}

fn initial_direction(pose_a: Pose, pose_b: Pose) -> Vector {
    let direction = pose_a.point() - pose_b.point();
    if direction.length() <= GJK_EPSILON {
        Vector::new(1.0, 0.0)
    } else {
        direction
    }
}

fn update_simplex(simplex: &mut Vec<SupportPoint>, direction: &mut Vector) -> bool {
    match simplex.len() {
        2 => update_line_simplex(simplex, direction),
        3 => update_triangle_simplex(simplex, direction),
        _ => {
            *direction = -simplex.last().map(|point| point.v).unwrap_or_default();
            false
        }
    }
}

fn update_line_simplex(simplex: &mut Vec<SupportPoint>, direction: &mut Vector) -> bool {
    let a = simplex[1].v;
    let b = simplex[0].v;
    let ab = b - a;
    let ao = -a;
    if ab.dot(ao) > 0.0 {
        *direction = triple_product(ab, ao, ab);
        if direction.length() <= GJK_EPSILON {
            *direction = perpendicular_toward(ab, ao);
        }
    } else {
        simplex.remove(0);
        *direction = ao;
    }
    false
}

fn update_triangle_simplex(simplex: &mut Vec<SupportPoint>, direction: &mut Vector) -> bool {
    let a = simplex[2].v;
    let b = simplex[1].v;
    let c = simplex[0].v;
    let ao = -a;
    let ab = b - a;
    let ac = c - a;
    // The triple products select the edge normal that points away from the
    // opposite triangle vertex. If the origin is outside that edge we shrink
    // the simplex; otherwise the triangle contains the origin.
    let ab_perp = triple_product(ac, ab, ab);
    if ab_perp.dot(ao) > 0.0 {
        simplex.remove(0);
        *direction = ab_perp;
        return false;
    }
    let ac_perp = triple_product(ab, ac, ac);
    if ac_perp.dot(ao) > 0.0 {
        simplex.remove(1);
        *direction = ac_perp;
        return false;
    }
    true
}

fn perpendicular_toward(edge: Vector, toward: Vector) -> Vector {
    let mut normal = edge.perp();
    if normal.dot(toward) < 0.0 {
        normal = -normal;
    }
    normal.normalized_or_zero()
}

fn triple_product(a: Vector, b: Vector, c: Vector) -> Vector {
    b * a.dot(c) - a * b.dot(c)
}

fn closest_distance_to_origin(simplex: &[SupportPoint]) -> FloatNum {
    match simplex {
        [] => 0.0,
        [point] => point.v.length(),
        _ => {
            let a = simplex[simplex.len() - 2].v;
            let b = simplex[simplex.len() - 1].v;
            closest_point_on_segment_to_origin(a, b).length()
        }
    }
}

fn closest_point_on_segment_to_origin(a: Vector, b: Vector) -> Vector {
    let ab = b - a;
    let denom = ab.length_squared();
    if denom <= GJK_EPSILON {
        return a;
    }
    let t = (-a).dot(ab) / denom;
    a + ab * t.clamp(0.0, 1.0)
}

#[derive(Clone, Copy, Debug)]
struct ClosestEdge {
    a: SupportPoint,
    normal: Vector,
    distance: FloatNum,
    insert_index: usize,
}

fn closest_edge(polytope: &[SupportPoint]) -> Option<ClosestEdge> {
    let mut best = None;
    for index in 0..polytope.len() {
        let next = (index + 1) % polytope.len();
        let a = polytope[index];
        let b = polytope[next];
        let edge = b.v - a.v;
        if edge.length() <= EPA_EPSILON {
            return None;
        }
        let mut normal = edge.perp().normalized_or_zero();
        let mut distance = normal.dot(a.v);
        if distance < 0.0 {
            normal = -normal;
            distance = -distance;
        }
        let candidate = ClosestEdge {
            a,
            normal,
            distance,
            insert_index: next,
        };
        if best
            .as_ref()
            .is_none_or(|current| edge_order(candidate, *current) == Ordering::Less)
        {
            best = Some(candidate);
        }
    }
    best
}

fn edge_order(a: ClosestEdge, b: ClosestEdge) -> Ordering {
    a.distance
        .partial_cmp(&b.distance)
        .unwrap_or(Ordering::Equal)
        .then_with(|| a.insert_index.cmp(&b.insert_index))
}

fn signed_area(polytope: &[SupportPoint]) -> FloatNum {
    let mut area = 0.0;
    for index in 0..polytope.len() {
        let a = polytope[index].v;
        let b = polytope[(index + 1) % polytope.len()].v;
        area += a.cross(b);
    }
    area * 0.5
}

fn gjk_done(
    intersects: bool,
    distance: FloatNum,
    termination: GjkTerminationReason,
    iterations: usize,
    simplex: Vec<SupportPoint>,
) -> GjkResult {
    GjkResult {
        intersects,
        distance: if distance.is_finite() { distance } else { 0.0 },
        termination,
        iterations,
        simplex_len: simplex.len(),
        simplex,
    }
}

fn gjk_failure(
    termination: GjkTerminationReason,
    iterations: usize,
    simplex: Vec<SupportPoint>,
) -> GjkResult {
    gjk_done(false, 0.0, termination, iterations, simplex)
}

fn epa_failure(
    termination: EpaTerminationReason,
    iterations: usize,
    gjk: &GjkResult,
) -> EpaFailure {
    EpaFailure {
        termination,
        iterations,
        gjk_termination: gjk.termination,
        gjk_iterations: gjk.iterations,
        simplex_len: gjk.simplex_len,
    }
}

#[cfg(test)]
mod tests {
    use super::{epa_penetration, generic_convex_contact, gjk_distance};
    use crate::{
        body::Pose,
        collider::SharedShape,
        events::{EpaTerminationReason, GjkTerminationReason},
        math::{point::Point, vector::Vector},
    };

    #[test]
    fn support_mapping_handles_supported_convex_shapes_and_rejects_concave() {
        let circle = SharedShape::circle(1.0);
        let rect = SharedShape::rect(2.0, 1.0);
        let polygon = SharedShape::convex_polygon(vec![
            Point::new(-1.0, -1.0),
            Point::new(2.0, -0.5),
            Point::new(0.5, 1.0),
        ]);
        let segment = SharedShape::segment(Point::new(-2.0, 0.0), Point::new(2.0, 0.0));
        let concave = SharedShape::concave_polygon(vec![
            Point::new(-1.0, -1.0),
            Point::new(1.0, -1.0),
            Point::new(0.0, 0.0),
            Point::new(1.0, 1.0),
            Point::new(-1.0, 1.0),
        ]);

        assert_eq!(
            circle.support_point(Pose::from_xy_angle(2.0, 0.0, 0.0), Vector::new(1.0, 0.0)),
            Some(Point::new(3.0, 0.0))
        );
        assert_eq!(
            rect.support_point(Pose::default(), Vector::new(1.0, 1.0)),
            Some(Point::new(1.0, 0.5))
        );
        assert_eq!(
            polygon.support_point(Pose::default(), Vector::new(1.0, 0.0)),
            Some(Point::new(2.0, -0.5))
        );
        assert_eq!(
            segment.support_point(Pose::default(), Vector::new(-1.0, 0.0)),
            Some(Point::new(-2.0, 0.0))
        );
        assert_eq!(
            concave.support_point(Pose::default(), Vector::new(1.0, 0.0)),
            None
        );
    }

    #[test]
    fn gjk_reports_separated_touching_overlapping_and_degenerate_inputs() {
        let circle = SharedShape::circle(1.0);

        let separated = gjk_distance(
            &circle,
            Pose::default(),
            &circle,
            Pose::from_xy_angle(3.0, 0.0, 0.0),
        );
        assert!(!separated.intersects);
        assert_eq!(separated.termination, GjkTerminationReason::Separated);
        assert!((separated.distance - 1.0).abs() < 1.0e-4);
        assert!(separated.iterations > 0);
        assert!(separated.simplex_len > 0);

        let touching = gjk_distance(
            &circle,
            Pose::default(),
            &circle,
            Pose::from_xy_angle(2.0, 0.0, 0.0),
        );
        assert!(!touching.intersects);
        assert_eq!(touching.termination, GjkTerminationReason::Touching);
        assert_eq!(touching.distance, 0.0);

        let overlapping = gjk_distance(
            &circle,
            Pose::default(),
            &circle,
            Pose::from_xy_angle(1.0, 0.0, 0.0),
        );
        assert!(overlapping.intersects);
        assert_eq!(overlapping.termination, GjkTerminationReason::Intersect);

        let point = SharedShape::circle(0.0);
        let degenerate = gjk_distance(&point, Pose::default(), &point, Pose::default());
        assert!(!degenerate.distance.is_nan());
        assert!(matches!(
            degenerate.termination,
            GjkTerminationReason::Touching | GjkTerminationReason::DegenerateDirection
        ));
    }

    #[test]
    fn epa_returns_penetration_for_overlapping_convex_shapes() {
        let left = SharedShape::rect(2.0, 2.0);
        let right = SharedShape::rect(2.0, 2.0);

        let penetration = epa_penetration(
            &left,
            Pose::default(),
            &right,
            Pose::from_xy_angle(1.5, 0.0, 0.0),
        )
        .expect("overlapping rectangles should produce EPA penetration");

        assert_eq!(penetration.termination, EpaTerminationReason::Converged);
        assert!((penetration.depth - 0.5).abs() < 1.0e-3);
        assert_eq!(penetration.normal, Vector::new(-1.0, 0.0));
        assert!(penetration.iterations > 0);
    }

    #[test]
    fn epa_failure_is_contained_for_ambiguous_degenerate_convex_input() {
        let segment = SharedShape::segment(Point::new(-1.0, 0.0), Point::new(1.0, 0.0));

        let failure = epa_penetration(&segment, Pose::default(), &segment, Pose::default())
            .expect_err("coincident segments have no stable 2D penetration face");

        assert!(matches!(
            failure.termination,
            EpaTerminationReason::GjkDidNotIntersect
                | EpaTerminationReason::DegenerateEdge
                | EpaTerminationReason::InvalidSupport
        ));
        assert!(failure.iterations <= 32);
    }

    #[test]
    fn generic_convex_contact_surfaces_contained_epa_failure_without_nan() {
        let segment = SharedShape::segment(Point::new(-1.0, 0.0), Point::new(1.0, 0.0));

        let contact = generic_convex_contact(&segment, Pose::default(), &segment, Pose::default())
            .expect("intersecting degenerate convex input should surface containment facts");

        assert_eq!(
            contact.trace.fallback_reason,
            crate::events::GenericConvexFallbackReason::EpaFailureContained
        );
        assert_eq!(
            contact.trace.gjk_termination,
            crate::events::GjkTerminationReason::Intersect
        );
        assert!(matches!(
            contact.trace.epa_termination,
            crate::events::EpaTerminationReason::DegenerateEdge
                | crate::events::EpaTerminationReason::InvalidSupport
                | crate::events::EpaTerminationReason::MaxIterations
        ));
        assert!(contact.normal.x().is_finite());
        assert!(contact.normal.y().is_finite());
        assert!(contact.depth.is_finite());
        assert!(contact.point.x().is_finite());
        assert!(contact.point.y().is_finite());
    }
}
