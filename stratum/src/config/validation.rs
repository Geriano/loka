use crate::error::{ConfigError, Result};
use crate::config::types::Config;
use std::time::Duration;

impl Config {
    pub fn validate(&self) -> Result<()> {
        // Validate server configuration
        if self.server.max_connections == 0 {
            return Err(ConfigError::InvalidConnectionLimit { limit: self.server.max_connections }.into());
        }
        
        // Validate pool configuration
        if self.pool.address.trim().is_empty() {
            return Err(ConfigError::MissingField { 
                field: "pool.address".to_string() 
            }.into());
        }

        // Validate legacy pool configuration if present
        if let Some(port) = self.pool.port {
            if port == 0 {
                return Err(ConfigError::InvalidPort { port }.into());
            }
        }
        
        if self.pool.name.trim().is_empty() {
            return Err(ConfigError::MissingField { 
                field: "pool.name".to_string() 
            }.into());
        }
        
        if self.pool.username.trim().is_empty() {
            return Err(ConfigError::MissingField { 
                field: "pool.username".to_string() 
            }.into());
        }
        
        // Validate limiter configuration
        if self.limiter.connections == 0 {
            return Err(ConfigError::InvalidConnectionLimit { limit: self.limiter.connections }.into());
        }
        
        if self.limiter.jobs == Duration::ZERO {
            return Err(ConfigError::InvalidDuration { 
                field: "limiter.jobs".to_string(),
                duration: self.limiter.jobs,
            }.into());
        }
        
        if self.limiter.submissions == Duration::ZERO {
            return Err(ConfigError::InvalidDuration {
                field: "limiter.submissions".to_string(), 
                duration: self.limiter.submissions,
            }.into());
        }
        
        // Validate separator format
        if self.pool.separator.0.is_empty() || self.pool.separator.1.is_empty() {
            return Err(ConfigError::MissingField { 
                field: "pool.separator".to_string() 
            }.into());
        }
        
        Ok(())
    }
    
    pub fn load_from_env() -> Result<Self> {
        // For now, return default config
        // TODO: Implement environment variable loading
        let config = Self::default();
        config.validate()?;
        Ok(config)
    }
}