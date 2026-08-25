use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::{json, pk_auto, timestamp_with_time_zone};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260813_115107_tunnel_access_log"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(TunnelAccessLog::Table)
                    .if_not_exists()
                    .col(pk_auto(TunnelAccessLog::Id))
                    .col(json(TunnelAccessLog::SentData))
                    .col(timestamp_with_time_zone(TunnelAccessLog::SentAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_tunnel_access_log_sent_at")
                    .table(TunnelAccessLog::Table)
                    .col(TunnelAccessLog::SentAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(TunnelAccessLog::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum TunnelAccessLog {
    Table,
    Id,
    SentData,
    SentAt,
}
