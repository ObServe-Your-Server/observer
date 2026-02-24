use log::info;
use sysinfo::{Disks, System};
use super::speedtest;

#[derive(Debug)]
pub struct Metrics {
    pub cpu_usage_percent: f32,
    pub ram_used_mb: u64,
    pub ram_total_mb: u64,
    pub storage_used_gb: u64,
    pub storage_total_gb: u64,
    pub uptime_secs: u64,
}

impl Metrics {
    pub fn collect() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();

        // CPU - average usage across all cores
        let cpu_count = sys.cpus().len();
        let cpu_usage = sys.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>() / cpu_count as f32;

        // RAM
        let ram_used_mb = sys.used_memory() / 1024 / 1024;
        let ram_total_mb = sys.total_memory() / 1024 / 1024;

        // Storage - sum across all disks
        let disks = Disks::new_with_refreshed_list();
        let storage_total_gb = disks.iter().map(|d| d.total_space()).sum::<u64>() / 1024 / 1024 / 1024;
        let storage_used_gb = disks.iter().map(|d| d.total_space() - d.available_space()).sum::<u64>() / 1024 / 1024 / 1024;

        // Uptime
        let uptime_secs = System::uptime();

        Self {
            cpu_usage_percent: cpu_usage,
            ram_used_mb,
            ram_total_mb,
            storage_used_gb,
            storage_total_gb,
            uptime_secs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_usage_in_valid_range() {
        let metrics = Metrics::collect();
        assert!(
            metrics.cpu_usage_percent >= 0.0 && metrics.cpu_usage_percent <= 100.0,
            "CPU usage out of range: {}", metrics.cpu_usage_percent
        );
    }

    #[test]
    fn test_ram_used_does_not_exceed_total() {
        let metrics = Metrics::collect();
        assert!(metrics.ram_total_mb > 0, "Total RAM should be greater than 0");
        assert!(
            metrics.ram_used_mb <= metrics.ram_total_mb,
            "Used RAM {}MB exceeds total {}MB", metrics.ram_used_mb, metrics.ram_total_mb
        );
    }

    #[test]
    fn test_storage_used_does_not_exceed_total() {
        let metrics = Metrics::collect();
        assert!(metrics.storage_total_gb > 0, "Total storage should be greater than 0");
        assert!(
            metrics.storage_used_gb <= metrics.storage_total_gb,
            "Used storage {}GB exceeds total {}GB", metrics.storage_used_gb, metrics.storage_total_gb
        );
    }

    #[test]
    fn test_uptime_is_positive() {
        let metrics = Metrics::collect();
        assert!(metrics.uptime_secs > 0, "Uptime should be greater than 0");
    }
}

pub async fn collect() {
    let metrics = Metrics::collect();

    info!("CPU: {:.1}%", metrics.cpu_usage_percent);
    info!("RAM: {}MB / {}MB", metrics.ram_used_mb, metrics.ram_total_mb);
    info!("Storage: {}GB / {}GB", metrics.storage_used_gb, metrics.storage_total_gb);
    info!("Uptime: {}s", metrics.uptime_secs);

    match speedtest::get_last_result() {
        Some(s) => info!("Speedtest: down={:.2}Mbps up={:.2}Mbps ping={:.1}ms", s.download_mbps, s.upload_mbps, s.ping_ms),
        None => info!("Speedtest: no measurement yet"),
    }
}
