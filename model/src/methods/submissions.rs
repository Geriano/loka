use chrono::{NaiveDate, Utc};
use rust_decimal::prelude::FromPrimitive;
use sea_orm::IntoActiveModel;
use sea_orm::Statement;
use sea_orm::prelude::*;

use crate::Result;
use crate::entities::{submissions, workers};

impl submissions::Model {
    #[tracing::instrument(skip(db))]
    pub async fn store(
        db: &impl ConnectionTrait,
        worker: &workers::Model,
        hashes: f64,
        valid: bool,
    ) -> Result<()> {
        let now = Utc::now().naive_utc();
        let submission = submissions::Model {
            id: Uuid::new_v4(),
            pool_id: worker.pool_id,
            worker_id: worker.id,
            hashes: Decimal::from_f64(hashes).unwrap(),
            valid,
            submitted_at: now,
        };

        submission.into_active_model().insert(db).await?;

        Ok(())
    }

    #[tracing::instrument(skip(db))]
    pub async fn distribute(db: &impl ConnectionTrait, date: NaiveDate) -> Result<()> {
        let query = r#"with ss as (
            select
              s.pool_id as pool_id,
              w.miner_id as miner_id,
              s.worker_id as worker_id,
              sum(s.hashes) as hashes_internal,
              0 as hashes_external,
              ($1)::date as date
            from submissions s
            join pools p on p.id = s.pool_id
            join workers w on w.id = s.worker_id
            where (s.submitted_at + (p.offsets * interval '1' minute))::date = ($1)::date
            group by s.pool_id, w.miner_id, s.worker_id, date
          )
          insert into distributions (id, pool_id, miner_id, worker_id, hashes_internal, hashes_external, date)
          select
            gen_random_uuid() as id,
            pool_id,
            miner_id,
            worker_id,
            hashes_internal,
            hashes_external,
            date
          from ss"#;

        let result = db
            .execute_raw(Statement::from_sql_and_values(
                db.get_database_backend(),
                query,
                [date.into()],
            ))
            .await?;

        if result.rows_affected() > 0 {
            let query = r#"delete from submissions s where (
                s.submitted_at + (
                  coalesce((select offsets from pools p where p.id = s.pool_id limit 1), 0) *
                  interval '1' minute
                )
              )::date = ($1)::date"#;

            db.execute_raw(Statement::from_sql_and_values(
                db.get_database_backend(),
                query,
                [date.into()],
            ))
            .await?;
        }

        Ok(())
    }
}
