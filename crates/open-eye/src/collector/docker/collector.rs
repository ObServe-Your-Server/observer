use docker_api::opts::ContainerListOpts;
use futures_util::StreamExt;
use log::debug;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, serde::Serialize)]
pub struct ContainerRuntimeStats {
    pub collected_at: chrono::DateTime<chrono::Utc>,
    pub container_stats: Vec<ContainerStats>,
}

#[derive(Debug, serde::Serialize)]
pub struct ContainerStats {
    pub container_runtime: ContainerRuntime,
    pub id: String,
    pub host_name: String,
    pub created_at: i64,
    pub status: String,
    pub running: bool,
    pub running_for_seconds: u64,
    pub image_name: String,
    pub networks: Vec<String>,
    pub cpu_usage_percent: f64,
    pub memory_usage_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContainerRuntime {
    Docker,
    DockerDesktop,
    Podman,
}

impl ContainerRuntime {
    fn socket_uri(&self) -> String {
        match self {
            ContainerRuntime::Docker => "unix:///var/run/docker.sock".to_string(),
            ContainerRuntime::DockerDesktop => {
                let home = std::env::var("HOME").unwrap_or_default();
                format!("unix://{}/.docker/desktop/docker.sock", home)
            }
            ContainerRuntime::Podman => {
                #[cfg(target_os = "linux")]
                {
                    let uid = nix::unistd::getuid().as_raw();
                    format!("unix:///run/user/{}/podman/podman.sock", uid)
                }
                #[cfg(not(target_os = "linux"))]
                {
                    std::process::Command::new("podman")
                        .args(["machine", "inspect", "--format", "{{.ConnectionInfo.PodmanSocket.Path}}"])
                        .output()
                        .ok()
                        .and_then(|o| String::from_utf8(o.stdout).ok())
                        .map(|s| format!("unix://{}", s.trim()))
                        .unwrap_or_default()
                }
            }
        }
    }
}

impl fmt::Display for ContainerRuntime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Docker => "docker socket",
            Self::DockerDesktop => "docker desktop",
            Self::Podman => "podman socket",
        };
        write!(f, "{}", s)
    }
}

pub fn check_runtime_availability() -> Option<Vec<ContainerRuntime>> {
    let runtimes = [
        ContainerRuntime::Docker,
        ContainerRuntime::DockerDesktop,
        ContainerRuntime::Podman,
    ];

    let mut available_runtimes = Vec::new();

    for runtime in runtimes.iter() {
        let socket_uri = runtime.socket_uri();
        let socket_path = socket_uri.trim_start_matches("unix://");
        debug!("Checking availability of {} at {}", runtime, socket_path);
        if std::path::Path::new(socket_path).exists() {
            debug!("Found available container runtime: {}", runtime);
            available_runtimes.push(runtime.clone());
        }
    }

    if available_runtimes.is_empty() {
        debug!("No container runtime sockets found");
        None
    } else {
        Some(available_runtimes)
    }
}

/// Calculates CPU usage % from a Docker stats JSON snapshot.
/// Docker requires two samples to compute a delta; the stats stream emits
/// the previous sample in `precpu_stats` alongside the current `cpu_stats`.
fn parse_cpu_percent(stats: serde_json::Value) -> f64 {
    let cpu_delta = stats["cpu_stats"]["cpu_usage"]["total_usage"]
        .as_u64()
        .unwrap_or(0)
        .saturating_sub(
            stats["precpu_stats"]["cpu_usage"]["total_usage"]
                .as_u64()
                .unwrap_or(0),
        );

    let system_delta = stats["cpu_stats"]["system_cpu_usage"]
        .as_u64()
        .unwrap_or(0)
        .saturating_sub(
            stats["precpu_stats"]["system_cpu_usage"]
                .as_u64()
                .unwrap_or(0),
        );

    let num_cpus = stats["cpu_stats"]["online_cpus"]
        .as_u64()
        .unwrap_or(1)
        .max(1);

    if system_delta == 0 {
        return 0.0;
    }

    (cpu_delta as f64 / system_delta as f64) * num_cpus as f64 * 100.0
}

pub async fn get_current_stats(
) -> Result<Option<ContainerRuntimeStats>, docker_api::Error> {
    let container_runtimes = check_runtime_availability();
    if container_runtimes.is_none() {
        debug!("No container runtimes found, skipping collection");
        return Ok(None);
    }

    let mut all_container_stats: Vec<ContainerStats> = Vec::new();
    let mut seen_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    for container_runtime in container_runtimes.unwrap() {
        let result: Result<Vec<ContainerStats>, docker_api::Error> = async {
            let socket_uri = container_runtime.socket_uri();
            debug!("Docker: attempting to connect to {}", socket_uri);

            let docker = docker_api::Docker::new(&socket_uri)?;
            debug!("Docker: client created successfully");

            debug!("Docker: sending ping...");
            docker.ping().await?;
            debug!("Docker: ping succeeded");

            debug!("Docker: listing containers...");
            let containers_api = docker.containers();
            let summaries = containers_api
                .list(&ContainerListOpts::builder().all(true).build())
                .await?;

            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            let mut container_stats = Vec::new();

            for c in summaries {
                let id = c.id.unwrap_or_default();
                let host_name = c
                    .names
                    .as_ref()
                    .and_then(|n| n.first())
                    .map(|n| n.trim_start_matches('/').to_string())
                    .unwrap_or_default();
                let created_at = c.created.unwrap_or(0);
                let status = c.status.unwrap_or_default();
                let state = c.state.unwrap_or_default();
                let running = state == "running";
                let running_for_seconds = if running && created_at > 0 {
                    now.saturating_sub(created_at as u64)
                } else {
                    0
                };
                let image_name = c.image.unwrap_or_default();
                let networks = c
                    .network_settings
                    .and_then(|ns| ns.networks)
                    .map(|map| map.into_keys().collect())
                    .unwrap_or_default();

                // Fetch one stats sample (only available for running containers)
                let container = containers_api.get(&id);
                let (cpu_usage_percent, memory_usage_bytes) = if running {
                    let mut stream = container.stats();
                    let next = stream.next().await;
                    if let Some(Ok(snapshot)) = next {
                        let mem = snapshot["memory_stats"]["usage"].as_u64().unwrap_or(0);
                        let cpu = parse_cpu_percent(snapshot);
                        (cpu, mem)
                    } else {
                        (0.0, 0)
                    }
                } else {
                    (0.0, 0)
                };

                container_stats.push(ContainerStats {
                    container_runtime: container_runtime.clone(),
                    id,
                    host_name,
                    created_at,
                    status,
                    running,
                    running_for_seconds,
                    image_name,
                    networks,
                    cpu_usage_percent,
                    memory_usage_bytes,
                });
            }
            Ok(container_stats)
        }.await;

        match result {
            Ok(stats) => {
                for stat in stats {
                    if seen_ids.insert(stat.id.clone()) {
                        all_container_stats.push(stat);
                    } else {
                        debug!("Skipping duplicate container {} from {}", stat.id, stat.container_runtime);
                    }
                }
            }
            Err(e) => {
                debug!("Runtime {} failed: {}, trying next", container_runtime, e);
            }
        }
    }

    if all_container_stats.is_empty() {
        Ok(None)
    } else {
        Ok(Some(ContainerRuntimeStats {
            collected_at: chrono::Utc::now(),
            container_stats: all_container_stats,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "linux")]
    #[test]
    fn test_podman_socket_uri_uses_real_uid() {
        let uri = ContainerRuntime::Podman.socket_uri();
        let real_uid = nix::unistd::getuid().as_raw();
        let expected = format!("unix:///run/user/{}/podman/podman.sock", real_uid);
        assert_eq!(uri, expected, "Podman socket path must use the real UID, not a hardcoded fallback");
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn test_podman_socket_uri_uses_machine_inspect_on_non_linux() {
        let uri = ContainerRuntime::Podman.socket_uri();
        assert!(
            std::path::Path::new(uri.trim_start_matches("unix://")).exists(),
            "Podman socket path '{}' must point to an existing file on non-Linux", uri
        );
    }

    #[ignore = "requires a running Docker socket"]
    #[tokio::test]
    async fn test_list_containers() {
        let Some(result) = get_current_stats().await.unwrap() else {
            println!("No container runtime available, skipping test");
            return;
        };

        for c in &result.container_stats {
            println!("---");
            println!("  id:              {}", c.id);
            println!("  name:            {}", c.host_name);
            println!("  image:           {}", c.image_name);
            println!("  status:          {}", c.status);
            println!("  running:         {}", c.running);
            println!("  uptime (s):      {}", c.running_for_seconds);
            println!("  created_at:      {}", c.created_at);
            println!("  networks:        {}", c.networks.join(", "));
            println!("  cpu %:           {:.2}", c.cpu_usage_percent);
            println!("  memory (bytes):  {}", c.memory_usage_bytes);
        }
    }
}
