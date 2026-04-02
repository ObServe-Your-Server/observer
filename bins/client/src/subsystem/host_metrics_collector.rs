use crate::client::metric_collection_errors::CollectionError;
use open_eye::collector::{
    cpu::collector::CpuStats,
    disk::collector::{DiskInfo, DiskStats},
    memory::collector::MemoryStats,
    network::collector::NetworkStats,
    systemstats::collector::SystemStats,
};
use serde::{Deserialize, Serialize};

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
            cpu: cpu.map_err(|e| log::error!("cpu collector panicked: {e}")).ok(),
            memory: memory.map_err(|e| log::error!("memory collector panicked: {e}")).ok(),
            disks: disks.map_err(|e| log::error!("disk collector panicked: {e}")).ok(),
            network: network.map_err(|e| log::error!("network collector panicked: {e}")).ok(),
            system: system.map_err(|e| log::error!("system stats collector panicked: {e}")).ok(),
        }
    }

    pub async fn run() -> Result<(), CollectionError> {
        let metrics = HostMetrics::collect().await;
        log::info!("Host metrics collected: {:?}", metrics);
        Ok(())
    }
}
