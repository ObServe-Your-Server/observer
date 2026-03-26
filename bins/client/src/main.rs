use log::{error, info};




use observer_client::config::init_config;
use observer_client::logging::init_logging;
use observer_client::scheduling_master::SchedulingMaster;

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
