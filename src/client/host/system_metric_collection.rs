use log::{debug, error, warn};
use reqwest::Client;
use std::sync::{RwLock, mpsc, Arc, LazyLock};
use std::time::Duration;
use sysinfo::System;

use crate::client::host::collectors::disk::DiskInfo;
use crate::client::host::collectors::{cpu, disk, network};
use crate::client::host::system_metric_sender;
use crate::client::host::speedtest::{self, SpeedtestResult};
use crate::client::notification::notification::Notification;
use crate::client::notification::notification::NotificationType::{CpuNotification, RamNotification, StorageNotification};
use crate::client::notification::notification::Status::{Critical, Healthy, Warning};

#[derive(Clone)]
struct NetworkBytesDelta {
    pub time_of_recording: std::time::Instant,
    pub bytes_received: u64,
    pub bytes_transmitted: u64,
}

static NETWORK_DELTA: RwLock<Option<NetworkBytesDelta>> = RwLock::new(None);

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Metrics {
    pub cpu_usage_percent: f32,
    pub cpu_count: usize,
    pub cpu_name: String,
    pub ram_used_bytes: u64,
    pub ram_total_bytes: u64,
    pub uptime_secs: u64,
    pub cpu_temp_celsius: Option<f32>,
    pub os_name: Option<String>,
    pub kernel_version: Option<String>,
    pub net_bytes_received: u64,
    pub net_bytes_transmitted: u64,
    pub net_bytes_received_per_second: u64,
    pub net_bytes_transmitted_per_second: u64,
    pub local_ip: Option<String>,
    pub disks: Vec<DiskInfo>,
    pub speedtest_result: Option<SpeedtestResult>,
    pub hostname: Option<String>,
}

impl Metrics {
    pub fn collect() -> Option<Self> {
        let (tx, rx) = mpsc::channel();

        std::thread::spawn(move || {
            let result = Self::do_collect();
            let _ = tx.send(result);
        });

        match rx.recv_timeout(Duration::from_millis(1800)) {
            Ok(metrics) => Some(metrics),
            Err(_) => {
                warn!("metrics collection exceeded 1800ms, aborting");
                None
            }
        }
    }

    fn do_collect() -> Self {
        let sys = cpu::new_sys_with_cpu();
        let cpu_info = cpu::collect(&sys);

        let ram_used_bytes = sys.used_memory();
        let ram_total_bytes = sys.total_memory();

        let disks = disk::collect();
        let net = network::collect();

        let now = std::time::Instant::now();
        let (net_bytes_received_per_second, net_bytes_transmitted_per_second) = {
            let prev = NETWORK_DELTA.read().unwrap();
            match prev.as_ref() {
                Some(snapshot) => {
                    let elapsed_secs = (now - snapshot.time_of_recording).as_secs_f64();
                    let delta_received =
                        ((net.bytes_received.saturating_sub(snapshot.bytes_received)) as f64
                            / elapsed_secs) as u64;
                    let delta_transmitted = ((net
                        .bytes_transmitted
                        .saturating_sub(snapshot.bytes_transmitted))
                        as f64
                        / elapsed_secs) as u64;
                    (delta_received, delta_transmitted)
                }
                None => (0, 0),
            }
        };

        *NETWORK_DELTA.write().unwrap() = Some(NetworkBytesDelta {
            time_of_recording: now,
            bytes_received: net.bytes_received,
            bytes_transmitted: net.bytes_transmitted,
        });

        Self {
            cpu_usage_percent: cpu_info.usage_percent,
            cpu_count: cpu_info.count,
            cpu_name: cpu_info.name,
            ram_used_bytes,
            ram_total_bytes,
            uptime_secs: System::uptime(),
            cpu_temp_celsius: cpu_info.temp_celsius,
            os_name: System::long_os_version(),
            kernel_version: System::kernel_version(),
            net_bytes_received: net.bytes_received,
            net_bytes_transmitted: net.bytes_transmitted,
            net_bytes_received_per_second,
            net_bytes_transmitted_per_second,
            local_ip: net.local_ip,
            disks,
            speedtest_result: None,
            hostname: System::host_name(),
        }
    }
}

pub async fn collection_job() {
    let client = Client::new();
    let Some(mut metrics) = Metrics::collect() else {
        return;
    };

    //debug!("Whole struct {:?}", metrics);
    debug!("CPU: {:.1}%", metrics.cpu_usage_percent);
    debug!(
        "RAM: {}MB / {}MB",
        metrics.ram_used_bytes / 1024 / 1024,
        metrics.ram_total_bytes / 1024 / 1024
    );
    debug!("Uptime: {}s", metrics.uptime_secs);
    if let Some(t) = metrics.cpu_temp_celsius {
        debug!("CPU temp: {:.1}°C", t);
    }
    for disk in &metrics.disks {
        debug!(
            "Disk [{}]: {}GB / {}GB",
            disk.name,
            disk.used_bytes / 1024 / 1024 / 1024,
            disk.total_bytes / 1024 / 1024 / 1024
        );
    }

    debug!(
        "Network bytes recieved per second: {}",
        metrics.net_bytes_received_per_second
    );
    debug!(
        "Network bytes transmitted per second: {}",
        metrics.net_bytes_transmitted_per_second
    );

    match speedtest::get_last_result() {
        Some(s) => {
            metrics.speedtest_result.replace(s.clone());
            debug!(
                "Speedtest: down={:.2}Mbps up={:.2}Mbps ping={:.1}ms",
                s.download_mbps, s.upload_mbps, s.ping_ms
            )
        }
        None => debug!("Speedtest: no measurement yet"),
    }


    // sending messages
    let metrics_clone = metrics.clone();
    tokio::spawn(async move {
        let result = send_metric_notification(metrics_clone).await;
        match result {
            Ok(_) => {}
            Err(e) => {
                error!("Failed to send metric notification: {}", e);
            }
        }
    });

    system_metric_sender::send(&client, &metrics).await;
}

#[allow(dead_code)]
struct State {
    cpu_health: crate::client::notification::notification::Status,
    memory_health: crate::client::notification::notification::Status,
    storage_health: crate::client::notification::notification::Status,
    packet_sender_health: crate::client::notification::notification::Status,
    network_health: crate::client::notification::notification::Status,
    docker_health: crate::client::notification::notification::Status,
}

#[allow(dead_code)]
static STATE: LazyLock<Arc<tokio::sync::RwLock<State>>> = LazyLock::new(|| {
    Arc::new(tokio::sync::RwLock::new(State {
        cpu_health: Healthy,
        memory_health: Healthy,
        storage_health: Healthy,
        packet_sender_health: Healthy,
        network_health: Healthy,
        docker_health: Healthy,
    }))
});

async fn send_metric_notification(metrics: Metrics) -> Result<(), std::io::Error> {
    // go through each metric and send notifications

    // -------------- cpu --------------
    let cpu_notification = build_cpu_message(metrics.cpu_usage_percent as f64);
    if cpu_notification.machine_status != STATE.read().await.cpu_health {
        debug!("Going to send notification for CPU health change: {:?}", cpu_notification);
        let mut state = STATE.write().await;
        state.cpu_health = cpu_notification.machine_status.clone();
        tokio::spawn(async move {
            cpu_notification.send().await;
        });
    }

    // -------------- ram --------------
    let ram_notification = build_ram_message((metrics.ram_used_bytes as f64 / metrics.ram_total_bytes as f64) * 100.0);
    if ram_notification.machine_status != STATE.read().await.memory_health {
        debug!("Going to send notification for RAM health change: {:?}", ram_notification);
        let mut state = STATE.write().await;
        state.memory_health = ram_notification.machine_status.clone();
        tokio::spawn(async move {
            ram_notification.send().await;
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
        debug!("Going to send notification for Storage health change: {:?}", disk_notification);
        let mut state = STATE.write().await;
        state.storage_health = disk_notification.machine_status.clone();
        tokio::spawn(async move {
            disk_notification.send().await;
        });
    }

    // -------------- packet sender --------------
    // TODO

    // -------------- network health --------------
    // TODO

    Ok(())
}

fn build_cpu_message(cpu_usage: f64) -> Notification {
    match cpu_usage {
        cpu_usage if cpu_usage > 90.0 => Notification::new(
            CpuNotification,
            format!("CPU usage is extremely high at {}%", cpu_usage),
            Critical,
        ),
        cpu_usage if cpu_usage > 80.0 => Notification::new(
            CpuNotification,
            format!("CPU usage is high at {}%", cpu_usage),
            Warning,
        ),
        cpu_usage if cpu_usage <= 50.0 => Notification::new(
            CpuNotification,
            format!("CPU usage is okay at {}%", cpu_usage),
            Healthy,
        ),
        _ => Notification::new(
            CpuNotification,
            format!("CPU usage is unknown: {}", cpu_usage),
            Healthy,
        ),
    }
}

fn build_ram_message(ram_usage: f64) -> Notification {
    match ram_usage {
        ram_usage if ram_usage > 90.0 => Notification::new(
            RamNotification,
            format!("RAM usage is extremely high at {}%", ram_usage),
            Critical,
        ),
        ram_usage if ram_usage > 80.0 => Notification::new(
            RamNotification,
            format!("RAM usage is high at {}%", ram_usage),
            Warning,
        ),
        ram_usage if ram_usage <= 80.0 => Notification::new(
            RamNotification,
            format!("RAM usage is low at {}%", ram_usage),
            Healthy,
        ),
        _ => Notification::new(
            RamNotification,
            format!("RAM usage is unknown: {}", ram_usage),
            Healthy,
        ),
    }
}

fn build_storage_message(storage_usage: f64) -> Notification {
    match storage_usage {
        storage_usage if storage_usage > 90.0 => Notification::new(
            StorageNotification,
            format!("Storage usage is extremely high at {}%", storage_usage),
            Critical,
        ),
        storage_usage if storage_usage > 80.0 => Notification::new(
            StorageNotification,
            format!("Storage usage is high at {}%", storage_usage),
            Warning,
        ),
        storage_usage if storage_usage <= 80.0 => Notification::new(
            StorageNotification,
            format!("Storage usage is low at {}%", storage_usage),
            Healthy,
        ),
        _ => Notification::new(
            StorageNotification,
            format!("Storage usage is unknown: {}", storage_usage),
            Healthy,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_usage_in_valid_range() {
        let metrics = Metrics::collect().expect("collection timed out");
        assert!(
            metrics.cpu_usage_percent >= 0.0 && metrics.cpu_usage_percent <= 100.0,
            "CPU usage out of range: {}",
            metrics.cpu_usage_percent
        );
    }

    #[test]
    fn test_ram_used_does_not_exceed_total() {
        let metrics = Metrics::collect().expect("collection timed out");
        assert!(
            metrics.ram_total_bytes > 0,
            "Total RAM should be greater than 0"
        );
        assert!(
            metrics.ram_used_bytes <= metrics.ram_total_bytes,
            "Used RAM {}B exceeds total {}B",
            metrics.ram_used_bytes,
            metrics.ram_total_bytes
        );
    }

    #[test]
    fn test_uptime_is_positive() {
        let metrics = Metrics::collect().expect("collection timed out");
        assert!(metrics.uptime_secs > 0, "Uptime should be greater than 0");
    }

    #[test]
    fn test_timeout_mechanism() {
        let (tx, rx) = mpsc::channel::<()>();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(1100));
            let _ = tx.send(());
        });
        assert!(
            rx.recv_timeout(Duration::from_millis(900)).is_err(),
            "should have timed out after 900ms"
        );
    }
}
