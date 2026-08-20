use serde::{Deserialize, Serialize};
use getset::Getters;

#[derive(Debug, Deserialize, Serialize, Getters)]
#[serde(rename_all = "snake_case")]
pub struct StorageConfig {
    #[getset(get = "pub")]
    database_url: String,
    #[getset(get = "pub")]
    metrics_retention_hours_full_resolution: u64,
    #[getset(get = "pub")]
    metrics_retention_hours_reduced_resolution: u64,
}