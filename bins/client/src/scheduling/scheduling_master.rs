use std::sync::Arc;
use anyhow::{anyhow, Context};
use chrono::Duration;
use crate::subsystem::speedtest::SpeedtestMetrics;
use crate::{config::get_config, subsystem::base_metrics_system::BaseMetrics};
use crate::jobs::data_cleanup_job::DataCleanupJob;
use crate::storage_engine::storage_engine::StorageEngine;

pub struct SchedulingMaster {}

impl SchedulingMaster {
    pub async fn register_and_start_background_jobs() {
        let config = get_config();

        // we can clone it around because the db connection is thread save and with the pool meant to be cloned
        let storage_engine = Arc::new(StorageEngine::new(config.server.database_url.clone()).connect_to_db_and_migrate().await.unwrap());
        log::info!("Database connected with no errors.");

        let metrics_retention_time_hours = config.server.metrics_retention_time_hours.clone().parse::<u64>().expect("Unable to parse metrics retention time from provided string to u64");
        let data_cleanup_job = DataCleanupJob::new(Arc::clone(&storage_engine), metrics_retention_time_hours, 4, Duration::minutes(5));

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
