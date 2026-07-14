#[cfg(feature = "exhaustive-joint-kind")]
fn joint_kind(value: picea::joint::JointKind) -> u8 {
    match value {
        picea::joint::JointKind::Distance => 0,
        picea::joint::JointKind::WorldAnchor => 1,
        picea::joint::JointKind::Revolute => 2,
    }
}

#[cfg(feature = "exhaustive-joint-desc")]
fn joint_desc(value: picea::joint::JointDesc) -> u8 {
    match value {
        picea::joint::JointDesc::Distance(_) => 0,
        picea::joint::JointDesc::WorldAnchor(_) => 1,
        picea::joint::JointDesc::Revolute(_) => 2,
    }
}

#[cfg(feature = "exhaustive-joint-patch")]
fn joint_patch(value: picea::joint::JointPatch) -> u8 {
    match value {
        picea::joint::JointPatch::Distance(_) => 0,
        picea::joint::JointPatch::WorldAnchor(_) => 1,
        picea::joint::JointPatch::Revolute(_) => 2,
    }
}

#[cfg(feature = "exhaustive-joint-bundle")]
fn joint_bundle(value: picea::recipe::JointBundle) -> u8 {
    match value {
        picea::recipe::JointBundle::Distance { .. } => 0,
        picea::recipe::JointBundle::WorldAnchor { .. } => 1,
        picea::recipe::JointBundle::Revolute { .. } => 2,
    }
}

#[cfg(feature = "exhaustive-debug-joint-kind")]
fn debug_joint_kind(value: picea::debug::DebugJointKind) -> u8 {
    match value {
        picea::debug::DebugJointKind::Distance => 0,
        picea::debug::DebugJointKind::WorldAnchor => 1,
        picea::debug::DebugJointKind::Revolute => 2,
    }
}

#[cfg(feature = "exhaustive-scene-joint-fixture")]
fn scene_joint_fixture(value: picea_lab::SceneJointFixture) -> u8 {
    match value {
        picea_lab::SceneJointFixture::Distance(_) => 0,
        picea_lab::SceneJointFixture::WorldAnchor(_) => 1,
        picea_lab::SceneJointFixture::Revolute(_) => 2,
    }
}

fn main() {}
