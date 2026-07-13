use anyhow::{anyhow, Context};
use crate::subsystem::speedtest::SpeedtestMetrics;
use crate::{config::get_config, subsystem::host_metrics_collector::HostMetrics};
use crate::storage_engine::storage_engine::StorageEngine;

pub struct SchedulingMaster {}

impl SchedulingMaster {
    pub async fn register_and_start_background_jobs() {
        let config = get_config();

        let storage_engine = StorageEngine::new(config.server.database_url.clone()).connect_to_db_and_migrate().await.unwrap();
        log::info!("Database connected with no errors.")

       /* let metrics = tokio::spawn(
            Scheduler::new(
                config.intervals.metric_secs as u32,
            )
            .run(HostMetrics::run),
        );*/

        /*
        let speedtest = tokio::spawn(
            Scheduler::new(
                SchedulerKind::SpeedtestCollection,
                config.intervals.speedtest_secs,
                15,
            )
            .run(SpeedtestMetrics::run),
        );

        let docker = tokio::spawn(async move {
            if config.intervals.enable_docker_socket {
                Scheduler::new(
                    SchedulerKind::DockerCollection,
                    config.intervals.docker_secs as u32,
                    15,
                )
                .run(DockerMetrics::run)
                .await;
            } else {
                info!("Docker socket collection is disabled, skipping docker metrics collection");
            }
        });*/

        //tokio::try_join!(metrics, speedtest, docker).unwrap();
    }
}
