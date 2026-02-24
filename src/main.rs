mod config;
mod client;

use log::{error, info};
use env_logger;
use std::env;
use config::{init_config, Mode};
use client::scheduler::{Scheduler, SchedulerKind};
use client::{metric_collection, command_polling};

fn init_logging() {
    let level_str = env::var("OBSERVER_LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
    let level = level_str.parse::<log::LevelFilter>().unwrap_or(log::LevelFilter::Info);

    env_logger::Builder::new()
        .filter(Some("observer"), level) // change to `None` to include dependency logs
        .init();

    log::debug!("Logging initialized at level: {}", level);
}

#[tokio::main]
async fn main() {
    init_logging();

    let config = match init_config() {
        Ok(c) => c,
        Err(e) => {
            error!("Config error: {}", e);
            std::process::exit(1);
        }
    };

    info!("Observer v{} started", config.version);
    match config.mode {
        Mode::Client => {
            let c = config.client_config.as_ref().unwrap();
            info!("Running in client mode with config: {:?}", c);
            info!("Application ready");

            let metric_scheduler = Scheduler::new(SchedulerKind::MetricCollection, c.inactive_streaming_interval_secs);
            let command_scheduler = Scheduler::new(SchedulerKind::CommandPolling, c.command_poll_interval_secs);

            tokio::join!(
                metric_scheduler.run(|| metric_collection::collect()),
                command_scheduler.run(|| command_polling::poll()),
            );
        }
        Mode::AllInOne => {
            let c = config.all_in_one_config.as_ref().unwrap();
            info!("Running in all-in-one mode with config: {:?}", c);
            info!("Application ready");
        }
    }
}
