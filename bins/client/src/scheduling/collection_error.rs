use std::fmt::{self};

#[derive(Debug)]
pub enum CollectionError {
    SendFailed(reqwest::Error),
    PullFailed(reqwest::Error),
    ServerRejected(reqwest::StatusCode),
    PaarsingFailed(reqwest::Error),
    ContainerSocketUnavailable(String),
}

impl std::fmt::Display for CollectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CollectionError::SendFailed(e) => write!(f, "Send failed: {}", e),
            Self::PullFailed(e) => write!(f, "Pull failed: {}", e),
            Self::ServerRejected(e) => write!(f, "Server rejected metrics: {}", e),
            Self::PaarsingFailed(e) => write!(f, "The parsing failed: {}", e),
            Self::ContainerSocketUnavailable(e) => write!(f, "Container socket unavailable: {}", e),
        }
    }
}

impl std::error::Error for CollectionError {}

#[derive(Debug)]
pub enum JobError {
    CollectionFailed(CollectionError),
    Timeout(String),
    SetupFailed(String),
}

impl fmt::Display for JobError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JobError::CollectionFailed(e) => write!(f, "Collection failed: {}", e),
            JobError::Timeout(msg) => write!(f, "Job timed out: {}", msg),
            JobError::SetupFailed(msg) => write!(f, "Job setup failed: {}", msg),
        }
    }
}

impl std::error::Error for JobError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            JobError::CollectionFailed(e) => Some(e),
            _ => None,
        }
    }
}

impl From<CollectionError> for JobError {
    fn from(e: CollectionError) -> Self {
        JobError::CollectionFailed(e)
    }
}
