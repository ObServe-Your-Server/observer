use crate::config::config_parts::storage_config::StorageConfig;
use crate::jobs::job::JobTrait;
use anyhow::{Ok, Result};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;

#[async_trait]
pub trait DataCleanupStorageEngine: Send + Sync {
    async fn remove_all_older_than(&self, cutoff_time: DateTime<Utc>) -> Result<()>;
    // the thin out functions delete every entry older than thin_out_before, except every
    // entry whose id is a multiple of keep_every_x. Rows keep their id forever, so running
    // it again never deletes entries which were kept before
    async fn thin_out_cpu_stats(
        &self,
        thin_out_before: DateTime<Utc>,
        keep_every_x: u16,
    ) -> Result<u64>;
    async fn thin_out_memory_stats(
        &self,
        thin_out_before: DateTime<Utc>,
        keep_every_x: u16,
    ) -> Result<u64>;
    async fn thin_out_container_runtime_stats(
        &self,
        thin_out_before: DateTime<Utc>,
        keep_every_x: u16,
    ) -> Result<u64>;
    async fn thin_out_partition_entries(
        &self,
        thin_out_before: DateTime<Utc>,
        keep_every_x: u16,
    ) -> Result<u64>;
    async fn thin_out_network_stats(
        &self,
        thin_out_before: DateTime<Utc>,
        keep_every_x: u16,
    ) -> Result<u64>;
    async fn thin_out_system_stats(
        &self,
        thin_out_before: DateTime<Utc>,
        keep_every_x: u16,
    ) -> Result<u64>;
    async fn thin_out_processes_stats(
        &self,
        thin_out_before: DateTime<Utc>,
        keep_every_x: u16,
    ) -> Result<u64>;
    async fn thin_out_speedtest_stats(
        &self,
        thin_out_before: DateTime<Utc>,
        keep_every_x: u16,
    ) -> Result<u64>;
}

pub struct DataCleanupJob {
    storage_engine: Arc<dyn DataCleanupStorageEngine>,
    storage_config: StorageConfig,
}

impl DataCleanupJob {
    pub fn new(
        storage_engine: Arc<dyn DataCleanupStorageEngine>,
        storage_config: StorageConfig,
    ) -> Self {
        DataCleanupJob {
            storage_engine,
            storage_config,
        }
    }

    // the deletion is based on the id from the entry which counts always up for the auto increment
}

#[async_trait]
impl JobTrait for DataCleanupJob {
    async fn run(&self) -> Result<()> {
        // TODO: change try into with ticket for config update to correct data types
        let full_resolution = Duration::hours(
            self.storage_config
                .metrics_retention_hours_full_resolution
                .try_into()?,
        );
        let reduced_resolution = Duration::hours(
            self.storage_config
                .metrics_retention_hours_reduced_resolution
                .try_into()?,
        );
        let now = Utc::now();
        // everything older than the full and the reduced time frame is removed completely
        let cutoff_time = now - (full_resolution + reduced_resolution);
        // everything older than the full time frame is reduced to every Xth entry
        let thin_out_before = now - full_resolution;
        let keep_every_x = self.storage_config.keep_every_x_metrics;

        // first remove the hard cutoff entries
        self.storage_engine
            .remove_all_older_than(cutoff_time)
            .await?;

        // run the deletion jobs one after another, so the db isn't bombarded and locked for long
        tokio::time::sleep(core::time::Duration::from_millis(250)).await;
        self.storage_engine
            .thin_out_cpu_stats(thin_out_before, keep_every_x)
            .await?;
        tokio::time::sleep(core::time::Duration::from_millis(250)).await;
        self.storage_engine
            .thin_out_memory_stats(thin_out_before, keep_every_x)
            .await?;
        tokio::time::sleep(core::time::Duration::from_millis(250)).await;
        self.storage_engine
            .thin_out_container_runtime_stats(thin_out_before, keep_every_x)
            .await?;
        tokio::time::sleep(core::time::Duration::from_millis(250)).await;
        self.storage_engine
            .thin_out_partition_entries(thin_out_before, keep_every_x)
            .await?;
        tokio::time::sleep(core::time::Duration::from_millis(250)).await;
        self.storage_engine
            .thin_out_network_stats(thin_out_before, keep_every_x)
            .await?;
        tokio::time::sleep(core::time::Duration::from_millis(250)).await;
        self.storage_engine
            .thin_out_system_stats(thin_out_before, keep_every_x)
            .await?;
        tokio::time::sleep(core::time::Duration::from_millis(250)).await;
        self.storage_engine
            .thin_out_processes_stats(thin_out_before, keep_every_x)
            .await?;
        tokio::time::sleep(core::time::Duration::from_millis(250)).await;
        self.storage_engine
            .thin_out_speedtest_stats(thin_out_before, keep_every_x)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// records which cutoff and which thin out calls the job sends to the storage engine
    #[derive(Default)]
    struct MockStorageEngine {
        cutoff_time: Mutex<Option<DateTime<Utc>>>,
        thin_outs: Mutex<Vec<(&'static str, DateTime<Utc>, u16)>>,
    }

    impl MockStorageEngine {
        fn record_thin_out(
            &self,
            table: &'static str,
            thin_out_before: DateTime<Utc>,
            keep_every_x: u16,
        ) -> Result<u64> {
            self.thin_outs
                .lock()
                .unwrap()
                .push((table, thin_out_before, keep_every_x));
            Ok(0)
        }
    }

    #[async_trait]
    impl DataCleanupStorageEngine for MockStorageEngine {
        async fn remove_all_older_than(&self, cutoff_time: DateTime<Utc>) -> Result<()> {
            *self.cutoff_time.lock().unwrap() = Some(cutoff_time);
            Ok(())
        }

        async fn thin_out_cpu_stats(
            &self,
            thin_out_before: DateTime<Utc>,
            keep_every_x: u16,
        ) -> Result<u64> {
            self.record_thin_out("cpu_stats", thin_out_before, keep_every_x)
        }

        async fn thin_out_memory_stats(
            &self,
            thin_out_before: DateTime<Utc>,
            keep_every_x: u16,
        ) -> Result<u64> {
            self.record_thin_out("memory_stats", thin_out_before, keep_every_x)
        }

        async fn thin_out_container_runtime_stats(
            &self,
            thin_out_before: DateTime<Utc>,
            keep_every_x: u16,
        ) -> Result<u64> {
            self.record_thin_out("container_runtime_stats", thin_out_before, keep_every_x)
        }

        async fn thin_out_partition_entries(
            &self,
            thin_out_before: DateTime<Utc>,
            keep_every_x: u16,
        ) -> Result<u64> {
            self.record_thin_out("partition_entry", thin_out_before, keep_every_x)
        }

        async fn thin_out_network_stats(
            &self,
            thin_out_before: DateTime<Utc>,
            keep_every_x: u16,
        ) -> Result<u64> {
            self.record_thin_out("network_stats", thin_out_before, keep_every_x)
        }

        async fn thin_out_system_stats(
            &self,
            thin_out_before: DateTime<Utc>,
            keep_every_x: u16,
        ) -> Result<u64> {
            self.record_thin_out("system_stats", thin_out_before, keep_every_x)
        }

        async fn thin_out_processes_stats(
            &self,
            thin_out_before: DateTime<Utc>,
            keep_every_x: u16,
        ) -> Result<u64> {
            self.record_thin_out("processes_stats", thin_out_before, keep_every_x)
        }

        async fn thin_out_speedtest_stats(
            &self,
            thin_out_before: DateTime<Utc>,
            keep_every_x: u16,
        ) -> Result<u64> {
            self.record_thin_out("speedtest_stats", thin_out_before, keep_every_x)
        }
    }

    #[tokio::test]
    async fn run_uses_correct_time_windows_test() {
        let storage_engine = Arc::new(MockStorageEngine::default());
        let job = DataCleanupJob::new(
            storage_engine.clone(),
            StorageConfig {
                database_url: String::new(),
                metrics_retention_hours_full_resolution: 24,
                metrics_retention_hours_reduced_resolution: 168,
                keep_every_x_metrics: 3,
            },
        );

        let before_run = Utc::now();
        job.run().await.unwrap();
        let after_run = Utc::now();

        // hard cutoff: everything older than full + reduced (24h + 168h) is removed
        let cutoff_time = storage_engine.cutoff_time.lock().unwrap().unwrap();
        assert!(cutoff_time >= before_run - Duration::hours(192));
        assert!(cutoff_time <= after_run - Duration::hours(192));

        // thin out: everything older than the full time frame (24h) is reduced, for every metric table
        let thin_outs = storage_engine.thin_outs.lock().unwrap();
        let tables: Vec<_> = thin_outs.iter().map(|(table, _, _)| *table).collect();
        assert_eq!(
            tables,
            vec![
                "cpu_stats",
                "memory_stats",
                "container_runtime_stats",
                "partition_entry",
                "network_stats",
                "system_stats",
                "processes_stats",
                "speedtest_stats"
            ]
        );
        for (_, thin_out_before, keep_every_x) in thin_outs.iter() {
            assert_eq!(*thin_out_before, cutoff_time + Duration::hours(168));
            assert_eq!(*keep_every_x, 3);
        }
    }
}
