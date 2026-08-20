use serde::{Deserialize, Serialize};
use getset::Getters;

#[derive(Debug, Deserialize, Serialize, Getters)]
#[serde(rename_all = "snake_case")]
pub struct NotificationConfig {
    #[getset(get = "pub")]
    renotify_after_x: Option<u16>,

    #[getset(get = "pub")]
    enable_cpu_notification: bool,
    #[getset(get = "pub")]
    cpu_notification_mode: Option<NotificationMode>,
    #[getset(get = "pub")]
    cpu_high_after: Option<u32>,
    #[getset(get = "pub")]
    cpu_low_after: Option<u32>,
    #[getset(get = "pub")]
    cpu_high_percentage: Option<u8>,
    #[getset(get = "pub")]
    cpu_low_percentage: Option<u8>,

    #[getset(get = "pub")]
    enable_memory_notification: bool,
    #[getset(get = "pub")]
    memory_notification_mode: Option<NotificationMode>,
    #[getset(get = "pub")]
    memory_high_after: Option<u32>,
    #[getset(get = "pub")]
    memory_low_after: Option<u32>,
    #[getset(get = "pub")]
    memory_high_percentage: Option<u8>,
    #[getset(get = "pub")]
    memory_low_percentage: Option<u8>,

    #[getset(get = "pub")]
    enable_disk_notification: bool,
    #[getset(get = "pub")]
    disk_notification_mode: Option<NotificationMode>,
    #[getset(get = "pub")]
    disk_high_percentage: Option<u8>,
    #[getset(get = "pub")]
    disk_low_percentage: Option<u8>,

    #[getset(get = "pub")]
    enable_advanced_container_socket_notifications: bool,
    #[getset(get = "pub")]
    notify_on_high_container_socket_usage: Option<bool>,
    #[getset(get = "pub")]
    high_container_socket_cpu_usage_percent: Option<u8>,
    #[getset(get = "pub")]
    low_container_socket_cpu_usage_percent: Option<u8>,
    #[getset(get = "pub")]
    notify_on_container_down: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationMode {
    Once,
    Continuous,
}