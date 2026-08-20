use std::process::exit;
use crate::grpc::v1::metrics_tunnel::MetricsTunnel;
use crate::jobs::base_metric_collection_job::BaseMetricCollectionJob;
use crate::jobs::container_stats_collection_job::ContainerStatsCollectionJob;
use crate::jobs::data_cleanup_job::DataCleanupJob;
use crate::storage_engine::storage_engine::StorageEngine;
use anyhow::anyhow;
use chrono::Duration;
use reqwest::{Client, StatusCode};
use std::sync::Arc;
use crate::config::app_config::AppConfig;
use crate::config::toml_config::TomlConfig;
use crate::scheduling::job::Job;
use crate::scheduling::scheduler::Scheduler;

pub struct SchedulingMaster {}


impl SchedulingMaster {
    pub async fn register_and_start_background_jobs(config: AppConfig) {

        // we can clone it around because the db connection is thread save and with the pool meant to be cloned
        let storage_engine = Arc::new(
            StorageEngine::new(config.toml_config().storage_config().database_url())
                .connect_to_db_and_migrate()
                .await
                .unwrap(),
        );
        log::info!("Database connected with no errors.");

        let machine_name = Self::pull_machine_name(&config.toml_config()).await.unwrap_or_else(|e| {
            log::error!("Failed to fetch machine name: {}", e);
            exit(1);
        });

        //let notification_handler = NotificationHandler::new(config.server.push_notification_url.to_string().clone(), config.server.api_key.clone(), machine_name.clone());

        // TODO data cleanup job

        let base_metric_collection_job = Job::new(
            "Base Metrics",
            Box::new(BaseMetricCollectionJob::new(storage_engine.clone())),
            Duration::seconds(config.toml_config().interval_config().base_metric_secs().clone() as i64),
            15 // TODO implement over the config
        );
        let data_cleanup_job = Job::new(
            "Data Cleanup",
            Box::new(DataCleanupJob::new(storage_engine.clone(), config.toml_config().storage_config().clone())),
            Duration::seconds(config.toml_config().interval_config().data_cleanup_job_secs().clone().unwrap_or(300) as i64), // TODO set default time more elegant
            15
        );

        /*let speedtest_stats_collection_job_schedule_time = Duration::seconds(config.intervals.speedtest_secs as i64);
        let speedtest_stats_collection_job = SpeedtestStatsCollectionJob::new(Arc::clone(&storage_engine), speedtest_stats_collection_job_schedule_time);
        let speedtest_stats_collection_job = SchedulableJob::new(Box::new(speedtest_stats_collection_job), 5);*/

        let metrics_tunnel = MetricsTunnel::new(
            config.toml_config().client_config().base_server_grpc_url().to_string(),
            config.toml_config().client_config().api_key().to_string(),
            storage_engine.clone(),
        );

        // -------------- first add essential jobs --------------
        let mut scheduler = Scheduler::new(vec![data_cleanup_job, base_metric_collection_job]);

        // -------------- addons like container stats --------------
        if config.toml_config().interval_config().enable_docker_socket().clone() {
            let container_stats_collection_job = Job::new(
                "Container Stats",
                Box::new(ContainerStatsCollectionJob::new(storage_engine.clone())),
                Duration::seconds(config.toml_config().interval_config().data_cleanup_job_secs().clone().unwrap_or(15) as i64),
                15
            );

            scheduler.add_job(container_stats_collection_job);
        }

        // scheduler in own task
        let scheduler_future_handle =
            tokio::spawn(async move { scheduler.start_jobs_blocking().await });

        // metrics grpc tunnel
        let metrics_tunnel_future_handle =
            tokio::spawn(async move { metrics_tunnel.run_blocking().await });

        // TODO send message that observer client started

        // whichever terminates first (cleanly, via signal, or not) brings the whole process down
        tokio::select! {
            res = scheduler_future_handle => {
                log::error!("Scheduler termination: {}", res.err().unwrap())
            }
            res = metrics_tunnel_future_handle => {
                log::error!("Metrics tunnel terminated: {}", res.err().unwrap());
            }
            _ = Self::watch_for_termination() => {
                log::info!("Termination signal received");
            }
        }

        // TODO send shutdown notification
        exit(1);
    }

    async fn pull_machine_name(toml_config: &TomlConfig) -> anyhow::Result<String> {
        let client = Client::new();

        let res = client
            .get(format!(
                "{}/machines/machine-name-over-api-key",
                toml_config.client_config().base_server_http_url()
            ))
            .header("X-Api-Key", toml_config.client_config().api_key())
            .send()
            .await?;

        match res.status() {
            StatusCode::OK => Ok(res.text().await?),
            status => Err(anyhow!("Failed to pull machine name: {}", status)),
        }
    }

    /// Resolves once SIGTERM or SIGINT is received. Can be awaited on its own
    /// or raced against other futures (e.g. inside a `tokio::select!`).
    pub async fn watch_for_termination() {
        use tokio::signal::unix::{SignalKind, signal};

        let mut sigterm =
            signal(SignalKind::terminate()).expect("failed to register SIGTERM handler");
        let mut sigint =
            signal(SignalKind::interrupt()).expect("failed to register SIGINT handler");

        tokio::select! {
            _ = sigterm.recv() => log::info!("received SIGTERM"),
            _ = sigint.recv() => log::info!("received SIGINT"),
        }
    }
}
