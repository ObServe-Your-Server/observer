use sea_orm_migration::{prelude::*, schema::*};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20261011_000000_gpu_stats"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(GpuEntry::Table)
                    .if_not_exists()
                    .col(pk_auto(GpuEntry::Id))
                    .col(timestamp_with_time_zone(GpuEntry::CollectedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(GpuStats::Table)
                    .if_not_exists()
                    .col(pk_auto(GpuStats::Id))
                    .col(integer(GpuStats::GpuEntryId))
                    .col(string(GpuStats::Name))
                    .col(float_null(GpuStats::UsagePercent))
                    .col(big_unsigned_null(GpuStats::MemoryUsedBytes))
                    .col(big_unsigned_null(GpuStats::MemoryTotalBytes))
                    .col(float_null(GpuStats::TemperatureCelsius))
                    .col(timestamp_with_time_zone(GpuStats::CollectedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_gpu_stats_gpu_entry")
                            .from(GpuStats::Table, GpuStats::GpuEntryId)
                            .to(GpuEntry::Table, GpuEntry::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(GpuStats::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(GpuEntry::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum GpuEntry {
    Table,
    Id,
    CollectedAt,
}

#[derive(DeriveIden)]
enum GpuStats {
    Table,
    Id,
    GpuEntryId,
    Name,
    UsagePercent,
    MemoryUsedBytes,
    MemoryTotalBytes,
    TemperatureCelsius,
    CollectedAt,
}
