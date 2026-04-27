use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
};

use crate::{
    body::{BodyType, Pose},
    collider::{CollisionFilter, ShapeAabb, SharedShape},
    events::CcdTrace,
    handles::{BodyHandle, ColliderHandle},
    math::{point::Point, vector::Vector, FloatNum},
    world::World,
};

const CCD_TOI_EPSILON: FloatNum = 1.0e-5;
const CCD_CLAMP_SLOP: FloatNum = 1.0e-3;

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct CcdPhaseOutcome {
    pub(crate) stats: CcdPhaseStats,
    pub(crate) traces: Vec<CcdTrace>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CcdPhaseStats {
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
struct StaticConvex {
    body: BodyHandle,
    collider: ColliderHandle,
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
    swept_start: Point,
    swept_end: Point,
    radius: FloatNum,
    toi: FloatNum,
    exit: FloatNum,
    normal: Vector,
}

pub(crate) fn run_ccd_phase(
    world: &mut World,
    previous_body_poses: &BTreeMap<BodyHandle, Pose>,
) -> CcdPhaseOutcome {
    let snapshots = collect_snapshots(world, previous_body_poses);
    let moving_circles = snapshots
        .iter()
        .filter_map(moving_circle)
        .collect::<Vec<_>>();
    let static_convexes = snapshots
        .iter()
        .filter_map(static_convex)
        .collect::<Vec<_>>();
    let mut stats = CcdPhaseStats::default();
    let mut hits = Vec::new();

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
            if let Some(hit) =
                swept_circle_convex_toi(moving.start, moving.end, moving.radius, &target.vertices)
            {
                hits.push(CcdHit {
                    moving_body: moving.body,
                    static_body: target.body,
                    moving_collider: moving.collider,
                    static_collider: target.collider,
                    swept_start: moving.start,
                    swept_end: moving.end,
                    radius: moving.radius,
                    toi: hit.toi,
                    exit: hit.exit,
                    normal: hit.normal,
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
        if !clamped_bodies.insert(hit.moving_body) {
            continue;
        }
        if let Some(trace) = clamp_body_to_toi(world, hit) {
            stats.clamp_count += 1;
            traces.push(trace);
        }
    }

    CcdPhaseOutcome { stats, traces }
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
            Some(CcdColliderSnapshot {
                handle,
                body: collider.body,
                body_type: body.body_type,
                shape: collider.shape.clone(),
                start_pose,
                end_pose,
                aabb: collider.shape.aabb(end_pose),
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

fn static_convex(snapshot: &CcdColliderSnapshot) -> Option<StaticConvex> {
    if snapshot.body_type != BodyType::Static {
        return None;
    }
    let vertices = match &snapshot.shape {
        SharedShape::Rect { .. }
        | SharedShape::RegularPolygon { .. }
        | SharedShape::ConvexPolygon { .. } => snapshot.shape.world_vertices(snapshot.end_pose),
        _ => return None,
    };
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
        vertices,
        aabb: snapshot.aabb,
        filter: snapshot.filter,
        is_sensor: snapshot.is_sensor,
    })
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

fn clamp_body_to_toi(world: &mut World, hit: CcdHit) -> Option<CcdTrace> {
    let sweep = hit.swept_end - hit.swept_start;
    let sweep_length = sweep.length();
    if sweep_length <= CCD_TOI_EPSILON {
        return None;
    }
    let slop_fraction = CCD_CLAMP_SLOP / sweep_length;
    let safe_exit = if hit.exit > hit.toi {
        hit.toi + (hit.exit - hit.toi) * 0.5
    } else {
        hit.toi
    };
    let advancement = (hit.toi + slop_fraction).min(safe_exit).clamp(hit.toi, 1.0);
    let clamped_center = hit.swept_start + sweep * advancement;
    let rollback = hit.swept_end - clamped_center;
    let record = world.body_record_mut(hit.moving_body).ok()?;
    if !record.body_type.is_dynamic() {
        return None;
    }
    crate::solver::body_state::translate_pose(&mut record.pose, -rollback, 0.0);
    record.sleeping = false;
    record.sleep_idle_time = 0.0;

    Some(CcdTrace {
        moving_body: hit.moving_body,
        static_body: hit.static_body,
        moving_collider: hit.moving_collider,
        static_collider: hit.static_collider,
        swept_start: hit.swept_start,
        swept_end: hit.swept_end,
        toi: hit.toi,
        advancement,
        clamp: rollback.length(),
        slop: CCD_CLAMP_SLOP,
        toi_point: hit.swept_start + sweep * hit.toi - hit.normal * hit.radius,
    })
}

fn compare_hits(a: &CcdHit, b: &CcdHit) -> Ordering {
    a.toi
        .partial_cmp(&b.toi)
        .unwrap_or(Ordering::Equal)
        .then(a.moving_body.cmp(&b.moving_body))
        .then(a.moving_collider.cmp(&b.moving_collider))
        .then(a.static_collider.cmp(&b.static_collider))
}

fn point_is_finite(point: Point) -> bool {
    point.x().is_finite() && point.y().is_finite()
}

#[cfg(test)]
mod tests {
    use super::swept_circle_convex_toi;
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
}
