use sea_orm_migration::{prelude::*, schema::*};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260713_143558_network_stats"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(NetworkStats::Table)
                    .if_not_exists()
                    .col(pk_auto(NetworkStats::Id))
                    .col(string(NetworkStats::LocalIp))
                    .col(big_unsigned(NetworkStats::TotalBytesTransmitted))
                    .col(big_unsigned(NetworkStats::TotalBytesReceived))
                    .col(big_unsigned(NetworkStats::TotalPacketsTransmitted))
                    .col(big_unsigned(NetworkStats::TotalPacketsReceived))
                    .col(timestamp_with_time_zone(NetworkStats::CollectedAt))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(NetworkStats::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum NetworkStats {
    Table,
    Id,
    LocalIp,
    TotalBytesTransmitted,
    TotalBytesReceived,
    TotalPacketsTransmitted,
    TotalPacketsReceived,
    CollectedAt,
}
