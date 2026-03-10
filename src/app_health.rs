use std::{sync::Arc, task::RawWakerVTable};

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

#[derive(Clone, PartialEq, Debug)]
pub enum States {
    Healthy,
    Unhealthy,
    Down,
    Unknown,
}

#[derive(Clone, PartialEq, Debug)]
pub struct HostSystemMetricCollector {
    pub metric_sender: States,
    pub disk_collector: States,
}

impl HostSystemMetricCollector {
    pub fn new() -> Self {
        Self {
            metric_sender: States::Unknown,
            disk_collector: States::Unknown,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Docker {
    pub socket: States,
    pub docker_metric_sender: States,
    pub metric_parser: States,
}

impl Docker {
    pub fn new() -> Self {
        Self {
            socket: States::Unknown,
            docker_metric_sender: States::Unknown,
            metric_parser: States::Unknown,
        }
    }
}
