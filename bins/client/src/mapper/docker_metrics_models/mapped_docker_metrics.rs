use serde::{Deserialize, Serialize};

pub type MappedDockerMetrics = Vec<MappedContainerStats>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MappedContainerStats {
    pub container_runtime: String,
    pub id: String,
    pub host_name: String,
    pub created_at: i64,
    pub status: String,
    pub running: bool,
    pub running_for_seconds: u64,
    pub image_name: String,
    pub networks: Vec<String>,
    pub cpu_usage_percent: f64,
    pub memory_usage_bytes: u64,
}
