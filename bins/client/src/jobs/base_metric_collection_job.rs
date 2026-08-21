use crate::scheduling::job::JobTrait;
use crate::storage_engine::storage_engine::StorageEngine;
use anyhow::Result;
use async_trait::async_trait;
use chrono::Duration;
use open_eye::collector::cpu::collector::CpuStats;
use open_eye::collector::partition::collector::PartitionInfo;
use open_eye::collector::memory::collector::MemoryStats;
use open_eye::collector::network::collector::NetworkStats;
use open_eye::collector::systemstats::collector::SystemStats;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[async_trait]
pub trait BaseMetricCollectionStorageEngine: Send + Sync {
    async fn save_base_metrics(&self, base_metrics: BaseMetrics) -> Result<()>;
}

pub struct BaseMetricCollectionJob {
    storage_engine: Arc<dyn BaseMetricCollectionStorageEngine>,
}

impl BaseMetricCollectionJob {
    pub fn new(
        storage_engine: Arc<dyn BaseMetricCollectionStorageEngine>,
    ) -> BaseMetricCollectionJob {
        BaseMetricCollectionJob {
            storage_engine
        }
    }
}

#[async_trait]
impl JobTrait for BaseMetricCollectionJob {
    async fn run(&self) -> Result<()> {
        let base_metrics = BaseMetrics::collect().await;
        self.storage_engine.save_base_metrics(base_metrics).await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseMetrics {
    pub cpu: Option<CpuStats>,
    pub memory: Option<MemoryStats>,
    pub disks: Option<Vec<PartitionInfo>>,
    pub network: Option<NetworkStats>,
    pub system: Option<SystemStats>,
}

impl BaseMetrics {
    pub async fn collect() -> BaseMetrics {
        // spawn blocking because no waiting internally
        let (cpu, memory, disks, network, system) = tokio::join!(
            tokio::task::spawn_blocking(CpuStats::get_current_stats),
            tokio::task::spawn_blocking(MemoryStats::get_current_stats),
            tokio::task::spawn_blocking(PartitionInfo::get_current_stats),
            tokio::task::spawn_blocking(NetworkStats::get_current_stats),
            tokio::task::spawn_blocking(SystemStats::get_current_stats),
        );

        BaseMetrics {
            cpu: cpu
                .map_err(|e| log::error!("cpu collector panicked: {e}"))
                .ok(),
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
