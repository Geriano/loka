use std::time::Duration;

use chrono::NaiveTime;
use loka_model::entities::{pools, prelude::*};
use migration::{Migrator, MigratorTrait};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectOptions, Database, DatabaseConnection, EntityTrait,
    IntoActiveModel, QueryFilter, Set,
};
use uuid::Uuid;

use crate::config::types::PoolConfig;
use crate::error::Result;

/// Database service for managing SeaORM connection and migrations
#[derive(Debug, Clone)]
pub struct DatabaseService {
    pub url: String,
    pub connection: DatabaseConnection,
}

impl DatabaseService {
    /// Create a new database service with automatic migrations
    pub async fn new(database_url: &str) -> Result<Self> {
        let mut opt = ConnectOptions::new(database_url.to_owned());

        // Configure SeaORM's built-in connection pool
        opt.max_connections(100)
            .min_connections(5)
            .connect_timeout(Duration::from_secs(1))
            .idle_timeout(Duration::from_secs(8))
            .sqlx_logging(false);

        let connection =
            Database::connect(opt)
                .await
                .map_err(|e| crate::error::StratumError::Database {
                    message: format!("Failed to connect to database: {e}"),
                    source: Some(Box::new(e)),
                })?;

        Ok(DatabaseService {
            url: database_url.to_owned(),
            connection,
        })
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    /// Create a new database service with custom connection options
    pub async fn new_with_options(
        database_url: &str,
        max_connections: u32,
        min_connections: u32,
    ) -> Result<Self> {
        let mut opt = ConnectOptions::new(database_url.to_owned());

        opt.max_connections(max_connections)
            .min_connections(min_connections)
            .connect_timeout(Duration::from_secs(8))
            .idle_timeout(Duration::from_secs(8));

        let connection =
            Database::connect(opt)
                .await
                .map_err(|e| crate::error::StratumError::Database {
                    message: format!("Failed to connect to database: {e}"),
                    source: Some(Box::new(e)),
                })?;

        tracing::info!(
            "Database connected with custom options (max: {}, min: {})",
            max_connections,
            min_connections
        );

        Ok(DatabaseService {
            url: database_url.to_owned(),
            connection,
        })
    }

    /// Run migrations manually
    pub async fn migrate(&self) -> Result<()> {
        Migrator::up(&self.connection, None).await.map_err(|e| {
            crate::error::StratumError::Database {
                message: format!("Failed to run migrations: {e}"),
                source: Some(Box::new(e)),
            }
        })?;

        tracing::info!("Migrations completed successfully");
        Ok(())
    }

    /// Health check for database connection
    pub async fn health_check(&self) -> Result<bool> {
        self.connection
            .ping()
            .await
            .map_err(|e| crate::error::StratumError::Database {
                message: format!("Failed to ping database: {e}"),
                source: Some(Box::new(e)),
            })?;

        Ok(true)
    }

    /// Get active pool configuration by ID
    pub async fn get_pool_config(&self, pool_id: Uuid) -> Result<Option<PoolConfig>> {
        let pool = Pools::find_by_id(pool_id)
            .filter(pools::Column::Active.eq(true))
            .one(&self.connection)
            .await
            .map_err(|e| crate::error::StratumError::Database {
                message: format!("Failed to fetch pool config: {e}"),
                source: Some(Box::new(e)),
            })?;

        match pool {
            Some(p) => Ok(Some(self.convert_pool_model_to_config(p)?)),
            None => Ok(None),
        }
    }

    /// Get default active pool configuration (first active pool)
    pub async fn get_default_pool_config(&self) -> Result<Option<PoolConfig>> {
        let pool = Pools::find()
            .filter(pools::Column::Active.eq(true))
            .one(&self.connection)
            .await
            .map_err(|e| crate::error::StratumError::Database {
                message: format!("Failed to fetch default pool config: {e}"),
                source: Some(Box::new(e)),
            })?;

        match pool {
            Some(p) => Ok(Some(self.convert_pool_model_to_config(p)?)),
            None => Ok(None),
        }
    }

    /// Get pool configuration by name
    pub async fn get_pool_config_by_name(&self, name: &str) -> Result<Option<PoolConfig>> {
        let pool = Pools::find()
            .filter(pools::Column::Name.eq(name))
            .filter(pools::Column::Active.eq(true))
            .one(&self.connection)
            .await
            .map_err(|e| crate::error::StratumError::Database {
                message: format!("Failed to fetch pool config by name: {e}"),
                source: Some(Box::new(e)),
            })?;

        match pool {
            Some(p) => Ok(Some(self.convert_pool_model_to_config(p)?)),
            None => Ok(None),
        }
    }

    /// Get pool configuration by host and port
    pub async fn get_pool_config_by_target(
        &self,
        host: &str,
        port: u16,
    ) -> Result<Option<PoolConfig>> {
        let pool = Pools::find()
            .filter(pools::Column::Host.eq(host))
            .filter(pools::Column::Port.eq(port as i16))
            .filter(pools::Column::Active.eq(true))
            .one(&self.connection)
            .await
            .map_err(|e| crate::error::StratumError::Database {
                message: format!(
                    "Failed to fetch pool config by target {host}:{port}: {e}"
                ),
                source: Some(Box::new(e)),
            })?;

        match pool {
            Some(p) => Ok(Some(self.convert_pool_model_to_config(p)?)),
            None => Ok(None),
        }
    }

    /// List all active pool configurations
    pub async fn list_active_pool_configs(&self) -> Result<Vec<PoolConfig>> {
        let pools = Pools::find()
            .filter(pools::Column::Active.eq(true))
            .all(&self.connection)
            .await
            .map_err(|e| crate::error::StratumError::Database {
                message: format!("Failed to fetch active pool configs: {e}"),
                source: Some(Box::new(e)),
            })?;

        let mut configs = Vec::new();
        for pool in pools {
            configs.push(self.convert_pool_model_to_config(pool)?);
        }

        Ok(configs)
    }

    /// Convert database pool model to PoolConfig
    fn convert_pool_model_to_config(&self, pool: pools::Model) -> Result<PoolConfig> {
        Ok(PoolConfig {
            id: pool.id,
            address: format!("{}:{}", pool.host, pool.port),
            name: pool.name,
            username: pool.username,
            password: pool.password,
            separator: (pool.sep1, pool.sep2),
            extranonce: pool.offsets > 0,
        })
    }

    /// Create new pool configuration in database
    pub async fn create_pool_config(&self, config: &PoolConfig) -> Result<Uuid> {
        let (host, port) = config.address.split_once(':').unwrap();

        let new_pool = pools::ActiveModel {
            id: Set(config.id),
            name: Set(config.name.clone()),
            bind: Set(3333), // Default stratum port for incoming connections
            host: Set(host.to_string()),
            port: Set(port.parse::<i16>().unwrap()),
            username: Set(config.username.clone()),
            password: Set(config.password.clone()),
            sep1: Set(config.separator.0.clone()),
            sep2: Set(config.separator.1.clone()),
            offsets: Set(if config.extranonce { 1 } else { 0 }),
            difficulty: Set(1024.0), // Default difficulty
            settlement: Set(NaiveTime::from_hms_opt(0, 0, 0).unwrap()), // Default settlement time
            active: Set(true),
        };

        new_pool.insert(&self.connection).await.map_err(|e| {
            crate::error::StratumError::Database {
                message: format!("Failed to create pool config: {e}"),
                source: Some(Box::new(e)),
            }
        })?;

        tracing::info!("Pool configuration created with ID: {}", config.id);
        Ok(config.id)
    }

    /// Deactivate pool configuration
    pub async fn deactivate_pool_config(&self, pool_id: Uuid) -> Result<()> {
        let existing_pool = Pools::find_by_id(pool_id)
            .one(&self.connection)
            .await
            .map_err(|e| crate::error::StratumError::Database {
                message: format!("Failed to fetch pool for deactivation: {e}"),
                source: Some(Box::new(e)),
            })?;

        match existing_pool {
            Some(pool) => {
                let mut active_model = pool.into_active_model();
                active_model.active = Set(false);

                active_model.update(&self.connection).await.map_err(|e| {
                    crate::error::StratumError::Database {
                        message: format!("Failed to deactivate pool config: {e}"),
                        source: Some(Box::new(e)),
                    }
                })?;

                tracing::info!("Pool configuration deactivated for pool ID: {}", pool_id);
                Ok(())
            }
            None => Err(crate::error::StratumError::Internal {
                message: format!("Pool with ID {pool_id} not found"),
            }),
        }
    }
}
