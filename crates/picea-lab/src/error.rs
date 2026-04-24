use std::fmt::{Display, Formatter};

/// Result alias for lab operations that touch scenarios, artifacts, or sessions.
pub type LabResult<T> = Result<T, LabError>;

/// Errors owned by the lab wrapper. The core engine remains free of server and
/// filesystem concerns; failures are translated at this boundary.
#[derive(Debug)]
pub enum LabError {
    Io(std::io::Error),
    Json(serde_json::Error),
    UnknownScenario(String),
    InvalidArtifactFile(String),
    SessionNotFound(String),
    InvalidControlAction(String),
    World(String),
}

impl Display for LabError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "I/O error: {error}"),
            Self::Json(error) => write!(f, "JSON error: {error}"),
            Self::UnknownScenario(id) => write!(f, "unknown scenario: {id}"),
            Self::InvalidArtifactFile(file) => write!(f, "invalid artifact file: {file}"),
            Self::SessionNotFound(id) => write!(f, "session not found: {id}"),
            Self::InvalidControlAction(action) => write!(f, "invalid control action: {action}"),
            Self::World(error) => write!(f, "world setup failed: {error}"),
        }
    }
}

impl std::error::Error for LabError {}

impl From<std::io::Error> for LabError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for LabError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}
