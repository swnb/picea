pub mod algo;
pub mod body;
pub mod collision;
pub mod collider;
mod constraints;
pub mod debug;
mod element;
pub mod events;
pub mod handles;
pub mod joint;
pub mod math;
mod meta;
pub mod pipeline;
pub mod query;
mod scene;
pub mod shape;
mod tools;
pub mod world;

/// Transitional namespace for legacy engine-facing APIs that have not yet been
/// migrated onto the v1 `World` surface.
#[doc(hidden)]
pub mod legacy {
    pub use super::constraints::{JoinConstraintConfig, JoinConstraintConfigBuilder};
    pub use super::element::{
        ComputeMomentOfInertia, ElementBuilder, ID, SelfClone, ShapeTraitUnion,
    };
    pub use super::meta::{Mass, Meta, MetaBuilder};
    pub use super::scene::Scene;
    pub use super::tools::{observability, snapshot};
}

pub mod prelude {
    #[doc(hidden)]
    pub use super::constraints::{
        JoinConstraintConfig as CoreJoinConstraintConfig,
        JoinConstraintConfigBuilder as CoreJoinConstraintConfigBuilder,
    };
    pub use super::body::{BodyDesc, BodyPatch, BodyType, BodyView, Pose};
    pub use super::collider::{
        ColliderDesc, ColliderPatch, ColliderView, CollisionFilter, Material, SharedShape,
    };
    pub use super::debug::{
        DebugBody, DebugCollider, DebugContact, DebugJoint, DebugManifold, DebugPrimitive,
        DebugSnapshot, DebugSnapshotOptions,
    };
    pub use super::events::{ContactEvent, SleepEvent, WorldEvent};
    pub use super::handles::{
        BodyHandle, ColliderHandle, ContactId, JointHandle, ManifoldId, WorldRevision,
    };
    pub use super::joint::{
        DistanceJointDesc, JointDesc, JointPatch, JointView, WorldAnchorJointDesc,
    };
    pub use super::math::{edge::Edge, point::Point, segment::Segment, vector::Vector, FloatNum};
    #[doc(hidden)]
    pub use super::meta::{Meta as CoreMeta, MetaBuilder as CoreMetaBuilder};
    pub use super::pipeline::{SimulationPipeline, StepConfig, StepReport, StepStats};
    pub use super::query::{AabbHit, PointHit, QueryFilter, QueryPipeline, RayHit};
    pub use super::world::{World, WorldDesc, WorldError};
}
