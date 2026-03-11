use log::{error, info};
use observer::client::docker::docker_job;
use observer::client::host::command_polling;
use observer::client::host::speedtest;
use observer::client::host::system_metric_collection;
use observer::config::init_config;
use observer::system_health::HostSytemHealth;
use observer::logging::init_logging;
use observer::scheduler::Scheduler;
use observer::scheduler::SchedulerKind;
use std::env;

#[tokio::main]
async fn main() {
    init_logging();

    let config_path = env::var("OBSERVER_CONFIG").unwrap_or_else(|_| "observer.toml".to_string());

    let config = match init_config(&config_path) {
        Ok(c) => c,
        Err(e) => {
            error!("Config error: {}", e);
            std::process::exit(1);
        }
    };

    info!("Observer v{} started", config.version);
    info!(
        "Server: {} / {}",
        config.server.base_metrics_url, config.server.base_commands_url
    );
    info!("Application ready");

    let metric_scheduler = Scheduler::new(
        SchedulerKind::MetricCollection,
        config.intervals.metric_secs as u32,
    );
    let command_scheduler = Scheduler::new(
        SchedulerKind::CommandPolling,
        config.intervals.command_poll_secs as u32,
    );
    let speedtest_scheduler =
        Scheduler::new(SchedulerKind::Speedtest, config.intervals.speedtest_secs);
    let docker_scheduler = Scheduler::new(
        SchedulerKind::DockerMetricCollection,
        config.intervals.docker_secs as u32,
    );

    let host_system_health = HostSytemHealth::new();

    tokio::join!(
        metric_scheduler
            .run(|| system_metric_collection::collection_job(host_system_health.clone())),
        command_scheduler.run(|| command_polling::poll()),
        speedtest_scheduler.run(|| speedtest::run()),
        docker_scheduler.run(|| docker_job::collect()),
    );
}
