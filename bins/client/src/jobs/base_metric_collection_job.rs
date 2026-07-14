use std::sync::Arc;
use anyhow::Result;
use async_trait::async_trait;
use chrono::Duration;
use crate::scheduling::job::Job;
use crate::storage_engine::storage_engine::StorageEngine;
use crate::subsystem::base_metrics::BaseMetrics;

pub struct BaseMetricCollectionJob{
    storage_engine: Arc<StorageEngine>,
    schedule_time: Duration,
}

impl BaseMetricCollectionJob{
    pub fn new(storage_engine: Arc<StorageEngine>, schedule_time: Duration)->BaseMetricCollectionJob{
        BaseMetricCollectionJob{
            storage_engine,
            schedule_time,
        }
    }
}

#[async_trait]
impl Job for BaseMetricCollectionJob {
    async fn run(&self) -> Result<()> {
        let base_metrics = BaseMetrics::collect().await;
        self.storage_engine.save_base_metrics_to_db(base_metrics).await
    }

    fn schedule_time(&self) -> Duration {
        self.schedule_time
    }

    fn name(&self) -> &str {
        "BaseMetricCollectionJob"
    }
}
