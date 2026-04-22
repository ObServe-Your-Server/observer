use serde::{Deserialize, Serialize};

use crate::mapper::host_metrics_models::{disk_info::DiskInfo, speed_test_result::SpeedtestResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MappedHostSystemMetrics {
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
