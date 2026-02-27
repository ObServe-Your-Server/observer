use super::{sender, speedtest};
use log::{debug, info, warn};
use reqwest::Client;
use std::sync::mpsc;
use std::time::Duration;
use sysinfo::{Components, Disks, System};

#[derive(Debug)]
pub struct CoreTemperature {
    pub label: String,
    pub temp_celsius: f32,
}

#[derive(Debug)]
pub struct DiskInfo {
    pub name: String,
    pub total_gb: u64,
    pub used_gb: u64,
}

#[derive(Debug)]
pub struct Metrics {
    pub cpu_usage_percent: f32,
    pub ram_used_mb: u64,
    pub ram_total_mb: u64,
    pub storage_used_gb: u64,
    pub storage_total_gb: u64,
    pub uptime_secs: u64,
    pub core_temperatures: Vec<CoreTemperature>,
    pub disks: Vec<DiskInfo>,
}

impl Metrics {
    pub fn collect() -> Option<Self> {
        // channel to send the result back from the worker thread to here
        let (tx, rx) = mpsc::channel();

        // do_collect blocks on sysinfo syscalls, so it runs in its own thread —
        // this is the only way to actually enforce a timeout on blocking code
        std::thread::spawn(move || {
            let result = Self::do_collect();
            let _ = tx.send(result); // silently fails if we already timed out and rx was dropped
        });

        // block here until we get a result or 900ms passes — whichever comes first
        match rx.recv_timeout(Duration::from_millis(900)) {
            Ok(metrics) => Some(metrics),
            Err(_) => {
                // thread is still running but we stop waiting for it —
                // it will eventually finish and try to send into a dead channel, which is fine
                warn!("metrics collection exceeded 900ms, aborting");
                None
            }
        }
    }

    fn do_collect() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();

        // CPU - average usage across all cores
        let cpu_count = sys.cpus().len();
        let cpu_usage = sys.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>() / cpu_count as f32;

        // RAM
        let ram_used_mb = sys.used_memory() / 1024 / 1024;
        let ram_total_mb = sys.total_memory() / 1024 / 1024;

        // Storage - per-disk info and aggregates
        let disks = Disks::new_with_refreshed_list();
        let storage_total_gb =
            disks.iter().map(|d| d.total_space()).sum::<u64>() / 1024 / 1024 / 1024;
        let storage_used_gb = disks
            .iter()
            .map(|d| d.total_space() - d.available_space())
            .sum::<u64>()
            / 1024
            / 1024
            / 1024;
        let disk_infos: Vec<DiskInfo> = disks
            .iter()
            .map(|d| DiskInfo {
                name: d.name().to_string_lossy().to_string(),
                total_gb: d.total_space() / 1024 / 1024 / 1024,
                used_gb: (d.total_space() - d.available_space()) / 1024 / 1024 / 1024,
            })
            .collect();

        // Uptime
        let uptime_secs = System::uptime();

        // Core temperatures
        let components = Components::new_with_refreshed_list();
        let core_temperatures = components
            .iter()
            .filter_map(|c| {
                c.temperature().map(|t| CoreTemperature {
                    label: c.label().to_string(),
                    temp_celsius: t,
                })
            })
            .collect();

        Self {
            cpu_usage_percent: cpu_usage,
            ram_used_mb,
            ram_total_mb,
            storage_used_gb,
            storage_total_gb,
            uptime_secs,
            core_temperatures,
            disks: disk_infos,
        }
    }
}

pub async fn collect(client: &Client) {
    let Some(metrics) = Metrics::collect() else {
        // TODO handle unsuccessful collection - report timeout metric or trigger an alert
        return;
    };

    debug!("CPU: {:.1}%", metrics.cpu_usage_percent);
    debug!(
        "RAM: {}MB / {}MB",
        metrics.ram_used_mb, metrics.ram_total_mb
    );
    debug!(
        "Storage: {}GB / {}GB",
        metrics.storage_used_gb, metrics.storage_total_gb
    );
    debug!("Uptime: {}s", metrics.uptime_secs);
    for ct in &metrics.core_temperatures {
        debug!("Temp [{}]: {:.1}°C", ct.label, ct.temp_celsius);
    }
    for disk in &metrics.disks {
        debug!(
            "Disk [{}]: {}GB / {}GB",
            disk.name, disk.used_gb, disk.total_gb
        );
    }

    match speedtest::get_last_result() {
        Some(s) => debug!(
            "Speedtest: down={:.2}Mbps up={:.2}Mbps ping={:.1}ms",
            s.download_mbps, s.upload_mbps, s.ping_ms
        ),
        None => debug!("Speedtest: no measurement yet"),
    }

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
        assert!(
            metrics.ram_total_mb > 0,
            "Total RAM should be greater than 0"
        );
        assert!(
            metrics.ram_used_mb <= metrics.ram_total_mb,
            "Used RAM {}MB exceeds total {}MB",
            metrics.ram_used_mb,
            metrics.ram_total_mb
        );
    }

    #[test]
    fn test_storage_used_does_not_exceed_total() {
        let metrics = Metrics::collect().expect("collection timed out");
        assert!(
            metrics.storage_total_gb > 0,
            "Total storage should be greater than 0"
        );
        assert!(
            metrics.storage_used_gb <= metrics.storage_total_gb,
            "Used storage {}GB exceeds total {}GB",
            metrics.storage_used_gb,
            metrics.storage_total_gb
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

        // simulate a collection that takes longer than 900ms
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(1100));
            let _ = tx.send(());
        });

        // should time out and return Err since the thread takes 1100ms
        assert!(
            rx.recv_timeout(Duration::from_millis(900)).is_err(),
            "should have timed out after 900ms"
        );
    }
}
