pub use sea_orm_migration::prelude::*;

mod m20220101_000001_create_pools_table;
mod m20250822_015441_create_earnings_table;
mod m20250822_021707_create_miners_table;
mod m20250822_021712_create_workers_table;
mod m20250822_021740_create_submissions_table;
mod m20250822_023619_create_distributions_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_pools_table::Migration),
            Box::new(m20250822_015441_create_earnings_table::Migration),
            Box::new(m20250822_021707_create_miners_table::Migration),
            Box::new(m20250822_021712_create_workers_table::Migration),
            Box::new(m20250822_021740_create_submissions_table::Migration),
            Box::new(m20250822_023619_create_distributions_table::Migration),
        ]
    }
}
