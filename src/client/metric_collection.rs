use super::{sender, speedtest};
use crate::client::speedtest::SpeedtestResult;
use log::{debug, info, warn};
use reqwest::Client;
use std::sync::mpsc;
use std::time::Duration;
use sysinfo::{Components, Disks, System};

#[derive(Debug)]
pub struct DiskInfo {
    pub name: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
}

#[derive(Debug)]
pub struct Metrics {
    pub cpu_usage_percent: f32,
    pub cpu_count: usize,
    pub cpu_name: String,
    pub ram_used_bytes: u64,
    pub ram_total_bytes: u64,
    pub uptime_secs: u64,
    pub cpu_temp_celsius: Option<f32>,
    pub disks: Vec<DiskInfo>,
    pub speedtest_result: Option<SpeedtestResult>,
}

/// Maps a partition device name to its parent disk.
/// e.g. "nvme0n1p2" → "nvme0n1", "sda1" → "sda", "mmcblk0p1" → "mmcblk0"
fn parent_disk_name(dev: &str) -> String {
    if dev.starts_with("nvme") || dev.starts_with("mmcblk") {
        // partition suffix is "p<digits>": find last 'p' followed only by digits
        if let Some(pos) = dev.rfind('p') {
            let suffix = &dev[pos + 1..];
            if !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()) {
                return dev[..pos].to_string();
            }
        }
    } else {
        // traditional disks: sda1 → sda
        let stripped = dev.trim_end_matches(|c: char| c.is_ascii_digit());
        if stripped.len() < dev.len() {
            return stripped.to_string();
        }
    }
    dev.to_string()
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
        match rx.recv_timeout(Duration::from_millis(1800)) {
            Ok(metrics) => Some(metrics),
            Err(_) => {
                // thread is still running but we stop waiting for it —
                // it will eventually finish and try to send into a dead channel, which is fine
                warn!("metrics collection exceeded 1800ms, aborting");
                None
            }
        }
    }

    fn do_collect() -> Self {
        let mut sys = System::new_all();
        sys.refresh_cpu_usage();
        //plus small increase because sometimes there is 0.0 for cpu
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL + Duration::from_millis(300));
        sys.refresh_cpu_usage();

        //info!("{:?}", sys);

        // CPU - average usage across all cores
        let cpu_count = sys.cpus().len();
        let cpu_usage = sys.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>() / cpu_count as f32;

        sys.refresh_memory();

        // RAM
        let ram_used_bytes = sys.used_memory();
        let ram_total_bytes = sys.total_memory();

        // Storage: group mounted partitions by physical disk, use the largest per disk
        let sysinfo_disks = Disks::new_with_refreshed_list();
        let mut best: std::collections::HashMap<String, (u64, u64)> =
            std::collections::HashMap::new();
        for d in sysinfo_disks.iter() {
            let full = d.name().to_string_lossy();
            let dev = full.rsplit('/').next().unwrap_or(full.as_ref());
            let parent = parent_disk_name(dev);
            let used = d.total_space() - d.available_space();
            best.entry(parent)
                .and_modify(|(tot, use_)| {
                    if d.total_space() > *tot {
                        *tot = d.total_space();
                        *use_ = used;
                    }
                })
                .or_insert((d.total_space(), used));
        }
        let mut disk_infos: Vec<DiskInfo> = best
            .into_iter()
            .filter(|(_, (total, _))| *total > 0)
            .map(|(name, (total_bytes, used_bytes))| DiskInfo {
                name,
                total_bytes,
                used_bytes,
            })
            .collect();
        disk_infos.sort_by(|a, b| a.name.cmp(&b.name));

        // Uptime
        let uptime_secs = System::uptime();

        // CPU temperature — AMD: "k10temp Tctl", Intel: "coretemp Package id 0"
        let components = Components::new_with_refreshed_list();
        let cpu_temp_celsius = components
            .iter()
            .find(|c| {
                let label = c.label();
                label.contains("Tctl")
                    || (label.starts_with("coretemp") && label.contains("Package id 0"))
            })
            .and_then(|c| c.temperature());

        Self {
            cpu_usage_percent: cpu_usage,
            cpu_count,
            cpu_name: sys.cpus().first().map(|c| c.brand()).unwrap_or_default().to_string(),
            ram_used_bytes,
            ram_total_bytes,
            uptime_secs,
            cpu_temp_celsius,
            disks: disk_infos,
            speedtest_result: None,
        }
    }
}

pub async fn collect(client: &Client) {
    let Some(mut metrics) = Metrics::collect() else {
        // TODO handle unsuccessful collection - report timeout metric or trigger an alert
        return;
    };
    
    debug!("Whole struct {:?}", metrics);

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
    fn test_storage_used_does_not_exceed_total() {
        let metrics = Metrics::collect().expect("collection timed out");
        for disk in &metrics.disks {
            assert!(
                disk.total_bytes > 0,
                "Disk [{}] total should be greater than 0",
                disk.name
            );
            assert!(
                disk.used_bytes <= disk.total_bytes,
                "Disk [{}] used {}B exceeds total {}B",
                disk.name,
                disk.used_bytes,
                disk.total_bytes
            );
        }
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
