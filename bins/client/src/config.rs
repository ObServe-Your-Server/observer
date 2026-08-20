use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::OnceLock;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
struct TomlConfig {
    server: ServerConfig,
    intervals: IntervalsConfig,
    notifications: NotificationConfig,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ServerConfig {
    pub base_server_grpc_url: String,
    pub base_server_http_url: String,
    pub push_notification_url: String,
    pub database_url: String,
    pub api_key: String,
    pub metrics_retention_time_hours: u64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct IntervalsConfig {
    pub metric_secs: u16,
    pub speedtest_secs: u32,
    pub enable_docker_socket: bool,
    pub docker_secs: u16,
    pub cpu_notification_cooldown: u32,
    pub memory_notification_cooldown: u32,
    pub disk_notification_cooldown: u32,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct NotificationConfig {
    pub cpu_high_after_secs: u16,
    pub cpu_high_notification_limit: u16,
    pub cpu_low_after_secs: u16,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct Config {
    pub version: &'static str,
    pub server: ServerConfig,
    pub intervals: IntervalsConfig,
}

impl Config {
    pub fn load_config(path: &str) -> Result<Config> {
        let raw = fs::read_to_string(path)?;

        let toml: TomlConfig = toml::from_str(&raw)?;

        let config = Config {
            version: env!("CARGO_PKG_VERSION"),
            server: toml.server,
            intervals: toml.intervals,
        };

        log::debug!("Config loaded from '{}'", path);
        log::debug!("Config: {:?}", config);

        Ok(config)
    }
}

#[cfg(test)]
mod tests {}
