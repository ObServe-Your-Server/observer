use crate::config::get_config;
use crate::mapper::docker_metrics_mapper::DockerMapper;
use crate::scheduling::collection_error::CollectionError;
use crate::sender::metrics_sender::MetricsSender;
use log::{debug, error};
use open_eye::collector::docker::collector::check_runtime_availability;
use open_eye::collector::docker::collector::ContainerRuntimeStats;
use open_eye::collector::docker::collector::get_current_stats;

#[derive(Debug, serde::Serialize, Clone)]
pub struct DockerMetrics {
    pub docker: ContainerRuntimeStats,
}

impl DockerMetrics {
    pub async fn collect() -> Result<DockerMetrics, CollectionError> {
        if check_runtime_availability().is_none() {
            return Err(CollectionError::ContainerSocketUnavailable(
                "No container sockets available".to_string(),
            ));
        }

        match get_current_stats().await {
            Ok(Some(stats)) => Ok(DockerMetrics { docker: stats }),
            Ok(None) => Err(CollectionError::ContainerSocketUnavailable(
                "No containers found".to_string(),
            )),
            Err(e) => {
                error!("Failed to collect docker metrics: {e}");
                Err(CollectionError::ContainerSocketUnavailable(e.to_string()))
            }
        }
    }

    pub async fn run() -> Result<(), CollectionError> {
        let config = get_config();
        let docker_metrics = DockerMetrics::collect().await?;

        debug!("Docker metrics collected: {:?}", docker_metrics);

        let mapped_metrics = DockerMapper::map_for_watch_tower(docker_metrics);
        MetricsSender::send(
            mapped_metrics,
            config.server.base_docker_url.to_string(),
            "docker",
        )
        .await
    }
}
