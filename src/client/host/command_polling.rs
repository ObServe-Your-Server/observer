use chrono::{DateTime, Utc};
use log::info;
use reqwest::Client;
use serde::Deserialize;

use crate::client::host::scheduler::get_state;
use crate::config::get_config;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
enum Command {
    StopApplication,
    StopMetricCollection,
    StartMetricCollection,
    StopSpeedtest,
    StartSpeedtest,
    StopDockerMetricCollection,
    StartDockerMetricCollection,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct CommandResponse {
    command: Command,
    message: Option<String>,
    issued_at: DateTime<Utc>,
}

pub async fn poll() {
    let config = get_config();
    let client = Client::new();

    let result = client
        .get(&config.server.base_commands_url)
        .header("X-Api-Key", &config.server.api_key)
        .send()
        .await;

    let resp = match result {
        Ok(r) if r.status().is_success() => r,
        Ok(r) => {
            log::warn!("Command poll: unexpected status {}", r.status());
            return;
        }
        Err(e) => {
            log::error!("Command poll error: {}", e);
            return;
        }
    };

    let commands: Vec<CommandResponse> = match resp.json().await {
        Ok(c) => c,
        Err(e) => {
            log::error!("Failed to deserialize commands: {}", e);
            return;
        }
    };

    let state = get_state();
    for item in commands {
        if item.issued_at < state.started_at {
            info!(
                "Ignoring stale command {:?} issued at {} (before startup at {})",
                item.command, item.issued_at, state.started_at
            );
            continue;
        }

        info!("Received command: {:?}", item.command);
        match item.command {
            Command::StopApplication => {
                log::error!(
                    "StopApplication received — {}. Shutting down.",
                    item.message.as_deref().unwrap_or("no message")
                );
                std::process::exit(1);
            }
            Command::StopMetricCollection => *state.metrics_enabled.write().unwrap() = false,
            Command::StartMetricCollection => *state.metrics_enabled.write().unwrap() = true,
            Command::StopSpeedtest => *state.speedtest_enabled.write().unwrap() = false,
            Command::StartSpeedtest => *state.speedtest_enabled.write().unwrap() = true,
            Command::StopDockerMetricCollection => {
                *state.docker_metrics_enabled.write().unwrap() = false
            }
            Command::StartDockerMetricCollection => {
                *state.docker_metrics_enabled.write().unwrap() = true
            }
        }
    }
}
