use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct NotificationConfig {
    pub renotify_after_x: Option<u16>,

    pub enable_cpu_notification: bool,
    pub cpu_notification_mode: Option<NotificationMode>,
    pub cpu_high_after: Option<u32>,
    pub cpu_low_after: Option<u32>,
    pub cpu_high_percentage: Option<u8>,
    pub cpu_low_percentage: Option<u8>,

    pub enable_memory_notification: bool,
    pub memory_notification_mode: Option<NotificationMode>,
    pub memory_high_after: Option<u32>,
    pub memory_low_after: Option<u32>,
    pub memory_high_percentage: Option<u8>,
    pub memory_low_percentage: Option<u8>,

    pub enable_disk_notification: bool,
    pub disk_notification_mode: Option<NotificationMode>,
    pub disk_high_percentage: Option<u8>,
    pub disk_low_percentage: Option<u8>,

    pub enable_advanced_container_socket_notifications: bool,
    pub notify_on_high_container_socket_usage: Option<bool>,
    pub high_container_socket_cpu_usage_percent: Option<u8>,
    pub low_container_socket_cpu_usage_percent: Option<u8>,
    pub notify_on_container_down: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationMode {
    Once,
    Continuous,
}