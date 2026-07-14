use std::sync::Arc;
use anyhow::{anyhow, Context};
use chrono::Duration;
use crate::subsystem::speedtest::SpeedtestMetrics;
use crate::config::get_config;
use crate::jobs::base_metric_collection_job::BaseMetricCollectionJob;
use crate::jobs::data_cleanup_job::DataCleanupJob;
use crate::scheduling::scheduler::{SchedulableJob, Scheduler};
use crate::storage_engine::storage_engine::StorageEngine;

pub struct SchedulingMaster {}

impl SchedulingMaster {
    pub async fn register_and_start_background_jobs() {
        let config = get_config();

        //TODO: implemement job registry where the error count etc. is managed not in the job

        // we can clone it around because the db connection is thread save and with the pool meant to be cloned
        let storage_engine = Arc::new(StorageEngine::new(config.server.database_url.clone()).connect_to_db_and_migrate().await.unwrap());
        log::info!("Database connected with no errors.");

        let metrics_retention_time_hours = config.server.metrics_retention_time_hours.clone();
        let data_cleanup_job = DataCleanupJob::new(Arc::clone(&storage_engine), metrics_retention_time_hours, Duration::minutes(5));

        let base_metric_collection_job_schedule_time = Duration::seconds(config.intervals.metric_secs as i64);
        let base_metric_collection_job = BaseMetricCollectionJob::new(Arc::clone(&storage_engine), base_metric_collection_job_schedule_time);
        let base_metric_collection_job = SchedulableJob::new(Box::new(base_metric_collection_job), 10);

        let mut scheduler = Scheduler::new(vec![base_metric_collection_job]);
        scheduler.start_jobs_blocking().await.expect("Error during scheduling");
    }
}
