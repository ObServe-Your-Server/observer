use docker_api::opts::ContainerListOpts;
use futures_util::StreamExt;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::system_health::HostSytemHealth;

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerStats {
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

pub async fn list_containers(_host_system_health: HostSytemHealth) -> Option<Vec<ContainerStats>> {
    let docker = match docker_api::Docker::new("unix:///var/run/docker.sock") {
        Ok(d) => d,
        Err(e) => {
            // The docker socket is not available so no docker installed or unavailable
            log::warn!("Docker socket unavailable: {}", e);

            return None;
        }
    };

    if docker.ping().await.is_err() {
        log::warn!("Docker socket unavailable: ping failed");
        return None;
    }

    let containers_api = docker.containers();
    let summaries = match containers_api
        .list(&ContainerListOpts::builder().all(true).build())
        .await
    {
        Ok(s) => s,
        Err(e) => {
            log::warn!("Failed to list containers: {}", e);
            return None;
        }
    };

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let mut results = Vec::new();

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

        results.push(ContainerStats {
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

    Some(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(ignore)]
    #[tokio::test]
    async fn test_list_containers() {
        let containers = list_containers().await.unwrap();
        for c in &containers {
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
