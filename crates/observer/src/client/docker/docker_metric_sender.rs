use log::{error, info};
use reqwest::Client;

use crate::config::get_config;
use super::collector::ContainerStats;

pub async fn send(client: &Client, containers: &[ContainerStats]) {
    let config = get_config();
    let url = format!("{}/docker", config.server.base_metrics_url);

    let result = client
        .post(&url)
        .header("X-Api-Key", &config.server.api_key)
        .json(containers)
        .send()
        .await;

    match result {
        Ok(resp) if resp.status().is_success() => {
            info!("Docker metrics sent ({}) to {}", resp.status(), url);
        }
        Ok(resp) => {
            error!("Server rejected docker metrics: {}", resp.status());
        }
        Err(e) => {
            error!("Failed to send docker metrics: {}", e);
        }
    }
}
