use sea_orm::prelude::*;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub pool: PoolConfig,
    pub limiter: LimiterConfig,
    pub database: DatabaseConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server bind address (default: 0.0.0.0:3333)
    pub bind_address: SocketAddr,
    /// Maximum concurrent connections
    pub max_connections: usize,
    /// Connection idle timeout
    pub idle_timeout: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    /// Pool unique identifier (UUID)
    pub id: Uuid,
    /// Pool address (host:port format)
    pub address: String,
    /// Pool name
    pub name: String,
    /// Pool username
    pub username: String,
    /// Pool password
    pub password: Option<String>,
    /// Separator for authentication override
    pub separator: (String, String),
    /// Extranonce
    pub extranonce: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimiterConfig {
    /// Maximum number of connections (default 1000)
    pub connections: usize,
    /// Job expiration time (default 10 minutes)
    pub jobs: Duration,
    /// Submissions expiration time (default 2 days)
    /// It will be used for cleanup auth when last activity is older than this duration
    pub submissions: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database connection URL (PostgreSQL format)
    pub url: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0:3333".parse().unwrap(),
            max_connections: 1000,
            idle_timeout: Duration::from_secs(300), // 5 minutes
        }
    }
}

impl Default for LimiterConfig {
    fn default() -> Self {
        Self {
            connections: 1000,
            jobs: Duration::from_secs(10 * 60), // 10 minutes
            submissions: Duration::from_secs(2 * 24 * 60 * 60), // 2 days
        }
    }
}

impl Config {
    pub async fn load(
        database_service: &crate::services::database::DatabaseService,
    ) -> crate::Result<Self> {
        let pool = loka_model::entities::pools::Entity::find()
            .filter(loka_model::entities::pools::Column::Active.eq(true))
            .one(&database_service.connection)
            .await;

        match pool {
            Ok(Some(pool)) => Ok(Self {
                server: ServerConfig::default(),
                pool: PoolConfig {
                    id: pool.id,
                    address: format!("{}:{}", pool.host, pool.port),
                    name: pool.name,
                    username: pool.username,
                    password: pool.password,
                    separator: (pool.sep1, pool.sep2),
                    // TODO: Add extranonce support
                    extranonce: false,
                },
                limiter: LimiterConfig::default(),
                database: DatabaseConfig {
                    url: database_service.url().to_string(),
                },
            }),
            Err(DbErr::RecordNotFound(_)) | Ok(None) => Err(crate::error::StratumError::Internal {
                message: "No active pool configuration found in database".to_string(),
            }),
            Err(e) => Err(crate::error::StratumError::Internal {
                message: format!("Failed to load pool configuration from database: {}", e),
            }),
        }
    }

    /// Load configuration from file
    pub fn load_from_file<P: AsRef<std::path::Path>>(path: P) -> crate::Result<Self> {
        let content =
            std::fs::read_to_string(path).map_err(|e| crate::error::StratumError::Internal {
                message: format!("Failed to read config file: {}", e),
            })?;
        let config: Config =
            toml::from_str(&content).map_err(|e| crate::error::StratumError::Internal {
                message: format!("Failed to parse config: {}", e),
            })?;
        Ok(config)
    }

    /// Get list of all available pools from database
    pub async fn list_available_pools(
        database_service: &crate::services::database::DatabaseService,
    ) -> crate::Result<Vec<PoolConfig>> {
        database_service.list_active_pool_configs().await
    }
}
