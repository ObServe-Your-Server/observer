use log::{error, info};

use observer_client::config::init_config;
use observer_client::logging::init_logging;
use observer_client::scheduling::scheduling_master::SchedulingMaster;

use std::env;
use std::time::Duration;
use sea_orm::{ConnectOptions, Database};
use migration::{Migrator, MigratorTrait};

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
    info!("Application ready");

    // connect to the database and run pending migrations
    let db = connect_db(&config.server.database_url).await;
    if let Err(e) = Migrator::up(&db, None).await {
        error!("Database migration error: {}", e);
        std::process::exit(1);
    }

    //SchedulingMaster::register_and_start_background_jobs().await;
}

async fn connect_db(db_place: &str) -> sea_orm::DatabaseConnection {
    let mut opt = ConnectOptions::new(db_place);
    opt.max_connections(100)
        .min_connections(5)
        .connect_timeout(Duration::from_secs(8))
        .acquire_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(8))
        .max_lifetime(Duration::from_secs(8))
        .sqlx_logging(false) // disable SQLx logging
        .sqlx_logging_level(log::LevelFilter::Info);
        //.set_schema_search_path("my_schema"); // set default Postgres schema

    Database::connect(opt).await.unwrap()
}
