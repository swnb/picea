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
fn static_concave_polygon_fixture_decomposes_into_ordered_convex_pieces() {
    let json = r#"
        {
          "schema_version": 1,
          "world": { "gravity": [0.0, 0.0], "enable_sleep": false },
          "bodies": [
            {
              "body_type": "static",
              "shape": {
                "type": "concave_polygon",
                "vertices": [[-1.0, -1.0], [1.0, -1.0], [1.0, 0.0], [0.0, 0.0], [0.0, 1.0], [-1.0, 1.0]]
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
    let instantiated = instantiate_scene_fixture_with_provenance(&fixture)
        .expect("static concave polygon should decompose into convex pieces");
    let body = instantiated
        .world
        .bodies()
        .next()
        .expect("fixture should create one body");
    let colliders: Vec<_> = instantiated
        .world
        .colliders_for_body(body)
        .expect("decomposed body should resolve")
        .collect();

    assert_eq!(colliders.len(), 4);
    for collider in &colliders {
        let collider = instantiated
            .world
            .collider(*collider)
            .expect("decomposed collider should resolve");
        match collider.shape() {
            SharedShape::ConvexPolygon { vertices } => assert_eq!(vertices.len(), 3),
            shape => panic!("expected generated triangle, got {shape:?}"),
        }
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

    let provenance = instantiated
        .compound_provenance
        .first()
        .expect("decomposed pieces should enter provenance");
    assert_eq!(provenance.validation_path, "scene.bodies[0].shape.vertices");
    assert_eq!(provenance.pieces.len(), 4);
    for (piece_index, piece) in provenance.pieces.iter().enumerate() {
        assert_eq!(piece.generated_piece_index, piece_index);
        assert_eq!(piece.collider_handle, Some(colliders[piece_index]));
        assert_eq!(
            piece.validation_path,
            format!("scene.bodies[0].shape.generated_pieces[{piece_index}]")
        );
    }
}

#[test]
fn static_concave_polygon_fixture_accepts_clockwise_winding() {
    let json = r#"
        {
          "schema_version": 1,
          "bodies": [
            {
              "body_type": "static",
              "shape": {
                "type": "concave_polygon",
                "vertices": [[-1.0, 1.0], [0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, -1.0], [-1.0, -1.0]]
              }
            }
          ]
        }
        "#;

    let fixture: SceneRecipeFixture =
        serde_json::from_str(json).expect("fixture json should deserialize");
    let world = instantiate_scene_fixture(&fixture)
        .expect("clockwise static concave polygon should normalize winding");
    let body = world
        .bodies()
        .next()
        .expect("fixture should create one body");
    assert_eq!(
        world
            .colliders_for_body(body)
            .expect("decomposed body should resolve")
            .count(),
        4
    );
}

#[test]
fn self_intersecting_concave_polygon_fixture_fails_with_stable_path() {
    let json = r#"
        {
          "schema_version": 1,
          "bodies": [
            {
              "body_type": "static",
              "shape": {
                "type": "concave_polygon",
                "vertices": [[0.0, 0.0], [2.0, 2.0], [0.0, 2.0], [2.0, 0.0], [1.0, -1.0]]
              }
            }
          ]
        }
        "#;

    let fixture: SceneRecipeFixture =
        serde_json::from_str(json).expect("fixture json should deserialize");
    let build_error = fixture
        .to_world_recipe()
        .expect_err("self-intersecting concave polygon should fail before instantiation");
    assert_eq!(
            build_error.to_string(),
            "world setup failed: scene.bodies[0].shape.vertices: concave_polygon must be simple and non-self-intersecting"
        );
}

#[test]
fn degenerate_concave_polygon_fixture_fails_with_stable_path() {
    let json = r#"
        {
          "schema_version": 1,
          "bodies": [
            {
              "body_type": "static",
              "shape": {
                "type": "concave_polygon",
                "vertices": [[0.0, 0.0], [1.0, 0.0], [1.0, 0.0], [0.0, 0.0]]
              }
            }
          ]
        }
        "#;

    let fixture: SceneRecipeFixture =
        serde_json::from_str(json).expect("fixture json should deserialize");
    let build_error = fixture
        .to_world_recipe()
        .expect_err("degenerate concave polygon should fail before instantiation");
    assert_eq!(
            build_error.to_string(),
            "world setup failed: scene.bodies[0].shape.vertices: concave_polygon requires at least 4 non-degenerate vertices"
        );
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

    let instantiate_error = instantiate_scene_fixture(&fixture)
        .expect_err("top-level convex_polygon fixture should also fail through the lab entrypoint");
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

    let instantiate_error = instantiate_scene_fixture(&fixture)
        .expect_err("top-level convex_polygon fixture should also fail through the lab entrypoint");
    assert_eq!(instantiate_error.to_string(), build_error.to_string());
}

#[test]
fn compound_piece_convex_polygon_rejects_adjacent_duplicate_vertices_with_stable_piece_index_path()
{
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
            "world setup failed: scene.bodies[0].shape: concave_polygon automatic decomposition is only supported for static bodies"
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

    let error = instantiate_scene_fixture(&fixture).expect_err("unsupported schema should fail");
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
    let error = instantiate_scene_fixture(&fixture).expect_err("invalid joint body should fail");
    assert_eq!(
            error.to_string(),
            "world setup failed: recipe.joints[0].desc.body_b: body handle does not belong to this world"
        );
}
