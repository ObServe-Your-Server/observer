use crate::entities::cpu_stats::ActiveModel;
use crate::entities::{
    container_runtime_stats, container_stats, cpu_core_stats, cpu_stats, disk_entry, disk_stats,
    memory_stats, network_stats, process_stats, processes_stats, speedtest_stats, system_stats,
};
use crate::jobs::base_metric_collection_job::BaseMetrics;
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use migration::{Migrator, MigratorTrait};
use open_eye::collector::container_runtime::collector::{ContainerRuntime, ContainerRuntimeStats};
use open_eye::collector::speedtest::collector::SpeedtestResult;
use sea_orm::{
    ActiveValue::Set, ColumnTrait, ConnectOptions, Database, DatabaseConnection, EntityTrait,
    QueryFilter, QueryOrder, QuerySelect,
};
use std::sync::OnceLock;
use std::time::Duration;

pub struct StorageEngine {
    database_path: String,
    db: OnceLock<DatabaseConnection>,
}

impl StorageEngine {
    pub fn new(database_path: String) -> StorageEngine {
        StorageEngine {
            database_path,
            db: OnceLock::new(),
        }
    }

    pub async fn connect_to_db_and_migrate(self) -> Result<Self> {
        let mut opt = ConnectOptions::new(self.database_path.clone());
        // SQLite serializes writes, so a large pool buys no throughput but costs
        // ~2MB page cache + lookaside + statement cache per open connection.
        opt.max_connections(8)
            .min_connections(1)
            .connect_timeout(Duration::from_secs(8))
            .acquire_timeout(Duration::from_secs(8))
            .idle_timeout(Duration::from_secs(60))
            .max_lifetime(Duration::from_secs(1800))
            .sqlx_logging(false) // disable SQLx logging
            .sqlx_logging_level(log::LevelFilter::Info);
        //.set_schema_search_path("my_schema"); // set default Postgres schema

        let db_conn = Database::connect(opt).await?;

        Migrator::up(&db_conn, None).await?;
        self.db
            .set(db_conn)
            .map_err(|_| anyhow!("database connection already set"))?;
        log::debug!("Connected and migrated db at: {}", &self.database_path);
        Ok(self)
    }

    fn db(&self) -> Result<&DatabaseConnection> {
        self.db
            .get()
            .ok_or_else(|| anyhow!("database connection not initialized"))
    }

    pub async fn cleanup_job(&self, clean_older_than: DateTime<Utc>) -> Result<()> {
        let db = self
            .db
            .get()
            .ok_or_else(|| anyhow!("database connection not initialized"))?;

        // no independent deletes to take load of connections
        // cpu_core_stats/disk_stats rows cascade-delete via the FK's on_delete = "Cascade"

        cpu_stats::Entity::delete_many()
            .filter(cpu_stats::Column::CollectedAt.lt(clean_older_than))
            .exec(db)
            .await?;
        memory_stats::Entity::delete_many()
            .filter(memory_stats::Column::CollectedAt.lt(clean_older_than))
            .exec(db)
            .await?;
        disk_entry::Entity::delete_many()
            .filter(disk_entry::Column::CollectedAt.lt(clean_older_than))
            .exec(db)
            .await?;
        network_stats::Entity::delete_many()
            .filter(network_stats::Column::CollectedAt.lt(clean_older_than))
            .exec(db)
            .await?;
        system_stats::Entity::delete_many()
            .filter(system_stats::Column::CollectedAt.lt(clean_older_than))
            .exec(db)
            .await?;

        Ok(())
    }

    pub async fn save_base_metrics_to_db(&self, base_metrics: BaseMetrics) -> Result<()> {
        let db = self.db()?;

        if let Some(cpu) = base_metrics.cpu {
            let model = cpu_stats::ActiveModel {
                cpu_name: Set(cpu.cpu_name),
                cpu_count: Set(cpu.cpu_count as i64),
                cpu_physical_count: Set(cpu.cpu_physical_count as i64),
                cpu_usage_percent: Set(cpu.cpu_usage_percent),
                cpu_temperature_celsius: Set(cpu.cpu_temperature_celsius),
                collected_at: Set(cpu.collected_at.into()),
                ..Default::default() // leaves id NotSet so the db auto-generates it
            };
            //gets the id to reference it
            let inserted = cpu_stats::Entity::insert(model).exec(db).await?;

            for core in cpu.core_information {
                let core_model = cpu_core_stats::ActiveModel {
                    cpu_stats_id: Set(inserted.last_insert_id),
                    core_name: Set(core.core_name),
                    core_usage_percent: Set(core.core_usage_percent),
                    core_frequency_mhz: Set(core.core_frequency_mhz as i64),
                    ..Default::default()
                };
                cpu_core_stats::Entity::insert(core_model).exec(db).await?;
            }
        }

        if let Some(memory) = base_metrics.memory {
            let model = memory_stats::ActiveModel {
                total_memory_in_byte: Set(memory.total_memory_in_byte as i64),
                available_memory_in_byte: Set(memory.available_memory_in_byte as i64),
                used_memory_in_byte: Set(memory.used_memory_in_byte as i64),
                total_swap_in_byte: Set(memory.total_swap_in_byte as i64),
                available_swap_in_byte: Set(memory.available_swap_in_byte as i64),
                used_swap_in_byte: Set(memory.used_swap_in_byte as i64),
                collected_at: Set(memory.collected_at.into()),
                ..Default::default()
            };
            memory_stats::Entity::insert(model).exec(db).await?;
        }

        if let Some(disks) = base_metrics.disks {
            if let Some(first_disk) = disks.first() {
                let entry_model = disk_entry::ActiveModel {
                    collected_at: Set(first_disk.collected_at.into()),
                    ..Default::default()
                };
                //gets the id to reference it
                let inserted_entry = disk_entry::Entity::insert(entry_model).exec(db).await?;

                for disk in disks {
                    let model = disk_stats::ActiveModel {
                        disk_entry_id: Set(inserted_entry.last_insert_id),
                        name: Set(disk.name),
                        total_bytes: Set(disk.total_bytes as i64),
                        used_bytes: Set(disk.used_bytes as i64),
                        available_bytes: Set(disk.available_bytes as i64),
                        used_blocks: Set(disk.used_blocks as i64),
                        available_blocks: Set(disk.available_blocks as i64),
                        block_size: Set(disk.block_size as i64),
                        collected_at: Set(disk.collected_at.into()),
                        ..Default::default()
                    };
                    disk_stats::Entity::insert(model).exec(db).await?;
                }
            }
        }

        if let Some(network) = base_metrics.network {
            let model = network_stats::ActiveModel {
                local_ip: Set(network.local_ip),
                total_bytes_transmitted: Set(network.total_bytes_transmitted as i64),
                total_bytes_received: Set(network.total_bytes_received as i64),
                total_packets_transmitted: Set(network.total_packets_transmitted as i64),
                total_packets_received: Set(network.total_packets_received as i64),
                collected_at: Set(network.collected_at.into()),
                ..Default::default()
            };
            network_stats::Entity::insert(model).exec(db).await?;
        }

        if let Some(system) = base_metrics.system {
            let model = system_stats::ActiveModel {
                os_name: Set(system.os_name),
                uptime_seconds: Set(system.uptime_seconds as i64),
                host_name: Set(system.host_name),
                kernel_version: Set(system.kernel_version),
                collected_at: Set(system.collected_at.into()),
                ..Default::default()
            };
            system_stats::Entity::insert(model).exec(db).await?;
        }

        Ok(())
    }

    pub async fn save_container_runtime_stats_to_db(
        &self,
        container_runtime_stats: ContainerRuntimeStats,
    ) -> Result<()> {
        let db = self.db()?;
        let model = container_runtime_stats::ActiveModel {
            collected_at: Set(container_runtime_stats.collected_at.into()),
            ..Default::default()
        };
        let inserted_container_runtime = container_runtime_stats::Entity::insert(model)
            .exec(db)
            .await?;

        let models: Vec<container_stats::ActiveModel> = container_runtime_stats
            .container_stats
            .into_iter()
            .map(|c| container_stats::ActiveModel {
                container_runtime_stats_id: Set(inserted_container_runtime.last_insert_id),
                container_runtime: Set(c.container_runtime.to_string()),
                container_id: Set(c.id),
                host_name: Set(c.host_name),
                created_at: Set(c.created_at.into()),
                status: Set(c.status),
                running: Set(c.running),
                running_for_seconds: Set(c.running_for_seconds as i64),
                image_name: Set(c.image_name),
                networks: Set(c.networks.join("|")),
                cpu_usage_percent: Set(c.cpu_usage_percent),
                memory_usage_bytes: Set(c.memory_usage_bytes as i64),
                collected_at: Set(c.collected_at.into()),
                ..Default::default()
            })
            .collect();

        for model in models {
            container_stats::Entity::insert(model).exec(db).await?;
        }
        Ok(())
    }

    pub async fn save_speedtest_stats_to_db(&self, speedtest_stats: SpeedtestResult) -> Result<()> {
        let db = self.db()?;

        let model = speedtest_stats::ActiveModel {
            download_mbps: Set(speedtest_stats.download_mbps),
            upload_mbps: Set(speedtest_stats.upload_mbps),
            ping_ms: Set(speedtest_stats.ping_ms),
            collected_at: Set(speedtest_stats.collected_at.into()),
            ..Default::default()
        };

        speedtest_stats::Entity::insert(model).exec(db).await?;
        Ok(())
    }

    pub async fn get_cpu_stats_between(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<(cpu_stats::Model, Vec<cpu_core_stats::Model>)>> {
        let db = self.db()?;
        let rows = cpu_stats::Entity::find()
            .filter(cpu_stats::Column::CollectedAt.between(start, end))
            .order_by_asc(cpu_stats::Column::CollectedAt)
            .find_with_related(cpu_core_stats::Entity)
            .all(db)
            .await?;
        Ok(rows)
    }

    pub async fn get_memory_stats_between(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<memory_stats::Model>> {
        let db = self.db()?;
        Ok(memory_stats::Entity::find()
            .filter(memory_stats::Column::CollectedAt.between(start, end))
            .order_by_asc(memory_stats::Column::CollectedAt)
            .all(db)
            .await?)
    }

    pub async fn get_disk_stats_between(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<(disk_entry::Model, Vec<disk_stats::Model>)>> {
        let db = self.db()?;
        let rows = disk_entry::Entity::find()
            .filter(disk_entry::Column::CollectedAt.between(start, end))
            .order_by_asc(disk_entry::Column::CollectedAt)
            .find_with_related(disk_stats::Entity)
            .all(db)
            .await?;
        Ok(rows)
    }

    pub async fn get_network_stats_between(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<network_stats::Model>> {
        let db = self.db()?;
        Ok(network_stats::Entity::find()
            .filter(network_stats::Column::CollectedAt.between(start, end))
            .order_by_asc(network_stats::Column::CollectedAt)
            .all(db)
            .await?)
    }

    pub async fn get_system_stats_between(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<system_stats::Model>> {
        let db = self.db()?;
        Ok(system_stats::Entity::find()
            .filter(system_stats::Column::CollectedAt.between(start, end))
            .order_by_asc(system_stats::Column::CollectedAt)
            .all(db)
            .await?)
    }

    pub async fn get_processes_stats_between(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<(processes_stats::Model, Vec<process_stats::Model>)>> {
        let db = self.db()?;
        let rows = processes_stats::Entity::find()
            .filter(processes_stats::Column::CollectedAt.between(start, end))
            .order_by_asc(processes_stats::Column::CollectedAt)
            .find_with_related(process_stats::Entity)
            .all(db)
            .await?;
        Ok(rows)
    }

    pub async fn get_container_runtime_stats_between(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<(container_runtime_stats::Model, Vec<container_stats::Model>)>> {
        let db = self.db()?;
        let rows = container_runtime_stats::Entity::find()
            .filter(container_runtime_stats::Column::CollectedAt.between(start, end))
            .order_by_asc(container_runtime_stats::Column::CollectedAt)
            .find_with_related(container_stats::Entity)
            .all(db)
            .await?;
        Ok(rows)
    }

    pub async fn get_speedtest_stats_between(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<speedtest_stats::Model>> {
        let db = self.db()?;
        Ok(speedtest_stats::Entity::find()
            .filter(speedtest_stats::Column::CollectedAt.between(start, end))
            .order_by_asc(speedtest_stats::Column::CollectedAt)
            .all(db)
            .await?)
    }

    pub async fn get_cpu_stats_latest(
        &self,
        last_n: u64,
    ) -> Result<Vec<(cpu_stats::Model, Vec<cpu_core_stats::Model>)>> {
        let db = self.db()?;
        let latest_ids: Vec<i64> = cpu_stats::Entity::find()
            .order_by_desc(cpu_stats::Column::CollectedAt)
            .limit(last_n)
            .all(db)
            .await?
            .into_iter()
            .map(|m| m.id)
            .collect();

        let mut rows = cpu_stats::Entity::find()
            .filter(cpu_stats::Column::Id.is_in(latest_ids))
            .order_by_desc(cpu_stats::Column::CollectedAt)
            .find_with_related(cpu_core_stats::Entity)
            .all(db)
            .await?;
        rows.reverse();
        Ok(rows)
    }

    pub async fn get_memory_stats_latest(&self, last_n: u64) -> Result<Vec<memory_stats::Model>> {
        let db = self.db()?;
        let mut rows = memory_stats::Entity::find()
            .order_by_desc(memory_stats::Column::CollectedAt)
            .limit(last_n)
            .all(db)
            .await?;
        rows.reverse();
        Ok(rows)
    }

    pub async fn get_disk_stats_latest(
        &self,
        last_n: u64,
    ) -> Result<Vec<(disk_entry::Model, Vec<disk_stats::Model>)>> {
        let db = self.db()?;
        let latest_ids: Vec<i64> = disk_entry::Entity::find()
            .order_by_desc(disk_entry::Column::CollectedAt)
            .limit(last_n)
            .all(db)
            .await?
            .into_iter()
            .map(|m| m.id)
            .collect();

        let mut rows = disk_entry::Entity::find()
            .filter(disk_entry::Column::Id.is_in(latest_ids))
            .order_by_desc(disk_entry::Column::CollectedAt)
            .find_with_related(disk_stats::Entity)
            .all(db)
            .await?;
        rows.reverse();
        Ok(rows)
    }

    pub async fn get_network_stats_latest(&self, last_n: u64) -> Result<Vec<network_stats::Model>> {
        let db = self.db()?;
        let mut rows = network_stats::Entity::find()
            .order_by_desc(network_stats::Column::CollectedAt)
            .limit(last_n)
            .all(db)
            .await?;
        rows.reverse();
        Ok(rows)
    }

    pub async fn get_system_stats_latest(&self, last_n: u64) -> Result<Vec<system_stats::Model>> {
        let db = self.db()?;
        let mut rows = system_stats::Entity::find()
            .order_by_desc(system_stats::Column::CollectedAt)
            .limit(last_n)
            .all(db)
            .await?;
        rows.reverse();
        Ok(rows)
    }

    pub async fn get_processes_stats_latest(
        &self,
        last_n: u64,
    ) -> Result<Vec<(processes_stats::Model, Vec<process_stats::Model>)>> {
        let db = self.db()?;
        let mut rows = processes_stats::Entity::find()
            .order_by_desc(processes_stats::Column::CollectedAt)
            .limit(last_n)
            .find_with_related(process_stats::Entity)
            .all(db)
            .await?;
        rows.reverse();
        Ok(rows)
    }

    pub async fn get_container_runtime_stats_latest(
        &self,
        last_n: u64,
    ) -> Result<Vec<(container_runtime_stats::Model, Vec<container_stats::Model>)>> {
        let db = self.db()?;
        let mut rows = container_runtime_stats::Entity::find()
            .order_by_desc(container_runtime_stats::Column::CollectedAt)
            .limit(last_n)
            .find_with_related(container_stats::Entity)
            .all(db)
            .await?;
        rows.reverse();
        Ok(rows)
    }

    pub async fn get_speedtest_stats_latest(
        &self,
        last_n: u64,
    ) -> Result<Vec<speedtest_stats::Model>> {
        let db = self.db()?;
        let mut rows = speedtest_stats::Entity::find()
            .order_by_desc(speedtest_stats::Column::CollectedAt)
            .limit(last_n)
            .all(db)
            .await?;
        rows.reverse();
        Ok(rows)
    }
}
