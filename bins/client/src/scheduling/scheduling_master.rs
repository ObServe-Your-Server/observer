use crate::scheduling::scheduler::{Scheduler, SchedulerKind};
use crate::{config::get_config, subsystem::host_metrics_collector::HostMetrics};

pub struct SchedulingMaster {}

impl SchedulingMaster {
    pub async fn register_and_start_background_jobs() {
        let config = get_config();

        let mut metric_scheduler = Scheduler::new(
            SchedulerKind::MetricCollection,
            config.intervals.metric_secs as u32,
            15,
        );

        metric_scheduler.run(|| HostMetrics::run()).await;
    }
}
