use picea::prelude::*;

use super::fixture::{
    default_fixture_density, SceneBodyFixture, SceneCompoundPieceFixture,
    SceneCompoundPieceShapeFixture, SceneFixtureWorld, SceneRecipeFixture, SceneShapeFixture,
};
use super::SCENE_RECIPE_SCHEMA_VERSION;

pub(super) fn falling_box_contact_fixture(gravity: [f32; 2]) -> SceneRecipeFixture {
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

pub(super) fn compound_provenance_fixture() -> SceneRecipeFixture {
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

pub(super) fn stack_stability_tower_fixture(gravity: [f32; 2]) -> SceneRecipeFixture {
    SceneRecipeFixture {
        schema_version: SCENE_RECIPE_SCHEMA_VERSION,
        world: SceneFixtureWorld {
            gravity,
            enable_sleep: true,
        },
        bodies: vec![
            SceneBodyFixture {
                body_type: BodyType::Static,
                pose: [0.0, 2.6, 0.0],
                linear_velocity: [0.0, 0.0],
                can_sleep: false,
                shape: SceneShapeFixture::Rect {
                    width: 9.0,
                    height: 0.6,
                },
                material: MaterialPreset::Rough,
                filter: CollisionLayerPreset::StaticGeometry,
                density: default_fixture_density(),
                is_sensor: false,
            },
            SceneBodyFixture {
                body_type: BodyType::Static,
                pose: [0.15, 2.05, 0.0],
                linear_velocity: [0.0, 0.0],
                can_sleep: false,
                shape: SceneShapeFixture::Rect {
                    width: 1.7,
                    height: 0.25,
                },
                material: MaterialPreset::Rough,
                filter: CollisionLayerPreset::StaticGeometry,
                density: default_fixture_density(),
                is_sensor: false,
            },
            SceneBodyFixture {
                body_type: BodyType::Dynamic,
                pose: [0.0, 1.55, 0.03],
                linear_velocity: [0.0, 0.0],
                can_sleep: true,
                shape: SceneShapeFixture::Rect {
                    width: 1.35,
                    height: 0.48,
                },
                material: MaterialPreset::Rough,
                filter: CollisionLayerPreset::DynamicBody,
                density: default_fixture_density(),
                is_sensor: false,
            },
            SceneBodyFixture {
                body_type: BodyType::Dynamic,
                pose: [0.08, 1.0, -0.035],
                linear_velocity: [0.0, 0.0],
                can_sleep: true,
                shape: SceneShapeFixture::Rect {
                    width: 1.1,
                    height: 0.46,
                },
                material: MaterialPreset::Default,
                filter: CollisionLayerPreset::DynamicBody,
                density: default_fixture_density(),
                is_sensor: false,
            },
            SceneBodyFixture {
                body_type: BodyType::Dynamic,
                pose: [-0.05, 0.48, 0.045],
                linear_velocity: [0.0, 0.0],
                can_sleep: true,
                shape: SceneShapeFixture::Rect {
                    width: 0.92,
                    height: 0.44,
                },
                material: MaterialPreset::Sticky,
                filter: CollisionLayerPreset::DynamicBody,
                density: default_fixture_density(),
                is_sensor: false,
            },
            SceneBodyFixture {
                body_type: BodyType::Dynamic,
                pose: [0.09, -0.02, -0.03],
                linear_velocity: [0.0, 0.0],
                can_sleep: true,
                shape: SceneShapeFixture::Rect {
                    width: 0.74,
                    height: 0.42,
                },
                material: MaterialPreset::Default,
                filter: CollisionLayerPreset::DynamicBody,
                density: default_fixture_density(),
                is_sensor: false,
            },
            SceneBodyFixture {
                body_type: BodyType::Dynamic,
                pose: [0.0, -0.49, 0.02],
                linear_velocity: [0.0, 0.0],
                can_sleep: true,
                shape: SceneShapeFixture::Rect {
                    width: 0.58,
                    height: 0.38,
                },
                material: MaterialPreset::Sticky,
                filter: CollisionLayerPreset::DynamicBody,
                density: default_fixture_density(),
                is_sensor: false,
            },
            SceneBodyFixture {
                body_type: BodyType::Dynamic,
                pose: [0.03, -0.93, -0.015],
                linear_velocity: [0.0, 0.0],
                can_sleep: true,
                shape: SceneShapeFixture::Rect {
                    width: 0.46,
                    height: 0.34,
                },
                material: MaterialPreset::Default,
                filter: CollisionLayerPreset::DynamicBody,
                density: default_fixture_density(),
                is_sensor: false,
            },
            SceneBodyFixture {
                body_type: BodyType::Dynamic,
                pose: [-1.45, 1.2, 0.0],
                linear_velocity: [1.2, 0.0],
                can_sleep: true,
                shape: SceneShapeFixture::Rect {
                    width: 0.36,
                    height: 0.36,
                },
                material: MaterialPreset::Default,
                filter: CollisionLayerPreset::DynamicBody,
                density: default_fixture_density(),
                is_sensor: false,
            },
        ],
        joints: Vec::new(),
    }
}

pub(super) fn concave_decomposition_fixture() -> SceneRecipeFixture {
    SceneRecipeFixture {
        schema_version: SCENE_RECIPE_SCHEMA_VERSION,
        world: SceneFixtureWorld {
            gravity: [0.0, 9.8],
            enable_sleep: true,
        },
        bodies: vec![
            SceneBodyFixture {
                body_type: BodyType::Static,
                pose: [0.0, 1.6, 0.0],
                linear_velocity: [0.0, 0.0],
                can_sleep: false,
                shape: SceneShapeFixture::ConcavePolygon {
                    vertices: vec![
                        [-2.0, -0.4],
                        [2.0, -0.4],
                        [2.0, 0.2],
                        [0.4, 0.2],
                        [0.4, 0.8],
                        [-2.0, 0.8],
                    ],
                },
                material: MaterialPreset::Rough,
                filter: CollisionLayerPreset::StaticGeometry,
                density: default_fixture_density(),
                is_sensor: false,
            },
            SceneBodyFixture {
                body_type: BodyType::Dynamic,
                pose: [0.0, -1.4, 0.0],
                linear_velocity: [0.0, 0.0],
                can_sleep: false,
                shape: SceneShapeFixture::Rect {
                    width: 0.5,
                    height: 0.5,
                },
                material: MaterialPreset::Default,
                filter: CollisionLayerPreset::DynamicBody,
                density: default_fixture_density(),
                is_sensor: false,
            },
        ],
        joints: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::fixture::{default_fixture_gravity, instantiate_scene_fixture};
    use crate::scenario::scene_dispatch::build_scenario;
    use crate::scenario::{ScenarioId, ScenarioOverrides};

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
