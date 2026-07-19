use sea_orm_migration::{prelude::*, schema::*};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260713_143634_system_stats"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SystemStats::Table)
                    .if_not_exists()
                    .col(pk_auto(SystemStats::Id))
                    .col(string_null(SystemStats::OsName))
                    .col(big_unsigned(SystemStats::UptimeSeconds))
                    .col(string_null(SystemStats::HostName))
                    .col(string(SystemStats::KernelVersion))
                    .col(timestamp_with_time_zone(SystemStats::CollectedAt))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(SystemStats::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum SystemStats {
    Table,
    Id,
    OsName,
    UptimeSeconds,
    HostName,
    KernelVersion,
    CollectedAt,
}
