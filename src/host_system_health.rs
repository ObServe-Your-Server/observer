use std::{fs::OpenOptions, sync::Arc};

use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct HostSystemState {
    cpu: Arc<RwLock<State>>,
    memory: Arc<RwLock<State>>,
    disk: Arc<RwLock<State>>,
    network_stats: Arc<RwLock<State>>,
    docker: Arc<RwLock<State>>,
}

impl HostSystemState {
    pub fn new() -> Self {
        Self {
            cpu: Arc::new(RwLock::new(State::new(Severity::Healthy, HostComponent::Cpu, String::new()))),
            memory: Arc::new(RwLock::new(State::new(Severity::Healthy, HostComponent::Memory, String::new()))),
            disk: Arc::new(RwLock::new(State::new(Severity::Healthy, HostComponent::Disk, String::new()))),
            network_stats: Arc::new(RwLock::new(State::new(Severity::Healthy, HostComponent::Network, String::new()))),
            docker: Arc::new(RwLock::new(State::new(Severity::Healthy, HostComponent::Docker, String::new()))),
        }
    }
    
    pub async fn set_cpu_state(&self, new_state: State) {
        let old_cpu_state = self.cpu.read().await;
        if old_cpu_state.severity != new_state.severity {
            *self.cpu.write().await = new_state;
            // TODO: send notification
        }
    }
    
    pub async fn set_memory_state(&self, new_state: State) {
        let old_memory_state = self.memory.read().await;
        if old_memory_state.severity != new_state.severity {
            *self.memory.write().await = new_state;
            // TODO: send notification
        }
    }
    
    pub async fn set_disk_state(&self, new_state: State) {
        let old_disk_state = self.disk.read().await;
        if old_disk_state.severity != new_state.severity {
            *self.disk.write().await = new_state;
            // TODO: send notification
        }
    }
    
    pub async fn set_network_stats_state(&self, new_state: State) {
        let old_network_stats_state = self.network_stats.read().await;
        if old_network_stats_state.severity != new_state.severity {
            *self.network_stats.write().await = new_state;
            // TODO: send notification
        }
    }
    
    pub async fn set_docker_state(&self, new_state: State) {
        let old_docker_state = self.docker.read().await;
        if old_docker_state.severity != new_state.severity {
            *self.docker.write().await = new_state;
            // TODO: send notification
        }
    }
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, Copy, PartialEq)]
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