//! Builtin deterministic scenarios and reset-time overrides.
//!
//! A scenario is the lab's reproducible input fixture. It constructs a fresh
//! `picea::World` from public core APIs so live sessions can reset after edits
//! without mutating an in-flight simulation.

use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

use picea::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{LabError, LabResult};

/// Stable identifiers for the builtin CS-simulator scenarios.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioId {
    FallingBoxContact,
    #[serde(rename = "stack_4")]
    Stack4,
    JointAnchor,
    BroadphaseSparse,
    SatPolygon,
    CompoundProvenance,
    CcdFastCircleWall,
    CcdFastConvexWalls,
    CcdDynamicConvexPair,
}

impl ScenarioId {
    pub const ALL: [Self; 9] = [
        Self::FallingBoxContact,
        Self::Stack4,
        Self::JointAnchor,
        Self::BroadphaseSparse,
        Self::SatPolygon,
        Self::CompoundProvenance,
        Self::CcdFastCircleWall,
        Self::CcdFastConvexWalls,
        Self::CcdDynamicConvexPair,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FallingBoxContact => "falling_box_contact",
            Self::Stack4 => "stack_4",
            Self::JointAnchor => "joint_anchor",
            Self::BroadphaseSparse => "broadphase_sparse",
            Self::SatPolygon => "sat_polygon",
            Self::CompoundProvenance => "compound_provenance",
            Self::CcdFastCircleWall => "ccd_fast_circle_wall",
            Self::CcdFastConvexWalls => "ccd_fast_convex_walls",
            Self::CcdDynamicConvexPair => "ccd_dynamic_convex_pair",
        }
    }
}

impl Display for ScenarioId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ScenarioId {
    type Err = LabError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "falling_box_contact" => Ok(Self::FallingBoxContact),
            "stack_4" => Ok(Self::Stack4),
            "joint_anchor" => Ok(Self::JointAnchor),
            "broadphase_sparse" => Ok(Self::BroadphaseSparse),
            "sat_polygon" => Ok(Self::SatPolygon),
            "compound_provenance" => Ok(Self::CompoundProvenance),
            "ccd_fast_circle_wall" => Ok(Self::CcdFastCircleWall),
            "ccd_fast_convex_walls" => Ok(Self::CcdFastConvexWalls),
            "ccd_dynamic_convex_pair" => Ok(Self::CcdDynamicConvexPair),
            other => Err(LabError::UnknownScenario(other.to_owned())),
        }
    }
}

/// Human-facing metadata for one builtin scenario.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ScenarioDescriptor {
    pub id: ScenarioId,
    pub name: &'static str,
    pub description: &'static str,
}

pub fn list_scenarios() -> Vec<ScenarioDescriptor> {
    ScenarioId::ALL
        .into_iter()
        .map(|id| ScenarioDescriptor {
            id,
            name: match id {
                ScenarioId::FallingBoxContact => "Falling box contact",
                ScenarioId::Stack4 => "Four box stack",
                ScenarioId::JointAnchor => "World anchor joint",
                ScenarioId::BroadphaseSparse => "Sparse broadphase",
                ScenarioId::SatPolygon => "SAT polygon manifold",
                ScenarioId::CompoundProvenance => "Compound provenance fixture",
                ScenarioId::CcdFastCircleWall => "CCD fast circle wall",
                ScenarioId::CcdFastConvexWalls => "CCD fast convex walls",
                ScenarioId::CcdDynamicConvexPair => "CCD dynamic convex pair",
            },
            description: match id {
                ScenarioId::FallingBoxContact => "A dynamic box falling into static floor contact.",
                ScenarioId::Stack4 => "Four dynamic boxes stacked above a static floor.",
                ScenarioId::JointAnchor => "A body constrained toward a fixed world-space anchor.",
                ScenarioId::BroadphaseSparse => {
                    "Five static boxes with exactly one broadphase overlap."
                }
                ScenarioId::SatPolygon => {
                    "A rectangle and convex polygon exposing clipped manifold points."
                }
                ScenarioId::CompoundProvenance => {
                    "An authored compound body fixture exposing stable piece order and inherited collider semantics."
                }
                ScenarioId::CcdFastCircleWall => {
                    "A fast dynamic circle swept against a static thin rectangle wall."
                }
                ScenarioId::CcdFastConvexWalls => {
                    "A fast dynamic rectangle swept against two static thin walls."
                }
                ScenarioId::CcdDynamicConvexPair => {
                    "Two fast dynamic rectangles swept against each other."
                }
            },
        })
        .collect()
}

/// Session/scenario overrides are user-supplied knobs stored outside the core
/// world. They are reapplied when a session is reset.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ScenarioOverrides {
    pub frame_count: Option<usize>,
    pub gravity: Option<[f32; 2]>,
}

/// Input for one deterministic scenario run.
#[derive(Clone, Debug, PartialEq)]
pub struct RunConfig {
    pub scenario_id: ScenarioId,
    pub frame_count: usize,
    pub run_id: Option<String>,
    pub overrides: ScenarioOverrides,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            scenario_id: ScenarioId::FallingBoxContact,
            frame_count: 120,
            run_id: None,
            overrides: ScenarioOverrides::default(),
        }
    }
}

impl RunConfig {
    pub(crate) fn effective_frame_count(&self) -> usize {
        self.overrides
            .frame_count
            .unwrap_or(self.frame_count)
            .max(1)
    }
}

/// Serializable scene setup fixture used by lab examples and smoke tests.
///
/// Schema v1 covers the stable authoring layer for world flags, body placement,
/// circle/rectangle assets, material/filter presets, and recipe-indexed
/// distance/world-anchor joints. The fixture stays above low-level `World`
/// commands: JSON is converted into a `WorldRecipe`, and the core command layer
/// still owns handle resolution and validation paths.
pub const SCENE_RECIPE_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneRecipeFixture {
    #[serde(default = "default_scene_recipe_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub world: SceneFixtureWorld,
    #[serde(default)]
    pub bodies: Vec<SceneBodyFixture>,
    #[serde(default)]
    pub joints: Vec<SceneJointFixture>,
}

impl SceneRecipeFixture {
    pub fn to_world_recipe(&self) -> LabResult<WorldRecipe> {
        self.validate_schema_version()?;
        let mut recipe = WorldRecipe::new(WorldDesc {
            gravity: self.world.gravity.into(),
            enable_sleep: self.world.enable_sleep,
        });
        for (body_index, body) in self.bodies.iter().enumerate() {
            recipe = recipe.with_scene_body(body.to_body_bundle(body_index)?);
        }
        for joint in &self.joints {
            recipe = recipe.with_joint(joint.to_joint_bundle());
        }
        Ok(recipe)
    }

    fn validate_schema_version(&self) -> LabResult<()> {
        if self.schema_version == SCENE_RECIPE_SCHEMA_VERSION {
            Ok(())
        } else {
            Err(LabError::UnsupportedSceneSchemaVersion {
                found: self.schema_version,
                expected: SCENE_RECIPE_SCHEMA_VERSION,
            })
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneFixtureWorld {
    #[serde(default = "default_fixture_gravity")]
    pub gravity: [f32; 2],
    #[serde(default = "default_fixture_enable_sleep")]
    pub enable_sleep: bool,
}

impl Default for SceneFixtureWorld {
    fn default() -> Self {
        Self {
            gravity: default_fixture_gravity(),
            enable_sleep: default_fixture_enable_sleep(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneBodyFixture {
    #[serde(default)]
    pub body_type: BodyType,
    #[serde(default)]
    pub pose: [f32; 3],
    #[serde(default)]
    pub linear_velocity: [f32; 2],
    #[serde(default = "default_fixture_can_sleep")]
    pub can_sleep: bool,
    pub shape: SceneShapeFixture,
    #[serde(default)]
    pub material: MaterialPreset,
    #[serde(default)]
    pub filter: CollisionLayerPreset,
    #[serde(default = "default_fixture_density")]
    pub density: f32,
    #[serde(default)]
    pub is_sensor: bool,
}

/// Lab-owned read model for authored compound fixtures.
///
/// This carrier is intentionally produced during fixture instantiation rather
/// than reconstructed in the web app. The tradeoff is some repeated artifact
/// data, but it keeps authoring provenance separate from runtime physics facts.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CompoundProvenance {
    #[serde(default)]
    pub authored_body_index: usize,
    #[serde(default)]
    pub body_handle: Option<BodyHandle>,
    #[serde(default)]
    pub validation_path: String,
    #[serde(default)]
    pub inherited_material: MaterialPreset,
    #[serde(default)]
    pub inherited_filter: CollisionLayerPreset,
    #[serde(default = "default_fixture_density")]
    pub inherited_density: f32,
    #[serde(default)]
    pub inherited_is_sensor: bool,
    #[serde(default)]
    pub pieces: Vec<CompoundProvenancePiece>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CompoundProvenancePiece {
    #[serde(default)]
    pub generated_piece_index: usize,
    #[serde(default)]
    pub collider_handle: Option<ColliderHandle>,
    #[serde(default)]
    pub validation_path: String,
    #[serde(default)]
    pub local_pose: [f32; 3],
}

impl SceneBodyFixture {
    fn to_body_bundle(&self, body_index: usize) -> LabResult<BodyBundle> {
        let colliders = self
            .shape
            .to_collider_bundles(body_index)?
            .into_iter()
            .map(|collider| {
                collider
                    .with_material(self.material)
                    .with_filter(self.filter)
                    .with_density(self.density)
                    .with_sensor(self.is_sensor)
            });
        let base = match self.body_type {
            BodyType::Static => BodyBundle::static_body(),
            BodyType::Dynamic => BodyBundle::dynamic(),
            BodyType::Kinematic => BodyBundle::kinematic(),
        }
        .with_colliders(colliders);
        let asset = BodyAsset::from_bundle(base);
        let [x, y, angle] = self.pose;
        let mut bundle = asset.at(Pose::from_xy_angle(x, y, angle));
        let [vx, vy] = self.linear_velocity;
        bundle.desc.linear_velocity = Vector::new(vx, vy);
        bundle.desc.can_sleep = self.can_sleep;
        Ok(bundle)
    }
}

/// Serializable joint setup for the lab scene schema.
///
/// The schema stays above low-level `World::create_joint`: fixture joints point
/// at recipe body indices and borrow the core descriptor defaults unless the
/// author explicitly overrides a field.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SceneJointFixture {
    Distance(SceneDistanceJointFixture),
    WorldAnchor(SceneWorldAnchorJointFixture),
}

impl SceneJointFixture {
    fn to_joint_bundle(&self) -> JointBundle {
        match self {
            Self::Distance(joint) => joint.to_joint_bundle(),
            Self::WorldAnchor(joint) => joint.to_joint_bundle(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneDistanceJointFixture {
    pub body_a: usize,
    pub body_b: usize,
    #[serde(default)]
    pub rest_length: Option<f32>,
    #[serde(default)]
    pub stiffness: Option<f32>,
    #[serde(default)]
    pub damping: Option<f32>,
    #[serde(default)]
    pub local_anchor_a: Option<[f32; 2]>,
    #[serde(default)]
    pub local_anchor_b: Option<[f32; 2]>,
}

impl SceneDistanceJointFixture {
    fn to_joint_bundle(&self) -> JointBundle {
        let mut desc = DistanceJointDesc::default();
        if let Some(rest_length) = self.rest_length {
            desc.rest_length = rest_length;
        }
        if let Some(stiffness) = self.stiffness {
            desc.stiffness = stiffness;
        }
        if let Some(damping) = self.damping {
            desc.damping = damping;
        }
        if let Some(local_anchor_a) = self.local_anchor_a {
            desc.local_anchor_a = point_from_array(local_anchor_a);
        }
        if let Some(local_anchor_b) = self.local_anchor_b {
            desc.local_anchor_b = point_from_array(local_anchor_b);
        }
        JointBundle::Distance {
            body_a: self.body_a,
            body_b: self.body_b,
            desc,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneWorldAnchorJointFixture {
    pub body: usize,
    #[serde(default)]
    pub world_anchor: Option<[f32; 2]>,
    #[serde(default)]
    pub local_anchor: Option<[f32; 2]>,
    #[serde(default)]
    pub stiffness: Option<f32>,
    #[serde(default)]
    pub damping: Option<f32>,
}

impl SceneWorldAnchorJointFixture {
    fn to_joint_bundle(&self) -> JointBundle {
        let mut desc = WorldAnchorJointDesc::default();
        if let Some(world_anchor) = self.world_anchor {
            desc.world_anchor = point_from_array(world_anchor);
        }
        if let Some(local_anchor) = self.local_anchor {
            desc.local_anchor = point_from_array(local_anchor);
        }
        if let Some(stiffness) = self.stiffness {
            desc.stiffness = stiffness;
        }
        if let Some(damping) = self.damping {
            desc.damping = damping;
        }
        JointBundle::WorldAnchor {
            body: self.body,
            desc,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SceneShapeFixture {
    Circle {
        radius: f32,
    },
    Rect {
        width: f32,
        height: f32,
    },
    ConvexPolygon {
        vertices: Vec<[f32; 2]>,
    },
    Compound {
        pieces: Vec<SceneCompoundPieceFixture>,
    },
    ConcavePolygon {
        vertices: Vec<[f32; 2]>,
    },
}

/// One authored convex piece inside a compound body fixture.
///
/// "Compound" means one rigid body with several collider pieces attached in a
/// deterministic order. This keeps the runtime model unchanged while making
/// the authoring boundary explicit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneCompoundPieceFixture {
    pub shape: SceneCompoundPieceShapeFixture,
    #[serde(default)]
    pub local_pose: Option<[f32; 3]>,
}

/// Authorable convex piece shapes for the M22 fixture boundary.
///
/// Convex pieces are safe to forward into the existing recipe/runtime path.
/// Direct concave loops are rejected above the world layer so M22 does not
/// pretend the narrowphase or solver supports arbitrary concave contacts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SceneCompoundPieceShapeFixture {
    Circle { radius: f32 },
    Rect { width: f32, height: f32 },
    ConvexPolygon { vertices: Vec<[f32; 2]> },
}

impl SceneCompoundPieceFixture {
    fn validate(&self, body_index: usize, piece_index: usize) -> LabResult<()> {
        self.shape.validate(body_index, piece_index)?;
        if let Some(local_pose) = self.local_pose {
            let path = format!("scene.bodies[{body_index}].shape.pieces[{piece_index}].local_pose");
            validate_local_pose(&path, local_pose)?;
        }
        Ok(())
    }

    fn to_collider_bundle(&self) -> ColliderBundle {
        let mut collider = self.shape.to_collider_bundle();
        if let Some(local_pose) = self.local_pose {
            collider = collider.with_local_pose(pose_from_array(local_pose));
        }
        collider
    }
}

impl SceneCompoundPieceShapeFixture {
    fn validate(&self, body_index: usize, piece_index: usize) -> LabResult<()> {
        let piece_shape_path =
            format!("scene.bodies[{body_index}].shape.pieces[{piece_index}].shape");
        match self {
            Self::Circle { radius } => {
                validate_circle_radius(&format!("{piece_shape_path}.radius"), *radius)
            }
            Self::Rect { width, height } => validate_rect_size(&piece_shape_path, *width, *height),
            Self::ConvexPolygon { vertices } => {
                validate_convex_vertices(&format!("{piece_shape_path}.vertices"), vertices)
            }
        }
    }

    fn to_collider_bundle(&self) -> ColliderBundle {
        match self {
            Self::Circle { radius } => ColliderBundle::circle(*radius),
            Self::Rect { width, height } => ColliderBundle::rect(*width, *height),
            Self::ConvexPolygon { vertices } => {
                ColliderBundle::new(SharedShape::convex_polygon(points_from_arrays(vertices)))
            }
        }
    }
}

impl SceneShapeFixture {
    fn to_collider_bundles(&self, body_index: usize) -> LabResult<Vec<ColliderBundle>> {
        match self {
            Self::Circle { radius } => Ok(vec![ColliderBundle::circle(*radius)]),
            Self::Rect { width, height } => Ok(vec![ColliderBundle::rect(*width, *height)]),
            Self::ConvexPolygon { vertices } => {
                validate_convex_vertices(
                    &format!("scene.bodies[{body_index}].shape.vertices"),
                    vertices,
                )?;
                Ok(vec![ColliderBundle::new(SharedShape::convex_polygon(
                    points_from_arrays(vertices),
                ))])
            }
            Self::Compound { pieces } => {
                validate_compound_pieces(body_index, pieces)?;
                Ok(pieces
                    .iter()
                    .map(SceneCompoundPieceFixture::to_collider_bundle)
                    .collect())
            }
            Self::ConcavePolygon { .. } => Err(LabError::World(format!(
                "scene.bodies[{body_index}].shape: concave_polygon is not supported directly; use compound with convex pieces"
            ))),
        }
    }

    fn collider_count(&self) -> usize {
        match self {
            Self::Compound { pieces } => pieces.len(),
            _ => 1,
        }
    }
}

pub fn instantiate_scene_fixture(fixture: &SceneRecipeFixture) -> LabResult<World> {
    instantiate_scene_fixture_with_provenance(fixture).map(|result| result.world)
}

pub(crate) struct InstantiatedSceneFixture {
    pub(crate) world: World,
    pub(crate) compound_provenance: Vec<CompoundProvenance>,
}

pub(crate) fn instantiate_scene_fixture_with_provenance(
    fixture: &SceneRecipeFixture,
) -> LabResult<InstantiatedSceneFixture> {
    let result = fixture.to_world_recipe().and_then(|recipe| {
        recipe
            .instantiate_with_context()
            .map_err(|error| LabError::World(format!("{}: {}", error.path, error.error.error)))
    })?;

    let compound_provenance = build_compound_provenance(fixture, &result.created);
    Ok(InstantiatedSceneFixture {
        world: result.world,
        compound_provenance,
    })
}

fn default_scene_recipe_schema_version() -> u32 {
    SCENE_RECIPE_SCHEMA_VERSION
}

fn default_fixture_gravity() -> [f32; 2] {
    [0.0, 9.8]
}

fn default_fixture_enable_sleep() -> bool {
    true
}

fn default_fixture_can_sleep() -> bool {
    true
}

fn default_fixture_density() -> f32 {
    1.0
}

fn point_from_array([x, y]: [f32; 2]) -> Point {
    Point::new(x, y)
}

fn validate_compound_pieces(
    body_index: usize,
    pieces: &[SceneCompoundPieceFixture],
) -> LabResult<()> {
    if pieces.is_empty() {
        return Err(LabError::World(format!(
            "scene.bodies[{body_index}].shape.pieces: compound must contain at least one piece"
        )));
    }

    for (piece_index, piece) in pieces.iter().enumerate() {
        piece.validate(body_index, piece_index)?;
    }

    Ok(())
}

/// The scene fixture owns stable authoring errors for the validated fixture
/// shape fields below, so these failures do not bounce back from recipe paths.
fn validate_convex_vertices(path: &str, vertices: &[[f32; 2]]) -> LabResult<()> {
    let has_enough_vertices = vertices.len() >= 3 && distinct_vertex_count(vertices) >= 3;
    let vertices_are_finite = polygon_vertices_are_finite(vertices);
    let has_no_zero_length_edges = polygon_has_no_zero_length_edges(vertices);
    let has_area = polygon_twice_area(vertices).abs() > f32::EPSILON;

    if !has_enough_vertices || !vertices_are_finite || !has_no_zero_length_edges || !has_area {
        Err(LabError::World(format!(
            "{path}: convex_polygon requires at least 3 non-degenerate vertices"
        )))
    } else if polygon_is_convex(vertices) {
        Ok(())
    } else {
        Err(LabError::World(format!(
            "{path}: convex_polygon requires convex vertices"
        )))
    }
}

fn validate_circle_radius(path: &str, radius: f32) -> LabResult<()> {
    if radius.is_finite() && radius > 0.0 {
        Ok(())
    } else {
        Err(LabError::World(format!(
            "{path}: circle radius must be finite and > 0"
        )))
    }
}

fn validate_rect_size(path: &str, width: f32, height: f32) -> LabResult<()> {
    validate_positive_finite_scalar(&format!("{path}.width"), width, "rect width")?;
    validate_positive_finite_scalar(&format!("{path}.height"), height, "rect height")
}

fn validate_local_pose(path: &str, [x, y, angle]: [f32; 3]) -> LabResult<()> {
    validate_finite_scalar(&format!("{path}.x"), x, "local_pose.x")?;
    validate_finite_scalar(&format!("{path}.y"), y, "local_pose.y")?;
    validate_finite_scalar(&format!("{path}.angle"), angle, "local_pose.angle")
}

fn validate_positive_finite_scalar(path: &str, value: f32, label: &str) -> LabResult<()> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(LabError::World(format!(
            "{path}: {label} must be finite and > 0"
        )))
    }
}

fn validate_finite_scalar(path: &str, value: f32, label: &str) -> LabResult<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(LabError::World(format!("{path}: {label} must be finite")))
    }
}

fn distinct_vertex_count(vertices: &[[f32; 2]]) -> usize {
    let mut distinct = Vec::with_capacity(vertices.len());
    for vertex in vertices {
        if !distinct.iter().any(|existing| existing == vertex) {
            distinct.push(*vertex);
        }
    }
    distinct.len()
}

fn polygon_vertices_are_finite(vertices: &[[f32; 2]]) -> bool {
    vertices.iter().all(|[x, y]| x.is_finite() && y.is_finite())
}

fn polygon_has_no_zero_length_edges(vertices: &[[f32; 2]]) -> bool {
    if vertices.len() < 2 {
        return true;
    }

    for index in 0..vertices.len() {
        let current = vertices[index];
        let next = vertices[(index + 1) % vertices.len()];
        let edge_x = next[0] - current[0];
        let edge_y = next[1] - current[1];
        if edge_x.abs() <= f32::EPSILON && edge_y.abs() <= f32::EPSILON {
            return false;
        }
    }

    true
}

fn polygon_twice_area(vertices: &[[f32; 2]]) -> f32 {
    if vertices.len() < 3 {
        return 0.0;
    }

    let mut twice_area = 0.0;
    for index in 0..vertices.len() {
        let [x1, y1] = vertices[index];
        let [x2, y2] = vertices[(index + 1) % vertices.len()];
        twice_area += x1 * y2 - x2 * y1;
    }
    twice_area
}

/// "Convex" here means the authored loop turns consistently around the shape.
/// Collinear edges are tolerated so authors can keep explicit seam vertices.
fn polygon_is_convex(vertices: &[[f32; 2]]) -> bool {
    let mut winding_sign = 0.0_f32;

    for index in 0..vertices.len() {
        let previous = vertices[(index + vertices.len() - 1) % vertices.len()];
        let current = vertices[index];
        let next = vertices[(index + 1) % vertices.len()];
        let turn = polygon_turn_cross(previous, current, next);
        if turn.abs() <= f32::EPSILON {
            continue;
        }

        if winding_sign == 0.0 {
            winding_sign = turn.signum();
            continue;
        }

        if turn.signum() != winding_sign {
            return false;
        }
    }

    true
}

fn polygon_turn_cross(previous: [f32; 2], current: [f32; 2], next: [f32; 2]) -> f32 {
    let incoming_x = current[0] - previous[0];
    let incoming_y = current[1] - previous[1];
    let outgoing_x = next[0] - current[0];
    let outgoing_y = next[1] - current[1];
    incoming_x * outgoing_y - incoming_y * outgoing_x
}

fn points_from_arrays(vertices: &[[f32; 2]]) -> Vec<Point> {
    vertices.iter().copied().map(point_from_array).collect()
}

fn pose_from_array([x, y, angle]: [f32; 3]) -> Pose {
    Pose::from_xy_angle(x, y, angle)
}

fn pose_to_array(pose: Option<[f32; 3]>) -> [f32; 3] {
    pose.unwrap_or([0.0, 0.0, 0.0])
}

fn falling_box_contact_fixture(gravity: [f32; 2]) -> SceneRecipeFixture {
    SceneRecipeFixture {
        schema_version: SCENE_RECIPE_SCHEMA_VERSION,
        world: SceneFixtureWorld {
            gravity,
            enable_sleep: true,
        },
        bodies: vec![
            SceneBodyFixture {
                body_type: BodyType::Static,
                pose: [0.0, 2.0, 0.0],
                linear_velocity: [0.0, 0.0],
                can_sleep: false,
                shape: SceneShapeFixture::Rect {
                    width: 8.0,
                    height: 0.5,
                },
                material: MaterialPreset::Default,
                filter: CollisionLayerPreset::Default,
                density: default_fixture_density(),
                is_sensor: false,
            },
            SceneBodyFixture {
                body_type: BodyType::Dynamic,
                pose: [0.0, -2.0, 0.0],
                linear_velocity: [0.0, 0.0],
                can_sleep: false,
                shape: SceneShapeFixture::Rect {
                    width: 1.0,
                    height: 1.0,
                },
                material: MaterialPreset::Default,
                filter: CollisionLayerPreset::Default,
                density: default_fixture_density(),
                is_sensor: false,
            },
        ],
        joints: Vec::new(),
    }
}

fn compound_provenance_fixture() -> SceneRecipeFixture {
    SceneRecipeFixture {
        schema_version: SCENE_RECIPE_SCHEMA_VERSION,
        world: SceneFixtureWorld {
            gravity: [0.0, 0.0],
            enable_sleep: true,
        },
        bodies: vec![
            SceneBodyFixture {
                body_type: BodyType::Static,
                pose: [0.0, 2.2, 0.0],
                linear_velocity: [0.0, 0.0],
                can_sleep: false,
                shape: SceneShapeFixture::Rect {
                    width: 8.0,
                    height: 0.4,
                },
                material: MaterialPreset::Rough,
                filter: CollisionLayerPreset::StaticGeometry,
                density: default_fixture_density(),
                is_sensor: false,
            },
            SceneBodyFixture {
                body_type: BodyType::Dynamic,
                pose: [0.6, 0.45, 0.2],
                linear_velocity: [0.0, 0.0],
                can_sleep: true,
                shape: SceneShapeFixture::Compound {
                    pieces: vec![
                        SceneCompoundPieceFixture {
                            shape: SceneCompoundPieceShapeFixture::Rect {
                                width: 1.4,
                                height: 0.4,
                            },
                            local_pose: Some([0.0, 0.0, 0.0]),
                        },
                        SceneCompoundPieceFixture {
                            shape: SceneCompoundPieceShapeFixture::Circle { radius: 0.25 },
                            local_pose: Some([0.9, -0.2, 0.0]),
                        },
                        SceneCompoundPieceFixture {
                            shape: SceneCompoundPieceShapeFixture::ConvexPolygon {
                                vertices: vec![[-0.4, 0.0], [0.0, -0.45], [0.45, 0.1], [0.0, 0.5]],
                            },
                            local_pose: Some([-0.85, 0.3, -0.35]),
                        },
                    ],
                },
                material: MaterialPreset::Sticky,
                filter: CollisionLayerPreset::DynamicBody,
                density: 1.75,
                is_sensor: false,
            },
        ],
        joints: Vec::new(),
    }
}

fn build_compound_provenance(
    fixture: &SceneRecipeFixture,
    created: &WorldCommandReport,
) -> Vec<CompoundProvenance> {
    let mut collider_cursor = 0usize;
    let mut provenance = Vec::new();

    for (body_index, body) in fixture.bodies.iter().enumerate() {
        let body_handle = created.body_handles.get(body_index).copied();
        if let SceneShapeFixture::Compound { pieces } = &body.shape {
            provenance.push(CompoundProvenance {
                authored_body_index: body_index,
                body_handle,
                validation_path: format!("scene.bodies[{body_index}].shape.pieces"),
                inherited_material: body.material,
                inherited_filter: body.filter,
                inherited_density: body.density,
                inherited_is_sensor: body.is_sensor,
                pieces: pieces
                    .iter()
                    .enumerate()
                    .map(|(piece_index, piece)| CompoundProvenancePiece {
                        generated_piece_index: piece_index,
                        collider_handle: created
                            .collider_handles
                            .get(collider_cursor + piece_index)
                            .copied(),
                        validation_path: format!(
                            "scene.bodies[{body_index}].shape.pieces[{piece_index}]"
                        ),
                        local_pose: pose_to_array(piece.local_pose),
                    })
                    .collect(),
            });
        }
        collider_cursor += body.shape.collider_count();
    }

    provenance
}

pub(crate) struct BuiltScenario {
    pub(crate) world: World,
    pub(crate) compound_provenance: Vec<CompoundProvenance>,
}

pub(crate) fn build_scenario(
    id: ScenarioId,
    overrides: &ScenarioOverrides,
) -> LabResult<BuiltScenario> {
    let gravity = overrides
        .gravity
        .map(|[x, y]| Vector::new(x, y))
        .unwrap_or_else(|| Vector::new(0.0, 9.8));
    let mut world = World::new(WorldDesc {
        gravity,
        enable_sleep: true,
    });

    match id {
        ScenarioId::FallingBoxContact => {
            world = instantiate_scene_fixture(&falling_box_contact_fixture([
                gravity.x(),
                gravity.y(),
            ]))?;
        }
        ScenarioId::Stack4 => {
            add_box(&mut world, BodyType::Static, 0.0, 2.5, 10.0, 0.5)?;
            for index in 0..4 {
                add_box(
                    &mut world,
                    BodyType::Dynamic,
                    0.0,
                    1.7 - index as f32,
                    0.9,
                    0.9,
                )?;
            }
        }
        ScenarioId::JointAnchor => {
            world = World::new(WorldDesc {
                gravity: Vector::default(),
                enable_sleep: false,
            });
            let body = add_box(&mut world, BodyType::Dynamic, 2.0, 0.0, 0.8, 0.8)?;
            world
                .create_joint(JointDesc::WorldAnchor(WorldAnchorJointDesc {
                    body,
                    world_anchor: Point::new(0.0, 0.0),
                    stiffness: 4.0,
                    damping: 0.2,
                    ..WorldAnchorJointDesc::default()
                }))
                .map_err(|error| LabError::World(error.to_string()))?;
        }
        ScenarioId::BroadphaseSparse => {
            world = World::new(WorldDesc {
                gravity: Vector::default(),
                enable_sleep: false,
            });
            add_box(&mut world, BodyType::Static, 0.0, 0.0, 1.0, 1.0)?;
            add_box(&mut world, BodyType::Static, 0.75, 0.0, 1.0, 1.0)?;
            add_box(&mut world, BodyType::Static, 5.0, 0.0, 1.0, 1.0)?;
            add_box(&mut world, BodyType::Static, 10.0, 0.0, 1.0, 1.0)?;
            add_box(&mut world, BodyType::Static, 15.0, 0.0, 1.0, 1.0)?;
        }
        ScenarioId::SatPolygon => {
            world = World::new(WorldDesc {
                gravity: Vector::default(),
                enable_sleep: false,
            });
            add_box(&mut world, BodyType::Static, 0.0, 0.0, 2.0, 2.0)?;
            let body = world
                .create_body(BodyDesc {
                    body_type: BodyType::Static,
                    pose: Pose::from_xy_angle(1.5, 0.0, 0.0),
                    can_sleep: false,
                    ..BodyDesc::default()
                })
                .map_err(|error| LabError::World(error.to_string()))?;
            world
                .create_collider(
                    body,
                    ColliderDesc {
                        shape: SharedShape::convex_polygon(vec![
                            Point::new(-1.0, -1.0),
                            Point::new(1.0, -1.0),
                            Point::new(1.0, 1.0),
                            Point::new(-1.0, 1.0),
                        ]),
                        ..ColliderDesc::default()
                    },
                )
                .map_err(|error| LabError::World(error.to_string()))?;
        }
        ScenarioId::CompoundProvenance => {
            let instantiated =
                instantiate_scene_fixture_with_provenance(&compound_provenance_fixture())?;
            return Ok(BuiltScenario {
                world: instantiated.world,
                compound_provenance: instantiated.compound_provenance,
            });
        }
        ScenarioId::CcdFastCircleWall => {
            world = World::new(WorldDesc {
                gravity: Vector::default(),
                enable_sleep: false,
            });
            add_box(&mut world, BodyType::Static, 0.0, 0.0, 0.1, 10.0)?;
            add_circle(
                &mut world,
                BodyType::Dynamic,
                -1.0,
                0.0,
                0.05,
                Vector::new(200.0, 0.0),
            )?;
        }
        ScenarioId::CcdFastConvexWalls => {
            world = World::new(WorldDesc {
                gravity: Vector::default(),
                enable_sleep: false,
            });
            add_box(&mut world, BodyType::Static, 0.0, 0.0, 0.1, 10.0)?;
            add_box(&mut world, BodyType::Static, 0.8, 0.0, 0.1, 10.0)?;
            add_box_with_velocity(
                &mut world,
                BodyType::Dynamic,
                -1.0,
                0.0,
                0.1,
                0.1,
                Vector::new(200.0, 0.0),
            )?;
        }
        ScenarioId::CcdDynamicConvexPair => {
            world = World::new(WorldDesc {
                gravity: Vector::default(),
                enable_sleep: false,
            });
            add_box_with_velocity(
                &mut world,
                BodyType::Dynamic,
                -1.0,
                0.0,
                0.1,
                0.1,
                Vector::new(200.0, 0.0),
            )?;
            add_box_with_velocity(
                &mut world,
                BodyType::Dynamic,
                1.0,
                0.0,
                0.1,
                0.1,
                Vector::new(-200.0, 0.0),
            )?;
        }
    }

    Ok(BuiltScenario {
        world,
        compound_provenance: Vec::new(),
    })
}

fn add_box(
    world: &mut World,
    body_type: BodyType,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> LabResult<BodyHandle> {
    let body = world
        .create_body(BodyDesc {
            body_type,
            pose: Pose::from_xy_angle(x, y, 0.0),
            can_sleep: false,
            ..BodyDesc::default()
        })
        .map_err(|error| LabError::World(error.to_string()))?;
    world
        .create_collider(
            body,
            ColliderDesc {
                shape: SharedShape::rect(width, height),
                ..ColliderDesc::default()
            },
        )
        .map_err(|error| LabError::World(error.to_string()))?;
    Ok(body)
}

fn add_box_with_velocity(
    world: &mut World,
    body_type: BodyType,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    linear_velocity: Vector,
) -> LabResult<BodyHandle> {
    let body = world
        .create_body(BodyDesc {
            body_type,
            pose: Pose::from_xy_angle(x, y, 0.0),
            linear_velocity,
            can_sleep: false,
            ..BodyDesc::default()
        })
        .map_err(|error| LabError::World(error.to_string()))?;
    world
        .create_collider(
            body,
            ColliderDesc {
                shape: SharedShape::rect(width, height),
                ..ColliderDesc::default()
            },
        )
        .map_err(|error| LabError::World(error.to_string()))?;
    Ok(body)
}

fn add_circle(
    world: &mut World,
    body_type: BodyType,
    x: f32,
    y: f32,
    radius: f32,
    linear_velocity: Vector,
) -> LabResult<BodyHandle> {
    let body = world
        .create_body(BodyDesc {
            body_type,
            pose: Pose::from_xy_angle(x, y, 0.0),
            linear_velocity,
            can_sleep: false,
            ..BodyDesc::default()
        })
        .map_err(|error| LabError::World(error.to_string()))?;
    world
        .create_collider(
            body,
            ColliderDesc {
                shape: SharedShape::circle(radius),
                ..ColliderDesc::default()
            },
        )
        .map_err(|error| LabError::World(error.to_string()))?;
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialized_scene_fixture_round_trips_into_a_recipe_world() {
        let json = r#"
        {
          "world": { "gravity": [0.0, 9.8], "enable_sleep": true },
          "bodies": [
            {
              "body_type": "static",
              "pose": [0.0, 2.0, 0.0],
              "can_sleep": false,
              "shape": { "type": "rect", "width": 8.0, "height": 0.5 },
              "material": "rough",
              "filter": "static_geometry"
            },
            {
              "body_type": "dynamic",
              "pose": [0.0, -2.0, 0.0],
              "linear_velocity": [1.0, 0.0],
              "can_sleep": false,
              "shape": { "type": "circle", "radius": 0.5 },
              "material": "bouncy",
              "filter": "dynamic_body"
            }
          ]
        }
        "#;

        let fixture: SceneRecipeFixture =
            serde_json::from_str(json).expect("fixture json should deserialize");
        assert_eq!(fixture.bodies[1].material, MaterialPreset::Bouncy);

        let encoded = serde_json::to_string(&fixture).expect("fixture should serialize");
        assert!(
            encoded.contains("\"bouncy\""),
            "preset names should remain fixture-readable"
        );

        let world = instantiate_scene_fixture(&fixture).expect("fixture should create a world");
        let bodies: Vec<_> = world.bodies().collect();
        assert_eq!(bodies.len(), 2);

        let ball_collider = world
            .colliders_for_body(bodies[1])
            .expect("dynamic fixture body should resolve")
            .next()
            .expect("dynamic fixture body should have one collider");
        assert_eq!(
            world
                .collider(ball_collider)
                .expect("fixture collider should resolve")
                .material(),
            Material::preset(MaterialPreset::Bouncy)
        );
    }

    #[test]
    fn falling_box_contact_fixture_preserves_builtin_scenario_contract() {
        let fixture = falling_box_contact_fixture(default_fixture_gravity());
        let encoded = serde_json::to_string(&fixture).expect("fixture should serialize");
        let decoded: SceneRecipeFixture =
            serde_json::from_str(&encoded).expect("fixture should deserialize");
        let fixture_world = instantiate_scene_fixture(&decoded).expect("fixture should build");
        assert_eq!(fixture_world.desc().gravity, Vector::new(0.0, 9.8));
        assert!(fixture_world.desc().enable_sleep);

        let bodies: Vec<_> = fixture_world.bodies().collect();
        assert_eq!(bodies.len(), 2);
        assert_eq!(
            fixture_world
                .body(bodies[0])
                .expect("floor body should resolve")
                .body_type(),
            BodyType::Static
        );
        assert_eq!(
            fixture_world
                .body(bodies[0])
                .expect("floor body should resolve")
                .pose(),
            Pose::from_xy_angle(0.0, 2.0, 0.0)
        );
        assert_eq!(
            fixture_world
                .body(bodies[1])
                .expect("falling body should resolve")
                .body_type(),
            BodyType::Dynamic
        );
        assert_eq!(
            fixture_world
                .body(bodies[1])
                .expect("falling body should resolve")
                .pose(),
            Pose::from_xy_angle(0.0, -2.0, 0.0)
        );

        let collider_counts: Vec<_> = bodies
            .iter()
            .copied()
            .map(|body| {
                fixture_world
                    .colliders_for_body(body)
                    .expect("fixture body should resolve")
                    .count()
            })
            .collect();
        assert_eq!(collider_counts, vec![1, 1]);

        let fixture_collider_shapes: Vec<_> = bodies
            .into_iter()
            .map(|body| {
                let collider = fixture_world
                    .colliders_for_body(body)
                    .expect("fixture body should resolve")
                    .next()
                    .expect("fixture body should have one collider");
                fixture_world
                    .collider(collider)
                    .expect("fixture collider should resolve")
                    .shape()
                    .clone()
            })
            .collect();
        assert_eq!(
            fixture_collider_shapes,
            vec![SharedShape::rect(8.0, 0.5), SharedShape::rect(1.0, 1.0)]
        );
    }

    #[test]
    fn falling_box_contact_builtin_uses_serialized_fixture_path() {
        let builtin = build_scenario(ScenarioId::FallingBoxContact, &ScenarioOverrides::default())
            .expect("builtin scenario should build");
        let bodies: Vec<_> = builtin.world.bodies().collect();
        assert_eq!(bodies.len(), 2);
        assert_eq!(
            builtin
                .world
                .body(bodies[0])
                .expect("builtin floor should resolve")
                .body_type(),
            BodyType::Static
        );
        assert_eq!(
            builtin
                .world
                .body(bodies[1])
                .expect("builtin falling body should resolve")
                .body_type(),
            BodyType::Dynamic
        );
        assert_eq!(
            bodies
                .into_iter()
                .map(|body| {
                    builtin
                        .world
                        .colliders_for_body(body)
                        .expect("builtin body should resolve")
                        .count()
                })
                .collect::<Vec<_>>(),
            vec![1, 1]
        );
    }

    #[test]
    fn compound_scene_fixture_builds_ordered_colliders_with_inherited_body_semantics() {
        let json = r#"
        {
          "schema_version": 1,
          "world": { "gravity": [0.0, 0.0], "enable_sleep": false },
          "bodies": [
            {
              "body_type": "dynamic",
              "pose": [3.0, -2.0, 0.75],
              "linear_velocity": [1.0, -0.5],
              "can_sleep": false,
              "shape": {
                "type": "compound",
                "pieces": [
                  {
                    "shape": { "type": "rect", "width": 2.0, "height": 1.0 },
                    "local_pose": [-1.0, 0.0, 0.0]
                  },
                  {
                    "shape": {
                      "type": "convex_polygon",
                      "vertices": [[-0.5, -0.25], [0.75, -0.1], [0.25, 0.8]]
                    },
                    "local_pose": [1.5, 0.25, 0.3]
                  },
                  {
                    "shape": { "type": "circle", "radius": 0.5 }
                  }
                ]
              },
              "material": "bouncy",
              "filter": "sensor",
              "density": 2.5,
              "is_sensor": true
            }
          ]
        }
        "#;

        let fixture: SceneRecipeFixture =
            serde_json::from_str(json).expect("fixture json should deserialize");
        let world = instantiate_scene_fixture(&fixture).expect("fixture should create a world");

        let body = world
            .bodies()
            .next()
            .expect("fixture should create one compound body");
        let colliders: Vec<_> = world
            .colliders_for_body(body)
            .expect("compound body should resolve")
            .collect();
        assert_eq!(colliders.len(), 3);

        let expected = [
            (
                SharedShape::rect(2.0, 1.0),
                Pose::from_xy_angle(-1.0, 0.0, 0.0),
            ),
            (
                SharedShape::convex_polygon(vec![
                    (-0.5, -0.25).into(),
                    (0.75, -0.1).into(),
                    (0.25, 0.8).into(),
                ]),
                Pose::from_xy_angle(1.5, 0.25, 0.3),
            ),
            (SharedShape::circle(0.5), Pose::default()),
        ];
        for (handle, (shape, local_pose)) in colliders.into_iter().zip(expected) {
            let collider = world
                .collider(handle)
                .expect("compound collider should resolve");
            assert_eq!(collider.shape(), &shape);
            assert_eq!(collider.local_pose(), local_pose);
            assert_eq!(collider.density(), 2.5);
            assert_eq!(
                collider.material(),
                Material::preset(MaterialPreset::Bouncy)
            );
            assert_eq!(
                collider.filter(),
                CollisionFilter::preset(CollisionLayerPreset::Sensor)
            );
            assert!(collider.is_sensor());
        }
    }

    #[test]
    fn empty_compound_scene_fixture_fails_before_world_instantiation_with_stable_piece_path() {
        let json = r#"
        {
          "schema_version": 1,
          "bodies": [
            {
              "body_type": "dynamic",
              "shape": {
                "type": "compound",
                "pieces": []
              }
            }
          ]
        }
        "#;

        let fixture: SceneRecipeFixture =
            serde_json::from_str(json).expect("fixture json should deserialize");
        let build_error = fixture
            .to_world_recipe()
            .expect_err("empty compound authoring should fail before instantiation");
        assert_eq!(
            build_error.to_string(),
            "world setup failed: scene.bodies[0].shape.pieces: compound must contain at least one piece"
        );

        let instantiate_error = instantiate_scene_fixture(&fixture)
            .expect_err("empty compound fixture should also fail through the lab entrypoint");
        assert_eq!(instantiate_error.to_string(), build_error.to_string());
    }

    #[test]
    fn invalid_compound_piece_convex_polygon_fails_with_stable_piece_index_path() {
        let json = r#"
        {
          "schema_version": 1,
          "bodies": [
            {
              "body_type": "dynamic",
              "shape": {
                "type": "compound",
                "pieces": [
                  {
                    "shape": {
                      "type": "convex_polygon",
                      "vertices": [[0.0, 0.0], [1.0, 0.0]]
                    }
                  }
                ]
              }
            }
          ]
        }
        "#;

        let fixture: SceneRecipeFixture =
            serde_json::from_str(json).expect("fixture json should deserialize");
        let build_error = fixture
            .to_world_recipe()
            .expect_err("invalid compound piece should fail before instantiation");
        assert_eq!(
            build_error.to_string(),
            "world setup failed: scene.bodies[0].shape.pieces[0].shape.vertices: convex_polygon requires at least 3 non-degenerate vertices"
        );

        let instantiate_error = instantiate_scene_fixture(&fixture)
            .expect_err("invalid compound piece should also fail through the lab entrypoint");
        assert_eq!(instantiate_error.to_string(), build_error.to_string());
    }

    #[test]
    fn concave_compound_piece_convex_polygon_fails_with_stable_piece_index_path() {
        let json = r#"
        {
          "schema_version": 1,
          "bodies": [
            {
              "body_type": "dynamic",
              "shape": {
                "type": "compound",
                "pieces": [
                  {
                    "shape": {
                      "type": "convex_polygon",
                      "vertices": [[-1.0, -1.0], [1.0, -1.0], [0.0, 0.0], [1.0, 1.0], [-1.0, 1.0]]
                    }
                  }
                ]
              }
            }
          ]
        }
        "#;

        let fixture: SceneRecipeFixture =
            serde_json::from_str(json).expect("fixture json should deserialize");
        let build_error = fixture
            .to_world_recipe()
            .expect_err("concave compound piece should fail before instantiation");
        assert_eq!(
            build_error.to_string(),
            "world setup failed: scene.bodies[0].shape.pieces[0].shape.vertices: convex_polygon requires convex vertices"
        );

        let instantiate_error = instantiate_scene_fixture(&fixture)
            .expect_err("concave compound fixture should also fail through the lab entrypoint");
        assert_eq!(instantiate_error.to_string(), build_error.to_string());
    }

    #[test]
    fn top_level_convex_polygon_scene_fixture_rejects_concave_vertices_with_stable_fixture_path() {
        let json = r#"
        {
          "schema_version": 1,
          "bodies": [
            {
              "body_type": "dynamic",
              "shape": {
                "type": "convex_polygon",
                "vertices": [[-1.0, -1.0], [1.0, -1.0], [0.0, 0.0], [1.0, 1.0], [-1.0, 1.0]]
              }
            }
          ]
        }
        "#;

        let fixture: SceneRecipeFixture =
            serde_json::from_str(json).expect("fixture json should deserialize");
        let build_error = fixture
            .to_world_recipe()
            .expect_err("top-level concave convex_polygon should fail before instantiation");
        assert_eq!(
            build_error.to_string(),
            "world setup failed: scene.bodies[0].shape.vertices: convex_polygon requires convex vertices"
        );

        let instantiate_error = instantiate_scene_fixture(&fixture).expect_err(
            "top-level convex_polygon fixture should also fail through the lab entrypoint",
        );
        assert_eq!(instantiate_error.to_string(), build_error.to_string());
    }

    #[test]
    fn top_level_convex_polygon_scene_fixture_rejects_closing_zero_length_edge_with_stable_fixture_path(
    ) {
        let json = r#"
        {
          "schema_version": 1,
          "bodies": [
            {
              "body_type": "dynamic",
              "shape": {
                "type": "convex_polygon",
                "vertices": [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [0.0, 0.0]]
              }
            }
          ]
        }
        "#;

        let fixture: SceneRecipeFixture =
            serde_json::from_str(json).expect("fixture json should deserialize");
        let build_error = fixture.to_world_recipe().expect_err(
            "top-level convex_polygon with closing zero-length edge should fail before instantiation",
        );
        assert_eq!(
            build_error.to_string(),
            "world setup failed: scene.bodies[0].shape.vertices: convex_polygon requires at least 3 non-degenerate vertices"
        );

        let instantiate_error = instantiate_scene_fixture(&fixture).expect_err(
            "top-level convex_polygon fixture should also fail through the lab entrypoint",
        );
        assert_eq!(instantiate_error.to_string(), build_error.to_string());
    }

    #[test]
    fn compound_piece_convex_polygon_rejects_adjacent_duplicate_vertices_with_stable_piece_index_path(
    ) {
        let json = r#"
        {
          "schema_version": 1,
          "bodies": [
            {
              "body_type": "dynamic",
              "shape": {
                "type": "compound",
                "pieces": [
                  {
                    "shape": {
                      "type": "convex_polygon",
                      "vertices": [[0.0, 0.0], [1.0, 0.0], [1.0, 0.0], [0.0, 1.0]]
                    }
                  }
                ]
              }
            }
          ]
        }
        "#;

        let fixture: SceneRecipeFixture =
            serde_json::from_str(json).expect("fixture json should deserialize");
        let build_error = fixture.to_world_recipe().expect_err(
            "compound piece with adjacent duplicate vertices should fail before instantiation",
        );
        assert_eq!(
            build_error.to_string(),
            "world setup failed: scene.bodies[0].shape.pieces[0].shape.vertices: convex_polygon requires at least 3 non-degenerate vertices"
        );

        let instantiate_error = instantiate_scene_fixture(&fixture)
            .expect_err("invalid compound piece should also fail through the lab entrypoint");
        assert_eq!(instantiate_error.to_string(), build_error.to_string());
    }

    #[test]
    fn compound_piece_circle_radius_scene_fixture_fails_with_stable_piece_shape_path() {
        let json = r#"
        {
          "schema_version": 1,
          "bodies": [
            {
              "body_type": "dynamic",
              "shape": {
                "type": "compound",
                "pieces": [
                  {
                    "shape": { "type": "circle", "radius": -0.5 }
                  }
                ]
              }
            }
          ]
        }
        "#;

        let fixture: SceneRecipeFixture =
            serde_json::from_str(json).expect("fixture json should deserialize");
        let build_error = fixture
            .to_world_recipe()
            .expect_err("invalid circle piece should fail before instantiation");
        assert_eq!(
            build_error.to_string(),
            "world setup failed: scene.bodies[0].shape.pieces[0].shape.radius: circle radius must be finite and > 0"
        );

        let instantiate_error = instantiate_scene_fixture(&fixture)
            .expect_err("invalid circle piece should also fail through the lab entrypoint");
        assert_eq!(instantiate_error.to_string(), build_error.to_string());
    }

    #[test]
    fn compound_piece_nonfinite_circle_radius_scene_fixture_fails_with_stable_piece_shape_path() {
        let fixture = SceneRecipeFixture {
            schema_version: SCENE_RECIPE_SCHEMA_VERSION,
            world: SceneFixtureWorld::default(),
            bodies: vec![SceneBodyFixture {
                body_type: BodyType::Dynamic,
                pose: [0.0, 0.0, 0.0],
                linear_velocity: [0.0, 0.0],
                can_sleep: true,
                shape: SceneShapeFixture::Compound {
                    pieces: vec![SceneCompoundPieceFixture {
                        shape: SceneCompoundPieceShapeFixture::Circle {
                            radius: f32::INFINITY,
                        },
                        local_pose: None,
                    }],
                },
                material: MaterialPreset::Default,
                filter: CollisionLayerPreset::Default,
                density: 1.0,
                is_sensor: false,
            }],
            joints: Vec::new(),
        };

        let build_error = fixture
            .to_world_recipe()
            .expect_err("non-finite circle piece should fail before instantiation");
        assert_eq!(
            build_error.to_string(),
            "world setup failed: scene.bodies[0].shape.pieces[0].shape.radius: circle radius must be finite and > 0"
        );

        let instantiate_error = instantiate_scene_fixture(&fixture)
            .expect_err("non-finite circle piece should also fail through the lab entrypoint");
        assert_eq!(instantiate_error.to_string(), build_error.to_string());
    }

    #[test]
    fn compound_piece_rect_size_scene_fixture_fails_with_stable_piece_shape_path() {
        let json = r#"
        {
          "schema_version": 1,
          "bodies": [
            {
              "body_type": "dynamic",
              "shape": {
                "type": "compound",
                "pieces": [
                  {
                    "shape": { "type": "rect", "width": 1.0, "height": 0.0 }
                  }
                ]
              }
            }
          ]
        }
        "#;

        let fixture: SceneRecipeFixture =
            serde_json::from_str(json).expect("fixture json should deserialize");
        let build_error = fixture
            .to_world_recipe()
            .expect_err("invalid rect piece should fail before instantiation");
        assert_eq!(
            build_error.to_string(),
            "world setup failed: scene.bodies[0].shape.pieces[0].shape.height: rect height must be finite and > 0"
        );

        let instantiate_error = instantiate_scene_fixture(&fixture)
            .expect_err("invalid rect piece should also fail through the lab entrypoint");
        assert_eq!(instantiate_error.to_string(), build_error.to_string());
    }

    #[test]
    fn compound_piece_local_pose_scene_fixture_fails_with_stable_piece_local_pose_path() {
        let fixture = SceneRecipeFixture {
            schema_version: SCENE_RECIPE_SCHEMA_VERSION,
            world: SceneFixtureWorld::default(),
            bodies: vec![SceneBodyFixture {
                body_type: BodyType::Dynamic,
                pose: [0.0, 0.0, 0.0],
                linear_velocity: [0.0, 0.0],
                can_sleep: true,
                shape: SceneShapeFixture::Compound {
                    pieces: vec![SceneCompoundPieceFixture {
                        shape: SceneCompoundPieceShapeFixture::Circle { radius: 0.5 },
                        local_pose: Some([0.0, 0.0, f32::INFINITY]),
                    }],
                },
                material: MaterialPreset::Default,
                filter: CollisionLayerPreset::Default,
                density: 1.0,
                is_sensor: false,
            }],
            joints: Vec::new(),
        };

        let build_error = fixture
            .to_world_recipe()
            .expect_err("non-finite local pose should fail before instantiation");
        assert_eq!(
            build_error.to_string(),
            "world setup failed: scene.bodies[0].shape.pieces[0].local_pose.angle: local_pose.angle must be finite"
        );

        let instantiate_error = instantiate_scene_fixture(&fixture)
            .expect_err("non-finite local pose should also fail through the lab entrypoint");
        assert_eq!(instantiate_error.to_string(), build_error.to_string());
    }

    #[test]
    fn direct_concave_scene_fixture_fails_before_world_instantiation() {
        let json = r#"
        {
          "schema_version": 1,
          "bodies": [
            {
              "body_type": "dynamic",
              "shape": {
                "type": "concave_polygon",
                "vertices": [[-1.0, -1.0], [1.0, -1.0], [1.0, 0.0], [0.0, 0.0], [0.0, 1.0], [-1.0, 1.0]]
              }
            }
          ]
        }
        "#;

        let fixture: SceneRecipeFixture =
            serde_json::from_str(json).expect("fixture json should deserialize");
        let build_error = fixture
            .to_world_recipe()
            .expect_err("direct concave authoring should fail before instantiation");
        assert_eq!(
            build_error.to_string(),
            "world setup failed: scene.bodies[0].shape: concave_polygon is not supported directly; use compound with convex pieces"
        );

        let instantiate_error = instantiate_scene_fixture(&fixture)
            .expect_err("concave fixture should also fail through the lab entrypoint");
        assert_eq!(instantiate_error.to_string(), build_error.to_string());
    }

    #[test]
    fn legacy_scene_fixture_json_defaults_schema_version_to_v1_and_builds_unchanged() {
        let json = r#"
        {
          "world": { "gravity": [0.0, 9.8], "enable_sleep": true },
          "bodies": [
            {
              "body_type": "dynamic",
              "shape": { "type": "circle", "radius": 0.5 }
            }
          ]
        }
        "#;

        let fixture: SceneRecipeFixture =
            serde_json::from_str(json).expect("fixture json should deserialize");

        assert_eq!(fixture.schema_version, SCENE_RECIPE_SCHEMA_VERSION);

        let world = instantiate_scene_fixture(&fixture).expect("legacy fixture should build");
        let body = world
            .bodies()
            .next()
            .expect("legacy fixture should create one body");
        let collider = world
            .colliders_for_body(body)
            .expect("legacy fixture body should resolve")
            .next()
            .expect("legacy fixture body should have one collider");
        assert_eq!(
            world
                .collider(collider)
                .expect("legacy fixture collider should resolve")
                .shape(),
            &SharedShape::circle(0.5)
        );
    }

    #[test]
    fn unsupported_scene_schema_version_fails_with_clear_error() {
        let fixture = SceneRecipeFixture {
            schema_version: SCENE_RECIPE_SCHEMA_VERSION + 1,
            world: SceneFixtureWorld::default(),
            bodies: vec![SceneBodyFixture {
                body_type: BodyType::Dynamic,
                pose: [0.0, 0.0, 0.0],
                linear_velocity: [0.0, 0.0],
                can_sleep: true,
                shape: SceneShapeFixture::Circle { radius: 0.5 },
                material: MaterialPreset::Default,
                filter: CollisionLayerPreset::Default,
                density: 1.0,
                is_sensor: false,
            }],
            joints: Vec::new(),
        };

        let error =
            instantiate_scene_fixture(&fixture).expect_err("unsupported schema should fail");
        assert_eq!(
            error.to_string(),
            format!(
                "unsupported scene schema version: {} (expected v{})",
                SCENE_RECIPE_SCHEMA_VERSION + 1,
                SCENE_RECIPE_SCHEMA_VERSION
            )
        );
    }

    #[test]
    fn scene_fixture_joints_round_trip_into_recipe_world() {
        let json = r#"
        {
          "schema_version": 1,
          "world": { "gravity": [0.0, 0.0], "enable_sleep": false },
          "bodies": [
            {
              "body_type": "static",
              "pose": [0.0, -1.0, 0.0],
              "shape": { "type": "rect", "width": 8.0, "height": 1.0 }
            },
            {
              "body_type": "dynamic",
              "pose": [0.0, 1.0, 0.0],
              "shape": { "type": "circle", "radius": 0.5 }
            }
          ],
          "joints": [
            {
              "type": "distance",
              "body_a": 0,
              "body_b": 1,
              "rest_length": 2.5,
              "stiffness": 3.0,
              "damping": 0.4,
              "local_anchor_a": [0.25, 0.0],
              "local_anchor_b": [-0.25, 0.0]
            },
            {
              "type": "world_anchor",
              "body": 1,
              "world_anchor": [0.0, 3.0],
              "local_anchor": [0.0, 0.5],
              "stiffness": 2.0,
              "damping": 0.1
            }
          ]
        }
        "#;

        let fixture: SceneRecipeFixture =
            serde_json::from_str(json).expect("fixture json should deserialize");
        let world = instantiate_scene_fixture(&fixture).expect("fixture should create a world");
        let joints: Vec<_> = world.joints().collect();
        assert_eq!(joints.len(), 2);

        match world
            .joint(joints[0])
            .expect("distance joint should resolve")
            .desc()
        {
            JointDesc::Distance(desc) => {
                assert_eq!(desc.rest_length, 2.5);
                assert_eq!(desc.stiffness, 3.0);
                assert_eq!(desc.damping, 0.4);
                assert_eq!(desc.local_anchor_a, Point::new(0.25, 0.0));
                assert_eq!(desc.local_anchor_b, Point::new(-0.25, 0.0));
            }
            other => panic!("expected distance joint, got {other:?}"),
        }

        match world
            .joint(joints[1])
            .expect("world-anchor joint should resolve")
            .desc()
        {
            JointDesc::WorldAnchor(desc) => {
                assert_eq!(desc.world_anchor, Point::new(0.0, 3.0));
                assert_eq!(desc.local_anchor, Point::new(0.0, 0.5));
                assert_eq!(desc.stiffness, 2.0);
                assert_eq!(desc.damping, 0.1);
            }
            other => panic!("expected world-anchor joint, got {other:?}"),
        }
    }

    #[test]
    fn scene_fixture_joint_optional_fields_preserve_core_defaults() {
        let json = r#"
        {
          "schema_version": 1,
          "bodies": [
            {
              "body_type": "static",
              "pose": [0.0, -1.0, 0.0],
              "shape": { "type": "rect", "width": 8.0, "height": 1.0 }
            },
            {
              "body_type": "dynamic",
              "pose": [0.0, 1.0, 0.0],
              "shape": { "type": "circle", "radius": 0.5 }
            }
          ],
          "joints": [
            { "type": "distance", "body_a": 0, "body_b": 1 },
            { "type": "world_anchor", "body": 1 }
          ]
        }
        "#;

        let fixture: SceneRecipeFixture =
            serde_json::from_str(json).expect("fixture json should deserialize");
        let world = instantiate_scene_fixture(&fixture).expect("fixture should create a world");
        let joints: Vec<_> = world.joints().collect();
        assert_eq!(joints.len(), 2);

        match world
            .joint(joints[0])
            .expect("distance joint should resolve")
            .desc()
        {
            JointDesc::Distance(desc) => {
                let expected = DistanceJointDesc::default();
                assert_eq!(desc.rest_length, expected.rest_length);
                assert_eq!(desc.stiffness, expected.stiffness);
                assert_eq!(desc.damping, expected.damping);
                assert_eq!(desc.local_anchor_a, expected.local_anchor_a);
                assert_eq!(desc.local_anchor_b, expected.local_anchor_b);
            }
            other => panic!("expected distance joint, got {other:?}"),
        }

        match world
            .joint(joints[1])
            .expect("world-anchor joint should resolve")
            .desc()
        {
            JointDesc::WorldAnchor(desc) => {
                let expected = WorldAnchorJointDesc::default();
                assert_eq!(desc.world_anchor, expected.world_anchor);
                assert_eq!(desc.local_anchor, expected.local_anchor);
                assert_eq!(desc.stiffness, expected.stiffness);
                assert_eq!(desc.damping, expected.damping);
            }
            other => panic!("expected world-anchor joint, got {other:?}"),
        }
    }

    #[test]
    fn scene_fixture_joint_body_reference_errors_keep_nested_recipe_paths() {
        let json = r#"
        {
          "schema_version": 1,
          "bodies": [
            {
              "body_type": "dynamic",
              "shape": { "type": "circle", "radius": 0.5 }
            }
          ],
          "joints": [
            { "type": "distance", "body_a": 0, "body_b": 3 }
          ]
        }
        "#;

        let fixture: SceneRecipeFixture =
            serde_json::from_str(json).expect("fixture json should deserialize");
        let error =
            instantiate_scene_fixture(&fixture).expect_err("invalid joint body should fail");
        assert_eq!(
            error.to_string(),
            "world setup failed: recipe.joints[0].desc.body_b: body handle does not belong to this world"
        );
    }
}
