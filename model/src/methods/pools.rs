use std::net::Ipv4Addr;

use chrono::NaiveTime;
use sea_orm::Condition;
use sea_orm::IntoActiveModel;
use sea_orm::prelude::*;

use crate::entities::pools;
use crate::{Error, Result, ValidationError};

/// Parameters for creating a new pool
#[derive(Debug)]
pub struct CreatePoolParams<'a> {
    pub name: &'a str,
    pub bind: u16,
    pub host: Ipv4Addr,
    pub port: u16,
    pub username: &'a str,
    pub password: Option<&'a str>,
    pub sep1: &'a str,
    pub sep2: &'a str,
    pub settlement: NaiveTime,
    pub offsets: Option<u16>,
    pub difficulty: Option<f32>,
}

impl pools::Model {
    #[tracing::instrument(skip(db, params))]
    pub async fn store(
        db: &impl ConnectionTrait,
        params: CreatePoolParams<'_>,
    ) -> Result<pools::Model> {
        let mut errors = Vec::new();
        let name = params.name.trim().to_lowercase();

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
                    .add(pools::Column::Bind.eq(params.bind as i16)),
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
            bind: params.bind as i16,
            host: params.host.to_string(),
            port: params.port as i16,
            username: params.username.to_string(),
            password: params.password.map(|p| p.to_string()),
            sep1: params.sep1.to_string(),
            sep2: params.sep2.to_string(),
            offsets: params.offsets.unwrap_or(0) as i16,
            difficulty: params.difficulty.unwrap_or(0.0),
            settlement: params.settlement,
            active: true,
        };

        model
            .into_active_model()
            .insert(db)
            .await
            .map_err(Error::from)
    }
}
