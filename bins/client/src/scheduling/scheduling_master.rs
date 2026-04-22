use crate::scheduling::scheduler::{Scheduler, SchedulerKind};
use crate::subsystem::docker_metrics_collector::DockerMetrics;
use crate::subsystem::speedtest::SpeedtestMetrics;
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

        let mut speetest_scheduler = Scheduler::new(
            SchedulerKind::SpeedtestCollection,
            config.intervals.speedtest_secs,
            15,
        );

        let mut docker_scheduler = Scheduler::new(
            SchedulerKind::DockerCollection,
            config.intervals.docker_secs as u32,
            15,
        );

        tokio::join!(
            metric_scheduler.run(|| HostMetrics::run()),
            speetest_scheduler.run(|| SpeedtestMetrics::run()),
            docker_scheduler.run(|| DockerMetrics::run())
        );
    }
}
