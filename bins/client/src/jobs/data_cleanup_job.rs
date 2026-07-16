use std::process::exit;
use std::sync::Arc;
use async_trait::async_trait;
use chrono::{Duration, Utc};
use crate::scheduling::job::Job;
use crate::storage_engine::storage_engine::StorageEngine;

pub struct DataCleanupJob{
    schedule_time: Duration,
    storage_engine: Arc<StorageEngine>,
    metrics_retention_time_hours: u64,
}

impl DataCleanupJob {
    pub fn new(storage_engine: Arc<StorageEngine>, metrics_retention_time_hours: u64, schedule_time: Duration) -> Self {
        DataCleanupJob{
            schedule_time,
            storage_engine,
            metrics_retention_time_hours,
        }
    }
}

#[async_trait]
impl Job for DataCleanupJob {
    async fn run(&self) -> anyhow::Result<()> {
        let erase_older_than = Utc::now() - Duration::hours(i64::try_from(self.metrics_retention_time_hours)?);
        self.storage_engine.cleanup_job(erase_older_than).await
    }

    fn schedule_time(&self) -> Duration {
        self.schedule_time
    }

    fn name(&self) -> &str {
        "Data Cleanup Job"
    }
}