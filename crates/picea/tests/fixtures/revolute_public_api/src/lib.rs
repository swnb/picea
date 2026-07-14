#![cfg(not(any(
    feature = "exhaustive-joint-kind",
    feature = "exhaustive-joint-desc",
    feature = "exhaustive-joint-patch",
    feature = "exhaustive-joint-bundle",
    feature = "exhaustive-debug-joint-kind",
    feature = "exhaustive-scene-joint-fixture"
)))]

use picea::{
    debug::DebugJointKind,
    joint::{JointDesc, JointKind, JointPatch},
    prelude::{
        BodyBundle, BodyDesc, BodyHandle, BodyType, DebugSnapshotOptions, JointBundle, Point,
        RevoluteJointDesc, RevoluteJointPatch, SimulationPipeline, SleepTransitionReason,
        StepConfig, World, WorldDesc, WorldEvent, WorldRecipe,
    },
};
use picea_lab::{
    SceneJointFixture, SceneRecipeFixture, SceneRevoluteJointFixture, SCENE_RECIPE_SCHEMA_VERSION,
};

/// Keeps the approved public types, struct literals, defaults, and enum variants
/// in a real external crate instead of relying on in-crate visibility.
pub fn approved_surface_literals(
    body_a: BodyHandle,
    body_b: BodyHandle,
) -> (RevoluteJointDesc, RevoluteJointPatch) {
    let default_desc = RevoluteJointDesc::default();
    let default_patch = RevoluteJointPatch::default();
    assert_eq!(default_desc.body_a, BodyHandle::default());
    assert_eq!(default_desc.body_b, BodyHandle::default());
    assert!(!default_desc.body_a.is_valid());
    assert!(!default_desc.body_b.is_valid());
    assert_eq!(default_desc.local_anchor_a, Point::default());
    assert_eq!(default_desc.local_anchor_b, Point::default());
    assert_eq!(default_desc.user_data, 0);
    assert_eq!(default_patch.local_anchor_a, None);
    assert_eq!(default_patch.local_anchor_b, None);
    assert_eq!(default_patch.user_data, None);
    let desc = RevoluteJointDesc {
        body_a,
        body_b,
        local_anchor_a: Point::new(0.0, -0.5),
        local_anchor_b: Point::new(0.0, 0.5),
        user_data: 41,
    };
    let patch = RevoluteJointPatch {
        local_anchor_a: Some(default_desc.local_anchor_a),
        local_anchor_b: Some(default_desc.local_anchor_b),
        user_data: Some(default_patch.user_data.unwrap_or(42)),
    };
    let _ = JointDesc::Revolute(desc.clone());
    let _ = JointPatch::Revolute(patch.clone());
    let _ = JointKind::Revolute;
    let _ = JointBundle::Revolute {
        body_a: 0,
        body_b: 1,
        desc: desc.clone(),
    };
    let _ = JointBundle::revolute(0, 1);
    let _ = DebugJointKind::Revolute;
    let _ = SceneJointFixture::Revolute(SceneRevoluteJointFixture {
        body_a: 0,
        body_b: 1,
        local_anchor_a: Some([0.0, -0.5]),
        local_anchor_b: Some([0.0, 0.5]),
        user_data: Some(43),
    });
    (desc, patch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    fn zero_gravity_world() -> World {
        World::new(WorldDesc {
            gravity: Default::default(),
            enable_sleep: true,
        })
    }

    #[test]
    fn approved_create_view_patch_recipe_debug_and_wake_surface_runs() {
        let mut world = zero_gravity_world();
        let static_body = world
            .create_body(BodyDesc {
                body_type: BodyType::Static,
                ..BodyDesc::default()
            })
            .expect("static body");
        let dynamic_body = world
            .create_body(BodyDesc {
                body_type: BodyType::Dynamic,
                sleeping: true,
                ..BodyDesc::default()
            })
            .expect("sleeping dynamic body");
        let (desc, patch) = approved_surface_literals(dynamic_body, static_body);
        let joint = world
            .create_joint(JointDesc::Revolute(desc))
            .expect("approved Revolute descriptor should create");
        assert_eq!(
            world.joint(joint).expect("joint view").kind(),
            JointKind::Revolute
        );
        world
            .apply_joint_patch(joint, JointPatch::Revolute(patch))
            .expect("approved Revolute patch should apply");

        let snapshot = world.debug_snapshot(&DebugSnapshotOptions::default());
        let debug_joint = snapshot
            .joints
            .iter()
            .find(|candidate| candidate.handle == joint)
            .expect("debug joint");
        assert_eq!(debug_joint.kind, DebugJointKind::Revolute);
        assert_eq!(debug_joint.anchors.len(), 2);

        let events = SimulationPipeline::new(StepConfig::default())
            .step(&mut world)
            .events;
        assert!(
            !world
                .body(dynamic_body)
                .expect("dynamic body view")
                .sleeping(),
            "successful create/constraint patch must wake the sleeping endpoint"
        );
        assert!(events.iter().any(|event| {
            matches!(
                event,
                WorldEvent::SleepChanged(sleep)
                    if sleep.body == dynamic_body
                        && !sleep.is_sleeping
                        && sleep.reason == SleepTransitionReason::UserPatch
            )
        }));

        let recipe = WorldRecipe::new(WorldDesc {
            gravity: Default::default(),
            enable_sleep: true,
        })
        .with_body(BodyBundle::static_body())
        .with_body(BodyBundle::dynamic())
        .with_joint(JointBundle::Revolute {
            body_a: 1,
            body_b: 0,
            desc: RevoluteJointDesc {
                user_data: 99,
                ..RevoluteJointDesc::default()
            },
        });
        let instantiated = recipe.instantiate().expect("Revolute recipe");
        let recipe_joint = instantiated.created.joint_handles[0];
        assert_eq!(
            instantiated
                .world
                .joint(recipe_joint)
                .expect("recipe joint")
                .kind(),
            JointKind::Revolute
        );
    }

    #[test]
    fn schema_v1_roundtrips_revolute_and_old_reader_rejects_it() {
        let json = r#"{
            "schema_version": 1,
            "joints": [{
                "type": "revolute",
                "body_a": 0,
                "body_b": 1,
                "local_anchor_a": [0.25, -0.5],
                "local_anchor_b": [-0.75, 1.25],
                "user_data": 42
            }]
        }"#;
        let fixture: SceneRecipeFixture = serde_json::from_str(json).expect("new schema-v1 reader");
        assert_eq!(fixture.schema_version, SCENE_RECIPE_SCHEMA_VERSION);
        match &fixture.joints[0] {
            SceneJointFixture::Revolute(revolute) => {
                assert_eq!((revolute.body_a, revolute.body_b), (0, 1));
                assert_eq!(revolute.local_anchor_a, Some([0.25, -0.5]));
                assert_eq!(revolute.local_anchor_b, Some([-0.75, 1.25]));
                assert_eq!(revolute.user_data, Some(42));
            }
            _ => panic!("fixture kind must remain revolute"),
        }
        let roundtrip = serde_json::to_value(&fixture).expect("serialize schema-v1 fixture");
        assert_eq!(roundtrip["schema_version"], 1);
        assert_eq!(roundtrip["joints"][0]["type"], "revolute");
        let roundtrip_joint = &roundtrip["joints"][0];
        assert_eq!(roundtrip_joint["body_a"], serde_json::json!(0));
        assert_eq!(roundtrip_joint["body_b"], serde_json::json!(1));
        assert_eq!(
            roundtrip_joint["local_anchor_a"],
            serde_json::json!([0.25, -0.5])
        );
        assert_eq!(
            roundtrip_joint["local_anchor_b"],
            serde_json::json!([-0.75, 1.25])
        );
        assert_eq!(roundtrip_joint["user_data"], serde_json::json!(42));

        #[derive(Debug, Deserialize)]
        #[serde(tag = "type", rename_all = "snake_case")]
        enum OldJointReader {
            Distance,
            WorldAnchor,
        }
        let error = serde_json::from_value::<OldJointReader>(roundtrip["joints"][0].clone())
            .expect_err("old closed reader must reject the new variant");
        assert!(error.to_string().contains("unknown variant"));
    }
}
