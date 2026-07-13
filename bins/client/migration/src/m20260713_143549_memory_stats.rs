use sea_orm_migration::{prelude::*, schema::*};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260713_143549_memory_stats"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(MemoryStats::Table)
                    .if_not_exists()
                    .col(pk_auto(MemoryStats::Id))
                    .col(big_unsigned(MemoryStats::TotalMemoryInByte))
                    .col(big_unsigned(MemoryStats::AvailableMemoryInByte))
                    .col(big_unsigned(MemoryStats::UsedMemoryInByte))
                    .col(big_unsigned(MemoryStats::TotalSwapInByte))
                    .col(big_unsigned(MemoryStats::AvailableSwapInByte))
                    .col(big_unsigned(MemoryStats::UsedSwapInByte))
                    .col(timestamp_with_time_zone(MemoryStats::CollectedAt))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(MemoryStats::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum MemoryStats {
    Table,
    Id,
    TotalMemoryInByte,
    AvailableMemoryInByte,
    UsedMemoryInByte,
    TotalSwapInByte,
    AvailableSwapInByte,
    UsedSwapInByte,
    CollectedAt,
}
