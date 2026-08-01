//! A throwaway database for tests.

use crate::storage_engine::storage_engine::StorageEngine;
use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;
use tempfile::TempDir;

/// A migrated storage engine backed by a SQLite file in a temp directory.
///
/// The directory is removed when this drops, so nothing leaks between tests and
/// each one starts from an empty schema. Seeding lives in
/// [`fixtures`](super::fixtures), which adds the `seed_*` methods.
pub struct TestDb {
    engine: Arc<StorageEngine>,
    // held only so the directory outlives the engine; dropping it deletes the db
    _dir: TempDir,
}

impl TestDb {
    /// Creates an empty database in a fresh temp directory and runs migrations.
    pub async fn new() -> Self {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let path = dir.path().join("observer-test.sqlite");
        // `mode=rwc` so sqlite creates the file instead of erroring on a missing one
        let url = format!("sqlite://{}?mode=rwc", path.display());

        let engine = StorageEngine::new(url)
            .connect_to_db_and_migrate()
            .await
            .expect("failed to migrate test database");

        Self {
            engine: Arc::new(engine),
            _dir: dir,
        }
    }

    /// The engine under test. Cheap to clone — the builders take it by `Arc`.
    pub fn engine(&self) -> Arc<StorageEngine> {
        self.engine.clone()
    }
}

/// A timestamp `minutes` in the past, for building a spread-out history.
pub fn minutes_ago(minutes: i64) -> DateTime<Utc> {
    Utc::now() - Duration::minutes(minutes)
}
