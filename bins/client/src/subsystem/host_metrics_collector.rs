use std::sync::OnceLock;
use tokio::sync::RwLock;
use log::{debug, error};

use open_eye::collector::{
    cpu::collector::CpuStats,
    disk::collector::{DiskInfo, DiskStats},
    memory::collector::MemoryStats,
    network::collector::NetworkStats,
    systemstats::collector::SystemStats,
};
use serde::{Deserialize, Serialize};

use crate::{
    mapper::{
        host_metrics_mapper::HostSystemMapper, host_metrics_models::mapped_host_system_metrics,
    },
    scheduling::collection_error::CollectionError, sender::host_system_metrics_sender::HostSystemMetricsSender,
};

static LAST_METRICS: OnceLock<RwLock<Option<HostMetrics>>> = OnceLock::new();

fn last_metrics() -> &'static RwLock<Option<HostMetrics>> {
    LAST_METRICS.get_or_init(|| RwLock::new(None))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostMetrics {
    pub cpu: Option<CpuStats>,
    pub memory: Option<MemoryStats>,
    pub disks: Option<Vec<DiskInfo>>,
    pub network: Option<NetworkStats>,
    pub system: Option<SystemStats>,
}

impl HostMetrics {
    pub async fn collect() -> HostMetrics {
        let (cpu, memory, disks, network, system) = tokio::join!(
            tokio::task::spawn_blocking(CpuStats::get_current_stats),
            tokio::task::spawn_blocking(MemoryStats::get_current_stats),
            tokio::task::spawn_blocking(DiskStats::get_current_stats),
            tokio::task::spawn_blocking(NetworkStats::get_current_stats),
            tokio::task::spawn_blocking(SystemStats::get_current_stats),
        );

        HostMetrics {
            cpu: cpu
                .map_err(|e| error!("cpu collector panicked: {e}"))
                .ok(),
            memory: memory
                .map_err(|e| error!("memory collector panicked: {e}"))
                .ok(),
            disks: disks
                .map_err(|e| error!("disk collector panicked: {e}"))
                .ok(),
            network: network
                .map_err(|e| error!("network collector panicked: {e}"))
                .ok(),
            system: system
                .map_err(|e| error!("system stats collector panicked: {e}"))
                .ok(),
        }
    }

    pub async fn run() -> Result<(), CollectionError> {
        let metrics = HostMetrics::collect().await;

        debug!("Host metrics collected: {:?}", metrics);

        // first map then send the metrics
        let mapped_metrics = HostSystemMapper::map_for_watch_tower(
            metrics,
            last_metrics().read().await.clone(),
        );
        return HostSystemMetricsSender::send(mapped_metrics).await;
    }
}

#[cfg(test)]
mod tests {
    use crate::subsystem::host_metrics_collector::HostMetrics;

    #[tokio::test]
    async fn run_test() {
        let metrics = HostMetrics::collect().await;

        println!("Collected metrics: {:?}", metrics);
    }
}
