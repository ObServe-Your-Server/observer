use crate::config::get_config;
use log::{debug, error};
use open_eye::collector::{
    cpu::collector::CpuStats,
    disk::collector::{DiskInfo, DiskStats},
    memory::collector::MemoryStats,
    network::collector::NetworkStats,
    systemstats::collector::SystemStats,
};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use tokio::sync::RwLock;
use anyhow::Result;

static LAST_METRICS: OnceLock<RwLock<Option<BaseMetrics>>> = OnceLock::new();

fn last_metrics() -> &'static RwLock<Option<BaseMetrics>> {
    LAST_METRICS.get_or_init(|| RwLock::new(None))
}

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

    pub async fn run() -> Result<()> {
        let config = get_config();
        let metrics = BaseMetrics::collect().await;

        debug!("Host metrics collected: {:?}", metrics);

        let last = last_metrics().read().await.clone();

        *last_metrics().write().await = Some(metrics);

        todo!("save to db")
    }
}

#[cfg(test)]
mod tests {
    use crate::subsystem::base_metrics_system::BaseMetrics;

    #[ignore = "just collects host metrics"]
    #[tokio::test]
    async fn run_test() {
        let metrics = BaseMetrics::collect().await;

        println!("Collected metrics: {:?}", metrics);
    }
}
