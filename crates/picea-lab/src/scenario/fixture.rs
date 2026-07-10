use picea::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{LabError, LabResult};

use super::geometry::{
    decompose_concave_polygon, validate_circle_radius, validate_convex_vertices,
    validate_local_pose, validate_rect_size,
};

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
            .to_collider_bundles(body_index, self.body_type)?
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
    fn to_collider_bundles(
        &self,
        body_index: usize,
        body_type: BodyType,
    ) -> LabResult<Vec<ColliderBundle>> {
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
            Self::ConcavePolygon { vertices } => {
                if body_type != BodyType::Static {
                    return Err(LabError::World(format!(
                        "scene.bodies[{body_index}].shape: concave_polygon automatic decomposition is only supported for static bodies"
                    )));
                }
                decompose_concave_polygon(
                    &format!("scene.bodies[{body_index}].shape.vertices"),
                    vertices,
                )
                .map(|pieces| {
                    pieces
                        .into_iter()
                        .map(|piece| {
                            ColliderBundle::new(SharedShape::convex_polygon(points_from_arrays(
                                &piece,
                            )))
                        })
                        .collect()
                })
            }
        }
    }

    fn collider_count(&self) -> usize {
        match self {
            Self::Compound { pieces } => pieces.len(),
            Self::ConcavePolygon { vertices } => vertices.len().saturating_sub(2),
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

pub(super) fn default_fixture_gravity() -> [f32; 2] {
    [0.0, 9.8]
}

fn default_fixture_enable_sleep() -> bool {
    true
}

fn default_fixture_can_sleep() -> bool {
    true
}

pub(super) fn default_fixture_density() -> f32 {
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

fn points_from_arrays(vertices: &[[f32; 2]]) -> Vec<Point> {
    vertices.iter().copied().map(point_from_array).collect()
}

fn pose_from_array([x, y, angle]: [f32; 3]) -> Pose {
    Pose::from_xy_angle(x, y, angle)
}

fn pose_to_array(pose: Option<[f32; 3]>) -> [f32; 3] {
    pose.unwrap_or([0.0, 0.0, 0.0])
}

fn build_compound_provenance(
    fixture: &SceneRecipeFixture,
    created: &WorldCommandReport,
) -> Vec<CompoundProvenance> {
    let mut collider_cursor = 0usize;
    let mut provenance = Vec::new();

    for (body_index, body) in fixture.bodies.iter().enumerate() {
        let body_handle = created.body_handles.get(body_index).copied();
        match &body.shape {
            SceneShapeFixture::Compound { pieces } => {
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
            SceneShapeFixture::ConcavePolygon { vertices }
                if body.body_type == BodyType::Static =>
            {
                let piece_count = vertices.len().saturating_sub(2);
                provenance.push(CompoundProvenance {
                    authored_body_index: body_index,
                    body_handle,
                    validation_path: format!("scene.bodies[{body_index}].shape.vertices"),
                    inherited_material: body.material,
                    inherited_filter: body.filter,
                    inherited_density: body.density,
                    inherited_is_sensor: body.is_sensor,
                    pieces: (0..piece_count)
                        .map(|piece_index| CompoundProvenancePiece {
                            generated_piece_index: piece_index,
                            collider_handle: created
                                .collider_handles
                                .get(collider_cursor + piece_index)
                                .copied(),
                            validation_path: format!(
                                "scene.bodies[{body_index}].shape.generated_pieces[{piece_index}]"
                            ),
                            local_pose: [0.0, 0.0, 0.0],
                        })
                        .collect(),
                });
            }
            _ => {}
        }
        collider_cursor += body.shape.collider_count();
    }

    provenance
}

#[cfg(test)]
mod tests;
