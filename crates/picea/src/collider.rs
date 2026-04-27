//! Stable collider descriptors and shape wrappers for the v1 world API.

use serde::{Deserialize, Serialize};

use crate::{
    body::{MassProperties, Pose},
    handles::{BodyHandle, ColliderHandle},
    math::{point::Point, vector::Vector, FloatNum},
    world::ValidationError,
};

/// Basic material parameters consumed by simulation, queries, and debug output.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Material {
    /// Surface friction coefficient.
    pub friction: FloatNum,
    /// Surface restitution coefficient.
    pub restitution: FloatNum,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            friction: 0.2,
            restitution: 0.0,
        }
    }
}

/// Bitmask-based collision filter used by broadphase and queries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollisionFilter {
    /// Membership bits contributed by this collider.
    pub memberships: u64,
    /// Mask bits that this collider is willing to interact with.
    pub collides_with: u64,
}

impl CollisionFilter {
    /// Returns `true` when two filters may interact.
    pub fn allows(&self, other: &Self) -> bool {
        (self.memberships & other.collides_with) != 0
            && (other.memberships & self.collides_with) != 0
    }
}

impl Default for CollisionFilter {
    fn default() -> Self {
        Self {
            memberships: u64::MAX,
            collides_with: u64::MAX,
        }
    }
}

/// Stable owned geometry descriptor used by colliders.
#[derive(Clone, Debug, PartialEq)]
pub enum SharedShape {
    /// Circle centered on the collider's local origin.
    Circle { radius: FloatNum },
    /// Axis-aligned rectangle centered on the collider's local origin.
    Rect { half_extents: Vector },
    /// Regular polygon centered on the collider's local origin.
    RegularPolygon { sides: usize, radius: FloatNum },
    /// Convex polygon expressed in local-space vertices.
    ConvexPolygon { vertices: Vec<Point> },
    /// Concave polygon expressed in local-space vertices.
    ConcavePolygon { vertices: Vec<Point> },
    /// Segment expressed in local space.
    Segment { start: Point, end: Point },
}

impl SharedShape {
    /// Creates a circle shape centered at the local origin.
    pub fn circle(radius: FloatNum) -> Self {
        Self::Circle { radius }
    }

    /// Creates a rectangle centered at the local origin.
    pub fn rect(width: FloatNum, height: FloatNum) -> Self {
        Self::Rect {
            half_extents: (width.abs() * 0.5, height.abs() * 0.5).into(),
        }
    }

    /// Creates a regular polygon centered at the local origin.
    pub fn regular_polygon(sides: usize, radius: FloatNum) -> Self {
        Self::RegularPolygon { sides, radius }
    }

    /// Creates a convex polygon from local-space vertices.
    pub fn convex_polygon(vertices: impl Into<Vec<Point>>) -> Self {
        Self::ConvexPolygon {
            vertices: vertices.into(),
        }
    }

    /// Creates a concave polygon from local-space vertices.
    pub fn concave_polygon(vertices: impl Into<Vec<Point>>) -> Self {
        Self::ConcavePolygon {
            vertices: vertices.into(),
        }
    }

    /// Creates a segment from local-space endpoints.
    pub fn segment(start: impl Into<Point>, end: impl Into<Point>) -> Self {
        Self::Segment {
            start: start.into(),
            end: end.into(),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn contains_point(&self, pose: Pose, point: Point) -> bool {
        let local_point = pose.inverse_transform_point(point);
        match self {
            Self::Circle { radius } => Vector::from(local_point).length() <= *radius,
            Self::Rect { half_extents } => {
                local_point.x().abs() <= half_extents.x()
                    && local_point.y().abs() <= half_extents.y()
            }
            Self::RegularPolygon { .. }
            | Self::ConvexPolygon { .. }
            | Self::ConcavePolygon { .. } => point_in_polygon(local_point, &self.local_vertices()),
            Self::Segment { start, end } => point_on_segment(local_point, *start, *end),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn aabb(&self, pose: Pose) -> ShapeAabb {
        match self {
            Self::Circle { radius } => ShapeAabb {
                min: (pose.point().x() - radius, pose.point().y() - radius).into(),
                max: (pose.point().x() + radius, pose.point().y() + radius).into(),
            },
            _ => ShapeAabb::from_points(self.world_vertices(pose)),
        }
    }

    pub(crate) fn mass_properties(
        &self,
        density: FloatNum,
        local_pose: Pose,
    ) -> Option<MassProperties> {
        if density == 0.0 {
            return Some(MassProperties::default());
        }

        let local = match self {
            Self::Circle { radius } => {
                let mass = density * std::f32::consts::PI * radius * radius;
                // A solid disk has half its mass times radius squared as centroid inertia.
                MassProperties {
                    mass,
                    local_center_of_mass: Point::default(),
                    inertia: 0.5 * mass * radius * radius,
                    ..MassProperties::default()
                }
            }
            Self::Rect { half_extents } => {
                let width = half_extents.x() * 2.0;
                let height = half_extents.y() * 2.0;
                let mass = density * width * height;
                MassProperties {
                    mass,
                    local_center_of_mass: Point::default(),
                    inertia: mass * (width * width + height * height) / 12.0,
                    ..MassProperties::default()
                }
            }
            Self::RegularPolygon { .. } => {
                polygon_mass_properties(&self.local_vertices(), density)?
            }
            Self::ConvexPolygon { vertices } | Self::ConcavePolygon { vertices } => {
                polygon_mass_properties(vertices, density)?
            }
            Self::Segment { .. } => MassProperties::default(),
        };

        Some(MassProperties {
            local_center_of_mass: local_pose.transform_point(local.local_center_of_mass),
            ..local
        })
    }

    pub(crate) fn validate_mass_properties(
        &self,
        density: FloatNum,
        local_pose: Pose,
        field_scope: &'static str,
    ) -> Result<MassProperties, ValidationError> {
        let mass_properties = self
            .mass_properties(density, local_pose)
            .ok_or_else(|| collider_mass_properties_error(field_scope))?;
        if !mass_properties.is_finite_non_negative() {
            return Err(collider_mass_properties_error(field_scope));
        }
        Ok(mass_properties)
    }

    #[allow(dead_code)]
    pub(crate) fn world_vertices(&self, pose: Pose) -> Vec<Point> {
        self.local_vertices()
            .into_iter()
            .map(|point| pose.transform_point(point))
            .collect()
    }

    pub(crate) fn support_point(&self, pose: Pose, direction: Vector) -> Option<Point> {
        if !direction.x().is_finite() || !direction.y().is_finite() {
            return None;
        }
        let direction = if direction.length() <= FloatNum::EPSILON {
            Vector::new(1.0, 0.0)
        } else {
            direction
        };
        match self {
            Self::Circle { radius } => {
                let unit = direction.normalized_or_zero();
                let point = pose.point() + unit * *radius;
                point_is_finite(point).then_some(point)
            }
            Self::Rect { .. } | Self::RegularPolygon { .. } | Self::ConvexPolygon { .. } => {
                support_from_points(self.world_vertices(pose), direction)
            }
            // A segment has a valid support map for distance/intersection tests,
            // but it may still be too degenerate for EPA to produce an area face.
            Self::Segment { start, end } => support_from_points(
                vec![pose.transform_point(*start), pose.transform_point(*end)],
                direction,
            ),
            // Concave polygons need decomposition first. Treating the full loop
            // as one support-mapped convex shape would fabricate contacts across
            // inward notches.
            Self::ConcavePolygon { .. } => None,
        }
    }

    #[allow(dead_code)]
    fn local_vertices(&self) -> Vec<Point> {
        match self {
            Self::Circle { .. } => Vec::new(),
            Self::Rect { half_extents } => vec![
                (-half_extents.x(), -half_extents.y()).into(),
                (half_extents.x(), -half_extents.y()).into(),
                (half_extents.x(), half_extents.y()).into(),
                (-half_extents.x(), half_extents.y()).into(),
            ],
            Self::RegularPolygon { sides, radius } => {
                let sides = (*sides).max(3);
                (0..sides)
                    .map(|index| {
                        let angle = (index as FloatNum) * crate::math::tau() / sides as FloatNum;
                        let vector = Vector::new(0.0, *radius).rotated(angle);
                        Point::from(vector)
                    })
                    .collect()
            }
            Self::ConvexPolygon { vertices } | Self::ConcavePolygon { vertices } => {
                vertices.clone()
            }
            Self::Segment { start, end } => vec![*start, *end],
        }
    }

    pub(crate) fn validate(&self, field_scope: &'static str) -> Result<(), ValidationError> {
        match self {
            Self::Circle { radius } => {
                if !radius.is_finite() || *radius < 0.0 {
                    return Err(match field_scope {
                        "patch" => ValidationError::ColliderPatch {
                            field: "shape.radius",
                        },
                        _ => ValidationError::ColliderDesc {
                            field: "shape.radius",
                        },
                    });
                }
            }
            Self::Rect { half_extents } => {
                if !half_extents.x().is_finite()
                    || !half_extents.y().is_finite()
                    || half_extents.x() < 0.0
                    || half_extents.y() < 0.0
                {
                    return Err(match field_scope {
                        "patch" => ValidationError::ColliderPatch {
                            field: "shape.half_extents",
                        },
                        _ => ValidationError::ColliderDesc {
                            field: "shape.half_extents",
                        },
                    });
                }
            }
            Self::RegularPolygon { sides, radius } => {
                if *sides < 3 {
                    return Err(match field_scope {
                        "patch" => ValidationError::ColliderPatch {
                            field: "shape.sides",
                        },
                        _ => ValidationError::ColliderDesc {
                            field: "shape.sides",
                        },
                    });
                }
                if !radius.is_finite() || *radius <= 0.0 {
                    return Err(match field_scope {
                        "patch" => ValidationError::ColliderPatch {
                            field: "shape.radius",
                        },
                        _ => ValidationError::ColliderDesc {
                            field: "shape.radius",
                        },
                    });
                }
            }
            Self::ConvexPolygon { vertices } | Self::ConcavePolygon { vertices } => {
                if vertices.len() < 3 {
                    return Err(match field_scope {
                        "patch" => ValidationError::ColliderPatch {
                            field: "shape.vertices",
                        },
                        _ => ValidationError::ColliderDesc {
                            field: "shape.vertices",
                        },
                    });
                }
                if polygon_area(vertices).abs() <= FloatNum::EPSILON {
                    return Err(match field_scope {
                        "patch" => ValidationError::ColliderPatch {
                            field: "shape.vertices",
                        },
                        _ => ValidationError::ColliderDesc {
                            field: "shape.vertices",
                        },
                    });
                }
                if vertices
                    .iter()
                    .any(|point| !point.x().is_finite() || !point.y().is_finite())
                {
                    return Err(match field_scope {
                        "patch" => ValidationError::ColliderPatch {
                            field: "shape.vertices",
                        },
                        _ => ValidationError::ColliderDesc {
                            field: "shape.vertices",
                        },
                    });
                }
            }
            Self::Segment { start, end } => {
                if !start.x().is_finite()
                    || !start.y().is_finite()
                    || !end.x().is_finite()
                    || !end.y().is_finite()
                    || Vector::from((*start, *end)).length() <= FloatNum::EPSILON
                {
                    return Err(match field_scope {
                        "patch" => ValidationError::ColliderPatch {
                            field: "shape.segment",
                        },
                        _ => ValidationError::ColliderDesc {
                            field: "shape.segment",
                        },
                    });
                }
            }
        }
        Ok(())
    }
}

fn support_from_points(points: Vec<Point>, direction: Vector) -> Option<Point> {
    points
        .into_iter()
        .filter(|point| point_is_finite(*point))
        .max_by(|lhs, rhs| {
            Vector::from(*lhs)
                .dot(direction)
                .partial_cmp(&Vector::from(*rhs).dot(direction))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    lhs.x()
                        .partial_cmp(&rhs.x())
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| {
                    lhs.y()
                        .partial_cmp(&rhs.y())
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        })
}

fn point_is_finite(point: Point) -> bool {
    point.x().is_finite() && point.y().is_finite()
}

fn collider_mass_properties_error(field_scope: &'static str) -> ValidationError {
    match field_scope {
        "patch" => ValidationError::ColliderPatch {
            field: "mass_properties",
        },
        _ => ValidationError::ColliderDesc {
            field: "mass_properties",
        },
    }
}

fn polygon_area(vertices: &[Point]) -> FloatNum {
    vertices
        .iter()
        .copied()
        .zip(vertices.iter().copied().cycle().skip(1))
        .take(vertices.len())
        .map(|(a, b)| a.x() * b.y() - b.x() * a.y())
        .sum::<FloatNum>()
        * 0.5
}

fn polygon_mass_properties(vertices: &[Point], density: FloatNum) -> Option<MassProperties> {
    let mut cross_sum = 0.0;
    let mut centroid_x_sum = 0.0;
    let mut centroid_y_sum = 0.0;
    let mut inertia_sum = 0.0;

    // Shoelace formulas assume a simple, non-self-intersecting loop. They work for convex
    // polygons and for simple concave polygons, but they are not a triangulation validator.
    for (a, b) in vertices
        .iter()
        .copied()
        .zip(vertices.iter().copied().cycle().skip(1))
        .take(vertices.len())
    {
        let cross = a.x() * b.y() - b.x() * a.y();
        cross_sum += cross;
        centroid_x_sum += (a.x() + b.x()) * cross;
        centroid_y_sum += (a.y() + b.y()) * cross;
        inertia_sum += cross
            * (a.x() * a.x()
                + a.x() * b.x()
                + b.x() * b.x()
                + a.y() * a.y()
                + a.y() * b.y()
                + b.y() * b.y());
    }

    if cross_sum.abs() <= FloatNum::EPSILON {
        return None;
    }

    let area = cross_sum * 0.5;
    let mass = density * area.abs();
    let centroid = Point::new(
        centroid_x_sum / (3.0 * cross_sum),
        centroid_y_sum / (3.0 * cross_sum),
    );
    let inertia_about_origin = density * inertia_sum.abs() / 12.0;
    let centroid_offset = Vector::from(centroid).length_squared();
    let inertia = (inertia_about_origin - mass * centroid_offset).max(0.0);

    Some(MassProperties {
        mass,
        local_center_of_mass: centroid,
        inertia,
        ..MassProperties::default()
    })
}

/// Descriptor used to create a collider attached to a body.
#[derive(Clone, Debug, PartialEq)]
pub struct ColliderDesc {
    /// Local-space geometry definition.
    pub shape: SharedShape,
    /// Local-space transform relative to the parent body.
    pub local_pose: Pose,
    /// Mass per unit area used when deriving body mass properties.
    pub density: FloatNum,
    /// Surface material parameters.
    pub material: Material,
    /// Collision filtering bits.
    pub filter: CollisionFilter,
    /// Whether this collider should skip impulse generation.
    pub is_sensor: bool,
    /// User-owned opaque payload preserved by the core API.
    pub user_data: u64,
}

impl Default for ColliderDesc {
    fn default() -> Self {
        Self {
            shape: SharedShape::circle(0.5),
            local_pose: Pose::default(),
            density: 1.0,
            material: Material::default(),
            filter: CollisionFilter::default(),
            is_sensor: false,
            user_data: 0,
        }
    }
}

impl ColliderDesc {
    pub(crate) fn validate(&self) -> Result<(), ValidationError> {
        self.shape.validate("desc")?;
        if !self.local_pose.translation().x().is_finite() {
            return Err(ValidationError::ColliderDesc {
                field: "local_pose.translation.x",
            });
        }
        if !self.local_pose.translation().y().is_finite() {
            return Err(ValidationError::ColliderDesc {
                field: "local_pose.translation.y",
            });
        }
        if !self.local_pose.angle().is_finite() {
            return Err(ValidationError::ColliderDesc {
                field: "local_pose.angle",
            });
        }
        if !self.density.is_finite() || self.density < 0.0 {
            return Err(ValidationError::ColliderDesc { field: "density" });
        }
        if !self.material.friction.is_finite() || self.material.friction < 0.0 {
            return Err(ValidationError::ColliderDesc {
                field: "material.friction",
            });
        }
        if !self.material.restitution.is_finite() || self.material.restitution < 0.0 {
            return Err(ValidationError::ColliderDesc {
                field: "material.restitution",
            });
        }
        Ok(())
    }
}

/// Partial update applied to an existing collider.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ColliderPatch {
    /// Replaces the shape when present.
    pub shape: Option<SharedShape>,
    /// Replaces the local pose when present.
    pub local_pose: Option<Pose>,
    /// Replaces the density when present.
    pub density: Option<FloatNum>,
    /// Replaces the material when present.
    pub material: Option<Material>,
    /// Replaces the collision filter when present.
    pub filter: Option<CollisionFilter>,
    /// Replaces the sensor flag when present.
    pub is_sensor: Option<bool>,
    /// Replaces the user payload when present.
    pub user_data: Option<u64>,
}

impl ColliderPatch {
    pub(crate) fn validate(&self) -> Result<(), ValidationError> {
        if let Some(shape) = &self.shape {
            shape.validate("patch")?;
        }
        if let Some(local_pose) = self.local_pose {
            if !local_pose.translation().x().is_finite() {
                return Err(ValidationError::ColliderPatch {
                    field: "local_pose.translation.x",
                });
            }
            if !local_pose.translation().y().is_finite() {
                return Err(ValidationError::ColliderPatch {
                    field: "local_pose.translation.y",
                });
            }
            if !local_pose.angle().is_finite() {
                return Err(ValidationError::ColliderPatch {
                    field: "local_pose.angle",
                });
            }
        }
        if self
            .density
            .is_some_and(|density| !density.is_finite() || density < 0.0)
        {
            return Err(ValidationError::ColliderPatch { field: "density" });
        }
        if self.material.is_some_and(|material| {
            !material.friction.is_finite()
                || material.friction < 0.0
                || !material.restitution.is_finite()
                || material.restitution < 0.0
        }) {
            return Err(ValidationError::ColliderPatch { field: "material" });
        }
        Ok(())
    }
}

/// Read-only collider snapshot resolved from a world handle.
#[derive(Clone, Debug, PartialEq)]
pub struct ColliderView {
    handle: ColliderHandle,
    body: BodyHandle,
    shape: SharedShape,
    local_pose: Pose,
    world_pose: Pose,
    density: FloatNum,
    material: Material,
    filter: CollisionFilter,
    is_sensor: bool,
    user_data: u64,
}

impl ColliderView {
    /// Returns the collider handle.
    pub fn handle(&self) -> ColliderHandle {
        self.handle
    }

    /// Returns the parent body handle.
    pub fn body(&self) -> BodyHandle {
        self.body
    }

    /// Returns the owned shape description.
    pub fn shape(&self) -> &SharedShape {
        &self.shape
    }

    /// Returns the local pose relative to the parent body.
    pub fn local_pose(&self) -> Pose {
        self.local_pose
    }

    /// Returns the resolved world pose.
    pub fn world_pose(&self) -> Pose {
        self.world_pose
    }

    /// Returns the density value.
    pub fn density(&self) -> FloatNum {
        self.density
    }

    /// Returns the material parameters.
    pub fn material(&self) -> Material {
        self.material
    }

    /// Returns the collision filter.
    pub fn filter(&self) -> CollisionFilter {
        self.filter
    }

    /// Returns whether the collider is a sensor.
    pub fn is_sensor(&self) -> bool {
        self.is_sensor
    }

    /// Returns the opaque user payload.
    pub fn user_data(&self) -> u64 {
        self.user_data
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code)]
pub(crate) struct ShapeAabb {
    pub(crate) min: Point,
    pub(crate) max: Point,
}

impl ShapeAabb {
    #[allow(dead_code)]
    pub(crate) fn from_points(points: Vec<Point>) -> Self {
        if points.is_empty() {
            return Self {
                min: Point::default(),
                max: Point::default(),
            };
        }

        let mut min_x = FloatNum::INFINITY;
        let mut min_y = FloatNum::INFINITY;
        let mut max_x = FloatNum::NEG_INFINITY;
        let mut max_y = FloatNum::NEG_INFINITY;
        for point in points {
            min_x = min_x.min(point.x());
            min_y = min_y.min(point.y());
            max_x = max_x.max(point.x());
            max_y = max_y.max(point.y());
        }
        Self {
            min: (min_x, min_y).into(),
            max: (max_x, max_y).into(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ColliderRecord {
    pub(crate) body: BodyHandle,
    pub(crate) shape: SharedShape,
    pub(crate) local_pose: Pose,
    pub(crate) density: FloatNum,
    pub(crate) material: Material,
    pub(crate) filter: CollisionFilter,
    pub(crate) is_sensor: bool,
    pub(crate) user_data: u64,
}

impl ColliderRecord {
    pub(crate) fn from_desc(body: BodyHandle, desc: ColliderDesc) -> Self {
        Self {
            body,
            shape: desc.shape,
            local_pose: desc.local_pose,
            density: desc.density,
            material: desc.material,
            filter: desc.filter,
            is_sensor: desc.is_sensor,
            user_data: desc.user_data,
        }
    }

    pub(crate) fn apply_patch(&mut self, patch: ColliderPatch) {
        if let Some(shape) = patch.shape {
            self.shape = shape;
        }
        if let Some(local_pose) = patch.local_pose {
            self.local_pose = local_pose;
        }
        if let Some(density) = patch.density {
            self.density = density;
        }
        if let Some(material) = patch.material {
            self.material = material;
        }
        if let Some(filter) = patch.filter {
            self.filter = filter;
        }
        if let Some(is_sensor) = patch.is_sensor {
            self.is_sensor = is_sensor;
        }
        if let Some(user_data) = patch.user_data {
            self.user_data = user_data;
        }
    }

    pub(crate) fn world_pose(&self, body_pose: Pose) -> Pose {
        body_pose.compose(self.local_pose)
    }

    pub(crate) fn view(&self, handle: ColliderHandle, body_pose: Pose) -> ColliderView {
        ColliderView {
            handle,
            body: self.body,
            shape: self.shape.clone(),
            local_pose: self.local_pose,
            world_pose: self.world_pose(body_pose),
            density: self.density,
            material: self.material,
            filter: self.filter,
            is_sensor: self.is_sensor,
            user_data: self.user_data,
        }
    }
}

#[allow(dead_code)]
fn point_in_polygon(point: Point, polygon: &[Point]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut previous = *polygon.last().expect("polygon has at least one vertex");
    for current in polygon.iter().copied() {
        let y_crosses = (current.y() > point.y()) != (previous.y() > point.y());
        if y_crosses {
            let denominator = previous.y() - current.y();
            if denominator.abs() > FloatNum::EPSILON {
                let x_intersection = (previous.x() - current.x()) * (point.y() - current.y())
                    / denominator
                    + current.x();
                if point.x() < x_intersection {
                    inside = !inside;
                }
            }
        }
        previous = current;
    }
    inside
}

#[allow(dead_code)]
fn point_on_segment(point: Point, start: Point, end: Point) -> bool {
    let ab: Vector = (start, end).into();
    let ap: Vector = (start, point).into();
    let cross = ab.cross(ap).abs();
    if cross > 1e-4 {
        return false;
    }
    let dot = (point.x() - start.x()) * (point.x() - end.x())
        + (point.y() - start.y()) * (point.y() - end.y());
    dot <= 0.0
}

#[cfg(test)]
mod tests {
    use super::SharedShape;
    use crate::{body::Pose, math::point::Point};

    const EPSILON: f32 = 1e-4;

    fn assert_near(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= EPSILON,
            "expected {actual} to be within {EPSILON} of {expected}"
        );
    }

    #[test]
    fn collider_mass_properties_follow_circle_rect_and_segment_formulas() {
        let circle = SharedShape::circle(2.0)
            .mass_properties(0.5, Pose::default())
            .expect("circle mass should compute");
        assert_near(circle.mass, 2.0 * std::f32::consts::PI);
        assert_eq!(circle.local_center_of_mass, Point::new(0.0, 0.0));
        assert_near(circle.inertia, 4.0 * std::f32::consts::PI);

        let rect = SharedShape::rect(4.0, 2.0)
            .mass_properties(3.0, Pose::default())
            .expect("rect mass should compute");
        assert_near(rect.mass, 24.0);
        assert_eq!(rect.local_center_of_mass, Point::new(0.0, 0.0));
        assert_near(rect.inertia, 40.0);

        let segment = SharedShape::segment(Point::new(-1.0, 0.0), Point::new(1.0, 0.0))
            .mass_properties(9.0, Pose::default())
            .expect("segment mass should compute");
        assert_eq!(segment.mass, 0.0);
        assert_eq!(segment.local_center_of_mass, Point::new(0.0, 0.0));
        assert_eq!(segment.inertia, 0.0);
    }

    #[test]
    fn collider_mass_properties_follow_regular_convex_and_concave_polygon_formulas() {
        let regular_square = SharedShape::regular_polygon(4, 2.0_f32.sqrt())
            .mass_properties(1.0, Pose::default())
            .expect("regular polygon mass should compute");
        assert_near(regular_square.mass, 4.0);
        assert_near(regular_square.local_center_of_mass.x(), 0.0);
        assert_near(regular_square.local_center_of_mass.y(), 0.0);
        assert_near(regular_square.inertia, 8.0 / 3.0);

        let triangle = SharedShape::convex_polygon(vec![
            Point::new(0.0, 0.0),
            Point::new(2.0, 0.0),
            Point::new(0.0, 2.0),
        ])
        .mass_properties(1.0, Pose::default())
        .expect("convex polygon mass should compute");
        assert_near(triangle.mass, 2.0);
        assert_near(triangle.local_center_of_mass.x(), 2.0 / 3.0);
        assert_near(triangle.local_center_of_mass.y(), 2.0 / 3.0);
        assert_near(triangle.inertia, 8.0 / 9.0);

        let l_shape = SharedShape::concave_polygon(vec![
            Point::new(0.0, 0.0),
            Point::new(2.0, 0.0),
            Point::new(2.0, 1.0),
            Point::new(1.0, 1.0),
            Point::new(1.0, 2.0),
            Point::new(0.0, 2.0),
        ])
        .mass_properties(1.0, Pose::default())
        .expect("simple concave polygon mass should compute");
        assert_near(l_shape.mass, 3.0);
        assert_near(l_shape.local_center_of_mass.x(), 5.0 / 6.0);
        assert_near(l_shape.local_center_of_mass.y(), 5.0 / 6.0);
        assert_near(l_shape.inertia, 11.0 / 6.0);
    }
}
