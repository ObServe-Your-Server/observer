use std::sync::Arc;

use docker_api::models::ContainerSummaryNetworkSettingsInlineItem;
use log::{debug, error, info};
use serde::Serialize;
use tokio::sync::RwLock;

use crate::{config::{Config, get_config}, logging::LogTarget};

// this struct handles all the logic for changing values for the individual component states
#[derive(Debug, Clone)]
pub struct HostSytemHealth {
    cpu: Arc<RwLock<State>>,
    memory: Arc<RwLock<State>>,
    disk: Arc<RwLock<State>>,
    network_stats: Arc<RwLock<State>>,
    docker: Arc<RwLock<State>>,
}

impl Serialize for HostSytemHealth {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("HostSytemHealth", 5)?;
        s.serialize_field("cpu", &*self.cpu.try_read().map_err(serde::ser::Error::custom)?)?;
        s.serialize_field("memory", &*self.memory.try_read().map_err(serde::ser::Error::custom)?)?;
        s.serialize_field("disk", &*self.disk.try_read().map_err(serde::ser::Error::custom)?)?;
        s.serialize_field("network_stats", &*self.network_stats.try_read().map_err(serde::ser::Error::custom)?)?;
        s.serialize_field("docker", &*self.docker.try_read().map_err(serde::ser::Error::custom)?)?;
        s.end()
    }
}

impl HostSytemHealth {
    pub fn new() -> Self {
        Self {
            cpu: Arc::new(RwLock::new(State::new(
                Severity::Healthy,
                HostComponent::Cpu,
                String::new(),
            ))),
            memory: Arc::new(RwLock::new(State::new(
                Severity::Healthy,
                HostComponent::Memory,
                String::new(),
            ))),
            disk: Arc::new(RwLock::new(State::new(
                Severity::Healthy,
                HostComponent::Disk,
                String::new(),
            ))),
            network_stats: Arc::new(RwLock::new(State::new(
                Severity::Healthy,
                HostComponent::Network,
                String::new(),
            ))),
            docker: Arc::new(RwLock::new(State::new(
                Severity::Healthy,
                HostComponent::Docker,
                String::new(),
            ))),
        }
    }

    pub async fn set_cpu_state(&self, new_state: State) {
        let old_cpu_state = self.cpu.read().await.clone();
        if old_cpu_state != new_state {
            debug!("cpu state changed: {:?}", new_state);
            *self.cpu.write().await = new_state.clone();
            // send notification
            self.handle_notification(new_state).await;
        }
    }

    pub async fn set_memory_state(&self, new_state: State) {
        let old_memory_state = self.memory.read().await.clone();
        if old_memory_state != new_state {
            debug!("memory state changed: {:?}", new_state);
            *self.memory.write().await = new_state.clone();
            // send notification
            self.handle_notification(new_state).await;
        }
    }

    pub async fn set_disk_state(&self, new_state: State) {
        let old_disk_state = self.disk.read().await.clone();
        if old_disk_state != new_state {
            debug!("disk state changed: {:?}", new_state);
            *self.disk.write().await = new_state.clone();
            // send notification
            self.handle_notification(new_state).await;
        }
    }

    pub async fn set_network_stats_state(&self, new_state: State) {
        let old_network_stats_state = self.network_stats.read().await.clone();
        if old_network_stats_state != new_state {
            debug!("network stats state changed: {:?}", new_state);
            *self.network_stats.write().await = new_state.clone();
            // send notification
            self.handle_notification(new_state).await;
        }
    }

    pub async fn set_docker_state(&self, new_state: State) {
        let old_docker_state = self.docker.read().await.clone();
        if old_docker_state != new_state {
            debug!("docker state changed: {:?}", new_state);
            *self.docker.write().await = new_state.clone();
            // send notification
            self.handle_notification(new_state).await;
        }
    }
    
    async fn handle_notification(&self, state: State) {
        debug!("State change to send: {:?}", state);
        let config: &Config = get_config();

        let url = &config.server.base_notifier_url;
        let client = reqwest::Client::new();
        debug!("Sending json: {:?}", serde_json::to_string(&state).unwrap());
        let result = client
            .post(url)
            .header("X-Api-Key", &config.server.api_key)
            .json(&state)
            .send()
            .await;

        match result {
            Ok(resp) if resp.status().is_success() => {
                info!("State change sent: {}", resp.status());
            }
            Ok(resp) => {
                error!("Server rejected notification: {}", resp.status());
            }
            Err(e) => {
                error!("Failed to send notification: {}", e);
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct State {
    pub severity: Severity,
    component: HostComponent,
    pub message: String,
}

impl State {
    pub fn new(severity: Severity, component: HostComponent, message: String) -> Self {
        Self {
            severity,
            component,
            message,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum HostComponent {
    Cpu,
    Memory,
    Disk,
    Network,
    Speedtest,
    Docker,
}

impl HostComponent {
    pub fn to_str(&self) -> &'static str {
        match self {
            HostComponent::Cpu => "cpu",
            HostComponent::Memory => "memory",
            HostComponent::Disk => "disk",
            HostComponent::Network => "network",
            HostComponent::Speedtest => "speedtest",
            HostComponent::Docker => "docker",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum Severity {
    Healthy,
    Info,
    Warning,
    Critical,
}

impl Severity {
    pub fn to_str(&self) -> &'static str {
        match self {
            Severity::Healthy => "healthy",
            Severity::Info => "info",
            Severity::Warning => "warning",
            Severity::Critical => "critical",
        }
    }
}
