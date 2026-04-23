use picea::prelude::{
    BodyDesc, BodyPatch, BodyType, ColliderDesc, CollisionFilter, DistanceJointDesc, JointDesc,
    Material, Pose, SharedShape, World, WorldAnchorJointDesc, WorldDesc, WorldError,
};
use picea::world::HandleError;

#[test]
fn body_handles_invalidate_after_destroy_and_recreate() {
    let mut world = World::new(WorldDesc::default());

    let body = world
        .create_body(BodyDesc::default())
        .expect("body should be created");
    let original_revision = world.revision();

    world.destroy_body(body).expect("body can be destroyed");
    assert!(matches!(
        world.destroy_body(body),
        Err(WorldError::Handle(HandleError::StaleBody { .. }))
    ));
    assert!(
        matches!(
            world.try_body(body),
            Err(WorldError::Handle(HandleError::StaleBody { .. }))
        ),
        "destroyed body should stop resolve through explicit read path"
    );

    let recreated = world
        .create_body(BodyDesc::default())
        .expect("body should be recreated");
    assert_ne!(body, recreated, "generation must change after slot reuse");
    assert!(world.revision() > original_revision);
}

#[test]
fn world_supports_multiple_colliders_per_body() {
    let mut world = World::new(WorldDesc::default());
    let body = world
        .create_body(BodyDesc {
            body_type: BodyType::Dynamic,
            ..BodyDesc::default()
        })
        .expect("body should be created");

    let first = world
        .create_collider(
            body,
            ColliderDesc {
                shape: SharedShape::circle(1.0),
                material: Material::default(),
                filter: CollisionFilter::default(),
                ..ColliderDesc::default()
            },
        )
        .expect("first collider can be created");
    let second = world
        .create_collider(
            body,
            ColliderDesc {
                shape: SharedShape::rect(2.0, 1.0),
                local_pose: Pose::from_xy_angle(2.0, 0.0, 0.0),
                material: Material::default(),
                filter: CollisionFilter::default(),
                ..ColliderDesc::default()
            },
        )
        .expect("second collider can be created");

    let attached: Vec<_> = world
        .colliders_for_body(body)
        .expect("body collider list should resolve")
        .collect();
    assert_eq!(attached, vec![first, second]);
}

#[test]
fn world_applies_body_patches_without_exposing_mutable_internals() {
    let mut world = World::new(WorldDesc::default());
    let body = world
        .create_body(BodyDesc::default())
        .expect("body should be created");

    world
        .apply_body_patch(
            body,
            BodyPatch {
                body_type: Some(BodyType::Kinematic),
                pose: Some(Pose::from_xy_angle(4.0, -3.0, 0.5)),
                linear_velocity: Some((1.0, 2.0).into()),
                angular_velocity: Some(3.0),
                wake: true,
                ..BodyPatch::default()
            },
        )
        .expect("body patch should apply");

    let view = world.try_body(body).expect("patched body exists");
    assert_eq!(view.body_type(), BodyType::Kinematic);
    assert_eq!(view.pose(), Pose::from_xy_angle(4.0, -3.0, 0.5));
    assert_eq!(view.linear_velocity(), (1.0, 2.0).into());
    assert_eq!(view.angular_velocity(), 3.0);
}

#[test]
fn world_accepts_distance_and_world_anchor_joint_descriptions() {
    let mut world = World::new(WorldDesc::default());
    let body_a = world
        .create_body(BodyDesc::default())
        .expect("body should be created");
    let body_b = world
        .create_body(BodyDesc::default())
        .expect("body should be created");

    let distance = world
        .create_joint(JointDesc::Distance(DistanceJointDesc {
            body_a,
            body_b,
            ..DistanceJointDesc::default()
        }))
        .expect("distance joint should be created");
    let world_anchor = world
        .create_joint(JointDesc::WorldAnchor(WorldAnchorJointDesc {
            body: body_a,
            ..WorldAnchorJointDesc::default()
        }))
        .expect("world-anchor joint should be created");

    assert_eq!(
        world.joints().collect::<Vec<_>>(),
        vec![distance, world_anchor]
    );
}

#[test]
fn destroying_a_body_cascades_attached_colliders_and_joints() {
    let mut world = World::new(WorldDesc::default());
    let body_a = world
        .create_body(BodyDesc::default())
        .expect("body should be created");
    let body_b = world
        .create_body(BodyDesc::default())
        .expect("body should be created");
    let collider = world
        .create_collider(
            body_a,
            ColliderDesc {
                shape: SharedShape::circle(1.0),
                ..ColliderDesc::default()
            },
        )
        .expect("collider should be created");
    let joint = world
        .create_joint(JointDesc::Distance(DistanceJointDesc {
            body_a,
            body_b,
            ..DistanceJointDesc::default()
        }))
        .expect("joint should be created");

    world.destroy_body(body_a).expect("body can be destroyed");

    assert!(matches!(
        world.try_collider(collider),
        Err(WorldError::Handle(HandleError::StaleCollider { .. }))
    ));
    assert!(matches!(
        world.try_joint(joint),
        Err(WorldError::Handle(HandleError::StaleJoint { .. }))
    ));
}
