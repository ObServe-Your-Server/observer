use crate::scheduling::scheduler::{Scheduler, SchedulerKind};
use crate::subsystem::docker_metrics_collector::DockerMetrics;
use crate::subsystem::speedtest::SpeedtestMetrics;
use crate::{config::get_config, subsystem::host_metrics_collector::HostMetrics};
use log::info;

pub struct SchedulingMaster {}

impl SchedulingMaster {
    pub async fn register_and_start_background_jobs() {
        let config = get_config();

        let metrics = tokio::spawn(
            Scheduler::new(
                SchedulerKind::MetricCollection,
                config.intervals.metric_secs as u32,
                15,
            )
            .run(|| HostMetrics::run()),
        );

        let speedtest = tokio::spawn(
            Scheduler::new(
                SchedulerKind::SpeedtestCollection,
                config.intervals.speedtest_secs,
                15,
            )
            .run(|| SpeedtestMetrics::run()),
        );

        let docker = tokio::spawn(async move {
            if config.intervals.enable_docker_socket {
                Scheduler::new(
                    SchedulerKind::DockerCollection,
                    config.intervals.docker_secs as u32,
                    15,
                )
                .run(|| DockerMetrics::run())
                .await;
            } else {
                info!("Docker socket collection is disabled, skipping docker metrics collection");
            }
        });

        tokio::try_join!(metrics, speedtest, docker).unwrap();
    }
}
