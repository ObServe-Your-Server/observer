use crate::scheduling::job::JobTrait;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::Duration;
use open_eye::collector::container_runtime::collector::{
    check_runtime_availability, get_current_stats, ContainerRuntimeStats,
};
use std::sync::Arc;

#[async_trait]
pub trait ContainerStatsCollectionStorageEngine: Send + Sync {
    async fn save_container_runtime_stats(
        &self,
        container_runtime_stats: ContainerRuntimeStats,
    ) -> Result<()>;
}

pub struct ContainerStatsCollectionJob {
    storage_engine: Arc<dyn ContainerStatsCollectionStorageEngine>,
}

impl ContainerStatsCollectionJob {
    pub fn new(
        storage_engine: Arc<dyn ContainerStatsCollectionStorageEngine>,
    ) -> ContainerStatsCollectionJob {
        ContainerStatsCollectionJob {
            storage_engine,
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
                    .save_container_runtime_stats(container_stats)
                    .await?;
            }
        }
        Ok(())
    }
}
