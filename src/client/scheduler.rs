use std::time::Duration;
use log::info;
use tokio::time;

pub enum SchedulerKind {
    MetricCollection,
    CommandPolling,
    Speedtest,
}

impl SchedulerKind {
    fn as_str(&self) -> &'static str {
        match self {
            SchedulerKind::MetricCollection => "metric-collection",
            SchedulerKind::CommandPolling => "command-polling",
            SchedulerKind::Speedtest => "speedtest",
        }
    }
}

pub struct Scheduler {
    kind: SchedulerKind,
    interval_secs: u32,
}

impl Scheduler {
    pub fn new(kind: SchedulerKind, interval_secs: u32) -> Self {
        Self { kind, interval_secs }
    }

    pub async fn run<F, Fut>(&self, job: F)
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        let duration = Duration::from_secs(self.interval_secs as u64);
        let mut interval = time::interval(duration);

        info!("Scheduler [{}] started, running every {}s", self.kind.as_str(), self.interval_secs);

        loop {
            interval.tick().await;
            job().await;
        }
    }
}
