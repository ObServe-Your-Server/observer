use crate::notification::notification_handler::{NotificationHandler, PushNotification};
use crate::scheduling::job::Job;
use crate::storage_engine::storage_engine::StorageEngine;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{Duration, Utc};
use open_eye::collector::cpu::collector::CpuStats;
use open_eye::collector::disk::collector::{DiskInfo, DiskStats};
use open_eye::collector::memory::collector::MemoryStats;
use open_eye::collector::network::collector::NetworkStats;
use open_eye::collector::systemstats::collector::SystemStats;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::config::{Config, ServerConfig};

struct Notification {
    notification_type: NotificationType,
    severity: Severity,
    notification: PushNotification,
    send_at: chrono::DateTime<Utc>,
}

#[derive(PartialEq, Clone, Copy)]
enum NotificationType {
    CPU,
    MEMORY,
    DISK,
}

#[derive(PartialEq, Clone, Copy)]
enum Severity {
    LOW,
    MEDIUM,
    HIGH
}

pub struct NotificationCooldowns {
    cpu: Duration,
    memory: Duration,
    disk: Duration,
}

impl NotificationCooldowns {
    pub fn from_config(config: &Config) -> NotificationCooldowns {
        NotificationCooldowns {
            cpu: Duration::seconds(config.intervals.cpu_notification_cooldown as i64),
            memory: Duration::seconds(config.intervals.memory_notification_cooldown as i64),
            disk: Duration::seconds(config.intervals.disk_notification_cooldown as i64),
        }
    }

    fn for_type(&self, notification_type: NotificationType) -> Duration {
        match notification_type {
            NotificationType::CPU => self.cpu,
            NotificationType::MEMORY => self.memory,
            NotificationType::DISK => self.disk,
        }
    }
}

pub struct BaseMetricCollectionJob {
    storage_engine: Arc<StorageEngine>,
    schedule_time: Duration,
    notification_handler: NotificationHandler,
    notification_cooldowns: NotificationCooldowns,
    last_sent_notifications: Mutex<Vec<Notification>>,
}

impl BaseMetricCollectionJob {
    pub fn new(
        storage_engine: Arc<StorageEngine>,
        schedule_time: Duration,
        notification_handler: NotificationHandler,
        notification_cooldowns: NotificationCooldowns,
    ) -> BaseMetricCollectionJob {
        BaseMetricCollectionJob {
            storage_engine,
            schedule_time,
            notification_handler,
            notification_cooldowns,
            last_sent_notifications: Mutex::new(vec![]),
        }
    }

    async fn try_send_notification(
        &self,
        notification_type: NotificationType,
        severity: Severity,
        title: &str,
        body: String,
    ) -> Result<()> {
        let cooldown = self.notification_cooldowns.for_type(notification_type);
        let mut last_sent_notifications = self.last_sent_notifications.lock().await;
        let already_sent = last_sent_notifications.iter().any(|n| {
            n.notification_type == notification_type
                && n.severity == severity
                && (cooldown.is_zero() || Utc::now() - n.send_at < cooldown)
        });

        if !already_sent {
            let notification = Notification {
                notification_type,
                severity,
                notification: PushNotification {
                    title: title.to_string(),
                    body,
                },
                send_at: Utc::now(),
            };
            self.notification_handler.send_push_notification(&notification.notification).await?;
            last_sent_notifications.push(notification);
        }

        Ok(())
    }

    async fn clear_notification(&self, notification_type: NotificationType, title: &str, body: String) -> Result<()> {
        let mut last_sent_notifications = self.last_sent_notifications.lock().await;
        let had_active_notification = last_sent_notifications
            .iter()
            .any(|n| n.notification_type == notification_type);

        if had_active_notification {
            last_sent_notifications.retain(|n| n.notification_type != notification_type);

            let notification = PushNotification {
                title: title.to_string(),
                body,
            };
            self.notification_handler.send_push_notification(&notification).await?;
        }

        Ok(())
    }

    async fn check_health_send_notification(&self, storage_engine: &StorageEngine) -> Result<()> {
        let cpu_metrics = storage_engine.get_cpu_stats_latest(3).await?;
        let memory_metrics = storage_engine.get_memory_stats_latest(3).await?;
        let disk_metrics = storage_engine.get_disk_stats_latest(3).await?;

        let cpu_usage_avg: f32 = cpu_metrics.iter().map(|(m, _)| m.cpu_usage_percent).sum::<f32>() / cpu_metrics.len() as f32;

        if cpu_usage_avg >= 90.0 {
            self.try_send_notification(
                NotificationType::CPU,
                Severity::HIGH,
                "CPU notification",
                format!("Cpu usage is high: {:.1} % avg", cpu_usage_avg),
            ).await?;
        } else if cpu_usage_avg >= 80.0 {
            self.try_send_notification(
                NotificationType::CPU,
                Severity::MEDIUM,
                "CPU notification",
                format!("Cpu usage is high: {:.1} % avg", cpu_usage_avg),
            ).await?;
        } else {
            self.clear_notification(
                NotificationType::CPU,
                "CPU notification",
                format!("Cpu usage is back to normal: {:.1} % avg", cpu_usage_avg),
            ).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl Job for BaseMetricCollectionJob {
    async fn run(&self) -> Result<()> {
        let base_metrics = BaseMetrics::collect().await;
        self.storage_engine.save_base_metrics_to_db(base_metrics).await?;
        self.check_health_send_notification(&self.storage_engine).await
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
