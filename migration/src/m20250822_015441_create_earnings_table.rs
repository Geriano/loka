use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::DatabaseBackend;

use crate::m20220101_000001_create_pools_table::Pool;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Earning::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Earning::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Earning::PoolId).uuid().not_null())
                    .col(ColumnDef::new(Earning::Reward).big_unsigned().not_null())
                    .col(
                        ColumnDef::new(Earning::HashrateExternal)
                            .decimal_len(3, 32)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Earning::HashrateInternal)
                            .decimal_len(3, 3)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Earning::Date).date().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(EarningIndex::PoolId.to_string())
                    .table(Earning::Table)
                    .col(Earning::PoolId)
                    .take(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(EarningIndex::Date.to_string())
                    .table(Earning::Table)
                    .col(Earning::Date)
                    .take(),
            )
            .await?;

        if manager.get_database_backend() != DatabaseBackend::Sqlite {
            manager
                .create_foreign_key(
                    ForeignKey::create()
                        .from(Earning::Table, Earning::PoolId)
                        .to(Pool::Table, Pool::Id)
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
            .drop_table(Table::drop().table(Earning::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub(crate) enum Earning {
    #[sea_orm(iden = "earnings")]
    Table,
    Id,
    PoolId,
    Reward,
    HashrateExternal,
    HashrateInternal,
    Date,
}

#[derive(DeriveIden)]
pub(crate) enum EarningIndex {
    #[sea_orm(iden = "idx_earnings_pool_id")]
    PoolId,
    #[sea_orm(iden = "idx_earnings_date")]
    Date,
}
