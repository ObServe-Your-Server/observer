use reqwest::{Error, StatusCode};

#[derive(Debug)]
pub enum CollectionErrorOld {
    MetricsCollectionTimeout,
    SendFailed(Error),
    PullFailed(Error),
    ServerRejected(StatusCode),
    ParsingFailed(Error),
    DockerSocketUnavailable(String),
}

impl std::fmt::Display for CollectionErrorOld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CollectionErrorOld::MetricsCollectionTimeout => {
                write!(f, "Metrics collection timed out")
            }
            CollectionErrorOld::SendFailed(e) => write!(f, "Send failed: {}", e),
            CollectionErrorOld::PullFailed(e) => write!(f, "Pull failed: {}", e),
            CollectionErrorOld::ServerRejected(status) => write!(f, "Server rejected: {}", status),
            CollectionErrorOld::ParsingFailed(e) => write!(f, "Parsing failed: {}", e),
            CollectionErrorOld::DockerSocketUnavailable(e) => {
                write!(f, "Docker socket unavailable: {}", e)
            }
        }
    }
}

impl std::error::Error for CollectionErrorOld {}
