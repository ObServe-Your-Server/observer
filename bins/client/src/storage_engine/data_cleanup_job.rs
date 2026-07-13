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
    err_count: u16,
    max_err_count: u16,
}

impl DataCleanupJob {
    pub fn new(storage_engine: Arc<StorageEngine>, metrics_retention_time_hours: u64, max_err_count: u16, schedule_time: Duration) -> Self {
        DataCleanupJob{
            schedule_time,
            storage_engine,
            metrics_retention_time_hours,
            err_count: 0,
            max_err_count,
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
        "DataCleanupJob"
    }

    fn increase_err_count(&mut self) {
        self.err_count += 1;
        if self.err_count == self.max_err_count {
            panic!("Application terminated, caused by: {} too many errors.", self.name())
        }
    }

    fn max_err_count(&self) -> u16 {
        self.max_err_count
    }

    fn current_err_count(&self) -> u16 {
        self.err_count
    }
}