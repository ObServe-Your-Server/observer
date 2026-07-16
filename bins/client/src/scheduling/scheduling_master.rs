use std::sync::Arc;
use anyhow::{anyhow, Context};
use chrono::Duration;
use crate::config::get_config;
use crate::grpc::v1::metrics_tunnel::MetricsTunnel;
use crate::jobs::base_metric_collection_job::BaseMetricCollectionJob;
use crate::jobs::container_stats_collection_job::ContainerStatsCollectionJob;
use crate::jobs::data_cleanup_job::DataCleanupJob;
use crate::jobs::speedtest_stats_collection_job::SpeedtestStatsCollectionJob;
use crate::scheduling::scheduler::{SchedulableJob, Scheduler};
use crate::storage_engine::storage_engine::StorageEngine;

pub struct SchedulingMaster {}

impl SchedulingMaster {
    pub async fn register_and_start_background_jobs() {
        let config = get_config();

        // we can clone it around because the db connection is thread save and with the pool meant to be cloned
        let storage_engine = Arc::new(StorageEngine::new(config.server.database_url.clone()).connect_to_db_and_migrate().await.unwrap());
        log::info!("Database connected with no errors.");

        let metrics_retention_time_hours = config.server.metrics_retention_time_hours.clone();
        let data_cleanup_job = DataCleanupJob::new(Arc::clone(&storage_engine), metrics_retention_time_hours, Duration::minutes(5));
        let data_cleanup_job = SchedulableJob::new(Box::new(data_cleanup_job), 5);

        let base_metric_collection_job_schedule_time = Duration::seconds(config.intervals.metric_secs as i64);
        let base_metric_collection_job = BaseMetricCollectionJob::new(Arc::clone(&storage_engine), base_metric_collection_job_schedule_time);
        let base_metric_collection_job = SchedulableJob::new(Box::new(base_metric_collection_job), 10);
        
        let speedtest_stats_collection_job_schedule_time = Duration::seconds(config.intervals.speedtest_secs as i64);
        let speedtest_stats_collection_job = SpeedtestStatsCollectionJob::new(Arc::clone(&storage_engine), speedtest_stats_collection_job_schedule_time);
        let speedtest_stats_collection_job = SchedulableJob::new(Box::new(speedtest_stats_collection_job), 5);
            
        let metrics_tunnel = MetricsTunnel::new(
            config.server.base_server_url.as_str(),
            config.server.api_key.clone(),
            Arc::clone(&storage_engine),
        );

        // -------------- first add essential jobs --------------
        let mut scheduler = Scheduler::new(vec![data_cleanup_job, base_metric_collection_job, speedtest_stats_collection_job]);

        // -------------- addons like container stats --------------
        if config.intervals.enable_docker_socket {
            let container_stats_collection_job_schedule_time = Duration::seconds(config.intervals.docker_secs as i64);
            let container_stats_collection_job = ContainerStatsCollectionJob::new(Arc::clone(&storage_engine), container_stats_collection_job_schedule_time);
            let container_stats_collection_job = SchedulableJob::new(Box::new(container_stats_collection_job), 10);

            scheduler.add_job(container_stats_collection_job);
        }

        // start the jobs in separate tasks
        let scheduler_handle = tokio::spawn(async move {
            scheduler.start_jobs_blocking().await.expect("Error during scheduling");
        });

        // metrics grpc tunnel
        let metrics_tunnel_handle = tokio::spawn(async move {
            if let Err(e) = metrics_tunnel.run_blocking().await {
                log::error!("metrics tunnel exited with error: {e}");
            }
        });

        // whichever of the two terminates first (cleanly or not) brings the whole process down
        tokio::select! {
            _ = scheduler_handle => {
                log::error!("scheduler terminated, shutting down");
            }
            _ = metrics_tunnel_handle => {
                log::error!("metrics tunnel terminated, shutting down");
            }
        }
        std::process::exit(1);
    }
}
