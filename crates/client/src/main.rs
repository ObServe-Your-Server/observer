use log::{error, info};
use observer::client::docker::docker_job;
use observer::client::host::command_polling;
use observer::client::host::speedtest;
use observer::client::host::system_metric_collection;
use observer::config::init_config;
use observer::logging::init_logging;
use observer::scheduler::Scheduler;
use observer::scheduler::SchedulerKind;
use observer::scheduling_master::SchedulingMaster;
use observer::system_health::HostSytemHealth;
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

    SchedulingMaster::register_and_start_background_jobs().await;
}
