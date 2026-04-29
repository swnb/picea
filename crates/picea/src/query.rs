//! Stable read-only query pipeline for Picea v1.
//!
//! The query layer consumes stable debug facts instead of borrowing world
//! internals directly. That keeps spatial queries portable across native,
//! wasm, and future client/server consumers.

use std::cmp::Ordering;
use std::sync::Mutex;

use crate::{
    body::Pose,
    collider::{CollisionFilter, ShapeAabb, SharedShape},
    debug::{
        sanitize_point, sanitize_scalar, sanitize_vector, DebugAabb, DebugCollider, DebugShape,
        DebugSnapshot, DebugSnapshotOptions,
    },
    handles::{BodyHandle, ColliderHandle, WorldRevision},
    math::{point::Point, vector::Vector, FloatNum},
    pipeline::broadphase::{ColliderProxy, DynamicAabbTree, TreeQueryStats},
    world::World,
};
use serde::{Deserialize, Serialize};

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

/// Sanitized world-space input shape for public read-only queries.
#[derive(Clone, Debug, PartialEq)]
pub struct QueryShape {
    shape: DebugShape,
    bounds: DebugAabb,
}

impl QueryShape {
    /// Creates a circle query in world space.
    pub fn circle(center: Point, radius: FloatNum) -> Result<Self, QueryShapeError> {
        let center = validate_point(center, "circle.center")?;
        let radius = validate_positive_scalar(radius, "circle.radius")?;
        Ok(Self {
            shape: DebugShape::Circle { center, radius },
            bounds: DebugAabb::from_circle(center, radius),
        })
    }

    /// Creates a convex polygon query in world space.
    pub fn polygon(vertices: impl Into<Vec<Point>>) -> Result<Self, QueryShapeError> {
        let vertices = validate_polygon(vertices.into(), "polygon.vertices")?;
        let bounds = DebugAabb::from_points(&vertices).ok_or(QueryShapeError::InvalidShape {
            kind: "polygon",
            reason: "empty",
        })?;
        Ok(Self {
            shape: DebugShape::Polygon { vertices },
            bounds,
        })
    }

    /// Creates a segment query in world space.
    pub fn segment(start: Point, end: Point) -> Result<Self, QueryShapeError> {
        let start = validate_point(start, "segment.start")?;
        let end = validate_point(end, "segment.end")?;
        if (end - start).length() <= FloatNum::EPSILON {
            return Err(QueryShapeError::InvalidShape {
                kind: "segment",
                reason: "degenerate",
            });
        }
        let bounds =
            DebugAabb::from_points(&[start, end]).ok_or(QueryShapeError::InvalidShape {
                kind: "segment",
                reason: "empty",
            })?;
        Ok(Self {
            shape: DebugShape::Segment {
                start,
                end,
                radius: 0.0,
            },
            bounds,
        })
    }

    /// Converts a stable owned collider shape into the public query surface.
    ///
    /// Concave polygons stay rejected in M21 so the query API does not imply
    /// authoring/decomposition support that belongs to M22.
    pub fn from_shared_shape(shape: &SharedShape, pose: Pose) -> Result<Self, QueryShapeError> {
        match shape {
            SharedShape::Circle { radius } => Self::circle(pose.point(), *radius),
            SharedShape::Rect { .. }
            | SharedShape::RegularPolygon { .. }
            | SharedShape::ConvexPolygon { .. } => Self::polygon(shape.world_vertices(pose)),
            SharedShape::Segment { start, end } => {
                Self::segment(pose.transform_point(*start), pose.transform_point(*end))
            }
            SharedShape::ConcavePolygon { .. } => Err(QueryShapeError::UnsupportedShape {
                kind: "concave_polygon",
            }),
        }
    }

    fn shape(&self) -> &DebugShape {
        &self.shape
    }

    fn bounds(&self) -> DebugAabb {
        self.bounds
    }
}

/// Stable public error for invalid or unsupported query-shape input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueryShapeError {
    /// The input shape contained non-finite or degenerate geometry.
    InvalidShape {
        kind: &'static str,
        reason: &'static str,
    },
    /// The shape kind is not supported by the M21 public surface.
    UnsupportedShape { kind: &'static str },
}

/// Exact distance hit for public shape queries.
#[derive(Clone, Debug, PartialEq)]
pub struct ShapeHit {
    /// Hit body.
    pub body: BodyHandle,
    /// Hit collider.
    pub collider: ColliderHandle,
    /// Non-negative distance between the query and collider shapes.
    pub distance: FloatNum,
    /// Closest point on the query shape.
    pub query_point: Point,
    /// Closest point on the collider shape.
    pub collider_point: Point,
    /// Direction from collider toward the query shape when stable.
    pub normal: Option<Vector>,
}

/// Deterministic counters for the most recent query call on a `QueryPipeline`.
///
/// The counters describe query work shape, not wall-clock time. They are useful
/// for benchmark/artifact evidence because they stay stable across machines
/// when the snapshot, query, and filter are the same.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryStats {
    /// Number of query tree nodes visited by the broadphase-style traversal.
    pub traversal_count: usize,
    /// Number of leaf candidates returned by the tree before public filtering.
    pub candidate_count: usize,
    /// Number of tree nodes rejected by coarse query bounds.
    pub pruned_count: usize,
    /// Number of candidates rejected by `QueryFilter`.
    pub filter_drop_count: usize,
    /// Number of public hits returned by the query.
    pub hit_count: usize,
}

#[derive(Debug, Default)]
struct QueryStatsCell(Mutex<QueryStats>);

impl Clone for QueryStatsCell {
    fn clone(&self) -> Self {
        Self::from_stats(self.get())
    }
}

impl QueryStatsCell {
    fn from_stats(stats: QueryStats) -> Self {
        Self(Mutex::new(stats))
    }

    fn get(&self) -> QueryStats {
        *self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn set(&self, stats: QueryStats) {
        // A single lock keeps the "most recent query" counters coherent under
        // shared reads/writes. Query work stays read-only; only the stats swap
        // serializes.
        *self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = stats;
    }
}

#[derive(Clone, Debug)]
struct QueryColliderRecord {
    snapshot_index: usize,
    handle: ColliderHandle,
    body: BodyHandle,
    is_sensor: bool,
    filter: CollisionFilter,
    bounds: DebugAabb,
    shape: DebugShape,
}

#[derive(Clone, Debug)]
struct OrderedShapeHit {
    snapshot_index: usize,
    hit: ShapeHit,
}

/// Cached query pipeline built from stable snapshot facts.
#[derive(Clone, Debug, Default)]
pub struct QueryPipeline {
    cached_revision: Option<WorldRevision>,
    colliders: Vec<QueryColliderRecord>,
    broadphase: DynamicAabbTree,
    last_stats: QueryStatsCell,
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

    /// Returns deterministic counters from the most recent query call.
    pub fn last_stats(&self) -> QueryStats {
        self.last_stats.get()
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
            .enumerate()
            .filter_map(|(snapshot_index, collider)| {
                QueryColliderRecord::from_debug_collider(snapshot_index, collider)
            })
            .collect();
        let proxies = self
            .colliders
            .iter()
            .map(|collider| ColliderProxy {
                handle: collider.handle,
                aabb: collider.bounds.into(),
            })
            .collect::<Vec<_>>();
        self.broadphase = DynamicAabbTree::from_proxies(&proxies);
        self.last_stats.set(QueryStats::default());
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

        let mut best_hit: Option<RayHit> = None;
        let tree_output = self
            .broadphase
            .query_ray_proxy_indices_with_stats(origin, direction, max_toi);
        let mut stats = query_stats_from_tree(tree_output.stats);

        for index in tree_output.proxy_indices {
            let Some(collider) = self.colliders.get(index) else {
                continue;
            };
            if !filter.matches(collider) {
                stats.filter_drop_count += 1;
                continue;
            }
            let Some(hit) =
                ray_cast_shape(origin, direction, max_toi, &collider.shape, collider.bounds).map(
                    |(toi, point, normal)| RayHit {
                        body: collider.body,
                        collider: collider.handle,
                        toi,
                        point,
                        normal,
                    },
                )
            else {
                continue;
            };
            match &best_hit {
                Some(current) if hit.toi >= current.toi => {}
                _ => best_hit = Some(hit),
            }
        }
        stats.hit_count = usize::from(best_hit.is_some());
        self.last_stats.set(stats);
        best_hit
    }

    /// Finds colliders that contain the given point.
    pub fn intersect_point(&self, point: Point, filter: QueryFilter) -> Vec<PointHit> {
        let point = sanitize_point(point);

        let tree_output = self.broadphase.query_point_proxy_indices_with_stats(point);
        let mut stats = query_stats_from_tree(tree_output.stats);
        let mut hits = Vec::new();
        for index in tree_output.proxy_indices {
            let Some(collider) = self.colliders.get(index) else {
                continue;
            };
            if !filter.matches(collider) {
                stats.filter_drop_count += 1;
                continue;
            }
            if !collider.bounds.contains_point(point) {
                continue;
            }
            let Some(distance_to_surface) = point_distance_to_shape(point, &collider.shape) else {
                continue;
            };
            hits.push(PointHit {
                body: collider.body,
                collider: collider.handle,
                point,
                distance_to_surface,
            });
        }
        stats.hit_count = hits.len();
        self.last_stats.set(stats);
        hits
    }

    /// Finds colliders whose bounds overlap the given region.
    pub fn intersect_aabb(&self, aabb: DebugAabb, filter: QueryFilter) -> Vec<AabbHit> {
        let query_bounds = DebugAabb::new(aabb.min, aabb.max);

        let tree_output = self
            .broadphase
            .query_aabb_proxy_indices_with_stats(query_bounds.into());
        let mut stats = query_stats_from_tree(tree_output.stats);
        let mut hits = Vec::new();
        for index in tree_output.proxy_indices {
            let Some(collider) = self.colliders.get(index) else {
                continue;
            };
            if !filter.matches(collider) {
                stats.filter_drop_count += 1;
                continue;
            }
            if !collider.bounds.overlaps(&query_bounds) {
                continue;
            }
            hits.push(AabbHit {
                body: collider.body,
                collider: collider.handle,
                bounds: collider.bounds,
            });
        }
        stats.hit_count = hits.len();
        self.last_stats.set(stats);
        hits
    }

    /// Finds colliders whose exact distance to the query shape is within the
    /// given limit.
    pub fn intersect_shape(
        &self,
        shape: &QueryShape,
        max_distance: FloatNum,
        filter: QueryFilter,
    ) -> Result<Vec<ShapeHit>, QueryShapeError> {
        self.shape_hits_internal(shape, max_distance, filter, false)
    }

    /// Finds the closest collider to the query shape within the given limit.
    pub fn closest_shape(
        &self,
        shape: &QueryShape,
        max_distance: FloatNum,
        filter: QueryFilter,
    ) -> Result<Option<ShapeHit>, QueryShapeError> {
        Ok(self
            .shape_hits_internal(shape, max_distance, filter, true)?
            .into_iter()
            .next())
    }

    fn shape_hits_internal(
        &self,
        shape: &QueryShape,
        max_distance: FloatNum,
        filter: QueryFilter,
        first_only: bool,
    ) -> Result<Vec<ShapeHit>, QueryShapeError> {
        let max_distance = validate_non_negative_scalar(max_distance, "max_distance")?;
        let query_bounds = expand_aabb(shape.bounds(), max_distance);
        let tree_output = self
            .broadphase
            .query_aabb_proxy_indices_with_stats(query_bounds.into());
        let mut stats = query_stats_from_tree(tree_output.stats);
        let mut hits = Vec::new();

        for index in tree_output.proxy_indices {
            let Some(collider) = self.colliders.get(index) else {
                continue;
            };
            if !filter.matches(collider) {
                stats.filter_drop_count += 1;
                continue;
            }
            let Some((distance, query_point, collider_point, normal)) =
                distance_between_shapes(shape.shape(), &collider.shape)
            else {
                continue;
            };
            if distance > max_distance {
                continue;
            }
            hits.push(OrderedShapeHit {
                snapshot_index: collider.snapshot_index,
                hit: ShapeHit {
                    body: collider.body,
                    collider: collider.handle,
                    distance,
                    query_point,
                    collider_point,
                    normal,
                },
            });
        }

        hits.sort_by(|lhs, rhs| {
            lhs.hit
                .distance
                .total_cmp(&rhs.hit.distance)
                .then_with(|| lhs.snapshot_index.cmp(&rhs.snapshot_index))
        });
        if first_only {
            hits.truncate(1);
        }
        let hits = hits.into_iter().map(|entry| entry.hit).collect::<Vec<_>>();
        stats.hit_count = hits.len();
        self.last_stats.set(stats);
        Ok(hits)
    }
}

fn query_stats_from_tree(stats: TreeQueryStats) -> QueryStats {
    QueryStats {
        traversal_count: stats.traversal_count,
        candidate_count: stats.candidate_count,
        pruned_count: stats.pruned_count,
        filter_drop_count: 0,
        hit_count: 0,
    }
}

fn validate_point(point: Point, kind: &'static str) -> Result<Point, QueryShapeError> {
    if !point.x().is_finite() || !point.y().is_finite() {
        return Err(QueryShapeError::InvalidShape {
            kind,
            reason: "non_finite",
        });
    }
    Ok(sanitize_point(point))
}

fn validate_positive_scalar(
    value: FloatNum,
    kind: &'static str,
) -> Result<FloatNum, QueryShapeError> {
    if !value.is_finite() || value <= 0.0 {
        return Err(QueryShapeError::InvalidShape {
            kind,
            reason: "non_positive",
        });
    }
    Ok(sanitize_scalar(value))
}

fn validate_non_negative_scalar(
    value: FloatNum,
    kind: &'static str,
) -> Result<FloatNum, QueryShapeError> {
    if !value.is_finite() || value < 0.0 {
        return Err(QueryShapeError::InvalidShape {
            kind,
            reason: "negative_or_non_finite",
        });
    }
    Ok(sanitize_scalar(value))
}

fn validate_polygon(
    vertices: Vec<Point>,
    kind: &'static str,
) -> Result<Vec<Point>, QueryShapeError> {
    if vertices.len() < 3 {
        return Err(QueryShapeError::InvalidShape {
            kind,
            reason: "too_few_vertices",
        });
    }
    let vertices = vertices
        .into_iter()
        .map(|point| validate_point(point, kind))
        .collect::<Result<Vec<_>, _>>()?;
    if signed_polygon_area(&vertices).abs() <= FloatNum::EPSILON {
        return Err(QueryShapeError::InvalidShape {
            kind,
            reason: "degenerate",
        });
    }
    if !is_convex_polygon(&vertices) {
        return Err(QueryShapeError::UnsupportedShape {
            kind: "concave_polygon",
        });
    }
    Ok(vertices)
}

fn signed_polygon_area(vertices: &[Point]) -> FloatNum {
    polygon_edges(vertices)
        .map(|(start, end)| start.x() * end.y() - end.x() * start.y())
        .sum::<FloatNum>()
        * 0.5
}

fn is_convex_polygon(vertices: &[Point]) -> bool {
    let mut sign: FloatNum = 0.0;
    for ((a, b), c) in polygon_edges(vertices).zip(vertices.iter().copied().cycle().skip(2)) {
        let cross = (b - a).cross(c - b);
        if cross.abs() <= FloatNum::EPSILON {
            continue;
        }
        if sign.abs() <= FloatNum::EPSILON {
            sign = cross;
            continue;
        }
        if cross.signum() != sign.signum() {
            return false;
        }
    }
    true
}

fn expand_aabb(bounds: DebugAabb, margin: FloatNum) -> DebugAabb {
    let margin = sanitize_scalar(margin).max(0.0);
    DebugAabb::new(
        Point::new(bounds.min.x() - margin, bounds.min.y() - margin),
        Point::new(bounds.max.x() + margin, bounds.max.y() + margin),
    )
}

fn distance_between_shapes(
    query: &DebugShape,
    collider: &DebugShape,
) -> Option<(FloatNum, Point, Point, Option<Vector>)> {
    // M21 keeps the public query surface on simple deterministic world-space
    // closest-point geometry over stable debug snapshots. That is enough for
    // convex polygons, circles, and segments/capsules without implying richer
    // penetration or concave decomposition semantics, which stay in later work.
    match (query, collider) {
        (
            DebugShape::Circle {
                center: query_center,
                radius: query_radius,
            },
            DebugShape::Circle {
                center: collider_center,
                radius: collider_radius,
            },
        ) => circle_circle_distance(
            *query_center,
            *query_radius,
            *collider_center,
            *collider_radius,
        ),
        (
            DebugShape::Circle {
                center: query_center,
                radius: query_radius,
            },
            DebugShape::Polygon { vertices },
        ) => circle_polygon_distance(*query_center, *query_radius, vertices),
        (
            DebugShape::Circle {
                center: query_center,
                radius: query_radius,
            },
            DebugShape::Segment { start, end, radius },
        ) => circle_segment_distance(*query_center, *query_radius, *start, *end, *radius),
        (
            DebugShape::Polygon { vertices },
            DebugShape::Circle {
                center: collider_center,
                radius: collider_radius,
            },
        ) => swap_shape_distance(circle_polygon_distance(
            *collider_center,
            *collider_radius,
            vertices,
        )),
        (
            DebugShape::Polygon {
                vertices: query_vertices,
            },
            DebugShape::Polygon {
                vertices: collider_vertices,
            },
        ) => polygon_polygon_distance(query_vertices, collider_vertices),
        (DebugShape::Polygon { vertices }, DebugShape::Segment { start, end, radius }) => {
            polygon_segment_distance(vertices, *start, *end, *radius)
        }
        (
            DebugShape::Segment { start, end, radius },
            DebugShape::Circle {
                center: collider_center,
                radius: collider_radius,
            },
        ) => swap_shape_distance(circle_segment_distance(
            *collider_center,
            *collider_radius,
            *start,
            *end,
            *radius,
        )),
        (DebugShape::Segment { start, end, radius }, DebugShape::Polygon { vertices }) => {
            swap_shape_distance(polygon_segment_distance(vertices, *start, *end, *radius))
        }
        (
            DebugShape::Segment {
                start: query_start,
                end: query_end,
                radius: query_radius,
            },
            DebugShape::Segment {
                start: collider_start,
                end: collider_end,
                radius: collider_radius,
            },
        ) => segment_segment_distance(
            *query_start,
            *query_end,
            *query_radius,
            *collider_start,
            *collider_end,
            *collider_radius,
        ),
    }
}

fn swap_shape_distance(
    value: Option<(FloatNum, Point, Point, Option<Vector>)>,
) -> Option<(FloatNum, Point, Point, Option<Vector>)> {
    value.map(|(distance, query_point, collider_point, normal)| {
        (
            distance,
            collider_point,
            query_point,
            normal.map(|normal| -normal),
        )
    })
}

fn circle_circle_distance(
    query_center: Point,
    query_radius: FloatNum,
    collider_center: Point,
    collider_radius: FloatNum,
) -> Option<(FloatNum, Point, Point, Option<Vector>)> {
    let query_radius = sanitize_scalar(query_radius).max(0.0);
    let collider_radius = sanitize_scalar(collider_radius).max(0.0);
    let delta = query_center - collider_center;
    let center_distance = delta.length();
    let normal = normalized_or_none(delta);
    let distance = (center_distance - query_radius - collider_radius).max(0.0);
    let (query_point, collider_point) = match normal {
        Some(normal) => (
            query_center - normal * query_radius,
            collider_center + normal * collider_radius,
        ),
        None => (query_center, collider_center),
    };
    Some((distance, query_point, collider_point, normal))
}

fn circle_polygon_distance(
    center: Point,
    radius: FloatNum,
    vertices: &[Point],
) -> Option<(FloatNum, Point, Point, Option<Vector>)> {
    let radius = sanitize_scalar(radius).max(0.0);
    let collider_point = closest_point_on_polygon(center, vertices)?;
    let offset = center - collider_point;
    let raw_distance = offset.length();
    let normal = normalized_or_none(offset);
    if contains_point_polygon(vertices, center) || raw_distance <= radius + FloatNum::EPSILON {
        let point = if raw_distance <= FloatNum::EPSILON {
            center
        } else {
            collider_point
        };
        return Some((0.0, point, collider_point, normal));
    }

    Some((
        (raw_distance - radius).max(0.0),
        center - normal.unwrap_or_default() * radius,
        collider_point,
        normal,
    ))
}

fn circle_segment_distance(
    center: Point,
    radius: FloatNum,
    start: Point,
    end: Point,
    segment_radius: FloatNum,
) -> Option<(FloatNum, Point, Point, Option<Vector>)> {
    let radius = sanitize_scalar(radius).max(0.0);
    let segment_radius = sanitize_scalar(segment_radius).max(0.0);
    let segment_point = closest_point_on_segment(center, start, end);
    let offset = center - segment_point;
    let raw_distance = offset.length();
    let normal = normalized_or_none(offset);
    let distance = (raw_distance - radius - segment_radius).max(0.0);
    let query_point = center - normal.unwrap_or_default() * radius;
    let collider_point = segment_point + normal.unwrap_or_default() * segment_radius;
    if distance <= FloatNum::EPSILON {
        return Some((0.0, collider_point, collider_point, normal));
    }
    Some((distance, query_point, collider_point, normal))
}

fn polygon_polygon_distance(
    query_vertices: &[Point],
    collider_vertices: &[Point],
) -> Option<(FloatNum, Point, Point, Option<Vector>)> {
    if let Some(point) = polygon_overlap_point(query_vertices, collider_vertices) {
        return Some((0.0, point, point, None));
    }

    let mut best: Option<(FloatNum, Point, Point)> = None;
    for &vertex in query_vertices {
        let collider_point = closest_point_on_polygon(vertex, collider_vertices)?;
        best = best_of_shape_distance(best, vertex, collider_point);
    }
    for &vertex in collider_vertices {
        let query_point = closest_point_on_polygon(vertex, query_vertices)?;
        best = best_of_shape_distance(best, query_point, vertex);
    }
    finalize_shape_distance(best)
}

fn polygon_segment_distance(
    vertices: &[Point],
    start: Point,
    end: Point,
    segment_radius: FloatNum,
) -> Option<(FloatNum, Point, Point, Option<Vector>)> {
    let segment_radius = sanitize_scalar(segment_radius).max(0.0);
    if contains_point_polygon(vertices, start)
        || contains_point_polygon(vertices, end)
        || polygon_edges(vertices).any(|(edge_start, edge_end)| {
            segment_segment_intersection(start, end, edge_start, edge_end).is_some()
        })
    {
        let point = if contains_point_polygon(vertices, start) {
            start
        } else {
            closest_point_on_polygon(start, vertices).unwrap_or(start)
        };
        return Some((0.0, point, point, None));
    }

    let mut best_centerline_overlap: Option<(FloatNum, Point)> = None;
    for (edge_start, edge_end) in polygon_edges(vertices) {
        let (segment_point, polygon_point) =
            closest_points_between_segments(start, end, edge_start, edge_end);
        let distance = (polygon_point - segment_point).length();
        match best_centerline_overlap {
            Some((current_distance, current_point))
                if current_distance < distance
                    || (current_distance == distance
                        && point_order(current_point) <= point_order(polygon_point)) => {}
            _ => best_centerline_overlap = Some((distance, polygon_point)),
        }
    }
    if let Some((distance, polygon_point)) = best_centerline_overlap {
        // A debug `Segment { radius > 0 }` represents a capsule. Even when the
        // centerline stays outside the polygon, the public M21 query contract
        // still treats radius-backed overlap as a zero-distance hit.
        if distance <= segment_radius + FloatNum::EPSILON {
            return Some((0.0, polygon_point, polygon_point, None));
        }
    }

    let mut best: Option<(FloatNum, Point, Point)> = None;
    for &vertex in vertices {
        let collider_point = closest_point_on_segment(vertex, start, end);
        let collider_point =
            move_point_along_normal(collider_point, vertex - collider_point, segment_radius);
        best = best_of_shape_distance(best, vertex, collider_point);
    }
    for &endpoint in &[start, end] {
        let query_point = closest_point_on_polygon(endpoint, vertices)?;
        let collider_point =
            move_point_along_normal(endpoint, query_point - endpoint, segment_radius);
        best = best_of_shape_distance(best, query_point, collider_point);
    }
    finalize_shape_distance(best)
}

fn segment_segment_distance(
    query_start: Point,
    query_end: Point,
    query_radius: FloatNum,
    collider_start: Point,
    collider_end: Point,
    collider_radius: FloatNum,
) -> Option<(FloatNum, Point, Point, Option<Vector>)> {
    let query_radius = sanitize_scalar(query_radius).max(0.0);
    let collider_radius = sanitize_scalar(collider_radius).max(0.0);
    if let Some(point) =
        segment_segment_intersection(query_start, query_end, collider_start, collider_end)
    {
        return Some((0.0, point, point, None));
    }
    let (query_centerline_point, collider_centerline_point) =
        closest_points_between_segments(query_start, query_end, collider_start, collider_end);
    let delta = query_centerline_point - collider_centerline_point;
    let normal = normalized_or_none(delta);
    let distance = (delta.length() - query_radius - collider_radius).max(0.0);
    let query_point = query_centerline_point - normal.unwrap_or_default() * query_radius;
    let collider_point = collider_centerline_point + normal.unwrap_or_default() * collider_radius;
    if distance <= FloatNum::EPSILON {
        return Some((0.0, collider_point, collider_point, normal));
    }
    Some((distance, query_point, collider_point, normal))
}

fn best_of_shape_distance(
    current: Option<(FloatNum, Point, Point)>,
    query_point: Point,
    collider_point: Point,
) -> Option<(FloatNum, Point, Point)> {
    let distance = (query_point - collider_point).length();
    match current {
        Some((current_distance, current_query_point, current_collider_point))
            if current_distance < distance =>
        {
            Some((
                current_distance,
                current_query_point,
                current_collider_point,
            ))
        }
        Some((current_distance, current_query_point, current_collider_point))
            if current_distance == distance
                && compare_point_pair(
                    (query_point, collider_point),
                    (current_query_point, current_collider_point),
                ) == Ordering::Greater =>
        {
            Some((
                current_distance,
                current_query_point,
                current_collider_point,
            ))
        }
        _ => Some((distance, query_point, collider_point)),
    }
}

fn compare_point_pair(lhs: (Point, Point), rhs: (Point, Point)) -> Ordering {
    point_order(lhs.0)
        .cmp(&point_order(rhs.0))
        .then_with(|| point_order(lhs.1).cmp(&point_order(rhs.1)))
}

fn point_order(point: Point) -> (u32, u32) {
    (point.x().to_bits(), point.y().to_bits())
}

fn finalize_shape_distance(
    best: Option<(FloatNum, Point, Point)>,
) -> Option<(FloatNum, Point, Point, Option<Vector>)> {
    best.map(|(distance, query_point, collider_point)| {
        let normal = if distance <= FloatNum::EPSILON {
            None
        } else {
            normalized_or_none(query_point - collider_point)
        };
        (distance, query_point, collider_point, normal)
    })
}

fn polygon_overlap_point(query_vertices: &[Point], collider_vertices: &[Point]) -> Option<Point> {
    for (query_start, query_end) in polygon_edges(query_vertices) {
        for (collider_start, collider_end) in polygon_edges(collider_vertices) {
            if let Some(point) =
                segment_segment_intersection(query_start, query_end, collider_start, collider_end)
            {
                return Some(point);
            }
        }
    }
    query_vertices
        .iter()
        .copied()
        .find(|point| contains_point_polygon(collider_vertices, *point))
        .or_else(|| {
            collider_vertices
                .iter()
                .copied()
                .find(|point| contains_point_polygon(query_vertices, *point))
        })
}

fn closest_point_on_polygon(point: Point, vertices: &[Point]) -> Option<Point> {
    polygon_edges(vertices)
        .map(|(start, end)| {
            let candidate = closest_point_on_segment(point, start, end);
            ((point - candidate).length(), candidate)
        })
        .min_by(|lhs, rhs| {
            lhs.0
                .total_cmp(&rhs.0)
                .then_with(|| point_order(lhs.1).cmp(&point_order(rhs.1)))
        })
        .map(|(_, candidate)| candidate)
}

fn closest_point_on_segment(point: Point, start: Point, end: Point) -> Point {
    let edge = end - start;
    let edge_length_squared = edge.dot(edge);
    if edge_length_squared <= FloatNum::EPSILON {
        return start;
    }
    let projection = ((point - start).dot(edge) / edge_length_squared).clamp(0.0, 1.0);
    start + edge * projection
}

fn move_point_along_normal(point: Point, normal_hint: Vector, radius: FloatNum) -> Point {
    point + normalized_or_none(normal_hint).unwrap_or_default() * sanitize_scalar(radius).max(0.0)
}

fn normalized_or_none(vector: Vector) -> Option<Vector> {
    let normalized = vector.normalized_or_zero();
    (normalized.length() > FloatNum::EPSILON).then_some(normalized)
}

fn segment_segment_intersection(a0: Point, a1: Point, b0: Point, b1: Point) -> Option<Point> {
    let direction_a = a1 - a0;
    let direction_b = b1 - b0;
    let denominator = direction_a.cross(direction_b);
    let offset = b0 - a0;
    if denominator.abs() <= FloatNum::EPSILON {
        return None;
    }

    let t = offset.cross(direction_b) / denominator;
    let u = offset.cross(direction_a) / denominator;
    if !(0.0..=1.0).contains(&t) || !(0.0..=1.0).contains(&u) {
        return None;
    }
    Some(a0 + direction_a * t)
}

fn closest_points_between_segments(a0: Point, a1: Point, b0: Point, b1: Point) -> (Point, Point) {
    let d1 = a1 - a0;
    let d2 = b1 - b0;
    let r = a0 - b0;
    let a = d1.dot(d1);
    let e = d2.dot(d2);
    let f = d2.dot(r);

    if a <= FloatNum::EPSILON && e <= FloatNum::EPSILON {
        return (a0, b0);
    }
    if a <= FloatNum::EPSILON {
        let t = (f / e).clamp(0.0, 1.0);
        return (a0, b0 + d2 * t);
    }
    if e <= FloatNum::EPSILON {
        let s = (-d1.dot(r) / a).clamp(0.0, 1.0);
        return (a0 + d1 * s, b0);
    }

    let c = d1.dot(r);
    let b = d1.dot(d2);
    let denominator = a * e - b * b;
    let mut s = if denominator.abs() <= FloatNum::EPSILON {
        0.0
    } else {
        ((b * f - c * e) / denominator).clamp(0.0, 1.0)
    };
    let mut t = (b * s + f) / e;

    if t < 0.0 {
        t = 0.0;
        s = (-c / a).clamp(0.0, 1.0);
    } else if t > 1.0 {
        t = 1.0;
        s = ((b - c) / a).clamp(0.0, 1.0);
    }

    (a0 + d1 * s, b0 + d2 * t)
}

impl QueryColliderRecord {
    fn from_debug_collider(snapshot_index: usize, collider: DebugCollider) -> Option<Self> {
        let bounds = collider.aabb.or_else(|| collider.shape.aabb())?;
        Some(Self {
            snapshot_index,
            handle: collider.handle,
            body: collider.body,
            is_sensor: collider.is_sensor,
            filter: collider.filter,
            bounds,
            shape: collider.shape,
        })
    }
}

impl From<DebugAabb> for ShapeAabb {
    fn from(value: DebugAabb) -> Self {
        Self {
            min: value.min,
            max: value.max,
        }
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
