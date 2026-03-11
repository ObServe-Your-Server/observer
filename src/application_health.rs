use std::sync::Arc;

use tokio::sync::RwLock;

#[derive(Clone, Debug)]
pub struct AppHealth {
    host_system: Arc<RwLock<HostSystemMetricCollector>>,
    docker: Arc<RwLock<Docker>>,
}

impl AppHealth {
    pub fn new() -> Self {
        Self {
            docker: Arc::new(RwLock::new(Docker::new())),
            host_system: Arc::new(RwLock::new(HostSystemMetricCollector::new())),
        }
    }
    
    pub async fn update_host_system_health<F>(&self, f: F)
    where
        F: FnOnce(&mut HostSystemMetricCollector),
    {
        let mut host_system = self.host_system.write().await;
        f(&mut host_system);
    }
    
    pub async fn update_docker_health<F>(&self, f: F)
    where
        F: FnOnce(&mut Docker),
    {
        let previous_state = self.docker.read().await.clone();
        let mut docker = self.docker.write().await;
        f(&mut docker);
        
        if previous_state != self.docker.read().await.clone() {
            println!("Docker state changed: {:?} -> {:?}", previous_state, self.docker.read().await.clone());
        }
    }
    
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum State {
    Healthy,
    Unhealthy,
    Down,
    Unknown,
}

impl State {
    pub fn to_str(&self) -> &'static str {
        match self {
            State::Healthy => "healthy",
            State::Unhealthy => "unhealthy",
            State::Down => "down",
            State::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct HostSystemMetricCollector {
    pub metric_sender: State,
    pub disk_collector: State,
}

impl HostSystemMetricCollector {
    pub fn new() -> Self {
        Self {
            metric_sender: State::Unknown,
            disk_collector: State::Unknown,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Docker {
    pub socket: State,
    pub docker_metric_sender: State,
    pub metric_parser: State,
}

impl Docker {
    pub fn new() -> Self {
        Self {
            socket: State::Unknown,
            docker_metric_sender: State::Unknown,
            metric_parser: State::Unknown,
        }
    }
}
