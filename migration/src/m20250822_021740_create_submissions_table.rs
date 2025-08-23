use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::DatabaseBackend;

use crate::m20220101_000001_create_pools_table::Pool;
use crate::m20250822_021712_create_workers_table::Worker;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Submission::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Submission::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Submission::PoolId).uuid().not_null())
                    .col(ColumnDef::new(Submission::WorkerId).uuid().not_null())
                    .col(
                        ColumnDef::new(Submission::Hashes)
                            .decimal_len(3, 32)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Submission::Valid).boolean().not_null())
                    .col(
                        ColumnDef::new(Submission::SubmittedAt)
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
                    .name(SubmissionIndex::PoolId.to_string())
                    .table(Submission::Table)
                    .col(Submission::PoolId)
                    .take(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(SubmissionIndex::WorkerId.to_string())
                    .table(Submission::Table)
                    .col(Submission::WorkerId)
                    .take(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(SubmissionIndex::Valid.to_string())
                    .table(Submission::Table)
                    .col(Submission::Valid)
                    .take(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(SubmissionIndex::SubmittedAt.to_string())
                    .table(Submission::Table)
                    .col(Submission::SubmittedAt)
                    .take(),
            )
            .await?;

        if manager.get_database_backend() != DatabaseBackend::Sqlite {
            manager
                .create_foreign_key(
                    ForeignKey::create()
                        .from(Submission::Table, Submission::PoolId)
                        .to(Pool::Table, Pool::Id)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                        .take(),
                )
                .await?;

            manager
                .create_foreign_key(
                    ForeignKey::create()
                        .from(Submission::Table, Submission::WorkerId)
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
            .drop_table(Table::drop().table(Submission::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub(crate) enum Submission {
    #[sea_orm(iden = "submissions")]
    Table,
    Id,
    PoolId,
    WorkerId,
    Hashes,
    Valid,
    SubmittedAt,
}

#[derive(DeriveIden)]
pub(crate) enum SubmissionIndex {
    #[sea_orm(iden = "idx_submissions_pool_id")]
    PoolId,
    #[sea_orm(iden = "idx_submissions_worker_id")]
    WorkerId,
    #[sea_orm(iden = "idx_submissions_valid")]
    Valid,
    #[sea_orm(iden = "idx_submissions_submitted_at")]
    SubmittedAt,
}
