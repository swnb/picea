pub mod algo;
pub mod body;
pub mod collider;
pub mod debug;
pub mod events;
pub mod handles;
pub mod joint;
pub mod math;
pub mod pipeline;
pub mod query;
mod solver;
pub mod world;

pub mod prelude {
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
        DistanceJointDesc, DistanceJointPatch, JointDesc, JointPatch, JointView,
        WorldAnchorJointDesc, WorldAnchorJointPatch,
    };
    pub use super::math::{edge::Edge, point::Point, segment::Segment, vector::Vector, FloatNum};
    pub use super::pipeline::{SimulationPipeline, StepConfig, StepReport, StepStats};
    pub use super::query::{AabbHit, PointHit, QueryFilter, QueryPipeline, RayHit};
    pub use super::world::{
        HandleError, TopologyError, ValidationError, World, WorldDesc, WorldError,
    };
}
