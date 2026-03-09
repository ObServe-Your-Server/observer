use std::sync::{Arc, LazyLock};

use log::{error, info};
use tokio::{sync::RwLock, task};

use crate::{
    client::{
        host::metric_collection::Metrics,
        notification::notification::{Notification, NotificationType, Status},
    },
    config::{Config, get_config},
};

#[allow(dead_code)]
struct State {
    cpu_health: Status,
    memory_health: Status,
    storage_health: Status,
    packet_sender_health: Status,
    network_health: Status,
    docker_health: Status,
}

#[allow(dead_code)]
static STATE: LazyLock<Arc<RwLock<State>>> = LazyLock::new(|| {
    Arc::new(RwLock::new(State {
        cpu_health: Status::Healthy,
        memory_health: Status::Healthy,
        storage_health: Status::Healthy,
        packet_sender_health: Status::Healthy,
        network_health: Status::Healthy,
        docker_health: Status::Healthy,
    }))
});

pub async fn send_metric_notification(metrics: Metrics) -> Result<(), Box<dyn std::error::Error>> {
    // go through each metric and send notifications
    
    // -------------- cpu --------------
    let cpu_notification = build_cpu_message(metrics.cpu_usage_percent as f64);
    if cpu_notification.machine_status != STATE.read().await.cpu_health {
        let mut state = STATE.write().await;
        state.cpu_health = cpu_notification.machine_status.clone();
        task::spawn(async move {
            send_notification(cpu_notification).await;
        });
    }

    // -------------- ram --------------
    let ram_notification =
        build_ram_message((metrics.ram_used_bytes as f64 / metrics.ram_total_bytes as f64) * 100.0);
    if ram_notification.machine_status != STATE.read().await.memory_health {
        let mut state = STATE.write().await;
        state.memory_health = ram_notification.machine_status.clone();
        task::spawn(async move {
            send_notification(ram_notification).await;
        });
    }

    // -------------- disk --------------
    let complete_disk_usage = metrics
        .disks
        .iter()
        .map(|d| (d.used_bytes as f64 / d.total_bytes as f64) * 100.0)
        .fold(0.0, |a, b| a + b)
        / metrics.disks.len() as f64;
    let disk_notification = build_storage_message(complete_disk_usage);
    if disk_notification.machine_status != STATE.read().await.storage_health {
        let mut state = STATE.write().await;
        state.storage_health = disk_notification.machine_status.clone();
        task::spawn(async move {
            send_notification(disk_notification).await;
        });
    }
    
    // -------------- packet sender --------------
    // TODO
    
    // -------------- network health --------------
    // TODO
    
    // -------------- docker --------------

    Ok(())
}

fn build_cpu_message(cpu_usage: f64) -> Notification {
    match cpu_usage {
        cpu_usage if cpu_usage > 90.0 => Notification {
            notification_type: NotificationType::CpuNotification,
            message: format!("CPU usage is extremely high at {}%", cpu_usage),
            machine_status: Status::Critical,
        },
        cpu_usage if cpu_usage > 80.0 => Notification {
            notification_type: NotificationType::CpuNotification,
            message: format!("CPU usage is high at {}%", cpu_usage),
            machine_status: Status::Warning,
        },
        cpu_usage if cpu_usage <= 50.0 => Notification {
            notification_type: NotificationType::CpuNotification,
            message: format!("CPU usage is okay at {}%", cpu_usage),
            machine_status: Status::Healthy,
        },
        _ => Notification {
            notification_type: NotificationType::CpuNotification,
            message: format!("CPU usage is unknown: {}", cpu_usage),
            machine_status: Status::Healthy,
        },
    }
}

fn build_ram_message(ram_usage: f64) -> Notification {
    match ram_usage {
        ram_usage if ram_usage > 90.0 => Notification {
            notification_type: NotificationType::RamNotification,
            message: format!("RAM usage is extremely high at {}%", ram_usage),
            machine_status: Status::Critical,
        },
        ram_usage if ram_usage > 80.0 => Notification {
            notification_type: NotificationType::RamNotification,
            message: format!("RAM usage is high at {}%", ram_usage),
            machine_status: Status::Warning,
        },
        ram_usage if ram_usage <= 50.0 => Notification {
            notification_type: NotificationType::RamNotification,
            message: format!("RAM usage is okay at {}%", ram_usage),
            machine_status: Status::Healthy,
        },
        _ => Notification {
            notification_type: NotificationType::RamNotification,
            message: format!("RAM usage is unknown: {}", ram_usage),
            machine_status: Status::Healthy,
        },
    }
}

fn build_storage_message(storage_usage: f64) -> Notification {
    match storage_usage {
        storage_usage if storage_usage > 90.0 => Notification {
            notification_type: NotificationType::StorageNotification,
            message: format!("Storage usage is extremely high at {}%", storage_usage),
            machine_status: Status::Critical,
        },
        storage_usage if storage_usage > 80.0 => Notification {
            notification_type: NotificationType::StorageNotification,
            message: format!("Storage usage is high at {}%", storage_usage),
            machine_status: Status::Warning,
        },
        storage_usage if storage_usage <= 50.0 => Notification {
            notification_type: NotificationType::StorageNotification,
            message: format!("Storage usage is low at {}%", storage_usage),
            machine_status: Status::Healthy,
        },
        _ => Notification {
            notification_type: NotificationType::StorageNotification,
            message: format!("Storage usage is unknown: {}", storage_usage),
            machine_status: Status::Healthy,
        },
    }
}

pub async fn send_notification(notification: Notification) {
    let config: &Config = get_config();

    let url = &config.server.base_notifier_url;
    let client = reqwest::Client::new();
    let result = client.post(url).json(&notification).send().await;

    match result {
        Ok(resp) if resp.status().is_success() => {
            info!("Metrics sent ({})", resp.status());
        }
        Ok(resp) => {
            error!("Server rejected metrics: {}", resp.status());
        }
        Err(e) => {
            error!("Failed to send metrics: {}", e);
        }
    }
}
