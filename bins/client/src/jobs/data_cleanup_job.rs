use crate::scheduling::job::JobTrait;
use crate::storage_engine::storage_engine::StorageEngine;
use async_trait::async_trait;
use chrono::{Duration, Utc};
use std::sync::Arc;
use crate::config::config_parts::storage_config::StorageConfig;

pub struct DataCleanupJob {
    storage_engine: Arc<StorageEngine>,
    storage_config: StorageConfig,
}

impl DataCleanupJob {
    pub fn new(
        storage_engine: Arc<StorageEngine>,
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
    async fn run(&self) -> anyhow::Result<()> {
        //let erase_older_than = Utc::now() - Duration::hours(i64::try_from(self.metrics_retention_time_hours)?);
        //self.storage_engine.cleanup_job(erase_older_than).await
        todo!()
    }
}
