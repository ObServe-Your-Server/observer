use crate::jobs::job::JobTrait;
use crate::notification::notification_manager::{NotificationManager, NotificationSender};
use crate::notification::types::memory_notification::MemoryNotification;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use futures::future::OptionFuture;
use open_eye::collector::cpu::collector::CpuStats;
use open_eye::collector::memory::collector::MemoryStats;
use open_eye::collector::network::collector::NetworkStats;
use open_eye::collector::partition::collector::PartitionInfo;
use open_eye::collector::systemstats::collector::SystemStats;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[async_trait]
pub trait BaseMetricCollectionStorageEngine: Send + Sync {
    //TODO remove the clone for base metrics and other jobs, so ref to base_metrics
    async fn save_base_metrics(&self, base_metrics: BaseMetrics) -> Result<()>;
}

pub struct BaseMetricCollectionJob {
    storage_engine: Arc<dyn BaseMetricCollectionStorageEngine>,
    notification_manager: Arc<dyn NotificationSender>,
}

fn flatten<T>(
    name: &str,
    res: Option<Result<anyhow::Result<T>, tokio::task::JoinError>>,
) -> anyhow::Result<()> {
    match res {
        None => {
            log::debug!("No notification call because no {name} data");
            Ok(())
        }
        Some(Ok(Ok(_))) => Ok(()),
        Some(Ok(Err(err))) => Err(err.context(format!("{name} notification failed"))),
        Some(Err(err)) => Err(anyhow!("Failed to join on {name} future: {err}")),
    }
}

impl BaseMetricCollectionJob {
    pub fn new(
        storage_engine: Arc<dyn BaseMetricCollectionStorageEngine>,
        notification_manager: Arc<NotificationManager>,
    ) -> BaseMetricCollectionJob {
        BaseMetricCollectionJob {
            storage_engine,
            notification_manager,
        }
    }
}

#[async_trait]
impl JobTrait for BaseMetricCollectionJob {
    async fn run(&self) -> Result<()> {
        let base_metrics = BaseMetrics::collect().await;
        let save_metrics_fut = self.storage_engine.save_base_metrics(base_metrics.clone());

        let cpu_metrics = base_metrics.cpu;
        let cpu_notification_fut = cpu_metrics.map(|stats| {
            // needed because otherwise not movable out of &self into future
            let notification_manager = self.notification_manager.clone();
            tokio::spawn(async move {
                notification_manager
                    .suggest_notification(&MemoryNotification {
                        memory_usage_in_percent: stats.cpu_usage_percent as u8,
                        title: "Cpu".to_string(),
                        body: format!("Cpu at {}%", stats.cpu_usage_percent as u8),
                    })
                    .await
            })
        });

        let memory_metrics = base_metrics.memory;
        let memory_notification_fut = memory_metrics.map(|memory_metrics| {
            let notification_manager = self.notification_manager.clone();
            tokio::spawn(async move {
                notification_manager
                    .suggest_notification(&MemoryNotification {
                        memory_usage_in_percent: (memory_metrics.used_memory_in_byte
                            / memory_metrics.available_memory_in_byte)
                            as u8,
                        title: "Memory".to_string(),
                        body: format!(
                            "Memory at {}%",
                            (memory_metrics.used_memory_in_byte
                                / memory_metrics.available_memory_in_byte)
                                as u8
                        ),
                    })
                    .await
            })
        });

        let (save_metrics_res, cpu_res, mem_res) = tokio::join!(
            save_metrics_fut,
            OptionFuture::from(cpu_notification_fut),
            OptionFuture::from(memory_notification_fut)
        );

        // saving metrics failed, which doesn't mean notifications worked.
        // notifications are not that serious so saving metrics throws first
        save_metrics_res?;

        // both tasks are finished here
        let cpu = flatten("cpu", cpu_res);
        let mem = flatten("memory", mem_res);

        cpu?;
        mem?;

        Ok(())
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
