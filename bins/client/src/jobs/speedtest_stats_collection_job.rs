use std::sync::Arc;
use async_trait::async_trait;
use chrono::Duration;
use crate::scheduling::job::Job;
use crate::storage_engine::storage_engine::StorageEngine;

pub struct SpeedtestStatsCollectionJob{
    schedule_time: Duration,
    storage_engine: Arc<StorageEngine>,
}

impl SpeedtestStatsCollectionJob {
    pub fn new(storage_engine: Arc<StorageEngine>, schedule_time: Duration) -> SpeedtestStatsCollectionJob {
        SpeedtestStatsCollectionJob{
            storage_engine,
            schedule_time,
        }
    }
}

#[async_trait]
impl Job for SpeedtestStatsCollectionJob {
    async fn run(&self) -> anyhow::Result<()> {
        let res = open_eye::collector::speedtest::collector::run().await?;
        self.storage_engine.save_speedtest_stats_to_db(res).await?;
        Ok(())
    }

    fn schedule_time(&self) -> Duration {
        self.schedule_time
    }

    fn name(&self) -> &str {
        "Speedtest Collection Job"
    }
}