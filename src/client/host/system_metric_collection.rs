use log::{debug, warn};
use reqwest::Client;
use std::sync::{RwLock, mpsc};
use std::time::Duration;
use sysinfo::System;

use crate::client::host::collectors::disk::DiskInfo;
use crate::client::host::collectors::{cpu, disk, network};
use crate::client::host::speedtest::{self, SpeedtestResult};
use crate::client::host::system_metric_sender;
use crate::system_health::{HostComponent, HostSytemHealth, Severity, State};

// --- network delta config ---
#[derive(Clone)]
struct NetworkBytesDelta {
    pub time_of_recording: std::time::Instant,
    pub bytes_received: u64,
    pub bytes_transmitted: u64,
}

static NETWORK_DELTA: RwLock<Option<NetworkBytesDelta>> = RwLock::new(None);

// --- disk cache config ---
const DISK_CACHE_ENABLED: bool = true;
const DISK_CACHE_DURATION: Duration = Duration::from_secs(120);

struct DiskCache {
    last_collected: std::time::Instant,
    disks: Vec<DiskInfo>,
}

static DISK_CACHE: RwLock<Option<DiskCache>> = RwLock::new(None);

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
            // sends the result from within the thread
            let _ = tx.send(result);
        });

        // receive the result from the thread
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

        let disks = {
            let cache = DISK_CACHE.read().unwrap();
            if DISK_CACHE_ENABLED && let Some(c) = cache.as_ref() {
                if c.last_collected.elapsed() < DISK_CACHE_DURATION {
                    debug!(
                        "disk: using cached result ({}s old)",
                        c.last_collected.elapsed().as_secs()
                    );
                    c.disks.clone()
                } else {
                    drop(cache);
                    let fresh = disk::collect();
                    *DISK_CACHE.write().unwrap() = Some(DiskCache {
                        last_collected: std::time::Instant::now(),
                        disks: fresh.clone(),
                    });
                    fresh
                }
            } else {
                drop(cache);
                let fresh = disk::collect();
                *DISK_CACHE.write().unwrap() = Some(DiskCache {
                    last_collected: std::time::Instant::now(),
                    disks: fresh.clone(),
                });
                fresh
            }
        };
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

pub async fn collection_job(host_sytem_health: HostSytemHealth) {
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

    // manage the notifications
    let metrics_clone = metrics.clone();
    tokio::spawn(async move {
        check_system_metrics_and_change_host_health(host_sytem_health, &metrics_clone).await;
    });

    system_metric_sender::send(&client, &metrics).await;
}

async fn check_system_metrics_and_change_host_health(
    host_sytem_health: HostSytemHealth,
    metrics: &Metrics,
) {
    // --- cpu usage ---
    let cpu_usage = metrics.cpu_usage_percent;
    if cpu_usage > 90.0 {
        host_sytem_health
            .set_cpu_state(State::new(
                Severity::Critical,
                HostComponent::Cpu,
                "cpu usage critical".to_string(),
            ))
            .await;
    } else if cpu_usage > 80.0 {
        host_sytem_health
            .set_cpu_state(State::new(
                Severity::Warning,
                HostComponent::Cpu,
                "cpu usage high".to_string(),
            ))
            .await;
    } else {
        host_sytem_health
            .set_cpu_state(State::new(
                Severity::Healthy,
                HostComponent::Cpu,
                "cpu usage normal".to_string(),
            ))
            .await;
    }

    // --- ram usage ---
    let ram_usage = metrics.ram_used_bytes as f32 / metrics.ram_total_bytes as f32 * 100.0;
    if ram_usage > 90.0 {
        host_sytem_health
            .set_memory_state(State::new(
                Severity::Critical,
                HostComponent::Memory,
                "ram usage critical".to_string(),
            ))
            .await;
    } else if ram_usage > 80.0 {
        host_sytem_health
            .set_memory_state(State::new(
                Severity::Warning,
                HostComponent::Memory,
                "ram usage high".to_string(),
            ))
            .await;
    } else {
        host_sytem_health
            .set_memory_state(State::new(
                Severity::Healthy,
                HostComponent::Memory,
                "ram usage normal".to_string(),
            ))
            .await;
    }

    // --- disk usage ---
    let avg_disk_usage = metrics
        .disks
        .iter()
        .map(|disk| disk.used_bytes as f32 / disk.total_bytes as f32 * 100.0)
        .sum::<f32>()
        / metrics.disks.len() as f32;
    if avg_disk_usage > 90.0 {
        host_sytem_health
            .set_disk_state(State::new(
                Severity::Critical,
                HostComponent::Disk,
                "disk usage critical".to_string(),
            ))
            .await;
    } else if avg_disk_usage > 80.0 {
        host_sytem_health
            .set_disk_state(State::new(
                Severity::Warning,
                HostComponent::Disk,
                "disk usage high".to_string(),
            ))
            .await;
    } else {
        host_sytem_health
            .set_disk_state(State::new(
                Severity::Healthy,
                HostComponent::Disk,
                "disk usage normal".to_string(),
            ))
            .await;
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
