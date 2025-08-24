use std::net::Ipv4Addr;

use chrono::NaiveTime;
use sea_orm::Condition;
use sea_orm::IntoActiveModel;
use sea_orm::prelude::*;

use crate::entities::pools;
use crate::{Error, Result, ValidationError};

impl pools::Model {
    #[tracing::instrument(skip(db))]
    pub async fn store(
        db: &impl ConnectionTrait,
        name: &str,
        bind: u16,
        host: Ipv4Addr,
        port: u16,
        username: &str,
        password: Option<&str>,
        sep1: &str,
        sep2: &str,
        settlement: NaiveTime,
        offsets: Option<u16>,
        difficulty: Option<f32>,
    ) -> Result<pools::Model> {
        let mut errors = Vec::new();
        let name = name.trim().to_lowercase();

        if name.is_empty() {
            errors.push(ValidationError {
                field: "name".to_string(),
                message: "Name is required".to_string(),
            });
        }

        let exists = pools::Entity::find()
            .filter(
                Condition::any()
                    .add(pools::Column::Name.eq(&name))
                    .add(pools::Column::Bind.eq(bind as i16)),
            )
            .one(db)
            .await;

        match exists {
            Err(e) => match e {
                DbErr::RecordNotFound(_) => {}
                e => return Err(e.into()),
            },
            Ok(_) => {
                errors.push(ValidationError {
                    field: "name".to_string(),
                    message: "Name already exists".to_string(),
                });
            }
        };

        if !errors.is_empty() {
            return Err(Error::ValidationError { errors });
        }

        let model = pools::Model {
            id: Uuid::new_v4(),
            name,
            bind: bind as i16,
            host: host.to_string(),
            port: port as i16,
            username: username.to_string(),
            password: password.map(|p| p.to_string()),
            sep1: sep1.to_string(),
            sep2: sep2.to_string(),
            offsets: offsets.unwrap_or(0) as i16,
            difficulty: difficulty.unwrap_or(0.0),
            settlement,
            active: true,
        };

        model
            .into_active_model()
            .insert(db)
            .await
            .map_err(Error::from)
    }
}
