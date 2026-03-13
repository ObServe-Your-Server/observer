use crate::client::docker::docker_job;
use crate::client::host::{command_polling, speedtest};
use crate::{
    client::host::system_metric_collection,
    config::get_config,
    scheduler::{Scheduler, SchedulerKind},
    system_health::HostSytemHealth,
};

pub struct SchedulingMaster {}

impl SchedulingMaster {
    pub async fn register_and_start_background_jobs() {
        let config = get_config();

        let mut metric_scheduler = Scheduler::new(
            SchedulerKind::MetricCollection,
            config.intervals.metric_secs as u32,
            15,
        );
        let mut command_scheduler = Scheduler::new(
            SchedulerKind::CommandPolling,
            config.intervals.command_poll_secs as u32,
            15,
        );
        let mut speedtest_scheduler = Scheduler::new(
            SchedulerKind::Speedtest,
            config.intervals.speedtest_secs,
            15,
        );
        let mut docker_scheduler = Scheduler::new(
            SchedulerKind::DockerMetricCollection,
            config.intervals.docker_secs as u32,
            15,
        );

        let host_system_health = HostSytemHealth::new();
        let health_for_metrics = host_system_health.clone();
        let health_for_docker = host_system_health.clone();

        tokio::join!(
            metric_scheduler.run(move || {
                let h = health_for_metrics.clone();
                system_metric_collection::collection_job(h)
            }),
            command_scheduler.run(|| command_polling::poll()),
            speedtest_scheduler.run(|| speedtest::run()),
            docker_scheduler.run(move || {
                let h = health_for_docker.clone();
                docker_job::collect(h)
            }),
        );
    }
}
