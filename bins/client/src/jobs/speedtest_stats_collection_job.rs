use crate::jobs::job::JobTrait;
use crate::logging::LogTarget;
use anyhow::Result;
use async_trait::async_trait;
use open_eye::collector::speedtest::collector::SpeedtestResult;
use std::sync::Arc;

#[async_trait]
pub trait SpeedtestStatsCollectionStorageEngine: Send + Sync {
    async fn save_speedtest_stats(&self, speedtest_stats: SpeedtestResult) -> Result<()>;
}

pub struct SpeedtestStatsCollectionJob {
    storage_engine: Arc<dyn SpeedtestStatsCollectionStorageEngine>,
}

impl SpeedtestStatsCollectionJob {
    pub fn new(
        storage_engine: Arc<dyn SpeedtestStatsCollectionStorageEngine>,
    ) -> SpeedtestStatsCollectionJob {
        SpeedtestStatsCollectionJob { storage_engine }
    }
}

#[async_trait]
impl JobTrait for SpeedtestStatsCollectionJob {
    async fn run(&self) -> anyhow::Result<()> {
        let res = match open_eye::collector::speedtest::collector::run().await {
            Ok(res) => res,
            Err(e) => {
                log::warn!(target: LogTarget::Speedtest.as_str(), "Speedtest skipped: {e}");
                return Ok(());
            }
        };
        self.storage_engine.save_speedtest_stats(res).await?;
        Ok(())
    }
}
