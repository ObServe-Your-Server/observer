use crate::config::get_config;
use crate::mapper::docker_metrics_mapper::DockerMapper;
use anyhow::{anyhow, Result};
use log::{debug, error};
use open_eye::collector::container_runtime::collector::check_runtime_availability;
use open_eye::collector::container_runtime::collector::get_current_stats;
use open_eye::collector::container_runtime::collector::{ContainerRuntimeStats, ContainerStats};

pub struct ContainerStatsSystem{

}

impl ContainerStatsSystem {
    async fn collect() -> Result<Option<ContainerRuntimeStats>> {
        check_runtime_availability().ok_or_else(|| anyhow!("No container runtime available"))?;

        match get_current_stats().await {
            Ok(Some(stats)) => Ok(Some(stats)),
            Ok(None) => Ok(None),
            Err(e) => {
                log::error!("Failed to collect container metrics: {e}");
                Err(e)
            }
        }
    }

    pub async fn run() -> Result<()> {
        let config = get_config();
        let docker_metrics = ContainerStatsSystem::collect().await;

        debug!("Docker metrics collected: {:?}", docker_metrics);

        let mapped_metrics = DockerMapper::map_for_watch_tower(docker_metrics);
        todo!("Save to db")
    }
}

