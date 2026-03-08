mod client;
mod config;

use client::scheduler::{Scheduler, SchedulerKind};
use client::{command_polling, metric_collection, speedtest};
use config::init_config;
use log::{error, info};
use std::env;

fn init_logging() {
    let level_str = env::var("OBSERVER_LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
    let level = level_str
        .parse::<log::LevelFilter>()
        .unwrap_or(log::LevelFilter::Info);

    env_logger::Builder::new()
        .filter(Some("observer"), level)
        .init();

    log::debug!("Logging initialized at level: {}", level);
}

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

    let metric_scheduler = Scheduler::new(SchedulerKind::MetricCollection, config.intervals.metric_secs as u32);
    let command_scheduler = Scheduler::new(SchedulerKind::CommandPolling, config.intervals.command_poll_secs as u32);
    let speedtest_scheduler = Scheduler::new(SchedulerKind::Speedtest, config.intervals.speedtest_secs);

    tokio::join!(
        metric_scheduler.run(|| metric_collection::collect()),
        command_scheduler.run(|| command_polling::poll()),
        speedtest_scheduler.run(|| speedtest::run()),
    );
}
