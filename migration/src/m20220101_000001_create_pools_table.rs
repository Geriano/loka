use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Pool::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Pool::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Pool::Name).string().not_null().unique_key())
                    .col(ColumnDef::new(Pool::Bind).small_unsigned().not_null())
                    .col(ColumnDef::new(Pool::Host).string_len(16).not_null())
                    .col(ColumnDef::new(Pool::Port).small_unsigned().not_null())
                    .col(ColumnDef::new(Pool::Username).string().not_null())
                    .col(
                        ColumnDef::new(Pool::Password)
                            .string()
                            .null()
                            .extra("DEFAULT NULL"),
                    )
                    .col(ColumnDef::new(Pool::Sep1).string_len(4).not_null())
                    .col(ColumnDef::new(Pool::Sep2).string_len(4).not_null())
                    .col(
                        ColumnDef::new(Pool::Offsets)
                            .small_unsigned()
                            .not_null()
                            .extra("DEFAULT 0")
                            .comment(
                                "minute offset for pool hashrate calculation (default UTC = 0)",
                            ),
                    )
                    .col(
                        ColumnDef::new(Pool::Difficulty)
                            .float()
                            .not_null()
                            .comment("minimum pool difficulty"),
                    )
                    .col(ColumnDef::new(Pool::Settlement).time().not_null())
                    .col(
                        ColumnDef::new(Pool::Active)
                            .boolean()
                            .not_null()
                            .extra("DEFAULT TRUE"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(PoolIndex::Name.to_string())
                    .table(Pool::Table)
                    .col(Pool::Active)
                    .take(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(PoolIndex::Bind.to_string())
                    .table(Pool::Table)
                    .col(Pool::Active)
                    .take(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(PoolIndex::Offsets.to_string())
                    .table(Pool::Table)
                    .col(Pool::Active)
                    .take(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(PoolIndex::Active.to_string())
                    .table(Pool::Table)
                    .col(Pool::Active)
                    .take(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Pool::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub(crate) enum Pool {
    #[sea_orm(iden = "pools")]
    Table,
    Id,
    Name,
    Bind,
    Host,
    Port,
    Username,
    Password,
    Sep1,
    Sep2,
    Offsets,
    Difficulty,
    Settlement,
    Active,
}

#[derive(DeriveIden)]
pub(crate) enum PoolIndex {
    #[sea_orm(iden = "idx_pools_name")]
    Name,
    #[sea_orm(iden = "idx_pools_bind")]
    Bind,
    #[sea_orm(iden = "idx_pools_offsets")]
    Offsets,
    #[sea_orm(iden = "idx_pools_active")]
    Active,
}
