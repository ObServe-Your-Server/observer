use serde::{Deserialize, Serialize};
use getset::{Getters, Setters};

#[derive(Debug, Deserialize, Serialize, Getters, Setters)]
#[serde(rename_all = "snake_case")]
pub struct ClientConfig {
    #[getset(get = "pub", set = "pub")]
    machine_name: Option<String>,
    #[getset(get = "pub")]
    base_server_grpc_url: String,
    #[getset(get = "pub")]
    base_server_http_url: String,
    #[getset(get = "pub")]
    push_notification_url: String,
    #[getset(get = "pub")]
    api_key: String,
    #[getset(get = "pub")]
    enable_container_metrics_collector: bool
}