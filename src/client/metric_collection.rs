use super::{sender, speedtest};
use crate::client::collectors::{cpu, disk, memory, network};
use crate::client::collectors::disk::DiskInfo;
use crate::client::speedtest::SpeedtestResult;
use log::{debug, warn};
use reqwest::Client;
use serde::Serialize;
use std::sync::mpsc;
use std::time::Duration;
use sysinfo::System;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RamModule {
    pub size_mb: u64,
    pub memory_type: String,
    pub speed_mhz: u32,
    pub manufacturer: Option<String>,
    pub part_number: Option<String>,
    pub slot: Option<String>,
    pub voltage: Option<f32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RamInfo {
    pub total_mb: u64,
    pub used_mb: u64,
    pub available_mb: u64,
    pub usage_percent: f32,
    pub speed_mhz: u32,
    pub channels: u32,
    pub ecc_support: bool,
    pub bandwidth_gb_s: Option<f32>,
    pub swap_total_mb: u64,
    pub swap_used_mb: u64,
    pub modules: Vec<RamModule>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Metrics {
    pub cpu_usage_percent: f32,
    pub cpu_count: usize,
    pub cpu_name: String,
    pub ram: RamInfo,
    pub uptime_secs: u64,
    pub cpu_temp_celsius: Option<f32>,
    pub os_name: Option<String>,
    pub kernel_version: Option<String>,
    pub net_bytes_received: u64,
    pub net_bytes_transmitted: u64,
    pub local_ip: Option<String>,
    pub disks: Vec<DiskInfo>,
    pub speedtest_result: Option<SpeedtestResult>,
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
        let disks = disk::collect();
        let net = network::collect();

        let ram = memory::ram_information()
            .map(|m| RamInfo {
                total_mb: m.total_mb,
                used_mb: m.used_mb,
                available_mb: m.available_mb,
                usage_percent: m.usage_percent,
                speed_mhz: m.speed_mhz,
                channels: m.channels,
                ecc_support: m.ecc_support,
                bandwidth_gb_s: m.bandwidth_gb_s,
                swap_total_mb: m.swap_total_mb,
                swap_used_mb: m.swap_used_mb,
                modules: m.modules.into_iter().map(|mod_| RamModule {
                    size_mb: mod_.size_mb,
                    memory_type: format!("{:?}", mod_.memory_type),
                    speed_mhz: mod_.speed_mhz,
                    manufacturer: mod_.manufacturer,
                    part_number: mod_.part_number,
                    slot: mod_.slot,
                    voltage: mod_.voltage,
                }).collect(),
            })
            .unwrap_or_else(|| {
                let used = sys.used_memory() / 1024 / 1024;
                let total = sys.total_memory() / 1024 / 1024;
                RamInfo {
                    total_mb: total,
                    used_mb: used,
                    available_mb: total.saturating_sub(used),
                    usage_percent: if total > 0 { used as f32 / total as f32 * 100.0 } else { 0.0 },
                    speed_mhz: 0,
                    channels: 0,
                    ecc_support: false,
                    bandwidth_gb_s: None,
                    swap_total_mb: 0,
                    swap_used_mb: 0,
                    modules: vec![],
                }
            });

        Self {
            cpu_usage_percent: cpu_info.usage_percent,
            cpu_count: cpu_info.count,
            cpu_name: cpu_info.name,
            ram,
            uptime_secs: System::uptime(),
            cpu_temp_celsius: cpu_info.temp_celsius,
            os_name: System::long_os_version(),
            kernel_version: System::kernel_version(),
            net_bytes_received: net.bytes_received,
            net_bytes_transmitted: net.bytes_transmitted,
            local_ip: net.local_ip,
            disks,
            speedtest_result: None,
        }
    }
}

pub async fn collect(client: &Client) {
    let Some(mut metrics) = Metrics::collect() else {
        return;
    };

    debug!("Whole struct {:?}", metrics);
    debug!("CPU: {:.1}%", metrics.cpu_usage_percent);
    debug!(
        "RAM: {}MB / {}MB ({:.1}%)",
        metrics.ram.used_mb,
        metrics.ram.total_mb,
        metrics.ram.usage_percent
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

    debug!("Whole metric struct: {:?}", metrics);

    sender::send(client, &metrics).await;
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
        assert!(metrics.ram.total_mb > 0, "Total RAM should be greater than 0");
        assert!(
            metrics.ram.used_mb <= metrics.ram.total_mb,
            "Used RAM {}MB exceeds total {}MB",
            metrics.ram.used_mb,
            metrics.ram.total_mb
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
