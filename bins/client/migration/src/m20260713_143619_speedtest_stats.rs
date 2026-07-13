use sea_orm_migration::{prelude::*, schema::*};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260713_143619_speedtest_stats"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SpeedtestStats::Table)
                    .if_not_exists()
                    .col(pk_auto(SpeedtestStats::Id))
                    .col(double(SpeedtestStats::DownloadMbps))
                    .col(double(SpeedtestStats::UploadMbps))
                    .col(double(SpeedtestStats::PingMs))
                    .col(timestamp_with_time_zone(SpeedtestStats::CollectedAt))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(SpeedtestStats::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum SpeedtestStats {
    Table,
    Id,
    DownloadMbps,
    UploadMbps,
    PingMs,
    CollectedAt,
}
