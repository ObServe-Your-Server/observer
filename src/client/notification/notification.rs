use chrono::Utc;
use log::{error, info};
use serde::Serialize;
use crate::config::{get_config, Config};

#[derive(Debug, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    Healthy,
    Info,
    Warning,
    Critical,
}

impl Status {
    fn as_str(&self) -> &'static str {
        match self {
            Status::Healthy => "healthy",
            Status::Info => "info",
            Status::Warning => "warning",
            Status::Critical => "critical",
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum NotificationType {
    CpuNotification,
    RamNotification,
    StorageNotification,
    PacketSenderNotification,
    NetworkNotification,
    DockerNotification,
}

impl NotificationType {
    fn as_str(&self) -> &'static str {
        match self {
            NotificationType::CpuNotification => "cpu",
            NotificationType::RamNotification => "ram",
            NotificationType::StorageNotification => "storage",
            NotificationType::PacketSenderNotification => "packet_sender",
            NotificationType::NetworkNotification => "network",
            NotificationType::DockerNotification => "docker",
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Notification {
    pub notification_type: NotificationType,
    pub message: String,
    pub machine_status: Status,
    time_code: chrono::DateTime<Utc>,
}

impl Notification {
    pub fn new(notification_type: NotificationType, message: String, machine_status: Status) -> Self {
        Self {
            notification_type,
            message,
            machine_status,
            time_code: Utc::now(),
        }
    }

    pub fn created_at(&self) -> chrono::DateTime<Utc> {
        self.time_code
    }

    pub async fn send(&self) {
        let config: &Config = get_config();

        let url = &config.server.base_notifier_url;
        let client = reqwest::Client::new();
        let result = client.post(url).json(self).send().await;

        match result {
            Ok(resp) if resp.status().is_success() => {
                info!("Metrics sent ({})", resp.status());
            }
            Ok(resp) => {
                error!("Server rejected notification: {}", resp.status());
            }
            Err(e) => {
                error!("Failed to send notification: {}", e);
            }
        }
    }
}