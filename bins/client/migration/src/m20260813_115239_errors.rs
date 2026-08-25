use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::{
    big_unsigned, pk_auto, string_null, timestamp_with_time_zone,
};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260813_115239_error_stats"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Error::Table)
                    .if_not_exists()
                    .col(pk_auto(Error::Id))
                    .col(string_null(Error::File))
                    .col(big_unsigned(Error::Line))
                    .col(string_null(Error::Severity))
                    .col(string_null(Error::Message))
                    .col(timestamp_with_time_zone(Error::CollectedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_errors_collected_at")
                    .table(Error::Table)
                    .col(Error::CollectedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Error::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Error {
    Table,
    Id,
    File,
    Line,
    Severity,
    Message,
    CollectedAt,
}
