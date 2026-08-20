use crate::scheduling::job::JobTrait;
use crate::storage_engine::storage_engine::StorageEngine;
use async_trait::async_trait;
use chrono::{Duration, Utc};
use std::sync::Arc;

pub struct DataCleanupJob {
    storage_engine: Arc<StorageEngine>,
    metrics_retention_time_hours: u64,
    // TODO rework with new config
}

impl DataCleanupJob {
    pub fn new(
        storage_engine: Arc<StorageEngine>,
        metrics_retention_time_hours: u64,
        schedule_time: Duration,
    ) -> Self {
        DataCleanupJob {
            storage_engine,
            metrics_retention_time_hours,
        }
    }
}

#[async_trait]
impl JobTrait for DataCleanupJob {
    async fn run(&self) -> anyhow::Result<()> {
        let erase_older_than =
            Utc::now() - Duration::hours(i64::try_from(self.metrics_retention_time_hours)?);
        self.storage_engine.cleanup_job(erase_older_than).await
    }
}
