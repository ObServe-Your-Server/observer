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
                    .table(DiskEntry::Table)
                    .if_not_exists()
                    .col(pk_auto(DiskEntry::Id))
                    .col(timestamp_with_time_zone(DiskEntry::CollectedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(DiskStats::Table)
                    .if_not_exists()
                    .col(pk_auto(DiskStats::Id))
                    .col(integer(DiskStats::DiskEntryId))
                    .col(string(DiskStats::Name))
                    .col(big_unsigned(DiskStats::TotalBytes))
                    .col(big_unsigned(DiskStats::UsedBytes))
                    .col(big_unsigned(DiskStats::AvailableBytes))
                    .col(big_unsigned(DiskStats::UsedBlocks))
                    .col(big_unsigned(DiskStats::AvailableBlocks))
                    .col(big_unsigned(DiskStats::BlockSize))
                    .col(timestamp_with_time_zone(DiskStats::CollectedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_disk_stats_disk_entry")
                            .from(DiskStats::Table, DiskStats::DiskEntryId)
                            .to(DiskEntry::Table, DiskEntry::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(DiskStats::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(DiskEntry::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum DiskEntry {
    Table,
    Id,
    CollectedAt,
}

#[derive(DeriveIden)]
enum DiskStats {
    Table,
    Id,
    DiskEntryId, //foreign key
    Name,
    TotalBytes,
    UsedBytes,
    AvailableBytes,
    UsedBlocks,
    AvailableBlocks,
    BlockSize,
    CollectedAt,
}
