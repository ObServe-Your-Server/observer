use crate::scheduling::job::JobTrait;
use crate::storage_engine::storage_engine::StorageEngine;
use async_trait::async_trait;
use chrono::{DateTime, Duration, Timelike, Utc};
use std::sync::Arc;
use log::debug;
use crate::config::config_parts::storage_config::StorageConfig;
use anyhow::Result;

#[async_trait]
pub trait DataCleanupStorageEngine: Send + Sync {
    async fn remove_all_older_than(&self, cutoff_time: DateTime<Utc>) -> Result<()>;
}

pub struct DataCleanupJob {
    storage_engine: Arc<dyn DataCleanupStorageEngine>,
    storage_config: StorageConfig,
}

impl DataCleanupJob {
    pub fn new(
        storage_engine: Arc<dyn DataCleanupStorageEngine>,
        storage_config: StorageConfig,
    ) -> Self {
        DataCleanupJob {
            storage_engine,
            storage_config,
        }
    }
}

#[async_trait]
impl JobTrait for DataCleanupJob {
    async fn run(&self) -> Result<()> {
        // TODO implement the thinout of data
        let cutoff_time = {
            Utc::now()
                - (Duration::hours(self.storage_config.metrics_retention_hours_reduced_resolution().clone().try_into()?)
                + Duration::hours(self.storage_config.metrics_retention_hours_full_resolution().clone().try_into()?))
        };
        debug!("Cutoff time for metrics: {}", cutoff_time);
        self.storage_engine.remove_all_older_than(cutoff_time).await
    }
}
