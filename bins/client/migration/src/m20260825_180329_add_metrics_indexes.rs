use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260825_180329_add_metrics_indexes"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // collected_at is filtered/ordered on every range and "latest N" query,
        // and used as the cutoff column in the retention cleanup job.
        manager
            .create_index(
                Index::create()
                    .name("idx_cpu_stats_collected_at")
                    .table(CpuStats::Table)
                    .col(CpuStats::CollectedAt)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_memory_stats_collected_at")
                    .table(MemoryStats::Table)
                    .col(MemoryStats::CollectedAt)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_disk_entry_collected_at")
                    .table(DiskEntry::Table)
                    .col(DiskEntry::CollectedAt)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_network_stats_collected_at")
                    .table(NetworkStats::Table)
                    .col(NetworkStats::CollectedAt)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_system_stats_collected_at")
                    .table(SystemStats::Table)
                    .col(SystemStats::CollectedAt)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_processes_stats_collected_at")
                    .table(ProcessesStats::Table)
                    .col(ProcessesStats::CollectedAt)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_speedtest_stats_collected_at")
                    .table(SpeedtestStats::Table)
                    .col(SpeedtestStats::CollectedAt)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_container_runtime_stats_collected_at")
                    .table(ContainerRuntimeStats::Table)
                    .col(ContainerRuntimeStats::CollectedAt)
                    .to_owned(),
            )
            .await?;

        // FK columns joined by find_with_related() on every range/latest query.
        manager
            .create_index(
                Index::create()
                    .name("idx_cpu_core_stats_cpu_stats_id")
                    .table(CpuCoreStats::Table)
                    .col(CpuCoreStats::CpuStatsId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_disk_stats_disk_entry_id")
                    .table(DiskStats::Table)
                    .col(DiskStats::DiskEntryId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_process_stats_processes_stats_id")
                    .table(ProcessStats::Table)
                    .col(ProcessStats::ProcessesStatsId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_container_stats_container_runtime_stats_id")
                    .table(ContainerStats::Table)
                    .col(ContainerStats::ContainerRuntimeStatsId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for index_name in [
            "idx_cpu_stats_collected_at",
            "idx_memory_stats_collected_at",
            "idx_disk_entry_collected_at",
            "idx_network_stats_collected_at",
            "idx_system_stats_collected_at",
            "idx_processes_stats_collected_at",
            "idx_speedtest_stats_collected_at",
            "idx_container_runtime_stats_collected_at",
            "idx_cpu_core_stats_cpu_stats_id",
            "idx_disk_stats_disk_entry_id",
            "idx_process_stats_processes_stats_id",
            "idx_container_stats_container_runtime_stats_id",
        ] {
            manager
                .drop_index(Index::drop().name(index_name).to_owned())
                .await?;
        }
        Ok(())
    }
}

#[derive(DeriveIden)]
enum CpuStats {
    Table,
    CollectedAt,
}

#[derive(DeriveIden)]
enum CpuCoreStats {
    Table,
    CpuStatsId,
}

#[derive(DeriveIden)]
enum MemoryStats {
    Table,
    CollectedAt,
}

#[derive(DeriveIden)]
enum DiskEntry {
    Table,
    CollectedAt,
}

#[derive(DeriveIden)]
enum DiskStats {
    Table,
    DiskEntryId,
}

#[derive(DeriveIden)]
enum NetworkStats {
    Table,
    CollectedAt,
}

#[derive(DeriveIden)]
enum SystemStats {
    Table,
    CollectedAt,
}

#[derive(DeriveIden)]
enum ProcessesStats {
    Table,
    CollectedAt,
}

#[derive(DeriveIden)]
enum ProcessStats {
    Table,
    ProcessesStatsId,
}

#[derive(DeriveIden)]
enum SpeedtestStats {
    Table,
    CollectedAt,
}

#[derive(DeriveIden)]
enum ContainerRuntimeStats {
    Table,
    CollectedAt,
}

#[derive(DeriveIden)]
enum ContainerStats {
    Table,
    ContainerRuntimeStatsId,
}
