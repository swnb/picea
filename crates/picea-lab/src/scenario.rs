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
    CcdFastCircleWall,
    CcdFastConvexWalls,
}

impl ScenarioId {
    pub const ALL: [Self; 7] = [
        Self::FallingBoxContact,
        Self::Stack4,
        Self::JointAnchor,
        Self::BroadphaseSparse,
        Self::SatPolygon,
        Self::CcdFastCircleWall,
        Self::CcdFastConvexWalls,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FallingBoxContact => "falling_box_contact",
            Self::Stack4 => "stack_4",
            Self::JointAnchor => "joint_anchor",
            Self::BroadphaseSparse => "broadphase_sparse",
            Self::SatPolygon => "sat_polygon",
            Self::CcdFastCircleWall => "ccd_fast_circle_wall",
            Self::CcdFastConvexWalls => "ccd_fast_convex_walls",
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
            "ccd_fast_circle_wall" => Ok(Self::CcdFastCircleWall),
            "ccd_fast_convex_walls" => Ok(Self::CcdFastConvexWalls),
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
                ScenarioId::CcdFastCircleWall => "CCD fast circle wall",
                ScenarioId::CcdFastConvexWalls => "CCD fast convex walls",
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
                ScenarioId::CcdFastCircleWall => {
                    "A fast dynamic circle swept against a static thin rectangle wall."
                }
                ScenarioId::CcdFastConvexWalls => {
                    "A fast dynamic rectangle swept against two static thin walls."
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
/// This intentionally covers a narrow first slice: world gravity/sleep flags,
/// body placement, linear velocity, and circle/rectangle body assets. It proves
/// the recipe path can be driven from JSON without freezing every low-level
/// core descriptor as a long-term file format.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneRecipeFixture {
    #[serde(default)]
    pub world: SceneFixtureWorld,
    #[serde(default)]
    pub bodies: Vec<SceneBodyFixture>,
}

impl SceneRecipeFixture {
    pub fn to_world_recipe(&self) -> WorldRecipe {
        let mut recipe = WorldRecipe::new(WorldDesc {
            gravity: self.world.gravity.into(),
            enable_sleep: self.world.enable_sleep,
        });
        for body in &self.bodies {
            recipe = recipe.with_scene_body(body.to_body_bundle());
        }
        recipe
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

impl SceneBodyFixture {
    fn to_body_bundle(&self) -> BodyBundle {
        let collider = self
            .shape
            .to_collider_bundle()
            .with_material(self.material)
            .with_filter(self.filter)
            .with_density(self.density)
            .with_sensor(self.is_sensor);
        let base = match self.body_type {
            BodyType::Static => BodyBundle::static_body(),
            BodyType::Dynamic => BodyBundle::dynamic(),
            BodyType::Kinematic => BodyBundle::kinematic(),
        }
        .with_collider(collider);
        let asset = BodyAsset::from_bundle(base);
        let [x, y, angle] = self.pose;
        let mut bundle = asset.at(Pose::from_xy_angle(x, y, angle));
        let [vx, vy] = self.linear_velocity;
        bundle.desc.linear_velocity = Vector::new(vx, vy);
        bundle.desc.can_sleep = self.can_sleep;
        bundle
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SceneShapeFixture {
    Circle { radius: f32 },
    Rect { width: f32, height: f32 },
}

impl SceneShapeFixture {
    fn to_collider_bundle(&self) -> ColliderBundle {
        match self {
            Self::Circle { radius } => ColliderBundle::circle(*radius),
            Self::Rect { width, height } => ColliderBundle::rect(*width, *height),
        }
    }
}

pub fn instantiate_scene_fixture(fixture: &SceneRecipeFixture) -> LabResult<World> {
    fixture
        .to_world_recipe()
        .instantiate_with_context()
        .map(|result| result.world)
        .map_err(|error| LabError::World(format!("{}: {}", error.path, error.error.error)))
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

fn falling_box_contact_fixture(gravity: [f32; 2]) -> SceneRecipeFixture {
    SceneRecipeFixture {
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
    }
}

pub(crate) struct BuiltScenario {
    pub(crate) world: World,
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
    }

    Ok(BuiltScenario { world })
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
}
