use sea_orm_migration::{prelude::*, schema::*};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260713_143522_container_runtime_stats"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ContainerRuntimeStats::Table)
                    .if_not_exists()
                    .col(pk_auto(ContainerRuntimeStats::Id))
                    .col(timestamp_with_time_zone(
                        ContainerRuntimeStats::CollectedAt,
                    ))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ContainerStats::Table)
                    .if_not_exists()
                    .col(pk_auto(ContainerStats::Id))
                    .col(integer(ContainerStats::ContainerRuntimeStatsId))
                    .col(string(ContainerStats::ContainerRuntime))
                    .col(string(ContainerStats::ContainerId))
                    .col(string(ContainerStats::HostName))
                    .col(big_integer(ContainerStats::CreatedAt))
                    .col(string(ContainerStats::Status))
                    .col(boolean(ContainerStats::Running))
                    .col(big_unsigned(ContainerStats::RunningForSeconds))
                    .col(string(ContainerStats::ImageName))
                    .col(string(ContainerStats::Networks))
                    .col(double(ContainerStats::CpuUsagePercent))
                    .col(big_unsigned(ContainerStats::MemoryUsageBytes))
                    .col(timestamp_with_time_zone(ContainerStats::CollectedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_container_stats_container_runtime_stats")
                            .from(
                                ContainerStats::Table,
                                ContainerStats::ContainerRuntimeStatsId,
                            )
                            .to(ContainerRuntimeStats::Table, ContainerRuntimeStats::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ContainerStats::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(ContainerRuntimeStats::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ContainerRuntimeStats {
    Table,
    Id,
    CollectedAt,
}

#[derive(DeriveIden)]
enum ContainerStats {
    Table,
    Id,
    ContainerRuntimeStatsId, // key from container_runtime_stats
    ContainerRuntime,
    ContainerId,
    HostName,
    CreatedAt,
    Status,
    Running,
    RunningForSeconds,
    ImageName,
    Networks,
    CpuUsagePercent,
    MemoryUsageBytes,
    CollectedAt,
}
