use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
};

use crate::{
    body::{BodyType, Pose},
    collider::{CollisionFilter, ShapeAabb, SharedShape},
    events::{CcdTargetKind, CcdTrace},
    handles::{BodyHandle, ColliderHandle},
    math::{point::Point, vector::Vector, FloatNum},
    world::World,
};

const CCD_TOI_EPSILON: FloatNum = 1.0e-5;
const CCD_CLAMP_SLOP: FloatNum = 1.0e-3;
const CLIP_SUPPORT_EPSILON: FloatNum = 1.0e-4;

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct CcdPoseClampOutcome {
    pub(crate) stats: CcdPoseClampStats,
    pub(crate) traces: Vec<CcdTrace>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CcdPoseClampStats {
    pub(crate) candidate_count: usize,
    pub(crate) hit_count: usize,
    pub(crate) miss_count: usize,
    pub(crate) clamp_count: usize,
}

#[derive(Clone, Debug)]
struct CcdColliderSnapshot {
    handle: ColliderHandle,
    body: BodyHandle,
    body_type: BodyType,
    shape: SharedShape,
    start_pose: Pose,
    end_pose: Pose,
    aabb: ShapeAabb,
    start_convex_vertices: Option<Vec<Point>>,
    end_convex_vertices: Option<Vec<Point>>,
    filter: CollisionFilter,
    is_sensor: bool,
}

#[derive(Clone, Copy, Debug)]
struct MovingCircle {
    body: BodyHandle,
    collider: ColliderHandle,
    radius: FloatNum,
    start: Point,
    end: Point,
    swept_aabb: ShapeAabb,
    filter: CollisionFilter,
    is_sensor: bool,
}

#[derive(Clone, Debug)]
struct MovingConvex {
    body: BodyHandle,
    collider: ColliderHandle,
    start: Point,
    end: Point,
    start_vertices: Vec<Point>,
    swept_aabb: ShapeAabb,
    filter: CollisionFilter,
    is_sensor: bool,
}

#[derive(Clone, Debug)]
struct DynamicConvexTarget {
    body: BodyHandle,
    collider: ColliderHandle,
    start: Point,
    end: Point,
    start_vertices: Vec<Point>,
    swept_aabb: ShapeAabb,
    filter: CollisionFilter,
    is_sensor: bool,
}

#[derive(Clone, Debug)]
struct StaticConvex {
    body: BodyHandle,
    collider: ColliderHandle,
    point: Point,
    vertices: Vec<Point>,
    aabb: ShapeAabb,
    filter: CollisionFilter,
    is_sensor: bool,
}

#[derive(Clone, Copy, Debug)]
struct CcdHit {
    moving_body: BodyHandle,
    static_body: BodyHandle,
    moving_collider: ColliderHandle,
    static_collider: ColliderHandle,
    target_kind: CcdTargetKind,
    swept_start: Point,
    swept_end: Point,
    target_swept_start: Point,
    target_swept_end: Point,
    toi: FloatNum,
    exit: FloatNum,
    slop_sweep_length: FloatNum,
    toi_point: Point,
}

pub(crate) fn run_pose_clamp_phase(
    world: &mut World,
    previous_body_poses: &BTreeMap<BodyHandle, Pose>,
) -> CcdPoseClampOutcome {
    let snapshots = collect_snapshots(world, previous_body_poses);
    let mut moving_circles = Vec::with_capacity(snapshots.len());
    let mut moving_convexes = Vec::with_capacity(snapshots.len());
    let mut dynamic_convex_targets = Vec::with_capacity(snapshots.len());
    let mut static_convexes = Vec::with_capacity(snapshots.len());
    for snapshot in &snapshots {
        if let Some(moving) = moving_circle(snapshot) {
            moving_circles.push(moving);
        }
        if let Some(target) = dynamic_convex_target(snapshot) {
            dynamic_convex_targets.push(target);
        }
        if let Some(moving) = moving_convex(snapshot) {
            moving_convexes.push(moving);
        }
        if let Some(target) = static_convex(snapshot) {
            static_convexes.push(target);
        }
    }
    let mut stats = CcdPoseClampStats::default();
    let mut hits = Vec::with_capacity(
        moving_circles
            .len()
            .saturating_mul(static_convexes.len())
            .saturating_add(moving_convexes.len().saturating_mul(static_convexes.len()))
            .saturating_add(
                moving_convexes
                    .len()
                    .saturating_mul(moving_convexes.len().saturating_sub(1))
                    / 2,
            ),
    );

    for moving in &moving_circles {
        if moving.is_sensor {
            continue;
        }
        for target in &static_convexes {
            if moving.body == target.body || target.is_sensor {
                continue;
            }
            if !moving.filter.allows(&target.filter) {
                continue;
            }
            if !aabb_overlaps(moving.swept_aabb, target.aabb) {
                continue;
            }
            stats.candidate_count += 1;
            let sweep = moving.end - moving.start;
            if let Some(hit) =
                swept_circle_convex_toi(moving.start, moving.end, moving.radius, &target.vertices)
            {
                hits.push(CcdHit {
                    moving_body: moving.body,
                    static_body: target.body,
                    moving_collider: moving.collider,
                    static_collider: target.collider,
                    target_kind: CcdTargetKind::Static,
                    swept_start: moving.start,
                    swept_end: moving.end,
                    target_swept_start: target.point,
                    target_swept_end: target.point,
                    toi: hit.toi,
                    exit: hit.exit,
                    slop_sweep_length: sweep.length(),
                    toi_point: moving.start + (moving.end - moving.start) * hit.toi
                        - hit.normal * moving.radius,
                });
            }
        }
    }
    for moving in &moving_convexes {
        if moving.is_sensor {
            continue;
        }
        for target in &static_convexes {
            if moving.body == target.body || target.is_sensor {
                continue;
            }
            if !moving.filter.allows(&target.filter) {
                continue;
            }
            if !aabb_overlaps(moving.swept_aabb, target.aabb) {
                continue;
            }
            stats.candidate_count += 1;
            let sweep = moving.end - moving.start;
            if let Some(hit) =
                swept_convex_convex_toi(&moving.start_vertices, sweep, &target.vertices)
            {
                hits.push(CcdHit {
                    moving_body: moving.body,
                    static_body: target.body,
                    moving_collider: moving.collider,
                    static_collider: target.collider,
                    target_kind: CcdTargetKind::Static,
                    swept_start: moving.start,
                    swept_end: moving.end,
                    target_swept_start: target.point,
                    target_swept_end: target.point,
                    toi: hit.toi,
                    exit: hit.exit,
                    slop_sweep_length: sweep.length(),
                    toi_point: hit.toi_point,
                });
            }
        }
    }
    for moving in &moving_convexes {
        for target in &dynamic_convex_targets {
            if moving.body == target.body || moving.is_sensor || target.is_sensor {
                continue;
            }
            if dynamic_convex_is_moving(target)
                && (moving.body, moving.collider) > (target.body, target.collider)
            {
                continue;
            }
            if !moving.filter.allows(&target.filter) {
                continue;
            }
            if !aabb_overlaps(moving.swept_aabb, target.swept_aabb) {
                continue;
            }
            stats.candidate_count += 1;
            let moving_sweep = moving.end - moving.start;
            let target_sweep = target.end - target.start;
            let relative_sweep = moving_sweep - target_sweep;
            if let Some(hit) = swept_convex_convex_toi(
                &moving.start_vertices,
                relative_sweep,
                &target.start_vertices,
            ) {
                hits.push(CcdHit {
                    moving_body: moving.body,
                    static_body: target.body,
                    moving_collider: moving.collider,
                    static_collider: target.collider,
                    target_kind: CcdTargetKind::Dynamic,
                    swept_start: moving.start,
                    swept_end: moving.end,
                    target_swept_start: target.start,
                    target_swept_end: target.end,
                    toi: hit.toi,
                    exit: hit.exit,
                    slop_sweep_length: relative_sweep.length(),
                    toi_point: hit.toi_point + target_sweep * hit.toi,
                });
            }
        }
    }

    stats.hit_count = hits.len();
    stats.miss_count = stats.candidate_count.saturating_sub(stats.hit_count);
    hits.sort_by(compare_hits);

    let mut clamped_bodies = BTreeSet::new();
    let mut traces = Vec::new();
    for hit in hits {
        let clamps_dynamic_target = hit.target_kind == CcdTargetKind::Dynamic;
        if clamped_bodies.contains(&hit.moving_body)
            || (clamps_dynamic_target && clamped_bodies.contains(&hit.static_body))
        {
            continue;
        }
        if let Some((trace, clamp_count)) = clamp_hit_to_toi(world, hit) {
            clamped_bodies.insert(trace.moving_body);
            if trace.target_kind == CcdTargetKind::Dynamic {
                clamped_bodies.insert(trace.static_body);
            }
            stats.clamp_count += clamp_count;
            traces.push(trace);
        }
    }

    CcdPoseClampOutcome { stats, traces }
}

fn collect_snapshots(
    world: &World,
    previous_body_poses: &BTreeMap<BodyHandle, Pose>,
) -> Vec<CcdColliderSnapshot> {
    world
        .collider_records()
        .filter_map(|(handle, collider)| {
            let body = world.body_record(collider.body).ok()?;
            let start_body_pose = previous_body_poses
                .get(&collider.body)
                .copied()
                .unwrap_or(body.pose);
            let start_pose = start_body_pose.compose(collider.local_pose);
            let end_pose = body.pose.compose(collider.local_pose);
            let end_geometry = collider.derived_geometry(body.pose);
            let start_convex_vertices = if start_body_pose == body.pose {
                end_geometry.convex_vertices.clone()
            } else {
                collider.convex_world_vertices(start_body_pose)
            };
            Some(CcdColliderSnapshot {
                handle,
                body: collider.body,
                body_type: body.body_type,
                shape: collider.shape.clone(),
                start_pose,
                end_pose,
                aabb: end_geometry.aabb,
                start_convex_vertices,
                end_convex_vertices: end_geometry.convex_vertices,
                filter: collider.filter,
                is_sensor: collider.is_sensor,
            })
        })
        .collect()
}

fn moving_circle(snapshot: &CcdColliderSnapshot) -> Option<MovingCircle> {
    let SharedShape::Circle { radius } = snapshot.shape else {
        return None;
    };
    if !snapshot.body_type.is_dynamic() || radius <= 0.0 || !radius.is_finite() {
        return None;
    }
    let start = snapshot.start_pose.point();
    let end = snapshot.end_pose.point();
    if !point_is_finite(start) || !point_is_finite(end) {
        return None;
    }
    let sweep = end - start;
    if sweep.length() <= CCD_TOI_EPSILON {
        return None;
    }
    Some(MovingCircle {
        body: snapshot.body,
        collider: snapshot.handle,
        radius,
        start,
        end,
        swept_aabb: swept_circle_aabb(start, end, radius),
        filter: snapshot.filter,
        is_sensor: snapshot.is_sensor,
    })
}

fn moving_convex(snapshot: &CcdColliderSnapshot) -> Option<MovingConvex> {
    let target = dynamic_convex_target(snapshot)?;
    let sweep = target.end - target.start;
    if sweep.length() <= CCD_TOI_EPSILON {
        return None;
    }
    Some(MovingConvex {
        body: target.body,
        collider: target.collider,
        start: target.start,
        end: target.end,
        start_vertices: target.start_vertices,
        swept_aabb: target.swept_aabb,
        filter: target.filter,
        is_sensor: target.is_sensor,
    })
}

fn dynamic_convex_target(snapshot: &CcdColliderSnapshot) -> Option<DynamicConvexTarget> {
    if !snapshot.body_type.is_dynamic() {
        return None;
    }
    if matches!(snapshot.shape, SharedShape::Circle { .. }) {
        return None;
    }
    // This first M13 slice is a translational convex shape cast. Rotational CCD
    // needs a wider angular sweep bound so it stays outside this minimal path.
    if (snapshot.start_pose.angle() - snapshot.end_pose.angle()).abs() > CCD_TOI_EPSILON {
        return None;
    }
    let start_vertices = convex_shape_vertices(
        &snapshot.shape,
        snapshot.start_pose,
        snapshot.start_convex_vertices.as_deref(),
    )?;
    let end_vertices = convex_shape_vertices(
        &snapshot.shape,
        snapshot.end_pose,
        snapshot.end_convex_vertices.as_deref(),
    )?;
    if start_vertices
        .iter()
        .chain(end_vertices.iter())
        .copied()
        .any(|point| !point_is_finite(point))
    {
        return None;
    }
    let start = snapshot.start_pose.point();
    let end = snapshot.end_pose.point();
    if !point_is_finite(start) || !point_is_finite(end) {
        return None;
    }
    let swept_aabb = swept_points_aabb(start_vertices.iter().chain(end_vertices.iter()).copied());
    Some(DynamicConvexTarget {
        body: snapshot.body,
        collider: snapshot.handle,
        start,
        end,
        start_vertices,
        swept_aabb,
        filter: snapshot.filter,
        is_sensor: snapshot.is_sensor,
    })
}

fn dynamic_convex_is_moving(target: &DynamicConvexTarget) -> bool {
    (target.end - target.start).length() > CCD_TOI_EPSILON
}

fn static_convex(snapshot: &CcdColliderSnapshot) -> Option<StaticConvex> {
    if snapshot.body_type != BodyType::Static {
        return None;
    }
    let vertices = convex_shape_vertices(
        &snapshot.shape,
        snapshot.end_pose,
        snapshot.end_convex_vertices.as_deref(),
    )?;
    if vertices.len() < 3
        || vertices
            .iter()
            .copied()
            .any(|point| !point_is_finite(point))
    {
        return None;
    }
    Some(StaticConvex {
        body: snapshot.body,
        collider: snapshot.handle,
        point: snapshot.end_pose.point(),
        vertices,
        aabb: snapshot.aabb,
        filter: snapshot.filter,
        is_sensor: snapshot.is_sensor,
    })
}

fn convex_shape_vertices(
    shape: &SharedShape,
    pose: Pose,
    cached_vertices: Option<&[Point]>,
) -> Option<Vec<Point>> {
    match shape {
        SharedShape::Rect { .. }
        | SharedShape::RegularPolygon { .. }
        | SharedShape::ConvexPolygon { .. } => Some(
            cached_vertices
                .map(|vertices| vertices.to_vec())
                .unwrap_or_else(|| shape.world_vertices(pose)),
        ),
        _ => None,
    }
}

fn swept_circle_aabb(start: Point, end: Point, radius: FloatNum) -> ShapeAabb {
    ShapeAabb {
        min: Point::new(
            start.x().min(end.x()) - radius,
            start.y().min(end.y()) - radius,
        ),
        max: Point::new(
            start.x().max(end.x()) + radius,
            start.y().max(end.y()) + radius,
        ),
    }
}

fn swept_points_aabb(points: impl IntoIterator<Item = Point>) -> ShapeAabb {
    ShapeAabb::from_points(points.into_iter().collect())
}

fn aabb_overlaps(a: ShapeAabb, b: ShapeAabb) -> bool {
    !(a.max.x() < b.min.x()
        || b.max.x() < a.min.x()
        || a.max.y() < b.min.y()
        || b.max.y() < a.min.y())
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct SweptCircleConvexToi {
    toi: FloatNum,
    exit: FloatNum,
    normal: Vector,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct SweptConvexConvexToi {
    toi: FloatNum,
    exit: FloatNum,
    normal: Vector,
    toi_point: Point,
}

fn swept_circle_convex_toi(
    start: Point,
    end: Point,
    radius: FloatNum,
    vertices: &[Point],
) -> Option<SweptCircleConvexToi> {
    if vertices.len() < 3 || radius <= 0.0 || !radius.is_finite() {
        return None;
    }
    let sweep = end - start;
    if sweep.length() <= CCD_TOI_EPSILON {
        return None;
    }

    let mut enter = 0.0;
    let mut exit = 1.0;
    let mut enter_normal = Vector::default();

    for edge_index in 0..vertices.len() {
        let normal = edge_outward_normal(vertices, edge_index);
        if normal.length() <= CCD_TOI_EPSILON {
            return None;
        }
        // Treat the circle as a moving point against the static convex expanded
        // by the circle radius. Each convex edge contributes one half-space
        // inequality: signed_distance(center) - radius <= 0.
        let start_distance = expanded_edge_distance(start, vertices[edge_index], normal, radius);
        let end_distance = expanded_edge_distance(end, vertices[edge_index], normal, radius);
        if start_distance > 0.0 && end_distance > 0.0 {
            return None;
        }
        let denominator = start_distance - end_distance;
        if denominator.abs() <= CCD_TOI_EPSILON {
            continue;
        }
        let t = start_distance / denominator;
        if start_distance > end_distance {
            if t > enter {
                enter = t;
                enter_normal = normal;
            }
        } else if t < exit {
            exit = t;
        }
        if enter > exit {
            return None;
        }
    }

    if !(CCD_TOI_EPSILON..=1.0).contains(&enter) || exit < 0.0 {
        return None;
    }
    if enter_normal.length() <= CCD_TOI_EPSILON {
        return None;
    }

    // The half-space sweep is exact for face hits, and conservative near
    // convex vertices because it uses mitered edge offsets instead of rounded
    // arcs. Bracket the first real circle-vs-convex overlap inside that
    // interval, then bisect the actual distance predicate.
    let (toi, normal) =
        refine_actual_circle_convex_toi(start, sweep, radius, vertices, enter, exit)?;

    Some(SweptCircleConvexToi {
        toi,
        exit: exit.clamp(0.0, 1.0),
        normal,
    })
}

fn swept_convex_convex_toi(
    moving_start_vertices: &[Point],
    sweep: Vector,
    static_vertices: &[Point],
) -> Option<SweptConvexConvexToi> {
    if moving_start_vertices.len() < 3
        || static_vertices.len() < 3
        || sweep.length() <= CCD_TOI_EPSILON
    {
        return None;
    }

    let mut enter = 0.0;
    let mut exit = 1.0;
    let mut enter_normal = Vector::default();
    let axes = polygon_axes(moving_start_vertices)
        .into_iter()
        .chain(polygon_axes(static_vertices));

    for axis in axes {
        let moving = project_points(moving_start_vertices, axis)?;
        let static_projection = project_points(static_vertices, axis)?;
        let velocity = sweep.dot(axis);
        if velocity.abs() <= CCD_TOI_EPSILON {
            if moving.max < static_projection.min || static_projection.max < moving.min {
                return None;
            }
            continue;
        }

        let (axis_enter, axis_exit, normal) = if velocity > 0.0 {
            (
                (static_projection.min - moving.max) / velocity,
                (static_projection.max - moving.min) / velocity,
                -axis,
            )
        } else {
            (
                (static_projection.max - moving.min) / velocity,
                (static_projection.min - moving.max) / velocity,
                axis,
            )
        };
        if axis_enter > enter {
            enter = axis_enter;
            enter_normal = normal.normalized_or_zero();
        }
        if axis_exit < exit {
            exit = axis_exit;
        }
        if enter > exit {
            return None;
        }
    }

    if !(CCD_TOI_EPSILON..=1.0).contains(&enter) || exit < 0.0 {
        return None;
    }
    if enter_normal.length() <= CCD_TOI_EPSILON {
        return None;
    }

    let toi_vertices = moving_start_vertices
        .iter()
        .map(|point| *point + sweep * enter)
        .collect::<Vec<_>>();
    let toi_point = support_feature_contact_point(&toi_vertices, static_vertices, enter_normal)?;

    Some(SweptConvexConvexToi {
        toi: enter.clamp(0.0, 1.0),
        exit: exit.clamp(enter, 1.0),
        normal: enter_normal,
        toi_point,
    })
}

#[derive(Clone, Copy, Debug)]
struct Projection {
    min: FloatNum,
    max: FloatNum,
}

fn project_points(points: &[Point], axis: Vector) -> Option<Projection> {
    let mut min = FloatNum::INFINITY;
    let mut max = FloatNum::NEG_INFINITY;
    for point in points {
        let projection = Vector::from(*point).dot(axis);
        if !projection.is_finite() {
            return None;
        }
        min = min.min(projection);
        max = max.max(projection);
    }
    (min.is_finite() && max.is_finite()).then_some(Projection { min, max })
}

fn polygon_axes(vertices: &[Point]) -> Vec<Vector> {
    let mut axes = Vec::new();
    for edge_index in 0..vertices.len() {
        let normal = edge_outward_normal(vertices, edge_index);
        if normal.length() > CCD_TOI_EPSILON {
            axes.push(normal.normalized_or_zero());
        }
    }
    axes
}

fn support_feature_contact_point(
    moving_vertices: &[Point],
    static_vertices: &[Point],
    normal: Vector,
) -> Option<Point> {
    let normal = normal.normalized_or_zero();
    if normal.length() <= CCD_TOI_EPSILON {
        return None;
    }
    let tangent = normal.perp().normalized_or_zero();
    let moving = support_feature(moving_vertices, -normal, normal, tangent)?;
    let target = support_feature(static_vertices, normal, normal, tangent)?;
    let overlap_min = moving.tangent_min.max(target.tangent_min);
    let overlap_max = moving.tangent_max.min(target.tangent_max);
    let tangent_coord = if overlap_min <= overlap_max + CCD_TOI_EPSILON {
        (overlap_min + overlap_max) * 0.5
    } else {
        (moving.tangent_center() + target.tangent_center()) * 0.5
    };
    let normal_coord = (moving.normal_coord + target.normal_coord) * 0.5;
    Some(Point::from(normal * normal_coord + tangent * tangent_coord))
}

#[derive(Clone, Copy, Debug)]
struct SupportFeature {
    normal_coord: FloatNum,
    tangent_min: FloatNum,
    tangent_max: FloatNum,
}

impl SupportFeature {
    fn tangent_center(self) -> FloatNum {
        (self.tangent_min + self.tangent_max) * 0.5
    }
}

fn support_feature(
    vertices: &[Point],
    direction: Vector,
    normal: Vector,
    tangent: Vector,
) -> Option<SupportFeature> {
    if direction.length() <= CCD_TOI_EPSILON {
        return None;
    }
    let mut support_projection = FloatNum::NEG_INFINITY;
    for point in vertices
        .iter()
        .copied()
        .filter(|point| point_is_finite(*point))
    {
        support_projection = support_projection.max(Vector::from(point).dot(direction));
    }
    if !support_projection.is_finite() {
        return None;
    }

    let mut tangent_min = FloatNum::INFINITY;
    let mut tangent_max = FloatNum::NEG_INFINITY;
    let mut normal_sum = 0.0;
    let mut count = 0;
    for point in vertices
        .iter()
        .copied()
        .filter(|point| point_is_finite(*point))
    {
        if (Vector::from(point).dot(direction) - support_projection).abs() <= CLIP_SUPPORT_EPSILON {
            let tangent_coord = Vector::from(point).dot(tangent);
            tangent_min = tangent_min.min(tangent_coord);
            tangent_max = tangent_max.max(tangent_coord);
            normal_sum += Vector::from(point).dot(normal);
            count += 1;
        }
    }
    (count > 0 && tangent_min.is_finite() && tangent_max.is_finite()).then_some(SupportFeature {
        normal_coord: normal_sum / count as FloatNum,
        tangent_min,
        tangent_max,
    })
}

fn refine_actual_circle_convex_toi(
    start: Point,
    sweep: Vector,
    radius: FloatNum,
    vertices: &[Point],
    enter: FloatNum,
    exit: FloatNum,
) -> Option<(FloatNum, Vector)> {
    let enter = enter.clamp(0.0, 1.0);
    let exit = exit.clamp(enter, 1.0);
    if let Some(contact) = circle_convex_contact(start + sweep * enter, radius, vertices) {
        return Some((enter, contact.normal));
    }

    let mut low = enter;
    let mut high = None;
    let sample_count = 32;
    for sample in 1..=sample_count {
        let t = enter + (exit - enter) * (sample as FloatNum / sample_count as FloatNum);
        if circle_convex_contact(start + sweep * t, radius, vertices).is_some() {
            high = Some(t);
            break;
        }
        low = t;
    }
    let mut high = high?;

    for _ in 0..24 {
        let mid = (low + high) * 0.5;
        if circle_convex_contact(start + sweep * mid, radius, vertices).is_some() {
            high = mid;
        } else {
            low = mid;
        }
    }
    let contact = circle_convex_contact(start + sweep * high, radius, vertices)?;
    Some((high, contact.normal))
}

#[derive(Clone, Copy, Debug)]
struct CircleConvexContact {
    normal: Vector,
}

fn circle_convex_contact(
    center: Point,
    radius: FloatNum,
    vertices: &[Point],
) -> Option<CircleConvexContact> {
    let mut inside = true;
    let mut deepest_inside = (FloatNum::NEG_INFINITY, Vector::default());
    for edge_index in 0..vertices.len() {
        let normal = edge_outward_normal(vertices, edge_index);
        let distance = (center - vertices[edge_index]).dot(normal);
        if distance > 0.0 {
            inside = false;
        }
        if distance > deepest_inside.0 {
            deepest_inside = (distance, normal);
        }
    }
    if inside {
        return Some(CircleConvexContact {
            normal: deepest_inside.1.normalized_or_zero(),
        });
    }

    let closest = closest_point_on_polygon(center, vertices)?;
    let offset = center - closest;
    let distance_squared = offset.length_squared();
    if distance_squared > radius * radius {
        return None;
    }
    let distance = distance_squared.sqrt();
    let normal = if distance <= CCD_TOI_EPSILON {
        deepest_inside.1
    } else {
        offset / distance
    };
    Some(CircleConvexContact {
        normal: normal.normalized_or_zero(),
    })
}

fn closest_point_on_polygon(point: Point, vertices: &[Point]) -> Option<Point> {
    let mut closest = None::<(Point, FloatNum)>;
    for edge_index in 0..vertices.len() {
        let start = vertices[edge_index];
        let end = vertices[(edge_index + 1) % vertices.len()];
        let point_on_edge = closest_point_on_segment(point, start, end);
        let distance_squared = (point - point_on_edge).length_squared();
        if closest
            .as_ref()
            .is_none_or(|(_, current_distance)| distance_squared < *current_distance)
        {
            closest = Some((point_on_edge, distance_squared));
        }
    }
    closest.map(|(point, _)| point)
}

fn closest_point_on_segment(point: Point, start: Point, end: Point) -> Point {
    let segment = end - start;
    let length_squared = segment.length_squared();
    let t = if length_squared <= CCD_TOI_EPSILON {
        0.0
    } else {
        (point - start).dot(segment) / length_squared
    }
    .clamp(0.0, 1.0);
    start + segment * t
}

fn expanded_edge_distance(
    center: Point,
    edge_start: Point,
    outward_normal: Vector,
    radius: FloatNum,
) -> FloatNum {
    (center - edge_start).dot(outward_normal) - radius
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

fn polygon_area(vertices: &[Point]) -> FloatNum {
    let mut area = 0.0;
    for index in 0..vertices.len() {
        let current = vertices[index];
        let next = vertices[(index + 1) % vertices.len()];
        area += current.x() * next.y() - next.x() * current.y();
    }
    area * 0.5
}

fn clamp_hit_to_toi(world: &mut World, hit: CcdHit) -> Option<(CcdTrace, usize)> {
    let sweep = hit.swept_end - hit.swept_start;
    let target_sweep = hit.target_swept_end - hit.target_swept_start;
    if sweep.length() <= CCD_TOI_EPSILON || hit.slop_sweep_length <= CCD_TOI_EPSILON {
        return None;
    }
    let target_is_dynamic = hit.target_kind == CcdTargetKind::Dynamic;
    if !world
        .body_record(hit.moving_body)
        .ok()?
        .body_type
        .is_dynamic()
    {
        return None;
    }
    if target_is_dynamic
        && !world
            .body_record(hit.static_body)
            .ok()?
            .body_type
            .is_dynamic()
    {
        return None;
    }

    let slop_fraction = CCD_CLAMP_SLOP / hit.slop_sweep_length;
    let safe_exit = if hit.exit > hit.toi {
        hit.toi + (hit.exit - hit.toi) * 0.5
    } else {
        hit.toi
    };
    let advancement = (hit.toi + slop_fraction).min(safe_exit).clamp(hit.toi, 1.0);
    let clamped_center = hit.swept_start + sweep * advancement;
    let rollback = hit.swept_end - clamped_center;
    let target_rollback = if target_is_dynamic {
        hit.target_swept_end - (hit.target_swept_start + target_sweep * advancement)
    } else {
        Vector::default()
    };

    {
        let record = world.body_record_mut(hit.moving_body).ok()?;
        crate::solver::body_state::translate_pose(&mut record.pose, -rollback, 0.0);
        record.sleeping = false;
        record.sleep_idle_time = 0.0;
    }
    let mut clamp_count = 1;
    if target_is_dynamic {
        let record = world.body_record_mut(hit.static_body).ok()?;
        crate::solver::body_state::translate_pose(&mut record.pose, -target_rollback, 0.0);
        record.sleeping = false;
        record.sleep_idle_time = 0.0;
        clamp_count += 1;
    }

    Some((
        CcdTrace {
            moving_body: hit.moving_body,
            static_body: hit.static_body,
            moving_collider: hit.moving_collider,
            static_collider: hit.static_collider,
            target_kind: hit.target_kind,
            swept_start: hit.swept_start,
            swept_end: hit.swept_end,
            target_swept_start: hit.target_swept_start,
            target_swept_end: hit.target_swept_end,
            toi: hit.toi,
            advancement,
            clamp: rollback.length(),
            target_clamp: target_rollback.length(),
            slop: CCD_CLAMP_SLOP,
            toi_point: hit.toi_point,
        },
        clamp_count,
    ))
}

fn compare_hits(a: &CcdHit, b: &CcdHit) -> Ordering {
    a.toi
        .partial_cmp(&b.toi)
        .unwrap_or(Ordering::Equal)
        .then(a.target_kind.cmp(&b.target_kind))
        .then(a.moving_body.cmp(&b.moving_body))
        .then(a.moving_collider.cmp(&b.moving_collider))
        .then(a.static_body.cmp(&b.static_body))
        .then(a.static_collider.cmp(&b.static_collider))
}

fn point_is_finite(point: Point) -> bool {
    point.x().is_finite() && point.y().is_finite()
}

#[cfg(test)]
mod tests {
    use super::{swept_circle_convex_toi, swept_convex_convex_toi};
    use crate::{
        body::Pose,
        collider::SharedShape,
        math::{point::Point, vector::Vector},
    };

    #[test]
    fn pipeline_ccd_toi_catches_sweep_through_thin_rectangle() {
        let wall = SharedShape::rect(0.1, 10.0).world_vertices(Pose::default());
        let hit = swept_circle_convex_toi(
            Point::new(-1.0, 0.0),
            Point::new(2.3333333, 0.0),
            0.05,
            &wall,
        )
        .expect("sweep should hit the expanded thin wall");

        assert!(hit.toi > 0.0 && hit.toi < 1.0);
        assert!(hit.exit > hit.toi);
        assert_eq!(hit.normal, Vector::new(-1.0, 0.0));
    }

    #[test]
    fn pipeline_ccd_toi_rejects_sweep_that_misses_convex() {
        let wall = SharedShape::rect(0.1, 10.0).world_vertices(Pose::default());

        assert_eq!(
            swept_circle_convex_toi(
                Point::new(-1.0, 6.0),
                Point::new(2.3333333, 6.0),
                0.05,
                &wall,
            ),
            None
        );
    }

    #[test]
    fn pipeline_ccd_convex_shape_cast_catches_translation_through_thin_rectangle() {
        let moving =
            SharedShape::rect(0.1, 0.1).world_vertices(Pose::from_xy_angle(-1.0, 0.0, 0.0));
        let wall = SharedShape::rect(0.1, 10.0).world_vertices(Pose::default());
        let hit = swept_convex_convex_toi(&moving, Vector::new(3.3333333, 0.0), &wall)
            .expect("dynamic convex sweep should hit the thin wall");

        assert!(hit.toi > 0.0 && hit.toi < 1.0);
        assert!(hit.exit > hit.toi);
        assert_eq!(hit.normal, Vector::new(-1.0, 0.0));
        assert!((hit.toi_point.x() + 0.05).abs() < 1.0e-4);
        assert!(hit.toi_point.y().abs() < 1.0e-4);
    }

    #[test]
    fn pipeline_ccd_convex_shape_cast_rejects_missed_translation() {
        let moving =
            SharedShape::rect(0.1, 0.1).world_vertices(Pose::from_xy_angle(-0.9, 2.0, 0.0));
        let diamond = SharedShape::convex_polygon(vec![
            Point::new(0.0, -1.0),
            Point::new(1.0, 0.0),
            Point::new(0.0, 1.0),
            Point::new(-1.0, 0.0),
        ])
        .world_vertices(Pose::default());

        assert_eq!(
            swept_convex_convex_toi(&moving, Vector::new(0.0, -1.1), &diamond),
            None
        );
    }
}
