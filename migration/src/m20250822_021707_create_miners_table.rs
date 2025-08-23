use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Miner::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Miner::Id).uuid().not_null().primary_key())
                    .col(
                        ColumnDef::new(Miner::Username)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Miner::CreatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT (now())"),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Miner::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub(crate) enum Miner {
    #[sea_orm(iden = "miners")]
    Table,
    Id,
    Username,
    CreatedAt,
}
