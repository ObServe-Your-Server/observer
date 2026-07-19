use sea_orm_migration::{prelude::*, schema::*};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260713_143538_disk_stats"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(DiskStats::Table)
                    .if_not_exists()
                    .col(pk_auto(DiskStats::Id))
                    .col(string(DiskStats::Name))
                    .col(big_unsigned(DiskStats::TotalBytes))
                    .col(big_unsigned(DiskStats::UsedBytes))
                    .col(big_unsigned(DiskStats::AvailableBytes))
                    .col(big_unsigned(DiskStats::UsedBlocks))
                    .col(big_unsigned(DiskStats::AvailableBlocks))
                    .col(big_unsigned(DiskStats::BlockSize))
                    .col(timestamp_with_time_zone(DiskStats::CollectedAt))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(DiskStats::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum DiskStats {
    Table,
    Id,
    Name,
    TotalBytes,
    UsedBytes,
    AvailableBytes,
    UsedBlocks,
    AvailableBlocks,
    BlockSize,
    CollectedAt,
}
