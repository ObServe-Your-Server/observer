use std::sync::Arc;
use anyhow::{anyhow, Context};
use chrono::Duration;
use reqwest::{Client, Response, StatusCode};
use crate::config::{get_config, Config};
use crate::grpc::v1::metrics_tunnel::MetricsTunnel;
use crate::jobs::base_metric_collection_job::{BaseMetricCollectionJob, NotificationCooldowns};
use crate::jobs::container_stats_collection_job::ContainerStatsCollectionJob;
use crate::jobs::data_cleanup_job::DataCleanupJob;
use crate::jobs::speedtest_stats_collection_job::SpeedtestStatsCollectionJob;
use crate::notification::notification_handler::{NotificationHandler, PushNotification};
use crate::scheduling::scheduler::{SchedulableJob, Scheduler};
use crate::storage_engine::storage_engine::StorageEngine;

pub struct SchedulingMaster {}

impl SchedulingMaster {
    pub async fn register_and_start_background_jobs() {
        let config = get_config();

        // we can clone it around because the db connection is thread save and with the pool meant to be cloned
        let storage_engine = Arc::new(StorageEngine::new(config.server.database_url.clone()).connect_to_db_and_migrate().await.unwrap());
        log::info!("Database connected with no errors.");

        let machine_name = Self::pull_machine_name(config).await.unwrap_or_else(|e|{
            log::error!("Failed to fetch machine name: {}", e);
            "Unknown".to_string()
        });

        let notification_handler = NotificationHandler::new(config.server.push_notification_url.to_string().clone(), config.server.api_key.clone(), machine_name.clone());

        let metrics_retention_time_hours = config.server.metrics_retention_time_hours.clone();
        let data_cleanup_job = DataCleanupJob::new(Arc::clone(&storage_engine), metrics_retention_time_hours, Duration::minutes(5));
        let data_cleanup_job = SchedulableJob::new(Box::new(data_cleanup_job), 5);

        let notification_cooldowns = NotificationCooldowns::from_config(config);

        let base_metric_collection_job_schedule_time = Duration::seconds(config.intervals.metric_secs as i64);
        let base_metric_collection_job = BaseMetricCollectionJob::new(Arc::clone(&storage_engine), base_metric_collection_job_schedule_time, notification_handler.clone(), notification_cooldowns);
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
        let sch_notification_handler = notification_handler.clone();
        let scheduler_handle = tokio::spawn(async move {
            let err = scheduler.start_jobs_blocking().await;
            log::error!("Metrics collection failed with: {:?}", err);
            // send message that observer client started
            match sch_notification_handler.send_push_notification(&PushNotification {
                title: "Shutdown".to_string(),
                body: "Collector failed to collect host metrics.".to_string(),
            }).await {
                Ok(_) => {
                    log::info!("Shutdown message sent successfully")
                }
                Err(_) => {
                    log::error!("Failed to send shutdown message")
                }
            }
        });

        // metrics grpc tunnel
        let metrics_tunnel_handle = tokio::spawn(async move {
            if let Err(e) = metrics_tunnel.run_blocking().await {
                log::error!("Metrics tunnel exited with error: {e}");
            }
        });

        // send message that observer client started
        match notification_handler.send_push_notification(&PushNotification {
            title: "Startup".to_string(),
            body: "Metrics collector started".to_string(),
        }).await {
            Ok(_) => {
                log::info!("Startup message sent successfully")
            }
            Err(_) => {
                log::error!("Failed to send startup message")
            }
        }

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

    async fn pull_machine_name(config: &Config) -> anyhow::Result<String> {
        let client = Client::new();

        let res = client.post(format!("{}/machines/machine-name-over-api-key", config.server.base_server_url))
            .header("X-Api-Key", &config.server.api_key)
            .send()
            .await?;

        match res.status() {
            StatusCode::OK => Ok(res.text().await?),
            status => Err(anyhow!("Failed to pull machine name: {}", status)),
        }
    }
}
