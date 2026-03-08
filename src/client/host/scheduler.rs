use chrono::{DateTime, Utc};
use log::info;
use std::sync::{OnceLock, RwLock};
use std::time::Duration;
use tokio::time;

pub struct AppState {
    pub metrics_enabled: RwLock<bool>,
    pub speedtest_enabled: RwLock<bool>,
    pub docker_metrics_enabled: RwLock<bool>,
    pub started_at: DateTime<Utc>,
}

static STATE: OnceLock<AppState> = OnceLock::new();

pub fn get_state() -> &'static AppState {
    STATE.get_or_init(|| AppState {
        metrics_enabled: RwLock::new(true),
        speedtest_enabled: RwLock::new(true),
        docker_metrics_enabled: RwLock::new(true),
        started_at: Utc::now(),
    })
}

pub enum SchedulerKind {
    MetricCollection,
    CommandPolling,
    Speedtest,
    DockerMetricCollection,
}

impl SchedulerKind {
    fn as_str(&self) -> &'static str {
        match self {
            SchedulerKind::MetricCollection => "metric-collection",
            SchedulerKind::CommandPolling => "command-polling",
            SchedulerKind::Speedtest => "speedtest",
            SchedulerKind::DockerMetricCollection => "docker-metric-collection",
        }
    }

    fn is_enabled(&self) -> bool {
        let state = get_state();
        match self {
            SchedulerKind::MetricCollection => *state.metrics_enabled.read().unwrap(),
            SchedulerKind::CommandPolling => true,
            SchedulerKind::Speedtest => *state.speedtest_enabled.read().unwrap(),
            SchedulerKind::DockerMetricCollection => *state.docker_metrics_enabled.read().unwrap(),
        }
    }
}

pub struct Scheduler {
    kind: SchedulerKind,
    interval_secs: u32,
}

impl Scheduler {
    pub fn new(kind: SchedulerKind, interval_secs: u32) -> Self {
        Self {
            kind,
            interval_secs,
        }
    }

    pub async fn run<F, Fut>(&self, job: F)
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        let duration = Duration::from_secs(self.interval_secs as u64);
        let mut interval = time::interval(duration);

        info!(
            "Scheduler [{}] started, running every {}s",
            self.kind.as_str(),
            self.interval_secs
        );

        loop {
            interval.tick().await;

            if !self.kind.is_enabled() {
                info!("Scheduler [{}] paused, skipping tick", self.kind.as_str());
                continue;
            }

            job().await;
        }
    }
}
