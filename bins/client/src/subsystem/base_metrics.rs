use log::{debug, error};
use serde::{Deserialize, Serialize};
use open_eye::collector::cpu::collector::CpuStats;
use open_eye::collector::disk::collector::{DiskInfo, DiskStats};
use open_eye::collector::memory::collector::MemoryStats;
use open_eye::collector::network::collector::NetworkStats;
use open_eye::collector::systemstats::collector::SystemStats;
use crate::config::get_config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseMetrics {
    pub cpu: Option<CpuStats>,
    pub memory: Option<MemoryStats>,
    pub disks: Option<Vec<DiskInfo>>,
    pub network: Option<NetworkStats>,
    pub system: Option<SystemStats>,
}

impl BaseMetrics {
    pub async fn collect() -> BaseMetrics {
        let (cpu, memory, disks, network, system) = tokio::join!(
            tokio::task::spawn_blocking(CpuStats::get_current_stats),
            tokio::task::spawn_blocking(MemoryStats::get_current_stats),
            tokio::task::spawn_blocking(DiskStats::get_current_stats),
            tokio::task::spawn_blocking(NetworkStats::get_current_stats),
            tokio::task::spawn_blocking(SystemStats::get_current_stats),
        );

        BaseMetrics {
            cpu: cpu.map_err(|e| error!("cpu collector panicked: {e}")).ok(),
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
}