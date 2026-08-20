use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct IntervalConfig {
    pub base_metric_secs: u16,
    pub speedtest_secs: u32,
    pub enable_docker_socket: bool,
    pub container_metrics_secs: Option<u16>,
}