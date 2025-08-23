use chrono::Utc;
use sea_orm::IntoActiveModel;
use sea_orm::prelude::*;

use crate::entities::miners;
use crate::{Error, Result};

impl miners::Model {
    pub async fn first_or_create(
        db: &impl ConnectionTrait,
        username: &str,
    ) -> Result<miners::Model> {
        let username = username.trim().to_lowercase();
        let exists = miners::Entity::find()
            .filter(miners::Column::Username.eq(&username))
            .one(db)
            .await?;

        match exists {
            Some(miner) => Ok(miner),
            None => {
                let miner = miners::Model {
                    id: Uuid::new_v4(),
                    username,
                    created_at: Utc::now().naive_utc(),
                };

                miner
                    .into_active_model()
                    .insert(db)
                    .await
                    .map_err(Error::from)
            }
        }
    }
}
