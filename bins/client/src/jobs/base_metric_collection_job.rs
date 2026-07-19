use std::sync::Arc;
use anyhow::Result;
use async_trait::async_trait;
use chrono::Duration;
use serde::{Deserialize, Serialize};
use open_eye::collector::cpu::collector::CpuStats;
use open_eye::collector::disk::collector::{DiskInfo, DiskStats};
use open_eye::collector::memory::collector::MemoryStats;
use open_eye::collector::network::collector::NetworkStats;
use open_eye::collector::systemstats::collector::SystemStats;
use crate::scheduling::job::Job;
use crate::storage_engine::storage_engine::StorageEngine;

pub struct BaseMetricCollectionJob{
    storage_engine: Arc<StorageEngine>,
    schedule_time: Duration,
}

impl BaseMetricCollectionJob{
    pub fn new(storage_engine: Arc<StorageEngine>, schedule_time: Duration)->BaseMetricCollectionJob{
        BaseMetricCollectionJob{
            storage_engine,
            schedule_time,
        }
    }
}

#[async_trait]
impl Job for BaseMetricCollectionJob {
    async fn run(&self) -> Result<()> {
        let base_metrics = BaseMetrics::collect().await;
        self.storage_engine.save_base_metrics_to_db(base_metrics).await
    }

    fn schedule_time(&self) -> Duration {
        self.schedule_time
    }

    fn name(&self) -> &str {
        "Base Metrics Collection Job"
    }
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
            cpu: cpu.map_err(|e| log::error!("cpu collector panicked: {e}")).ok(),
            memory: memory
                .map_err(|e| log::error!("memory collector panicked: {e}"))
                .ok(),
            disks: disks
                .map_err(|e| log::error!("disk collector panicked: {e}"))
                .ok(),
            network: network
                .map_err(|e| log::error!("network collector panicked: {e}"))
                .ok(),
            system: system
                .map_err(|e| log::error!("system stats collector panicked: {e}"))
                .ok(),
        }
    }
}
