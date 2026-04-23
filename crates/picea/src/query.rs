//! Stable read-only query pipeline for Picea v1.
//!
//! The query layer consumes stable debug facts instead of borrowing world
//! internals directly. That keeps spatial queries portable across native,
//! wasm, and future client/server consumers.

use crate::{
    collider::CollisionFilter,
    debug::{
        sanitize_point, sanitize_scalar, sanitize_vector, DebugAabb, DebugCollider, DebugShape,
        DebugSnapshot, DebugSnapshotOptions,
    },
    handles::{BodyHandle, ColliderHandle, WorldRevision},
    math::{point::Point, vector::Vector, FloatNum},
    world::World,
};

/// Source of stable facts for the query cache.
pub trait QuerySource {
    /// Returns the revision represented by the source, when known.
    fn query_revision(&self) -> Option<WorldRevision>;

    /// Builds a query-oriented snapshot.
    fn query_snapshot(&self) -> DebugSnapshot;
}

impl QuerySource for DebugSnapshot {
    fn query_revision(&self) -> Option<WorldRevision> {
        self.meta.revision
    }

    fn query_snapshot(&self) -> DebugSnapshot {
        self.clone()
    }
}

impl QuerySource for World {
    fn query_revision(&self) -> Option<WorldRevision> {
        Some(self.revision())
    }

    fn query_snapshot(&self) -> DebugSnapshot {
        self.debug_snapshot(&DebugSnapshotOptions::for_query())
    }
}

/// Coarse filtering applied to point, AABB, and ray queries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QueryFilter {
    /// Restrict the query to a single body.
    pub body: Option<BodyHandle>,
    /// Exclude a body from the query.
    pub exclude_body: Option<BodyHandle>,
    /// Restrict the query to a single collider.
    pub collider: Option<ColliderHandle>,
    /// Exclude a collider from the query.
    pub exclude_collider: Option<ColliderHandle>,
    /// Restrict the query to colliders that can interact with this filter.
    pub interaction_filter: Option<CollisionFilter>,
    /// Whether sensors should be included.
    pub include_sensors: bool,
}

impl QueryFilter {
    /// Restricts the query to a single body.
    pub fn with_body(mut self, body: BodyHandle) -> Self {
        self.body = Some(body);
        self
    }

    /// Excludes a body from the query.
    pub fn excluding_body(mut self, body: BodyHandle) -> Self {
        self.exclude_body = Some(body);
        self
    }

    /// Restricts the query to a single collider.
    pub fn with_collider(mut self, collider: ColliderHandle) -> Self {
        self.collider = Some(collider);
        self
    }

    /// Excludes a collider from the query.
    pub fn excluding_collider(mut self, collider: ColliderHandle) -> Self {
        self.exclude_collider = Some(collider);
        self
    }

    /// Restricts results to colliders whose collision groups allow the
    /// provided filter to interact with them.
    pub fn colliding_with(mut self, filter: CollisionFilter) -> Self {
        self.interaction_filter = Some(filter);
        self
    }

    /// Includes sensors in query results.
    pub fn including_sensors(mut self) -> Self {
        self.include_sensors = true;
        self
    }

    fn matches(&self, collider: &QueryColliderRecord) -> bool {
        if !self.include_sensors && collider.is_sensor {
            return false;
        }
        if let Some(body) = self.body {
            if collider.body != body {
                return false;
            }
        }
        if let Some(body) = self.exclude_body {
            if collider.body == body {
                return false;
            }
        }
        if let Some(handle) = self.collider {
            if collider.handle != handle {
                return false;
            }
        }
        if let Some(handle) = self.exclude_collider {
            if collider.handle == handle {
                return false;
            }
        }
        if let Some(filter) = self.interaction_filter {
            if !collider.filter.allows(&filter) {
                return false;
            }
        }
        true
    }
}

/// Result of a ray query.
#[derive(Clone, Debug, PartialEq)]
pub struct RayHit {
    /// Hit body.
    pub body: BodyHandle,
    /// Hit collider.
    pub collider: ColliderHandle,
    /// Time of impact in ray-parameter units.
    pub toi: FloatNum,
    /// Hit point in world space.
    pub point: Point,
    /// Approximate surface normal in world space.
    pub normal: Vector,
}

/// Result of a point query.
#[derive(Clone, Debug, PartialEq)]
pub struct PointHit {
    /// Hit body.
    pub body: BodyHandle,
    /// Hit collider.
    pub collider: ColliderHandle,
    /// Point used for the query.
    pub point: Point,
    /// Distance from the point to the closest surface.
    pub distance_to_surface: FloatNum,
}

/// Result of an AABB overlap query.
#[derive(Clone, Debug, PartialEq)]
pub struct AabbHit {
    /// Hit body.
    pub body: BodyHandle,
    /// Hit collider.
    pub collider: ColliderHandle,
    /// Bounds that overlapped the query region.
    pub bounds: DebugAabb,
}

#[derive(Clone, Debug)]
struct QueryColliderRecord {
    handle: ColliderHandle,
    body: BodyHandle,
    is_sensor: bool,
    filter: CollisionFilter,
    bounds: DebugAabb,
    shape: DebugShape,
}

/// Cached query pipeline built from stable snapshot facts.
#[derive(Clone, Debug, Default)]
pub struct QueryPipeline {
    cached_revision: Option<WorldRevision>,
    colliders: Vec<QueryColliderRecord>,
}

impl QueryPipeline {
    /// Creates an empty query pipeline.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the world revision represented by the cache.
    pub fn revision(&self) -> Option<WorldRevision> {
        self.cached_revision
    }

    /// Rebuilds the query cache from a stable source.
    pub fn sync(&mut self, source: &impl QuerySource) {
        let snapshot = source.query_snapshot().sanitized();

        // The cache is keyed by world revision, but the query layer never holds
        // references into the world. Sync always rebuilds from owned facts.
        self.cached_revision = source.query_revision().or(snapshot.meta.revision);
        self.colliders = snapshot
            .colliders
            .into_iter()
            .filter_map(QueryColliderRecord::from_debug_collider)
            .collect();
    }

    /// Casts a ray against the cached colliders.
    pub fn cast_ray(
        &self,
        origin: Point,
        direction: Vector,
        max_toi: FloatNum,
        filter: QueryFilter,
    ) -> Option<RayHit> {
        let origin = sanitize_point(origin);
        let direction = sanitize_vector(direction);
        let max_toi = sanitize_scalar(max_toi).max(0.0);

        self.colliders
            .iter()
            .filter(|collider| filter.matches(collider))
            .filter_map(|collider| {
                ray_cast_shape(origin, direction, max_toi, &collider.shape, collider.bounds).map(
                    |(toi, point, normal)| RayHit {
                        body: collider.body,
                        collider: collider.handle,
                        toi,
                        point,
                        normal,
                    },
                )
            })
            .min_by(|lhs, rhs| lhs.toi.total_cmp(&rhs.toi))
    }

    /// Finds colliders that contain the given point.
    pub fn intersect_point(&self, point: Point, filter: QueryFilter) -> Vec<PointHit> {
        let point = sanitize_point(point);

        self.colliders
            .iter()
            .filter(|collider| filter.matches(collider))
            .filter(|collider| collider.bounds.contains_point(point))
            .filter_map(|collider| {
                point_distance_to_shape(point, &collider.shape).map(|distance_to_surface| {
                    PointHit {
                        body: collider.body,
                        collider: collider.handle,
                        point,
                        distance_to_surface,
                    }
                })
            })
            .collect()
    }

    /// Finds colliders whose bounds overlap the given region.
    pub fn intersect_aabb(&self, aabb: DebugAabb, filter: QueryFilter) -> Vec<AabbHit> {
        let query_bounds = DebugAabb::new(aabb.min, aabb.max);

        self.colliders
            .iter()
            .filter(|collider| filter.matches(collider))
            .filter(|collider| collider.bounds.overlaps(&query_bounds))
            .map(|collider| AabbHit {
                body: collider.body,
                collider: collider.handle,
                bounds: collider.bounds,
            })
            .collect()
    }
}

impl QueryColliderRecord {
    fn from_debug_collider(collider: DebugCollider) -> Option<Self> {
        let bounds = collider.aabb.or_else(|| collider.shape.aabb())?;
        Some(Self {
            handle: collider.handle,
            body: collider.body,
            is_sensor: collider.is_sensor,
            filter: collider.filter,
            bounds,
            shape: collider.shape,
        })
    }
}

fn point_distance_to_shape(point: Point, shape: &DebugShape) -> Option<FloatNum> {
    match shape {
        DebugShape::Circle { center, radius } => {
            let radius = sanitize_scalar(*radius).max(0.0);
            let distance = (point - *center).length();
            (distance <= radius + FloatNum::EPSILON).then_some((radius - distance).max(0.0))
        }
        DebugShape::Polygon { vertices } => contains_point_polygon(vertices, point)
            .then(|| distance_to_polygon_edges(point, vertices)),
        DebugShape::Segment { start, end, radius } => {
            let radius = sanitize_scalar(*radius).max(0.0);
            let distance = distance_point_to_segment(point, *start, *end);
            (distance <= radius + FloatNum::EPSILON).then_some((radius - distance).max(0.0))
        }
    }
}

fn ray_cast_shape(
    origin: Point,
    direction: Vector,
    max_toi: FloatNum,
    shape: &DebugShape,
    bounds: DebugAabb,
) -> Option<(FloatNum, Point, Vector)> {
    let _ = ray_cast_aabb(origin, direction, max_toi, bounds)?;

    match shape {
        DebugShape::Circle { center, radius } => {
            ray_cast_circle(origin, direction, max_toi, *center, *radius)
        }
        DebugShape::Polygon { vertices } => ray_cast_polygon(origin, direction, max_toi, vertices),
        DebugShape::Segment { start, end, radius } => {
            if sanitize_scalar(*radius).max(0.0) <= FloatNum::EPSILON {
                ray_cast_segment(origin, direction, max_toi, *start, *end)
            } else {
                ray_cast_aabb(origin, direction, max_toi, bounds)
            }
        }
    }
}

fn ray_cast_circle(
    origin: Point,
    direction: Vector,
    max_toi: FloatNum,
    center: Point,
    radius: FloatNum,
) -> Option<(FloatNum, Point, Vector)> {
    let radius = sanitize_scalar(radius).max(0.0);
    let a = direction.dot(direction);
    if a <= FloatNum::EPSILON {
        return None;
    }

    let offset = origin - center;
    let b = 2.0 * offset.dot(direction);
    let c = offset.dot(offset) - radius * radius;
    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return None;
    }

    let sqrt_discriminant = discriminant.sqrt();
    let denominator = 2.0 * a;
    let candidates = [
        (-b - sqrt_discriminant) / denominator,
        (-b + sqrt_discriminant) / denominator,
    ];
    let toi = candidates
        .into_iter()
        .filter(|candidate| *candidate >= 0.0 && *candidate <= max_toi)
        .min_by(|lhs, rhs| lhs.total_cmp(rhs))?;

    let point = origin + direction * toi;
    let normal = (point - center).normalized();
    Some((toi, point, normal))
}

fn ray_cast_polygon(
    origin: Point,
    direction: Vector,
    max_toi: FloatNum,
    vertices: &[Point],
) -> Option<(FloatNum, Point, Vector)> {
    if vertices.len() < 2 {
        return None;
    }
    if contains_point_polygon(vertices, origin) {
        return Some((0.0, origin, Vector::new(0.0, 0.0)));
    }

    let mut best: Option<(FloatNum, Point, Vector)> = None;
    for edge in polygon_edges(vertices) {
        let Some((toi, point)) = ray_segment_intersection(origin, direction, edge.0, edge.1) else {
            continue;
        };
        if toi < 0.0 || toi > max_toi {
            continue;
        }

        let edge_vector = edge.1 - edge.0;
        let mut normal = edge_vector.perp().normalized();
        if normal.dot(direction) > 0.0 {
            normal = -normal;
        }

        let candidate = (toi, point, normal);
        if best
            .as_ref()
            .map_or(true, |(current_toi, _, _)| toi < *current_toi)
        {
            best = Some(candidate);
        }
    }
    best
}

fn ray_cast_segment(
    origin: Point,
    direction: Vector,
    max_toi: FloatNum,
    start: Point,
    end: Point,
) -> Option<(FloatNum, Point, Vector)> {
    let (toi, point) = ray_segment_intersection(origin, direction, start, end)?;
    if toi > max_toi {
        return None;
    }

    let edge = end - start;
    let mut normal = edge.perp().normalized();
    if normal.dot(direction) > 0.0 {
        normal = -normal;
    }
    Some((toi, point, normal))
}

fn ray_cast_aabb(
    origin: Point,
    direction: Vector,
    max_toi: FloatNum,
    bounds: DebugAabb,
) -> Option<(FloatNum, Point, Vector)> {
    let (mut t_min, mut t_max) = (0.0, max_toi);
    let mut hit_normal = Vector::new(0.0, 0.0);

    for (origin_component, direction_component, min, max, axis_normal) in [
        (
            origin.x(),
            direction.x(),
            bounds.min.x(),
            bounds.max.x(),
            Vector::new(-1.0, 0.0),
        ),
        (
            origin.y(),
            direction.y(),
            bounds.min.y(),
            bounds.max.y(),
            Vector::new(0.0, -1.0),
        ),
    ] {
        if direction_component.abs() <= FloatNum::EPSILON {
            if origin_component < min || origin_component > max {
                return None;
            }
            continue;
        }

        let inv = direction_component.recip();
        let mut near = (min - origin_component) * inv;
        let mut far = (max - origin_component) * inv;
        let mut normal = axis_normal;
        if near > far {
            std::mem::swap(&mut near, &mut far);
            normal = -normal;
        }
        if near > t_min {
            t_min = near;
            hit_normal = normal;
        }
        t_max = t_max.min(far);
        if t_min > t_max {
            return None;
        }
    }

    let point = origin + direction * t_min;
    Some((t_min.max(0.0), point, hit_normal))
}

fn contains_point_polygon(vertices: &[Point], point: Point) -> bool {
    if vertices.len() < 3 {
        return false;
    }

    let mut inside = false;
    for (start, end) in polygon_edges(vertices) {
        if distance_point_to_segment(point, start, end) <= FloatNum::EPSILON {
            return true;
        }

        let crosses = (start.y() > point.y()) != (end.y() > point.y());
        if !crosses {
            continue;
        }

        let intersect_x =
            (end.x() - start.x()) * (point.y() - start.y()) / (end.y() - start.y()) + start.x();
        if point.x() <= intersect_x {
            inside = !inside;
        }
    }
    inside
}

fn distance_to_polygon_edges(point: Point, vertices: &[Point]) -> FloatNum {
    polygon_edges(vertices)
        .map(|(start, end)| distance_point_to_segment(point, start, end))
        .min_by(|lhs, rhs| lhs.total_cmp(rhs))
        .unwrap_or(0.0)
}

fn distance_point_to_segment(point: Point, start: Point, end: Point) -> FloatNum {
    let edge = end - start;
    let edge_length_squared = edge.dot(edge);
    if edge_length_squared <= FloatNum::EPSILON {
        return (point - start).length();
    }

    let from_start = point - start;
    let projection = (from_start.dot(edge) / edge_length_squared).clamp(0.0, 1.0);
    let closest = start + edge * projection;
    (point - closest).length()
}

fn ray_segment_intersection(
    origin: Point,
    direction: Vector,
    start: Point,
    end: Point,
) -> Option<(FloatNum, Point)> {
    let edge = end - start;
    let denominator = direction.cross(edge);
    if denominator.abs() <= FloatNum::EPSILON {
        return None;
    }

    let offset = start - origin;
    let ray_t = offset.cross(edge) / denominator;
    let segment_t = offset.cross(direction) / denominator;
    if ray_t < 0.0 || !(0.0..=1.0).contains(&segment_t) {
        return None;
    }

    Some((ray_t, origin + direction * ray_t))
}

fn polygon_edges(vertices: &[Point]) -> impl Iterator<Item = (Point, Point)> + '_ {
    vertices
        .iter()
        .copied()
        .zip(vertices.iter().copied().cycle().skip(1))
        .take(vertices.len())
}

#[cfg(test)]
mod tests {
    use super::{
        contains_point_polygon, distance_point_to_segment, ray_cast_aabb, ray_cast_circle,
        ray_cast_polygon,
    };
    use crate::{
        debug::DebugAabb,
        math::{point::Point, vector::Vector, FloatNum},
    };

    #[test]
    fn point_in_polygon_counts_boundary_points_as_inside() {
        let polygon = vec![
            Point::new(-1.0, -1.0),
            Point::new(1.0, -1.0),
            Point::new(1.0, 1.0),
            Point::new(-1.0, 1.0),
        ];

        assert!(contains_point_polygon(&polygon, Point::new(0.0, 0.0)));
        assert!(contains_point_polygon(&polygon, Point::new(1.0, 0.0)));
        assert!(!contains_point_polygon(&polygon, Point::new(2.0, 0.0)));
    }

    #[test]
    fn distance_to_segment_handles_degenerate_edges() {
        let point = Point::new(3.0, 4.0);
        let anchor = Point::new(0.0, 0.0);

        let distance = distance_point_to_segment(point, anchor, anchor);
        assert!((distance - 5.0).abs() <= FloatNum::EPSILON);
    }

    #[test]
    fn ray_cast_circle_returns_first_hit() {
        let hit = ray_cast_circle(
            Point::new(-5.0, 0.0),
            Vector::new(1.0, 0.0),
            10.0,
            Point::new(0.0, 0.0),
            1.0,
        )
        .expect("circle should be hit");

        assert!((hit.0 - 4.0).abs() <= FloatNum::EPSILON);
        assert_eq!(hit.1, Point::new(-1.0, 0.0));
    }

    #[test]
    fn ray_cast_polygon_hits_front_face() {
        let polygon = vec![
            Point::new(-1.0, -1.0),
            Point::new(1.0, -1.0),
            Point::new(1.0, 1.0),
            Point::new(-1.0, 1.0),
        ];

        let hit = ray_cast_polygon(Point::new(-3.0, 0.0), Vector::new(1.0, 0.0), 10.0, &polygon)
            .expect("polygon should be hit");

        assert!((hit.0 - 2.0).abs() <= FloatNum::EPSILON);
        assert_eq!(hit.1, Point::new(-1.0, 0.0));
    }

    #[test]
    fn ray_cast_aabb_returns_none_for_parallel_miss() {
        let bounds = DebugAabb::new(Point::new(-1.0, -1.0), Point::new(1.0, 1.0));

        assert!(
            ray_cast_aabb(Point::new(2.0, 0.0), Vector::new(0.0, 1.0), 10.0, bounds,).is_none()
        );
    }
}
