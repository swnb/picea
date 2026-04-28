use std::borrow::Cow;

use std::cmp::Ordering;

use crate::{
    body::Pose,
    collider::{ShapeAabb, SharedShape},
    events::{ContactReductionReason, GenericConvexFallbackReason, GenericConvexTrace},
    handles::ContactFeatureId,
    math::{point::Point, vector::Vector, FloatNum},
    pipeline::gjk,
};

const CLIP_EPSILON: FloatNum = 1.0e-4;
const DUPLICATE_POINT_EPSILON: FloatNum = 1.0e-3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ContactPointGeometry {
    pub(crate) point: Point,
    pub(crate) depth: FloatNum,
    pub(crate) feature_id: ContactFeatureId,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ContactManifoldGeometry {
    pub(crate) normal: Vector,
    pub(crate) depth: FloatNum,
    pub(crate) points: Vec<ContactPointGeometry>,
    pub(crate) reduction_reason: ContactReductionReason,
    pub(crate) generic_convex_trace: Option<GenericConvexTrace>,
}

#[allow(dead_code)]
pub(crate) fn contact_from_shapes(
    shape_a: &SharedShape,
    pose_a: Pose,
    aabb_a: ShapeAabb,
    shape_b: &SharedShape,
    pose_b: Pose,
    aabb_b: ShapeAabb,
) -> Option<ContactManifoldGeometry> {
    contact_from_shapes_with_cached_vertices(
        shape_a, pose_a, aabb_a, None, shape_b, pose_b, aabb_b, None,
    )
}

pub(crate) fn contact_from_shapes_with_cached_vertices(
    shape_a: &SharedShape,
    pose_a: Pose,
    aabb_a: ShapeAabb,
    cached_vertices_a: Option<&[Point]>,
    shape_b: &SharedShape,
    pose_b: Pose,
    aabb_b: ShapeAabb,
    cached_vertices_b: Option<&[Point]>,
) -> Option<ContactManifoldGeometry> {
    match (shape_a, shape_b) {
        (SharedShape::Circle { radius: radius_a }, SharedShape::Circle { radius: radius_b }) => {
            contact_from_circles(pose_a.point(), *radius_a, pose_b.point(), *radius_b)
        }
        (SharedShape::Circle { radius }, SharedShape::Segment { start, end }) => {
            contact_from_circle_segment(
                pose_a.point(),
                *radius,
                pose_b.transform_point(*start),
                pose_b.transform_point(*end),
                false,
            )
        }
        (SharedShape::Segment { start, end }, SharedShape::Circle { radius }) => {
            contact_from_circle_segment(
                pose_b.point(),
                *radius,
                pose_a.transform_point(*start),
                pose_a.transform_point(*end),
                true,
            )
        }
        (SharedShape::Circle { radius }, _)
            if convex_vertices(shape_b, pose_b, cached_vertices_b).is_some() =>
        {
            contact_from_circle_polygon(
                pose_a.point(),
                *radius,
                convex_vertices(shape_b, pose_b, cached_vertices_b)?.as_ref(),
                false,
            )
        }
        (_, SharedShape::Circle { radius })
            if convex_vertices(shape_a, pose_a, cached_vertices_a).is_some() =>
        {
            contact_from_circle_polygon(
                pose_b.point(),
                *radius,
                convex_vertices(shape_a, pose_a, cached_vertices_a)?.as_ref(),
                true,
            )
        }
        _ if should_try_generic_convex(
            shape_a,
            pose_a,
            cached_vertices_a,
            shape_b,
            pose_b,
            cached_vertices_b,
        ) =>
        {
            generic_convex_contact(
                shape_a,
                pose_a,
                cached_vertices_a,
                shape_b,
                pose_b,
                cached_vertices_b,
            )
        }
        _ => {
            let Some(poly_a) = convex_vertices(shape_a, pose_a, cached_vertices_a) else {
                // M2 only owns convex SAT. Concave polygons need decomposition
                // before they can enter the generic convex fallback, so keep the
                // legacy fallback explicit instead of pretending they have convex
                // feature ids.
                return overlap_from_aabbs(aabb_a, aabb_b, ContactReductionReason::NonM2Fallback);
            };
            let Some(poly_b) = convex_vertices(shape_b, pose_b, cached_vertices_b) else {
                return overlap_from_aabbs(aabb_a, aabb_b, ContactReductionReason::NonM2Fallback);
            };
            contact_from_convex_polygons(poly_a.as_ref(), poly_b.as_ref())
        }
    }
}

fn contact_from_circles(
    center_a: Point,
    radius_a: FloatNum,
    center_b: Point,
    radius_b: FloatNum,
) -> Option<ContactManifoldGeometry> {
    let offset_to_a = center_a - center_b;
    let distance = offset_to_a.length();
    let radius_sum = radius_a + radius_b;
    let depth = radius_sum - distance;
    if depth <= 0.0 {
        return None;
    }

    let normal = if distance <= FloatNum::EPSILON {
        Vector::new(-1.0, 0.0)
    } else {
        offset_to_a / distance
    };
    let point_on_a = center_a - normal * radius_a;
    let point_on_b = center_b + normal * radius_b;
    let point = Point::from((Vector::from(point_on_a) + Vector::from(point_on_b)) * 0.5);

    Some(single_point_manifold(
        point,
        normal,
        depth,
        feature_id(0, 0, 0, 0),
        ContactReductionReason::SinglePoint,
    ))
}

pub(crate) fn overlap_from_aabbs(
    a: ShapeAabb,
    b: ShapeAabb,
    reduction_reason: ContactReductionReason,
) -> Option<ContactManifoldGeometry> {
    let overlap_x = a.max.x().min(b.max.x()) - a.min.x().max(b.min.x());
    let overlap_y = a.max.y().min(b.max.y()) - a.min.y().max(b.min.y());
    if overlap_x <= 0.0 || overlap_y <= 0.0 {
        return None;
    }

    let point = Point::new(
        (a.min.x().max(b.min.x()) + a.max.x().min(b.max.x())) * 0.5,
        (a.min.y().max(b.min.y()) + a.max.y().min(b.max.y())) * 0.5,
    );
    let center_a = Point::new((a.min.x() + a.max.x()) * 0.5, (a.min.y() + a.max.y()) * 0.5);
    let center_b = Point::new((b.min.x() + b.max.x()) * 0.5, (b.min.y() + b.max.y()) * 0.5);
    let delta = center_a - center_b;

    if overlap_x <= overlap_y {
        let normal = if delta.x() <= 0.0 {
            Vector::new(-1.0, 0.0)
        } else {
            Vector::new(1.0, 0.0)
        };
        Some(single_point_manifold(
            point,
            normal,
            overlap_x,
            feature_id(15, 0, 0, 0),
            reduction_reason,
        ))
    } else {
        let normal = if delta.y() <= 0.0 {
            Vector::new(0.0, -1.0)
        } else {
            Vector::new(0.0, 1.0)
        };
        Some(single_point_manifold(
            point,
            normal,
            overlap_y,
            feature_id(15, 1, 0, 0),
            reduction_reason,
        ))
    }
}

fn contact_from_convex_polygons(a: &[Point], b: &[Point]) -> Option<ContactManifoldGeometry> {
    let sat = sat_minimum_axis(a, b)?;
    let (reference, incident, reference_edge, reference_to_incident, output_normal) =
        if sat.source == PolySource::B {
            (
                b,
                a,
                best_edge_for_normal(b, sat.normal),
                sat.normal,
                sat.normal,
            )
        } else {
            (
                a,
                b,
                best_edge_for_normal(a, -sat.normal),
                -sat.normal,
                sat.normal,
            )
        };
    let incident_edge = incident_edge_for_normal(incident, reference_to_incident);

    let mut clipped = clip_incident_edge(
        incident,
        incident_edge,
        reference,
        reference_edge,
        reference_to_incident,
        output_normal,
        sat.depth,
    );
    if clipped.is_empty() {
        clipped.push(ContactPointGeometry {
            point: support_midpoint(a, b, output_normal),
            depth: sat.depth,
            feature_id: feature_id(3, reference_edge, incident_edge, 0),
        });
    }

    let (points, reduction_reason) = reduce_contact_points(clipped);
    Some(ContactManifoldGeometry {
        normal: output_normal.normalized_or_zero(),
        depth: sat.depth,
        points,
        reduction_reason,
        generic_convex_trace: None,
    })
}

fn contact_from_circle_polygon(
    circle_center: Point,
    radius: FloatNum,
    polygon: &[Point],
    flip_normal: bool,
) -> Option<ContactManifoldGeometry> {
    let mut best = None::<ClosestSegmentPoint>;
    let mut inside = true;
    let center = polygon_center(polygon);
    for edge_index in 0..polygon.len() {
        let start = polygon[edge_index];
        let end = polygon[(edge_index + 1) % polygon.len()];
        let outward = edge_outward_normal(polygon, edge_index);
        if (circle_center - start).dot(outward) > CLIP_EPSILON {
            inside = false;
        }
        let closest = closest_point_on_segment(circle_center, start, end, edge_index);
        if best
            .as_ref()
            .is_none_or(|current| closest.distance_squared < current.distance_squared)
        {
            best = Some(closest);
        }
    }

    let best = best?;
    let (mut normal, depth, point) = if inside {
        let edge_index = best.edge_index;
        let outward = edge_outward_normal(polygon, edge_index);
        let face_distance = (circle_center - polygon[edge_index]).dot(outward).abs();
        (
            outward,
            radius + face_distance,
            circle_center - outward * radius,
        )
    } else {
        let offset = circle_center - best.point;
        let distance = offset.length();
        let depth = radius - distance;
        if depth <= 0.0 {
            return None;
        }
        let normal = if distance <= FloatNum::EPSILON {
            (circle_center - center).normalized_or_zero()
        } else {
            offset / distance
        };
        (normal, depth, best.point)
    };
    if normal.length() <= FloatNum::EPSILON {
        normal = Vector::new(-1.0, 0.0);
    }
    normal = stabilize_normal(normal);
    if flip_normal {
        normal = -normal;
    }

    Some(single_point_manifold(
        point,
        normal,
        depth,
        feature_id(4, best.edge_index, 0, 0),
        ContactReductionReason::SinglePoint,
    ))
}

fn contact_from_circle_segment(
    circle_center: Point,
    radius: FloatNum,
    segment_start: Point,
    segment_end: Point,
    flip_normal: bool,
) -> Option<ContactManifoldGeometry> {
    let closest = closest_point_on_segment(circle_center, segment_start, segment_end, 0);
    let offset = circle_center - closest.point;
    let distance = offset.length();
    let depth = radius - distance;
    if depth <= 0.0 {
        return None;
    }
    let mut normal = if distance <= FloatNum::EPSILON {
        let segment = segment_end - segment_start;
        segment.perp().normalized_or_zero()
    } else {
        offset / distance
    };
    if normal.length() <= FloatNum::EPSILON {
        normal = Vector::new(-1.0, 0.0);
    }
    normal = stabilize_normal(normal);
    if flip_normal {
        normal = -normal;
    }
    Some(single_point_manifold(
        closest.point,
        normal,
        depth,
        feature_id(5, 0, (closest.t * 65535.0) as usize, 0),
        ContactReductionReason::SinglePoint,
    ))
}

fn single_point_manifold(
    point: Point,
    normal: Vector,
    depth: FloatNum,
    feature_id: ContactFeatureId,
    reduction_reason: ContactReductionReason,
) -> ContactManifoldGeometry {
    ContactManifoldGeometry {
        normal,
        depth,
        points: vec![ContactPointGeometry {
            point,
            depth,
            feature_id,
        }],
        reduction_reason,
        generic_convex_trace: None,
    }
}

fn should_try_generic_convex(
    shape_a: &SharedShape,
    pose_a: Pose,
    cached_vertices_a: Option<&[Point]>,
    shape_b: &SharedShape,
    pose_b: Pose,
    cached_vertices_b: Option<&[Point]>,
) -> bool {
    // SAT/clipping remains primary for polygonal area shapes because it
    // produces stable 1-2 point manifolds with feature ids. GJK/EPA is only the
    // fallback for convex support-mapped pairs that do not have a better local
    // narrowphase path, such as segment-vs-polygon.
    let sat_ready = convex_vertices(shape_a, pose_a, cached_vertices_a).is_some()
        && convex_vertices(shape_b, pose_b, cached_vertices_b).is_some();
    !sat_ready
        && shape_a
            .support_point_with_cached_vertices(pose_a, Vector::new(1.0, 0.0), cached_vertices_a)
            .is_some()
        && shape_b
            .support_point_with_cached_vertices(pose_b, Vector::new(1.0, 0.0), cached_vertices_b)
            .is_some()
}

fn generic_convex_contact(
    shape_a: &SharedShape,
    pose_a: Pose,
    cached_vertices_a: Option<&[Point]>,
    shape_b: &SharedShape,
    pose_b: Pose,
    cached_vertices_b: Option<&[Point]>,
) -> Option<ContactManifoldGeometry> {
    let contact = gjk::generic_convex_contact_with_cached_vertices(
        shape_a,
        pose_a,
        cached_vertices_a,
        shape_b,
        pose_b,
        cached_vertices_b,
    )?;
    Some(ContactManifoldGeometry {
        normal: contact.normal,
        depth: contact.depth,
        points: vec![ContactPointGeometry {
            point: contact.point,
            depth: contact.depth,
            feature_id: generic_convex_feature_id(shape_a, shape_b, contact.trace),
        }],
        reduction_reason: ContactReductionReason::GenericConvexFallback,
        generic_convex_trace: Some(contact.trace),
    })
}

fn generic_convex_feature_id(
    shape_a: &SharedShape,
    shape_b: &SharedShape,
    trace: GenericConvexTrace,
) -> ContactFeatureId {
    let kind = match trace.fallback_reason {
        GenericConvexFallbackReason::EpaFailureContained => 7,
        _ => 6,
    };
    feature_id(
        kind,
        shape_feature_kind(shape_a),
        shape_feature_kind(shape_b),
        0,
    )
}

fn shape_feature_kind(shape: &SharedShape) -> usize {
    match shape {
        SharedShape::Circle { .. } => 0,
        SharedShape::Rect { .. } => 1,
        SharedShape::RegularPolygon { .. } => 2,
        SharedShape::ConvexPolygon { .. } => 3,
        SharedShape::ConcavePolygon { .. } => 4,
        SharedShape::Segment { .. } => 5,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PolySource {
    A,
    B,
}

#[derive(Clone, Copy, Debug)]
struct SatAxis {
    normal: Vector,
    depth: FloatNum,
    source: PolySource,
    edge_index: usize,
}

fn sat_minimum_axis(a: &[Point], b: &[Point]) -> Option<SatAxis> {
    let center_delta = polygon_center(a) - polygon_center(b);
    let mut best = None::<SatAxis>;
    for (source, polygon) in [(PolySource::A, a), (PolySource::B, b)] {
        for edge_index in 0..polygon.len() {
            let mut axis = edge_outward_normal(polygon, edge_index);
            if axis.length() <= FloatNum::EPSILON {
                continue;
            }
            if axis.dot(center_delta) < 0.0 {
                axis = -axis;
            }
            let (min_a, max_a) = project_polygon(a, axis);
            let (min_b, max_b) = project_polygon(b, axis);
            let depth = max_a.min(max_b) - min_a.max(min_b);
            if depth <= CLIP_EPSILON {
                return None;
            }
            let candidate = SatAxis {
                normal: axis,
                depth,
                source,
                edge_index,
            };
            if best
                .as_ref()
                .is_none_or(|current| sat_axis_order(candidate, *current) == Ordering::Less)
            {
                best = Some(candidate);
            }
        }
    }
    best
}

fn sat_axis_order(a: SatAxis, b: SatAxis) -> Ordering {
    if (a.depth - b.depth).abs() > CLIP_EPSILON {
        return a.depth.partial_cmp(&b.depth).unwrap_or(Ordering::Equal);
    }
    let source_order = |source| match source {
        PolySource::A => 0,
        PolySource::B => 1,
    };
    (
        source_order(a.source),
        a.edge_index,
        quantize_axis(a.normal).0,
        quantize_axis(a.normal).1,
    )
        .cmp(&(
            source_order(b.source),
            b.edge_index,
            quantize_axis(b.normal).0,
            quantize_axis(b.normal).1,
        ))
}

fn project_polygon(points: &[Point], axis: Vector) -> (FloatNum, FloatNum) {
    let mut min = FloatNum::INFINITY;
    let mut max = FloatNum::NEG_INFINITY;
    for point in points {
        let projected = Vector::from(*point).dot(axis);
        min = min.min(projected);
        max = max.max(projected);
    }
    (min, max)
}

fn clip_incident_edge(
    incident: &[Point],
    incident_edge: usize,
    reference: &[Point],
    reference_edge: usize,
    reference_to_incident: Vector,
    output_normal: Vector,
    manifold_depth: FloatNum,
) -> Vec<ContactPointGeometry> {
    let ref_start = reference[reference_edge];
    let ref_end = reference[(reference_edge + 1) % reference.len()];
    let side = (ref_end - ref_start).normalized_or_zero();
    if side.length() <= FloatNum::EPSILON {
        return Vec::new();
    }

    let start = incident[incident_edge];
    let end = incident[(incident_edge + 1) % incident.len()];
    let points = vec![
        ClippedPoint {
            point: start,
            feature_id: feature_id(1, reference_edge, incident_edge, 0),
        },
        ClippedPoint {
            point: end,
            feature_id: feature_id(1, reference_edge, incident_edge, 1),
        },
    ];

    let points = clip_to_plane(points, ref_start, side);
    let points = clip_to_plane(points, ref_end, -side);
    points
        .into_iter()
        .filter_map(|clipped| {
            let separation = (clipped.point - ref_start).dot(reference_to_incident);
            let depth = (-separation).max(0.0).min(manifold_depth);
            (separation <= CLIP_EPSILON).then_some(ContactPointGeometry {
                // Box2D stores clipped incident features, then builds a world
                // contact point between both surfaces. Keep the exported point
                // on that mid-plane so solver anchors are not biased outside
                // the overlapping faces.
                point: clipped.point + output_normal * (separation.min(0.0) * 0.5),
                depth,
                feature_id: clipped.feature_id,
            })
        })
        .collect()
}

#[derive(Clone, Copy, Debug)]
struct ClippedPoint {
    point: Point,
    feature_id: ContactFeatureId,
}

fn clip_to_plane(
    points: Vec<ClippedPoint>,
    plane_point: Point,
    plane_normal: Vector,
) -> Vec<ClippedPoint> {
    if points.len() < 2 {
        return points;
    }
    let mut output = Vec::new();
    let mut previous = points[points.len() - 1];
    let mut previous_distance = (previous.point - plane_point).dot(plane_normal);
    for current in points {
        let current_distance = (current.point - plane_point).dot(plane_normal);
        if current_distance >= -CLIP_EPSILON {
            if previous_distance < -CLIP_EPSILON {
                output.push(interpolate_clip(
                    previous,
                    current,
                    previous_distance,
                    current_distance,
                ));
            }
            output.push(current);
        } else if previous_distance >= -CLIP_EPSILON {
            output.push(interpolate_clip(
                previous,
                current,
                previous_distance,
                current_distance,
            ));
        }
        previous = current;
        previous_distance = current_distance;
    }
    output
}

fn interpolate_clip(
    a: ClippedPoint,
    b: ClippedPoint,
    distance_a: FloatNum,
    distance_b: FloatNum,
) -> ClippedPoint {
    let denominator = distance_a - distance_b;
    let t = if denominator.abs() <= FloatNum::EPSILON {
        0.0
    } else {
        distance_a / denominator
    }
    .clamp(0.0, 1.0);
    let point = a.point + (b.point - a.point) * t;
    ClippedPoint {
        point,
        feature_id: min_feature_id(a.feature_id, b.feature_id),
    }
}

fn reduce_contact_points(
    mut points: Vec<ContactPointGeometry>,
) -> (Vec<ContactPointGeometry>, ContactReductionReason) {
    points.sort_by(|a, b| compare_points(a.point, b.point).then(a.feature_id.cmp(&b.feature_id)));
    let mut reduced: Vec<ContactPointGeometry> = Vec::new();
    let mut merged_duplicate = false;
    for point in points {
        if reduced
            .iter()
            .any(|existing| (existing.point - point.point).length() <= DUPLICATE_POINT_EPSILON)
        {
            merged_duplicate = true;
            // Keep the deepest representative for correction while choosing
            // the smallest feature id as the stable identity for warm-start
            // style consumers in later milestones.
            if let Some(existing) = reduced
                .iter_mut()
                .find(|existing| (existing.point - point.point).length() <= DUPLICATE_POINT_EPSILON)
            {
                existing.depth = existing.depth.max(point.depth);
                existing.feature_id = min_feature_id(existing.feature_id, point.feature_id);
            }
            continue;
        }
        reduced.push(point);
    }
    if reduced.len() > 2 {
        reduced.sort_by(|a, b| compare_points(a.point, b.point));
        reduced = vec![
            reduced[0],
            *reduced.last().expect("reduced has at least one point"),
        ];
    }
    let reason = if merged_duplicate {
        ContactReductionReason::DuplicateReduced
    } else if reduced.len() == 1 {
        ContactReductionReason::SinglePoint
    } else {
        ContactReductionReason::Clipped
    };
    (reduced, reason)
}

fn convex_vertices<'a>(
    shape: &SharedShape,
    pose: Pose,
    cached_vertices: Option<&'a [Point]>,
) -> Option<Cow<'a, [Point]>> {
    match shape {
        SharedShape::Rect { .. }
        | SharedShape::RegularPolygon { .. }
        | SharedShape::ConvexPolygon { .. } => Some(match cached_vertices {
            Some(vertices) => Cow::Borrowed(vertices),
            None => Cow::Owned(shape.world_vertices(pose)),
        }),
        _ => None,
    }
}

fn edge_outward_normal(vertices: &[Point], edge_index: usize) -> Vector {
    let start = vertices[edge_index];
    let end = vertices[(edge_index + 1) % vertices.len()];
    let mut normal = (end - start).perp().normalized_or_zero();
    if polygon_area(vertices) < 0.0 {
        normal = -normal;
    }
    normal
}

fn best_edge_for_normal(vertices: &[Point], normal: Vector) -> usize {
    (0..vertices.len())
        .max_by(|lhs, rhs| {
            let lhs_dot = edge_outward_normal(vertices, *lhs).dot(normal);
            let rhs_dot = edge_outward_normal(vertices, *rhs).dot(normal);
            lhs_dot
                .partial_cmp(&rhs_dot)
                .unwrap_or(Ordering::Equal)
                .then_with(|| rhs.cmp(lhs))
        })
        .unwrap_or(0)
}

fn incident_edge_for_normal(vertices: &[Point], reference_to_incident: Vector) -> usize {
    (0..vertices.len())
        .min_by(|lhs, rhs| {
            let lhs_dot = edge_outward_normal(vertices, *lhs).dot(reference_to_incident);
            let rhs_dot = edge_outward_normal(vertices, *rhs).dot(reference_to_incident);
            lhs_dot
                .partial_cmp(&rhs_dot)
                .unwrap_or(Ordering::Equal)
                .then_with(|| lhs.cmp(rhs))
        })
        .unwrap_or(0)
}

fn support_midpoint(a: &[Point], b: &[Point], normal: Vector) -> Point {
    let support_a = a
        .iter()
        .copied()
        .min_by(|lhs, rhs| {
            Vector::from(*lhs)
                .dot(normal)
                .partial_cmp(&Vector::from(*rhs).dot(normal))
                .unwrap_or(Ordering::Equal)
        })
        .unwrap_or_default();
    let support_b = b
        .iter()
        .copied()
        .max_by(|lhs, rhs| {
            Vector::from(*lhs)
                .dot(normal)
                .partial_cmp(&Vector::from(*rhs).dot(normal))
                .unwrap_or(Ordering::Equal)
        })
        .unwrap_or_default();
    Point::from((Vector::from(support_a) + Vector::from(support_b)) * 0.5)
}

fn polygon_area(vertices: &[Point]) -> FloatNum {
    let mut area = 0.0;
    for index in 0..vertices.len() {
        let current = vertices[index];
        let next = vertices[(index + 1) % vertices.len()];
        area += current.x() * next.y() - next.x() * current.y();
    }
    area * 0.5
}

fn polygon_center(vertices: &[Point]) -> Point {
    if vertices.is_empty() {
        return Point::default();
    }
    let sum = vertices
        .iter()
        .fold(Vector::default(), |acc, point| acc + Vector::from(*point));
    Point::from(sum / vertices.len() as FloatNum)
}

#[derive(Clone, Copy, Debug)]
struct ClosestSegmentPoint {
    point: Point,
    distance_squared: FloatNum,
    edge_index: usize,
    t: FloatNum,
}

fn closest_point_on_segment(
    point: Point,
    start: Point,
    end: Point,
    edge_index: usize,
) -> ClosestSegmentPoint {
    let segment = end - start;
    let length_squared = segment.length_squared();
    let t = if length_squared <= FloatNum::EPSILON {
        0.0
    } else {
        (point - start).dot(segment) / length_squared
    }
    .clamp(0.0, 1.0);
    let closest = start + segment * t;
    ClosestSegmentPoint {
        point: closest,
        distance_squared: (point - closest).length_squared(),
        edge_index,
        t,
    }
}

fn feature_id(
    kind: usize,
    reference_edge: usize,
    incident_edge: usize,
    point_slot: usize,
) -> ContactFeatureId {
    ContactFeatureId::from_raw_parts(
        ((kind as u32) << 24) | ((reference_edge as u32) << 12) | incident_edge as u32,
        point_slot as u32,
    )
}

fn min_feature_id(a: ContactFeatureId, b: ContactFeatureId) -> ContactFeatureId {
    if a <= b {
        a
    } else {
        b
    }
}

fn compare_points(a: Point, b: Point) -> Ordering {
    quantize(a.x())
        .cmp(&quantize(b.x()))
        .then(quantize(a.y()).cmp(&quantize(b.y())))
}

fn quantize(value: FloatNum) -> i32 {
    (value * 10000.0).round() as i32
}

fn quantize_axis(axis: Vector) -> (i32, i32) {
    (quantize(axis.x()), quantize(axis.y()))
}

fn stabilize_normal(normal: Vector) -> Vector {
    let snapped = Vector::new(
        if normal.x().abs() <= CLIP_EPSILON {
            0.0
        } else {
            normal.x()
        },
        if normal.y().abs() <= CLIP_EPSILON {
            0.0
        } else {
            normal.y()
        },
    );
    snapped.normalized_or_zero()
}

#[cfg(test)]
mod tests {
    use super::{
        contact_from_shapes, contact_from_shapes_with_cached_vertices, feature_id,
        overlap_from_aabbs, reduce_contact_points, ContactPointGeometry,
    };
    use crate::{
        body::Pose,
        collider::{ShapeAabb, SharedShape},
        events::ContactReductionReason,
        math::{point::Point, vector::Vector},
    };

    fn aabb(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> ShapeAabb {
        ShapeAabb {
            min: Point::new(min_x, min_y),
            max: Point::new(max_x, max_y),
        }
    }

    #[test]
    fn narrowphase_rejects_separated_circles_with_overlapping_aabbs() {
        let shape = SharedShape::circle(1.0);
        let contact = contact_from_shapes(
            &shape,
            Pose::from_xy_angle(0.0, 0.0, 0.0),
            aabb(-1.0, -1.0, 1.0, 1.0),
            &shape,
            Pose::from_xy_angle(1.5, 1.5, 0.0),
            aabb(0.5, 0.5, 2.5, 2.5),
        );

        assert_eq!(contact, None);
    }

    #[test]
    fn narrowphase_reports_circle_contact_toward_a() {
        let shape = SharedShape::circle(1.0);
        let contact = contact_from_shapes(
            &shape,
            Pose::from_xy_angle(0.0, 0.0, 0.0),
            aabb(-1.0, -1.0, 1.0, 1.0),
            &shape,
            Pose::from_xy_angle(1.5, 0.0, 0.0),
            aabb(0.5, -1.0, 2.5, 1.0),
        )
        .expect("overlapping circles should contact");

        assert_eq!(contact.normal, Vector::new(-1.0, 0.0));
        assert!((contact.depth - 0.5).abs() < f32::EPSILON);
        assert_eq!(contact.points.len(), 1);
        assert!((contact.points[0].point.x() - 0.75).abs() < f32::EPSILON);
        assert!((contact.points[0].point.y() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn coincident_circle_centers_use_deterministic_aabb_tie_normal() {
        let shape = SharedShape::circle(1.0);
        let contact = contact_from_shapes(
            &shape,
            Pose::from_xy_angle(2.0, 3.0, 0.0),
            aabb(1.0, 2.0, 3.0, 4.0),
            &shape,
            Pose::from_xy_angle(2.0, 3.0, 0.0),
            aabb(1.0, 2.0, 3.0, 4.0),
        )
        .expect("coincident circles should contact");

        assert_eq!(contact.normal, Vector::new(-1.0, 0.0));
        assert_eq!(contact.depth, 2.0);
        assert!(contact.points[0].point.x().is_finite());
        assert!(contact.points[0].point.y().is_finite());
    }

    #[test]
    fn concave_pairs_keep_explicit_non_m2_fallback() {
        let concave = SharedShape::concave_polygon(vec![
            Point::new(-1.0, -1.0),
            Point::new(1.0, -1.0),
            Point::new(0.0, 0.0),
            Point::new(1.0, 1.0),
            Point::new(-1.0, 1.0),
        ]);
        let rect = SharedShape::rect(2.0, 2.0);
        let fallback = overlap_from_aabbs(
            aabb(-1.0, -1.0, 1.0, 1.0),
            aabb(0.5, -1.0, 2.5, 1.0),
            ContactReductionReason::NonM2Fallback,
        );
        let contact = contact_from_shapes(
            &concave,
            Pose::default(),
            aabb(-1.0, -1.0, 1.0, 1.0),
            &rect,
            Pose::from_xy_angle(1.5, 0.0, 0.0),
            aabb(0.5, -1.0, 2.5, 1.0),
        );

        assert_eq!(contact, fallback);
        assert_eq!(
            contact.unwrap().reduction_reason,
            ContactReductionReason::NonM2Fallback
        );
    }

    #[test]
    fn convex_sat_produces_two_point_manifold_with_stable_feature_ids() {
        let shape_a = SharedShape::rect(2.0, 2.0);
        let shape_b = SharedShape::convex_polygon(vec![
            Point::new(-1.0, -1.0),
            Point::new(1.0, -1.0),
            Point::new(1.0, 1.0),
            Point::new(-1.0, 1.0),
        ]);
        let first = contact_from_shapes(
            &shape_a,
            Pose::from_xy_angle(0.0, 0.0, 0.0),
            aabb(-1.0, -1.0, 1.0, 1.0),
            &shape_b,
            Pose::from_xy_angle(1.5, 0.0, 0.0),
            aabb(0.5, -1.0, 2.5, 1.0),
        )
        .expect("convex overlap should contact through SAT");
        let nudged = contact_from_shapes(
            &shape_a,
            Pose::from_xy_angle(0.0, 0.0, 0.0),
            aabb(-1.0, -1.0, 1.0, 1.0),
            &shape_b,
            Pose::from_xy_angle(1.5001, 0.0, 0.0),
            aabb(0.5001, -1.0, 2.5001, 1.0),
        )
        .expect("small movement should keep the same contact features");

        assert_eq!(first.normal, Vector::new(-1.0, 0.0));
        assert!((first.depth - 0.5).abs() < 1.0e-4);
        assert_eq!(first.points.len(), 2);
        assert_eq!(first.reduction_reason, ContactReductionReason::Clipped);
        assert_eq!(
            first
                .points
                .iter()
                .map(|point| point.feature_id)
                .collect::<Vec<_>>(),
            nudged
                .points
                .iter()
                .map(|point| point.feature_id)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn polygon_clipping_places_points_between_reference_and_incident_faces() {
        let shape_a = SharedShape::rect(2.0, 2.0);
        let shape_b = SharedShape::rect(2.0, 2.0);
        let contact = contact_from_shapes(
            &shape_a,
            Pose::default(),
            aabb(-1.0, -1.0, 1.0, 1.0),
            &shape_b,
            Pose::from_xy_angle(1.5, 0.0, 0.0),
            aabb(0.5, -1.0, 2.5, 1.0),
        )
        .expect("overlapping rectangles should produce a clipped manifold");

        assert_eq!(contact.normal, Vector::new(-1.0, 0.0));
        assert_eq!(contact.points.len(), 2);
        for point in &contact.points {
            assert!((point.point.x() - 0.75).abs() < 1.0e-4);
            assert!((point.depth - 0.5).abs() < 1.0e-4);
        }
    }

    #[test]
    fn rotated_rectangles_reject_aabb_only_false_positive() {
        let shape = SharedShape::rect(2.0, 0.25);
        let contact = contact_from_shapes(
            &shape,
            Pose::from_xy_angle(0.0, 0.0, 0.75),
            aabb(-1.0, -1.0, 1.0, 1.0),
            &shape,
            Pose::from_xy_angle(0.0, 1.75, -0.75),
            aabb(-1.0, 0.75, 1.0, 2.75),
        );

        assert_eq!(contact, None);
    }

    #[test]
    fn near_duplicate_contact_reduction_keeps_deepest_feature_stable_point() {
        let (points, reason) = reduce_contact_points(vec![
            ContactPointGeometry {
                point: Point::new(1.0, 2.0),
                depth: 0.125,
                feature_id: feature_id(1, 2, 3, 1),
            },
            ContactPointGeometry {
                point: Point::new(1.0005, 2.0005),
                depth: 0.25,
                feature_id: feature_id(1, 2, 3, 0),
            },
        ]);

        assert_eq!(reason, ContactReductionReason::DuplicateReduced);
        assert_eq!(points.len(), 1);
        assert!((points[0].depth - 0.25).abs() < 1.0e-4);
        assert_eq!(points[0].feature_id, feature_id(1, 2, 3, 0));
    }

    #[test]
    fn circle_segment_uses_analytic_nearest_point() {
        let circle = SharedShape::circle(0.5);
        let segment = SharedShape::segment(Point::new(-1.0, 0.0), Point::new(1.0, 0.0));
        let contact = contact_from_shapes(
            &circle,
            Pose::from_xy_angle(0.25, -0.25, 0.0),
            aabb(-0.25, -0.75, 0.75, 0.25),
            &segment,
            Pose::default(),
            aabb(-1.0, 0.0, 1.0, 0.0),
        )
        .expect("circle should contact segment analytically");

        assert_eq!(contact.points.len(), 1);
        assert_eq!(contact.normal, Vector::new(0.0, -1.0));
        assert!((contact.depth - 0.25).abs() < 1.0e-4);
        assert!((contact.points[0].point.x() - 0.25).abs() < 1.0e-4);
    }

    #[test]
    fn circle_rectangle_uses_analytic_polygon_edge_contact() {
        let rect = SharedShape::rect(10.0, 1.0);
        let circle = SharedShape::circle(0.5);
        let contact = contact_from_shapes(
            &rect,
            Pose::from_xy_angle(0.0, 0.5, 0.0),
            aabb(-5.0, 0.0, 5.0, 1.0),
            &circle,
            Pose::from_xy_angle(0.1, -0.45, 0.0),
            aabb(-0.4, -0.95, 0.6, 0.05),
        )
        .expect("circle should contact rectangle edge");

        assert_eq!(contact.normal, Vector::new(0.0, 1.0));
        assert!((contact.depth - 0.05).abs() < 1.0e-4);
    }

    #[test]
    fn segment_rectangle_uses_generic_convex_fallback_with_trace_facts() {
        let segment = SharedShape::segment(Point::new(-1.0, 0.0), Point::new(1.0, 0.0));
        let rect = SharedShape::rect(1.0, 1.0);
        let contact = contact_from_shapes(
            &segment,
            Pose::default(),
            aabb(-1.0, 0.0, 1.0, 0.0),
            &rect,
            Pose::from_xy_angle(0.25, 0.0, 0.0),
            aabb(-0.25, -0.5, 0.75, 0.5),
        )
        .expect("segment/rectangle overlap should use the generic convex fallback");

        assert_eq!(
            contact.reduction_reason,
            ContactReductionReason::GenericConvexFallback
        );
        let trace = contact
            .generic_convex_trace
            .expect("generic fallback should explain GJK/EPA decisions");
        assert_eq!(
            trace.fallback_reason,
            crate::events::GenericConvexFallbackReason::GenericConvexFallback
        );
        assert_eq!(
            trace.gjk_termination,
            crate::events::GjkTerminationReason::Intersect
        );
        assert_eq!(
            trace.epa_termination,
            crate::events::EpaTerminationReason::Converged
        );
        assert!(trace.gjk_iterations > 0);
        assert!(trace.simplex_len > 0);
        assert_eq!(contact.points[0].feature_id, feature_id(6, 5, 1, 0));
    }

    #[test]
    fn generic_convex_fallback_honors_cached_convex_vertices_for_support_map() {
        let segment = SharedShape::segment(Point::new(-1.0, 0.0), Point::new(1.0, 0.0));
        let rect = SharedShape::rect(1.0, 1.0);
        let cached_rect_vertices = rect.world_vertices(Pose::from_xy_angle(0.25, 0.0, 0.0));

        let contact = contact_from_shapes_with_cached_vertices(
            &segment,
            Pose::default(),
            aabb(-1.0, 0.0, 1.0, 0.0),
            None,
            &rect,
            Pose::from_xy_angle(100.0, 0.0, 0.0),
            aabb(-0.25, -0.5, 0.75, 0.5),
            Some(&cached_rect_vertices),
        )
        .expect("generic fallback should use cached world vertices for support points");

        assert_eq!(
            contact.reduction_reason,
            ContactReductionReason::GenericConvexFallback
        );
        assert!(contact.generic_convex_trace.is_some());
    }

    #[test]
    fn segment_segment_containment_surfaces_epa_failure_trace() {
        let segment = SharedShape::segment(Point::new(-1.0, 0.0), Point::new(1.0, 0.0));
        let contact = contact_from_shapes(
            &segment,
            Pose::default(),
            aabb(-1.0, 0.0, 1.0, 0.0),
            &segment,
            Pose::default(),
            aabb(-1.0, 0.0, 1.0, 0.0),
        )
        .expect("overlapping degenerate convex input should surface contained fallback facts");

        let trace = contact
            .generic_convex_trace
            .expect("contained generic fallback should keep trace facts");
        assert_eq!(
            trace.fallback_reason,
            crate::events::GenericConvexFallbackReason::EpaFailureContained
        );
        assert_eq!(
            trace.gjk_termination,
            crate::events::GjkTerminationReason::Intersect
        );
        assert_eq!(
            trace.epa_termination,
            crate::events::EpaTerminationReason::DegenerateEdge
        );
        assert_eq!(contact.points.len(), 1);
        assert_eq!(contact.points[0].feature_id, feature_id(7, 5, 5, 0));
        assert!(contact.points[0].point.x().is_finite());
        assert!(contact.points[0].point.y().is_finite());
        assert!(contact.points[0].depth.is_finite());
    }

    #[test]
    fn rectangle_and_convex_polygon_keep_sat_clipping_primary() {
        let rect = SharedShape::rect(2.0, 2.0);
        let polygon = SharedShape::convex_polygon(vec![
            Point::new(-1.0, -1.0),
            Point::new(1.0, -1.0),
            Point::new(1.0, 1.0),
            Point::new(-1.0, 1.0),
        ]);
        let contact = contact_from_shapes(
            &rect,
            Pose::default(),
            aabb(-1.0, -1.0, 1.0, 1.0),
            &polygon,
            Pose::from_xy_angle(1.5, 0.0, 0.0),
            aabb(0.5, -1.0, 2.5, 1.0),
        )
        .expect("SAT should still handle rectangle/convex polygon pairs");

        assert_eq!(contact.reduction_reason, ContactReductionReason::Clipped);
        assert_eq!(contact.points.len(), 2);
        assert_eq!(contact.generic_convex_trace, None);
    }
}
