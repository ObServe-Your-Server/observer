use crate::storage_engine::storage_engine::StorageEngine;
use chrono::Duration;
use std::sync::Arc;
use anyhow::anyhow;
use async_trait::async_trait;
use open_eye::collector::container_runtime::collector::{check_runtime_availability, get_current_stats, ContainerRuntimeStats};
use crate::scheduling::job::Job;
use crate::subsystem::container_stats_system::ContainerStatsSystem;

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
impl Job for ContainerStatsCollectionJob {
    async fn run(&self) -> anyhow::Result<()> {
        check_runtime_availability().ok_or_else(|| anyhow!("No container runtime available"))?;

        let res = get_current_stats()
            .await
            .map_err(|err| anyhow!("Error during ContainerStats collection: {}", err))?;

        match res {
            None => return Ok(()),
            Some(container_stats) => {
                self.storage_engine.save_container_runtime_stats_to_db(container_stats).await?;
            }
        }
        Ok(())
    }

    fn schedule_time(&self) -> Duration {
        self.schedule_time
    }

    fn name(&self) -> &str {
        "ContainerStatsCollectionJob"
    }
}
