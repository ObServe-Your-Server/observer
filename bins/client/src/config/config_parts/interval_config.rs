use serde::{Deserialize, Serialize};
use getset::Getters;

#[derive(Debug, Deserialize, Serialize, Getters)]
#[serde(rename_all = "snake_case")]
pub struct IntervalConfig {
    #[getset(get = "pub")]
    base_metric_secs: u16,
    #[getset(get = "pub")]
    speedtest_secs: u32,
    #[getset(get = "pub")]
    enable_docker_socket: bool,
    #[getset(get = "pub")]
    container_metrics_secs: Option<u16>,
    #[getset(get = "pub")]
    data_cleanup_job_secs: Option<u16>,
}