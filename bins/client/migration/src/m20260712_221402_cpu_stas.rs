use sea_orm_migration::{prelude::*, schema::*};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260712_221402_cpu_stas"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CpuStats::Table)
                    .if_not_exists()
                    .col(pk_auto(CpuStats::Id))
                    .col(string(CpuStats::CpuName))
                    .col(integer(CpuStats::CpuCount))
                    .col(integer(CpuStats::CpuPhysicalCount))
                    .col(float(CpuStats::CpuUsagePercent))
                    .col(float(CpuStats::CpuTemperatureCelsius))
                    .col(timestamp_with_time_zone(CpuStats::CollectedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(CpuCoreStats::Table)
                    .if_not_exists()
                    .col(pk_auto(CpuCoreStats::Id))
                    .col(integer(CpuCoreStats::CpuStatsId))
                    .col(string(CpuCoreStats::CoreName))
                    .col(float(CpuCoreStats::CoreUsagePercent))
                    .col(big_unsigned(CpuCoreStats::CoreFrequencyMhz))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_cpu_core_stats_cpu_stats")
                            .from(CpuCoreStats::Table, CpuCoreStats::CpuStatsId)
                            .to(CpuStats::Table, CpuStats::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CpuCoreStats::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(CpuStats::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum CpuStats {
    Table,
    Id,
    CpuName,
    CpuCount,
    CpuPhysicalCount,
    CpuUsagePercent,
    CpuTemperatureCelsius,
    CollectedAt,
}

#[derive(DeriveIden)]
enum CpuCoreStats {
    Table,
    Id,
    CpuStatsId, // key from cpustats
    CoreName,
    CoreUsagePercent,
    CoreFrequencyMhz,
}
