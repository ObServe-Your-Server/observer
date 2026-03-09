use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    Healthy,
    Warning,
    Critical,
}

impl Status {
    fn as_str(&self) -> &'static str {
        match self {
            Status::Healthy => "healthy",
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
}

impl Notification {
    fn new(notification_type: NotificationType, message: String, machine_status: Status) -> Self {
        Self {
            notification_type,
            message,
            machine_status,
        }
    }
}