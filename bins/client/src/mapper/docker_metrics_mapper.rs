use crate::mapper::docker_metrics_models::mapped_docker_metrics::{
    MappedContainerStats, MappedDockerMetrics,
};
use crate::subsystem::docker_metrics_collector::DockerMetrics;

pub struct DockerMapper {}

impl DockerMapper {
    pub fn map_for_watch_tower(current: DockerMetrics) -> MappedDockerMetrics {
        current
            .docker
            .container_stats
            .into_iter()
            .map(|c| MappedContainerStats {
                container_runtime: c.container_runtime.to_string(),
                id: c.id,
                host_name: c.host_name,
                created_at: c.created_at,
                status: c.status,
                running: c.running,
                running_for_seconds: c.running_for_seconds,
                image_name: c.image_name,
                networks: c.networks,
                cpu_usage_percent: c.cpu_usage_percent,
                memory_usage_bytes: c.memory_usage_bytes,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::DockerMapper;
    use crate::subsystem::docker_metrics_collector::DockerMetrics;
    use chrono::Utc;
    use open_eye::collector::docker::collector::{
        ContainerRuntime, ContainerRuntimeStats, ContainerStats,
    };

    fn make_container(runtime: ContainerRuntime, id: &str, running: bool) -> ContainerStats {
        ContainerStats {
            container_runtime: runtime,
            id: id.to_string(),
            host_name: format!("{}-name", id),
            created_at: 1700000000,
            status: if running {
                "running".to_string()
            } else {
                "exited".to_string()
            },
            running,
            running_for_seconds: if running { 3600 } else { 0 },
            image_name: "nginx:latest".to_string(),
            networks: vec!["bridge".to_string()],
            cpu_usage_percent: 1.5,
            memory_usage_bytes: 104857600,
        }
    }

    fn make_metrics(container_stats: Vec<ContainerStats>) -> DockerMetrics {
        DockerMetrics {
            docker: ContainerRuntimeStats {
                collected_at: Utc::now(),
                container_stats,
            },
        }
    }

    #[test]
    fn maps_container_runtime_to_string() {
        let result = DockerMapper::map_for_watch_tower(make_metrics(vec![
            make_container(ContainerRuntime::Docker, "abc", true),
        ]));

        assert_eq!(result[0].container_runtime, "docker socket");
    }

    #[test]
    fn maps_all_container_fields() {
        let result = DockerMapper::map_for_watch_tower(make_metrics(vec![
            make_container(ContainerRuntime::Docker, "abc123", true),
        ]));

        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.id, "abc123");
        assert_eq!(c.host_name, "abc123-name");
        assert_eq!(c.created_at, 1700000000);
        assert_eq!(c.status, "running");
        assert!(c.running);
        assert_eq!(c.running_for_seconds, 3600);
        assert_eq!(c.image_name, "nginx:latest");
        assert_eq!(c.networks, vec!["bridge"]);
        assert!((c.cpu_usage_percent - 1.5).abs() < f64::EPSILON);
        assert_eq!(c.memory_usage_bytes, 104857600);
    }

    #[test]
    fn maps_empty_container_list() {
        let result = DockerMapper::map_for_watch_tower(make_metrics(vec![]));

        assert!(result.is_empty());
    }

    #[test]
    fn maps_multiple_containers() {
        let result = DockerMapper::map_for_watch_tower(make_metrics(vec![
            make_container(ContainerRuntime::Docker, "c1", true),
            make_container(ContainerRuntime::Podman, "c2", false),
        ]));

        assert_eq!(result.len(), 2);
        assert_eq!(result[1].container_runtime, "podman socket");
        assert!(!result[1].running);
    }
}
