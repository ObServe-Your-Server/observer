use std::iter::Once;
use std::sync::OnceLock;
use std::time::Duration;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use anyhow::{anyhow, Result};
use sea_orm::sqlx::encode::IsNull::No;
use migration::{Migrator, MigratorTrait};

pub struct StorageEngine{
    database_path: String,
    db: OnceLock<DatabaseConnection>
}

impl StorageEngine {
    pub fn new(database_path: String) -> StorageEngine {
        StorageEngine{
            database_path,
            db: OnceLock::new()
        }
    }

    pub async fn connect_to_db_and_migrate(self) -> Result<Self> {
        let mut opt = ConnectOptions::new(self.database_path.clone());
        opt.max_connections(100)
            .min_connections(5)
            .connect_timeout(Duration::from_secs(8))
            .acquire_timeout(Duration::from_secs(8))
            .idle_timeout(Duration::from_secs(8))
            .max_lifetime(Duration::from_secs(8))
            .sqlx_logging(false) // disable SQLx logging
            .sqlx_logging_level(log::LevelFilter::Info);
        //.set_schema_search_path("my_schema"); // set default Postgres schema

        let db_conn = Database::connect(opt).await?;

        Migrator::up(&db_conn, None).await?;
        self.db.set(db_conn).map_err(|_| anyhow!("database connection already set"))?;
        log::debug!("Connected and migrated db at: {}", &self.database_path);
        Ok(self)
    }
}