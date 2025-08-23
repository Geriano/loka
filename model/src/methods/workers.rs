use chrono::Utc;
use sea_orm::IntoActiveModel;
use sea_orm::prelude::*;

use crate::entities::miners;
use crate::entities::workers;
use crate::{Error, Result};

impl workers::Model {
    #[tracing::instrument(skip(db))]
    pub async fn first_or_create(
        db: &impl ConnectionTrait,
        pool_id: Uuid,
        miner: &str,
        worker: &str,
    ) -> Result<(miners::Model, workers::Model)> {
        let miner = miners::Model::first_or_create(db, miner).await?;
        let worker = worker.trim().to_lowercase();

        let exists = workers::Entity::find()
            .filter(workers::Column::PoolId.eq(pool_id))
            .filter(workers::Column::MinerId.eq(miner.id))
            .filter(workers::Column::Name.eq(&worker))
            .one(db)
            .await;

        match exists {
            Ok(Some(mut worker)) => {
                worker.last_seen = Utc::now().naive_utc();
                worker.update(db).await?;

                return Ok((miner, worker));
            }
            Err(e) => {
                if let DbErr::RecordNotFound(_) = &e {
                    //
                } else {
                    return Err(Error::from(e));
                }
            }
            _ => {
                // Worker not found
            }
        };

        let now = Utc::now().naive_utc();
        let worker = workers::Model {
            id: Uuid::new_v4(),
            pool_id,
            miner_id: miner.id,
            name: worker,
            created_at: now,
            last_seen: now,
        };

        let worker = worker.into_active_model().insert(db).await?;

        Ok((miner, worker))
    }

    #[tracing::instrument(skip(db))]
    pub async fn update(&self, db: &impl ConnectionTrait) -> Result<()> {
        self.clone().into_active_model().update(db).await?;

        Ok(())
    }
}
