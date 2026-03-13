use reqwest::{Error, Response, StatusCode};

#[derive(Debug)]
pub enum CollectionError {
    MetricsCollectionTimeout,
    SendFailed(Error),
    PullFailed(Error),
    ServerRejected(StatusCode),
    ParsingFailed(Error),
}

impl std::fmt::Display for CollectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CollectionError::MetricsCollectionTimeout => write!(f, "Metrics collection timed out"),
            CollectionError::SendFailed(e) => write!(f, "Send failed: {}", e),
            CollectionError::PullFailed(e) => write!(f, "Pull failed: {}", e),
            CollectionError::ServerRejected(status) => write!(f, "Server rejected: {}", status),
            CollectionError::ParsingFailed(e) => write!(f, "Parsing failed: {}", e),
        }
    }
}

impl std::error::Error for CollectionError {}
