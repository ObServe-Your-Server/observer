use log::{error, info};
use reqwest::Client;
use serde::Serialize;

use crate::config::get_config;
use super::metric_collection::Metrics;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskPayload {
    pub name: String,
    pub total: i64,
    pub used: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricPayload {
    pub cpu_usage: Option<f64>,
    pub cpu_temperature: Option<f64>,
    pub mem_used: Option<i64>,
    pub mem_total: Option<i64>,
    pub disks: Vec<DiskPayload>,
    pub net_bytes_in: Option<i64>,
    pub net_bytes_out: Option<i64>,
    pub uptime: Option<i64>,
}

impl MetricPayload {
    pub fn from_metrics(metrics: &Metrics) -> Self {
        let avg_temp = if metrics.core_temperatures.is_empty() {
            None
        } else {
            let sum: f32 = metrics.core_temperatures.iter().map(|t| t.temp_celsius).sum();
            Some((sum / metrics.core_temperatures.len() as f32) as f64)
        };

        // Deduplicate by name — keep only the first occurrence of each disk name
        let mut seen_names = std::collections::HashSet::new();
        let disks = metrics
            .disks
            .iter()
            .filter(|d| seen_names.insert(d.name.clone()))
            .map(|d| DiskPayload {
                name: d.name.clone(),
                total: d.total_bytes as i64,
                used: d.used_bytes as i64,
            })
            .collect();

        Self {
            cpu_usage: Some(metrics.cpu_usage_percent as f64),
            cpu_temperature: avg_temp,
            mem_used: Some(metrics.ram_used_bytes as i64),
            mem_total: Some(metrics.ram_total_bytes as i64),
            disks,
            net_bytes_in: None,
            net_bytes_out: None,
            uptime: Some(metrics.uptime_secs as i64),
        }
    }
}

pub async fn send(client: &Client, metrics: &Metrics) {
    let config = get_config();
    let payload = MetricPayload::from_metrics(metrics);

    let result = client
        .post(&config.server.base_metrics_url)
        .header("X-Api-Key", &config.server.api_key)
        .json(&payload)
        .send()
        .await;

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
