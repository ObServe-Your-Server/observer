use crate::config::get_config;
use crate::mapper::docker_metrics_mapper::DockerMapper;
use crate::scheduling::collection_error::CollectionError;
use crate::sender::metrics_sender::MetricsSender;
use log::{debug, error};
use open_eye::collector::docker::collector::ContainerRuntimeStats;
use open_eye::collector::docker::collector::get_current_stats;

#[derive(Debug, serde::Serialize, Clone)]
pub struct DockerMetrics {
    pub docker: Option<ContainerRuntimeStats>,
}

impl DockerMetrics {
    pub async fn collect() -> DockerMetrics {
        let docker = match get_current_stats().await {
            Ok(stats) => stats,
            Err(e) => {
                error!("Failed to collect docker metrics: {e}");
                None
            }
        };

        DockerMetrics { docker }
    }

    pub async fn run() -> Result<(), CollectionError> {
        let config = get_config();
        let docker_metrics = DockerMetrics::collect().await;

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
