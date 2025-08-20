use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub pool: PoolConfig,
    pub limiter: LimiterConfig,
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
    /// Pool address (host:port format)
    pub address: String,
    /// Pool name
    pub name: String,
    /// Pool host (deprecated, use address instead)
    pub host: Option<Ipv4Addr>,
    /// Pool port (deprecated, use address instead)
    pub port: Option<u16>,
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

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            pool: PoolConfig {
                address: "127.0.0.1:4444".to_string(),
                name: "default".to_string(),
                host: None,
                port: None,
                username: "default_user".to_string(),
                password: None,
                separator: (".".to_string(), "_".to_string()),
                extranonce: false,
            },
            limiter: LimiterConfig::default(),
        }
    }
}

impl Config {
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
}
