use chrono::{DateTime, Utc};
use log::{error, info, warn};
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time;
use tokio::time::MissedTickBehavior;

use crate::scheduling::collection_error::CollectionError;

pub struct SubsystemState {
    pub metrics_enabled: RwLock<bool>,
    pub started_at: DateTime<Utc>,
}

static STATE: OnceLock<SubsystemState> = OnceLock::new();

pub fn get_state() -> &'static SubsystemState {
    STATE.get_or_init(|| SubsystemState {
        metrics_enabled: RwLock::new(true),
        started_at: Utc::now(),
    })
}

pub enum SchedulerKind {
    MetricCollection,
    SpeedtestCollection,
    DockerCollection,
}

impl SchedulerKind {
    fn as_str(&self) -> &'static str {
        match self {
            SchedulerKind::MetricCollection => "metric-collection",
            SchedulerKind::SpeedtestCollection => "speedtest-collection",
            SchedulerKind::DockerCollection => "docker-collection",
        }
    }

    async fn is_enabled(&self) -> bool {
        let state = get_state();
        match self {
            SchedulerKind::MetricCollection => *state.metrics_enabled.read().await,
            SchedulerKind::SpeedtestCollection => *state.metrics_enabled.read().await,
            SchedulerKind::DockerCollection => *state.metrics_enabled.read().await,
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
        let duration = Duration::from_secs(*self.interval_secs.read().await as u64);
        let mut interval = time::interval(duration);
        // skip if the execution took too long or other issues
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        info!(
            "Scheduler [{}] starting, running every {}s",
            self.kind.as_str(),
            *self.interval_secs.read().await
        );

        loop {
            interval.tick().await;

            // check over the polled state if the job should be running
            if !self.kind.is_enabled().await {
                info!("Scheduler [{}] paused, skipping tick", self.kind.as_str());
                continue;
            }

            let name = self.kind.as_str();
            // run the job and wait on the result
            match time::timeout(duration, job()).await {
                Ok(Ok(())) => match self.error_level {
                    ErrorLevel::ErrorCount(_) => {
                        self.reset_error_count();
                        info!("Scheduler [{}] job succeeded after error", name);
                    }
                    ErrorLevel::HealthyJob => {}
                },
                Ok(Err(e)) => {
                    error!("Scheduler [{}] job failed: {}", name, e);
                    // handle the error from the job
                    // first reference as a dynamic Any. Then try do downcast it to a collection error
                    if let Some(collection_error) =
                        (&e as &dyn std::any::Any).downcast_ref::<CollectionError>()
                    {
                        // increase the error count
                        self.increment_error_count();

                        if self.error_level == ErrorLevel::ErrorCount(self.max_error_count) {
                            // if the container socket is unavailable then just
                            // stop this job and not exit in a whole
                            if matches!(
                                collection_error,
                                CollectionError::ContainerSocketUnavailable(_)
                            ) {
                                warn!(
                                    "Scheduler [{}] container socket unavailable, stopping job",
                                    name
                                );
                                return;
                            }

                            // exit the application if it is another error
                            panic!(
                                "Scheduler [{}] max error count reached: {}. The last error was: {}",
                                name, self.max_error_count, collection_error
                            );
                        }
                    } else {
                        error!("Another error occurred during a collection run: {}", e);
                    }
                }
                Err(_) => {
                    // executes when the job takes too long
                    error!(
                        "Scheduler [{}] job exceeded interval ({}s), cancelled.
                        You may need to increase the metrics collection interval.",
                        name,
                        *self.interval_secs.read().await
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

    fn reset_error_count(&mut self) {
        self.error_level = ErrorLevel::HealthyJob;
    }
}
