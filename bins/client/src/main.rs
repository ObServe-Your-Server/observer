use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

use std::env;
use std::path::PathBuf;
use observer_client::config::app_config::AppConfig;
use observer_client::logging::init_logging;
use observer_client::scheduling::scheduling_master::SchedulingMaster;

#[tokio::main]
async fn main() {
    init_logging();

    let config_path = env::var("OBSERVER_CONFIG").unwrap_or_else(|_| "observer.toml".to_string());
    let config = AppConfig::load_from_path(PathBuf::from(config_path))
        .expect("Failed to load config. Check if file exists and observer can read it.");
    log::info!("Observer v{} starting", config.version());

    SchedulingMaster::register_and_start_background_jobs(config).await;
}
