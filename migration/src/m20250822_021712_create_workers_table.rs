use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::DatabaseBackend;

use crate::m20220101_000001_create_pools_table::Pool;
use crate::m20250822_021707_create_miners_table::Miner;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Worker::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Worker::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Worker::PoolId).uuid().not_null())
                    .col(ColumnDef::new(Worker::MinerId).uuid().not_null())
                    .col(ColumnDef::new(Worker::Name).string().not_null())
                    .col(
                        ColumnDef::new(Worker::CreatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT (now())"),
                    )
                    .col(
                        ColumnDef::new(Worker::LastSeen)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT (now())"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(WorkerIndex::PoolId.to_string())
                    .table(Worker::Table)
                    .col(Worker::PoolId)
                    .take(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(WorkerIndex::MinerId.to_string())
                    .table(Worker::Table)
                    .col(Worker::MinerId)
                    .take(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(WorkerIndex::Name.to_string())
                    .table(Worker::Table)
                    .col(Worker::Name)
                    .take(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(WorkerIndex::LastSeen.to_string())
                    .table(Worker::Table)
                    .col(Worker::LastSeen)
                    .take(),
            )
            .await?;

        if manager.get_database_backend() != DatabaseBackend::Sqlite {
            manager
                .create_foreign_key(
                    ForeignKey::create()
                        .from(Worker::Table, Worker::PoolId)
                        .to(Pool::Table, Pool::Id)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                        .take(),
                )
                .await?;

            manager
                .create_foreign_key(
                    ForeignKey::create()
                        .from(Worker::Table, Worker::MinerId)
                        .to(Miner::Table, Miner::Id)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                        .take(),
                )
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Worker::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub(crate) enum Worker {
    #[sea_orm(iden = "workers")]
    Table,
    Id,
    PoolId,
    MinerId,
    Name,
    CreatedAt,
    LastSeen,
}

#[derive(DeriveIden)]
pub(crate) enum WorkerIndex {
    #[sea_orm(iden = "idx_workers_pool_id")]
    PoolId,
    #[sea_orm(iden = "idx_workers_miner_id")]
    MinerId,
    #[sea_orm(iden = "idx_workers_name")]
    Name,
    #[sea_orm(iden = "idx_workers_last_seen")]
    LastSeen,
}
