use picea::prelude::*;

use super::fixture::{
    default_fixture_density, SceneBodyFixture, SceneFixtureWorld, SceneJointFixture,
    SceneRecipeFixture, SceneRevoluteJointFixture, SceneShapeFixture,
};
use super::{ScenarioRuntimeConfig, SCENE_RECIPE_SCHEMA_VERSION};

const PENDULUM_LENGTH: f32 = 1.2;
const INITIAL_ANGLE: f32 = 0.55;

pub(super) fn revolute_pendulum_runtime_config() -> ScenarioRuntimeConfig {
    ScenarioRuntimeConfig {
        step: StepConfig {
            dt: 1.0 / 240.0,
            joint_velocity_projection: true,
            ..StepConfig::default()
        },
        substeps_per_frame: 4,
        ..ScenarioRuntimeConfig::default()
    }
}

pub(super) fn revolute_pendulum_fixture() -> SceneRecipeFixture {
    let local_anchor_b = Point::new(0.0, -PENDULUM_LENGTH);
    let rotated_anchor = Vector::from(local_anchor_b).rotated(INITIAL_ANGLE);

    SceneRecipeFixture {
        schema_version: SCENE_RECIPE_SCHEMA_VERSION,
        world: SceneFixtureWorld {
            gravity: [0.0, 9.8],
            enable_sleep: false,
        },
        bodies: vec![
            // The query-only pivot marker stays visible at the physical anchor
            // without adding connected-body contact noise to this joint demo.
            SceneBodyFixture {
                body_type: BodyType::Static,
                pose: [0.0, 0.0, 0.0],
                linear_velocity: [0.0, 0.0],
                can_sleep: false,
                shape: SceneShapeFixture::Circle { radius: 0.12 },
                material: MaterialPreset::Default,
                filter: CollisionLayerPreset::QueryOnly,
                density: default_fixture_density(),
                is_sensor: false,
            },
            SceneBodyFixture {
                body_type: BodyType::Dynamic,
                pose: [-rotated_anchor.x(), -rotated_anchor.y(), INITIAL_ANGLE],
                linear_velocity: [0.0, 0.0],
                can_sleep: false,
                shape: SceneShapeFixture::Rect {
                    width: 0.36,
                    height: PENDULUM_LENGTH * 2.0,
                },
                material: MaterialPreset::Default,
                filter: CollisionLayerPreset::DynamicBody,
                density: default_fixture_density(),
                is_sensor: false,
            },
        ],
        joints: vec![SceneJointFixture::Revolute(SceneRevoluteJointFixture {
            body_a: 0,
            body_b: 1,
            local_anchor_a: Some([0.0, 0.0]),
            local_anchor_b: Some([0.0, -PENDULUM_LENGTH]),
            user_data: Some(5),
        })],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::{instantiate_scene_fixture, scene_dispatch::build_scenario};
    use crate::ScenarioOverrides;

    #[test]
    fn revolute_pendulum_fixture_preserves_schema_v1_and_aligned_world_anchors() {
        let fixture = revolute_pendulum_fixture();
        assert_eq!(fixture.schema_version, SCENE_RECIPE_SCHEMA_VERSION);
        assert_eq!(fixture.bodies.len(), 2);
        assert_eq!(fixture.joints.len(), 1);
        assert_eq!(fixture.bodies[0].pose, [0.0, 0.0, 0.0]);
        assert_eq!(fixture.bodies[0].filter, CollisionLayerPreset::QueryOnly);
        let SceneJointFixture::Revolute(authored_joint) = &fixture.joints[0] else {
            panic!("expected authored revolute joint");
        };
        assert_eq!(authored_joint.local_anchor_a, Some([0.0, 0.0]));

        let world = instantiate_scene_fixture(&fixture).expect("fixture should instantiate");
        let joint_handle = world.joints().next().expect("revolute joint");
        let joint = world.joint(joint_handle).expect("revolute view");
        let JointDesc::Revolute(desc) = joint.desc() else {
            panic!("expected revolute descriptor");
        };
        let anchor_a = world
            .body(desc.body_a)
            .expect("static endpoint")
            .pose()
            .transform_point(desc.local_anchor_a);
        let anchor_b = world
            .body(desc.body_b)
            .expect("dynamic endpoint")
            .pose()
            .transform_point(desc.local_anchor_b);
        assert!((anchor_a - anchor_b).length() <= 1.0e-6);
    }

    #[test]
    fn revolute_pendulum_builtin_uses_fixture_and_fixed_runtime_config() {
        let builtin = build_scenario(
            crate::ScenarioId::RevolutePendulum,
            &ScenarioOverrides::default(),
        )
        .expect("builtin should instantiate");
        assert_eq!(builtin.world.bodies().count(), 2);
        assert_eq!(builtin.world.joints().count(), 1);
        assert_eq!(builtin.effective_runtime_config.step.dt, 1.0 / 240.0);
        assert_eq!(builtin.effective_runtime_config.substeps_per_frame, 4);
        assert!(builtin.effective_runtime_config.scene_params.is_empty());
    }
}
