use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct StorageConfig {
    pub database_url: String,    
    pub metrics_retention_hours_full_resolution: u64,
    pub metrics_retention_hours_reduced_resolution: u64,
}