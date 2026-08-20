use crate::scheduling::job::JobTrait;
use crate::storage_engine::storage_engine::StorageEngine;
use async_trait::async_trait;
use chrono::Duration;
use std::sync::Arc;

pub struct SpeedtestStatsCollectionJob {
    schedule_time: Duration,
    storage_engine: Arc<StorageEngine>,
}

impl SpeedtestStatsCollectionJob {
    pub fn new(
        storage_engine: Arc<StorageEngine>,
        schedule_time: Duration,
    ) -> SpeedtestStatsCollectionJob {
        SpeedtestStatsCollectionJob {
            storage_engine,
            schedule_time,
        }
    }
}

#[async_trait]
impl JobTrait for SpeedtestStatsCollectionJob {
    async fn run(&self) -> anyhow::Result<()> {
        let res = open_eye::collector::speedtest::collector::run().await?;
        self.storage_engine.save_speedtest_stats_to_db(res).await?;
        Ok(())
    }
}
