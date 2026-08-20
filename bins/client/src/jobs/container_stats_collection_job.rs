use crate::scheduling::job::JobTrait;
use crate::storage_engine::storage_engine::StorageEngine;
use anyhow::anyhow;
use async_trait::async_trait;
use chrono::Duration;
use open_eye::collector::container_runtime::collector::{
    check_runtime_availability, get_current_stats,
};
use std::sync::Arc;

pub struct ContainerStatsCollectionJob {
    storage_engine: Arc<StorageEngine>,
    schedule_time: Duration,
}

impl ContainerStatsCollectionJob {
    pub fn new(
        storage_engine: Arc<StorageEngine>,
        schedule_time: Duration,
    ) -> ContainerStatsCollectionJob {
        ContainerStatsCollectionJob {
            storage_engine,
            schedule_time,
        }
    }
}

#[async_trait]
impl JobTrait for ContainerStatsCollectionJob {
    async fn run(&self) -> anyhow::Result<()> {
        check_runtime_availability().ok_or_else(|| anyhow!("No container runtime available"))?;

        let res = get_current_stats()
            .await
            .map_err(|err| anyhow!("Error during ContainerStats collection: {}", err))?;

        match res {
            None => return Ok(()),
            Some(container_stats) => {
                self.storage_engine
                    .save_container_runtime_stats_to_db(container_stats)
                    .await?;
            }
        }
        Ok(())
    }
}
