use chrono::{DateTime, Utc};
use docker_api::models::ServiceCreateBodyParamRollbackConfigInlineItemFailureActionInlineItem;
use log::info;
use std::sync::{Arc, OnceLock, RwLock};
use std::time::Duration;
use tokio::time;
use tokio::time::MissedTickBehavior;

use crate::client::metric_collection_errors::CollectionError;

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

#[derive(PartialEq)]
enum ErrorLevel {
    HealthyJob,
    ErrorCount(u8),
}

pub struct Scheduler {
    kind: SchedulerKind,
    interval_secs: Arc<RwLock<u32>>,
    error_level: ErrorLevel,
    max_error_count: u8,
}

impl Scheduler {
    pub fn new(kind: SchedulerKind, interval_secs: u32, max_error_count: u8) -> Self {
        Self {
            kind,
            interval_secs: Arc::new(RwLock::new(interval_secs)),
            error_level: ErrorLevel::HealthyJob,
            max_error_count,
        }
    }

    pub async fn run<F, Fut, E>(&mut self, job: F)
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<(), E>>,
        E: std::error::Error + 'static,
    {
        let duration = Duration::from_secs(*self.interval_secs.read().unwrap() as u64);
        let mut interval = time::interval(duration);
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        info!(
            "Scheduler [{}] starting, running every {}s",
            self.kind.as_str(),
            self.interval_secs.read().unwrap()
        );

        loop {
            interval.tick().await;

            // check over the polled state if the job should be running
            if !self.kind.is_enabled() {
                info!("Scheduler [{}] paused, skipping tick", self.kind.as_str());
                continue;
            }

            let name = self.kind.as_str();
            match time::timeout(duration, job()).await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    log::error!("Scheduler [{}] job failed: {}", name, e);
                    // handle the error from the job
                    // first reference as a dynamic Any. Then try do downcast it to a collection error
                    if let Some(collection_error) =
                        (&e as &dyn std::any::Any).downcast_ref::<CollectionError>()
                    {
                        self.increment_error_count();

                        if self.error_level == ErrorLevel::ErrorCount(self.max_error_count) {
                            // exit the application because max error count has been reached
                            panic!(
                                "Scheduler [{}] max error count reached: {}. The last error was: {}",
                                name, self.max_error_count, collection_error
                            );
                        }
                    }
                }
                Err(_) => {
                    log::error!(
                        "Scheduler [{}] job exceeded interval ({}s), cancelled",
                        name,
                        self.interval_secs.read().unwrap()
                    );
                }
            }
        }
    }

    fn increment_error_count(&mut self) {
        match self.error_level {
            ErrorLevel::HealthyJob => {
                self.error_level = ErrorLevel::ErrorCount(1);
            }
            ErrorLevel::ErrorCount(count) if count < 255 => {
                self.error_level = ErrorLevel::ErrorCount(count + 1);
            }
            _ => {}
        }
    }
}
