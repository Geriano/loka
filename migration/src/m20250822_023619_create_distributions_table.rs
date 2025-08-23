use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::DatabaseBackend;

use crate::m20220101_000001_create_pools_table::Pool;
use crate::m20250822_021707_create_miners_table::Miner;
use crate::m20250822_021712_create_workers_table::Worker;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Distribution::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Distribution::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Distribution::PoolId).uuid().not_null())
                    .col(ColumnDef::new(Distribution::MinerId).uuid().not_null())
                    .col(ColumnDef::new(Distribution::WorkerId).uuid().not_null())
                    .col(
                        ColumnDef::new(Distribution::Reward)
                            .big_unsigned()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Distribution::HashrateExternal)
                            .decimal_len(3, 32)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Distribution::HashrateInternal)
                            .decimal_len(3, 32)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Distribution::Date).date().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(DistributionIndex::PoolId.to_string())
                    .table(Distribution::Table)
                    .col(Distribution::PoolId)
                    .take(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(DistributionIndex::MinerId.to_string())
                    .table(Distribution::Table)
                    .col(Distribution::MinerId)
                    .take(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(DistributionIndex::WorkerId.to_string())
                    .table(Distribution::Table)
                    .col(Distribution::WorkerId)
                    .take(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(DistributionIndex::Date.to_string())
                    .table(Distribution::Table)
                    .col(Distribution::Date)
                    .take(),
            )
            .await?;

        if manager.get_database_backend() != DatabaseBackend::Sqlite {
            manager
                .create_foreign_key(
                    ForeignKey::create()
                        .from(Distribution::Table, Distribution::PoolId)
                        .to(Pool::Table, Pool::Id)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                        .take(),
                )
                .await?;

            manager
                .create_foreign_key(
                    ForeignKey::create()
                        .from(Distribution::Table, Distribution::MinerId)
                        .to(Miner::Table, Miner::Id)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                        .take(),
                )
                .await?;

            manager
                .create_foreign_key(
                    ForeignKey::create()
                        .from(Distribution::Table, Distribution::WorkerId)
                        .to(Worker::Table, Worker::Id)
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
            .drop_table(Table::drop().table(Distribution::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub(crate) enum Distribution {
    #[sea_orm(iden = "distributions")]
    Table,
    Id,
    PoolId,
    MinerId,
    WorkerId,
    Reward,
    HashrateExternal,
    HashrateInternal,
    Date,
}

#[derive(DeriveIden)]
pub(crate) enum DistributionIndex {
    #[sea_orm(iden = "idx_distributions_pool_id")]
    PoolId,
    #[sea_orm(iden = "idx_distributions_miner_id")]
    MinerId,
    #[sea_orm(iden = "idx_distributions_worker_id")]
    WorkerId,
    #[sea_orm(iden = "idx_distributions_date")]
    Date,
}
