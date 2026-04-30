use sqlx::migrate::MigrateError;
use sqlx::sqlite::SqlitePool;
use sysinfo::Cpu;

pub struct Db{
    pool: SqlitePool,
}
impl Db{
    pub async fn new(database_url: String) -> Self {
        let pool = SqlitePool::connect(&database_url).await.expect("Couldn't create database connection pool.");
        match Db::db_migration(&pool).await {
            Ok(_) => log::info!("Database migration successful."),
            Err(e) => {
                log::error!("Database migration failed: {}. Wiping DB and retrying.", e);
                pool.close().await;
                Db::delete_db(&database_url);
                let pool = SqlitePool::connect(&database_url).await.expect("Couldn't reconnect after wiping database.");
                Db::db_migration(&pool).await.expect("Database migration failed after wipe.");
                panic!("Database migration failed after wipe. Please check the logs for details.");
            },
        }
        log::debug!("Database ready. Location: {}", &database_url);
        Db{ pool }
    }

    fn delete_db(database_url: &str) {
        // strip the sqlite:// prefix and any query params to get the file path
        let path = database_url
            .trim_start_matches("sqlite://")
            .split('?')
            .next()
            .unwrap_or("");
        if let Err(e) = std::fs::remove_file(path) {
            log::error!("Failed to delete database file '{}': {}", path, e);
        }
    }

    async fn db_migration(pool: &SqlitePool) -> Result<(), MigrateError> {
        sqlx::migrate!()
            .run(pool)
            .await
    }

    pub async fn store_cpu(&self, cpu: Cpu) -> Result<(), sqlx::Error> {
        let mut conn = self.pool.acquire().await?;

        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_db_creation() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("observer.db");
        println!("Creating DB at: {}", db_path.display());
        let url = format!("sqlite://{}?mode=rwc", db_path.display());
        let _db = Db::new(url).await;
        println!("DB created at: {}", db_path.display());
    }
}