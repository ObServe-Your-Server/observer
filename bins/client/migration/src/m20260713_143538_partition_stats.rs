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
                    .table(PartitionEntry::Table)
                    .if_not_exists()
                    .col(pk_auto(PartitionEntry::Id))
                    .col(timestamp_with_time_zone(PartitionEntry::CollectedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_partition_entry_collected_at")
                    .table(PartitionEntry::Table)
                    .col(PartitionEntry::CollectedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(PartitionStats::Table)
                    .if_not_exists()
                    .col(pk_auto(PartitionStats::Id))
                    .col(integer(PartitionStats::PartitionEntryId))
                    .col(string(PartitionStats::Name))
                    .col(string(PartitionStats::Device))
                    .col(string(PartitionStats::MountPoint))
                    .col(string(PartitionStats::FsType))
                    .col(big_unsigned(PartitionStats::TotalBytes))
                    .col(big_unsigned(PartitionStats::UsedBytes))
                    .col(big_unsigned(PartitionStats::AvailableBytes))
                    .col(big_unsigned(PartitionStats::UsedBlocks))
                    .col(big_unsigned(PartitionStats::AvailableBlocks))
                    .col(big_unsigned(PartitionStats::BlockSize))
                    .col(timestamp_with_time_zone(PartitionStats::CollectedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_partition_stats_partition_entry")
                            .from(PartitionStats::Table, PartitionStats::PartitionEntryId)
                            .to(PartitionEntry::Table, PartitionEntry::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_partition_stats_collected_at")
                    .table(PartitionStats::Table)
                    .col(PartitionStats::CollectedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_partition_stats_name")
                    .table(PartitionStats::Table)
                    .col(PartitionStats::Name)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PartitionStats::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(PartitionEntry::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum PartitionEntry {
    Table,
    Id,
    CollectedAt,
}

#[derive(DeriveIden)]
enum PartitionStats {
    Table,
    Id,
    PartitionEntryId, //foreign key
    Name,
    Device,
    MountPoint,
    FsType,
    TotalBytes,
    UsedBytes,
    AvailableBytes,
    UsedBlocks,
    AvailableBlocks,
    BlockSize,
    CollectedAt,
}
