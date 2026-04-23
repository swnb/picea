use std::{error::Error, fmt};

use crate::{
    handles::{BodyHandle, ColliderHandle, JointHandle},
    joint::JointKind,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValidationError {
    BodyDesc { field: &'static str },
    BodyPatch { field: &'static str },
    ColliderDesc { field: &'static str },
    ColliderPatch { field: &'static str },
    JointDesc { field: &'static str },
    JointPatch { field: &'static str },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HandleError {
    MissingBody {
        handle: BodyHandle,
    },
    StaleBody {
        handle: BodyHandle,
    },
    MissingCollider {
        handle: ColliderHandle,
    },
    StaleCollider {
        handle: ColliderHandle,
    },
    MissingJoint {
        handle: JointHandle,
    },
    StaleJoint {
        handle: JointHandle,
    },
    WrongJointKind {
        handle: JointHandle,
        expected: JointKind,
        actual: JointKind,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TopologyError {
    SameBodyJointPair { body: BodyHandle, kind: JointKind },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorldError {
    Validation(ValidationError),
    Handle(HandleError),
    Topology(TopologyError),
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (scope, field) = match self {
            Self::BodyDesc { field } => ("body descriptor", field),
            Self::BodyPatch { field } => ("body patch", field),
            Self::ColliderDesc { field } => ("collider descriptor", field),
            Self::ColliderPatch { field } => ("collider patch", field),
            Self::JointDesc { field } => ("joint descriptor", field),
            Self::JointPatch { field } => ("joint patch", field),
        };
        write!(f, "{scope} contains an invalid `{field}` value")
    }
}

impl fmt::Display for HandleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingBody { .. } => f.write_str("body handle does not belong to this world"),
            Self::StaleBody { .. } => f.write_str("body handle refers to a recycled body slot"),
            Self::MissingCollider { .. } => {
                f.write_str("collider handle does not belong to this world")
            }
            Self::StaleCollider { .. } => {
                f.write_str("collider handle refers to a recycled collider slot")
            }
            Self::MissingJoint { .. } => f.write_str("joint handle does not belong to this world"),
            Self::StaleJoint { .. } => f.write_str("joint handle refers to a recycled joint slot"),
            Self::WrongJointKind { .. } => {
                f.write_str("joint patch kind does not match the stored joint")
            }
        }
    }
}

impl fmt::Display for TopologyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SameBodyJointPair { .. } => {
                f.write_str("joint endpoints must not target the same body")
            }
        }
    }
}

impl fmt::Display for WorldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(error) => error.fmt(f),
            Self::Handle(error) => error.fmt(f),
            Self::Topology(error) => error.fmt(f),
        }
    }
}

impl Error for ValidationError {}
impl Error for HandleError {}
impl Error for TopologyError {}
impl Error for WorldError {}
