use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ClientConfig {
    pub base_server_grpc_url: String,
    pub base_server_http_url: String,
    pub push_notification_url: String,
    pub api_key: String,
    pub enable_container_metrics_collector: bool
}