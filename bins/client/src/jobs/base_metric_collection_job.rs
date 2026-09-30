use crate::jobs::job::JobTrait;
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
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::config::config_parts::notification_config::NotificationConfig;
use crate::notification::metric_notification::MetricNotification;
use crate::notification::notification_kind::NotificationKind;
use crate::notification::notification_manager::NotificationManager;
use crate::notification::notification_urgency::NotificationUrgency;

#[async_trait]
pub trait BaseMetricCollectionStorageEngine: Send + Sync {
    //TODO remove the clone for base metrics and other jobs, so ref to base_metrics
    async fn save_base_metrics(&self, base_metrics: BaseMetrics) -> Result<()>;
}

pub struct BaseMetricCollectionJob {
    storage_engine: Arc<dyn BaseMetricCollectionStorageEngine>,
    notification_manager: Arc<NotificationManager>,
    notification_config: NotificationConfig,
    disk_states: Mutex<HashMap<String, NotificationUrgency>>,
}

impl BaseMetricCollectionJob {
    pub fn new(
        storage_engine: Arc<dyn BaseMetricCollectionStorageEngine>,
        notification_manager: Arc<NotificationManager>,
        notification_config: NotificationConfig,
    ) -> BaseMetricCollectionJob {
        BaseMetricCollectionJob {
            storage_engine,
            notification_manager,
            notification_config,
            disk_states: Mutex::new(HashMap::new()),
        }
    }

    async fn send_base_metric_notifications(&self, base_metrics: &BaseMetrics) -> Result<()> {
        let config = &self.notification_config;

        if let Some(cpu) = &base_metrics.cpu {
            self.notify_usage(
                NotificationKind::Cpu,
                "CPU",
                cpu.cpu_usage_percent,
                config.cpu_high_percentage,
                config.cpu_low_percentage,
            )
            .await?;
        }

        if let Some(memory) = &base_metrics.memory {
            let usage_percent =
                memory.used_memory_in_byte as f32 / memory.total_memory_in_byte as f32 * 100.0;
            self.notify_usage(
                NotificationKind::Memory,
                "Memory",
                usage_percent,
                config.memory_high_percentage,
                config.memory_low_percentage,
            )
            .await?;
        }

        for disk in base_metrics.disks.iter().flatten() {
            let usage_percent = disk.used_bytes as f32 / disk.total_bytes as f32 * 100.0;
            let notification = Self::classify(
                NotificationKind::Disk,
                &format!("Disk ({})", disk.mount_point),
                usage_percent,
                config.disk_high_percentage,
                config.disk_low_percentage,
            );

            // Disks only notify once per status change, tracked per mount point.
            let previous = self
                .disk_states
                .lock()
                .unwrap()
                .insert(disk.mount_point.clone(), notification.urgency);
            let notify = match notification.urgency {
                NotificationUrgency::HighUsage => true,
                NotificationUrgency::LowUsage => previous.is_some(),
                _ => false,
            };
            if notify && previous != Some(notification.urgency) {
                self.notification_manager.send_notification(&notification).await?;
            }
        }

        Ok(())
    }

    async fn notify_usage(
        &self,
        kind: NotificationKind,
        label: &str,
        usage_percent: f32,
        high_percentage: Option<u8>,
        low_percentage: Option<u8>,
    ) -> Result<()> {
        let notification = Self::classify(kind, label, usage_percent, high_percentage, low_percentage);
        self.notification_manager.send_notification(&notification).await
    }

    /// `MediumUsage` means "between the thresholds".
    fn classify(
        kind: NotificationKind,
        label: &str,
        usage_percent: f32,
        high_percentage: Option<u8>,
        low_percentage: Option<u8>,
    ) -> MetricNotification {
        let (urgency, level) = if high_percentage.is_some_and(|high| usage_percent >= high as f32) {
            (NotificationUrgency::HighUsage, "high")
        } else if low_percentage.is_some_and(|low| usage_percent <= low as f32) {
            (NotificationUrgency::LowUsage, "low")
        } else {
            (NotificationUrgency::MediumUsage, "normal")
        };

        MetricNotification {
            kind,
            urgency,
            title: format!("{label} usage {level}"),
            body: format!("{label} usage is at {usage_percent:.1}%"),
        }
    }
}

#[async_trait]
impl JobTrait for BaseMetricCollectionJob {
    async fn run(&self) -> Result<()> {
        let base_metrics = BaseMetrics::collect().await;
        self.storage_engine.save_base_metrics(base_metrics.clone()).await?;

        if let Err(e) = self.send_base_metric_notifications(&base_metrics).await {
            log::error!("failed to send base metric notifications: {e}");
        }
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
