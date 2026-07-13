use sea_orm_migration::{prelude::*, schema::*};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260713_143612_process_stats"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ProcessesStats::Table)
                    .if_not_exists()
                    .col(pk_auto(ProcessesStats::Id))
                    .col(timestamp_with_time_zone(ProcessesStats::CollectedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ProcessStats::Table)
                    .if_not_exists()
                    .col(pk_auto(ProcessStats::Id))
                    .col(integer(ProcessStats::ProcessesStatsId))
                    .col(string(ProcessStats::Kind))
                    .col(integer(ProcessStats::Pid))
                    .col(string(ProcessStats::Name))
                    .col(string(ProcessStats::UserName))
                    .col(string(ProcessStats::Status))
                    .col(float(ProcessStats::CpuUsagePercent))
                    .col(big_unsigned(ProcessStats::MemoryUsageBytes))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_process_stats_processes_stats")
                            .from(ProcessStats::Table, ProcessStats::ProcessesStatsId)
                            .to(ProcessesStats::Table, ProcessesStats::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ProcessStats::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(ProcessesStats::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ProcessesStats {
    Table,
    Id,
    CollectedAt,
}

#[derive(DeriveIden)]
enum ProcessStats {
    Table,
    Id,
    ProcessesStatsId, // key from processes_stats
    Kind,             // "cpu" or "memory", which top-N list this row belongs to
    Pid,
    Name,
    UserName,
    Status,
    CpuUsagePercent,
    MemoryUsageBytes,
}
