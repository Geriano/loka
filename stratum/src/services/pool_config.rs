use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::time::{MissedTickBehavior, interval};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::config::types::PoolConfig;
use crate::error::Result;
use crate::services::database::DatabaseService;

/// Pool configuration service with caching and hot-reload capabilities
#[derive(Debug, Clone)]
pub struct PoolConfigService {
    inner: Arc<PoolConfigServiceInner>,
}

#[derive(Debug)]
struct PoolConfigServiceInner {
    database: Arc<DatabaseService>,
    cache: RwLock<PoolConfigCache>,
    config: PoolConfigServiceConfig,
}

#[derive(Debug)]
struct PoolConfigCache {
    pools: HashMap<Uuid, CachedPoolConfig>,
    default_pool_id: Option<Uuid>,
    last_refresh: Instant,
}

#[derive(Debug, Clone)]
struct CachedPoolConfig {
    config: PoolConfig,
    cached_at: Instant,
}

#[derive(Debug, Clone)]
pub struct PoolConfigServiceConfig {
    /// Cache expiration time
    pub cache_ttl: Duration,
    /// How often to refresh the cache
    pub refresh_interval: Duration,
    /// Enable hot-reload from database
    pub hot_reload_enabled: bool,
}

impl Default for PoolConfigServiceConfig {
    fn default() -> Self {
        Self {
            cache_ttl: Duration::from_secs(300),       // 5 minutes
            refresh_interval: Duration::from_secs(60), // 1 minute
            hot_reload_enabled: true,
        }
    }
}

impl PoolConfigService {
    /// Create a new pool configuration service
    pub fn new(database: Arc<DatabaseService>, config: PoolConfigServiceConfig) -> Self {
        let inner = Arc::new(PoolConfigServiceInner {
            database,
            cache: RwLock::new(PoolConfigCache {
                pools: HashMap::new(),
                default_pool_id: None,
                last_refresh: Instant::now(),
            }),
            config,
        });

        Self { inner }
    }

    /// Start the background refresh task
    pub async fn start_background_refresh(&self) -> tokio::task::JoinHandle<()> {
        let inner = Arc::clone(&self.inner);

        tokio::spawn(async move {
            if !inner.config.hot_reload_enabled {
                return;
            }

            let mut interval = interval(inner.config.refresh_interval);
            interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

            loop {
                interval.tick().await;

                if let Err(e) = inner.refresh_cache().await {
                    error!("Failed to refresh pool configuration cache: {}", e);
                } else {
                    info!("Pool configuration cache refreshed successfully");
                }
            }
        })
    }

    /// Get pool configuration by ID (with caching)
    pub async fn get_pool_config(&self, pool_id: Uuid) -> Result<Option<PoolConfig>> {
        // Try to get from cache first
        if let Some(cached) = self.get_from_cache(pool_id) {
            return Ok(Some(cached));
        }

        // Cache miss - load from database
        match self.inner.database.get_pool_config(pool_id).await? {
            Some(config) => {
                self.cache_pool_config(pool_id, config.clone());
                Ok(Some(config))
            }
            None => Ok(None),
        }
    }

    /// Get default pool configuration (with caching)
    pub async fn get_default_pool_config(&self) -> Result<Option<PoolConfig>> {
        // Check if we have a cached default pool
        if let Some(default_id) = self.get_default_pool_id() {
            if let Some(config) = self.get_from_cache(default_id) {
                return Ok(Some(config));
            }
        }

        // Cache miss - load from database
        match self.inner.database.get_default_pool_config().await? {
            Some(config) => {
                self.set_default_pool_id(config.id);
                self.cache_pool_config(config.id, config.clone());
                Ok(Some(config))
            }
            None => Ok(None),
        }
    }

    /// Get pool configuration by name (with caching)
    pub async fn get_pool_config_by_name(&self, name: &str) -> Result<Option<PoolConfig>> {
        // For name-based lookup, we need to check all cached configs
        if let Some(config) = self.find_in_cache_by_name(name) {
            return Ok(Some(config));
        }

        // Cache miss - load from database
        match self.inner.database.get_pool_config_by_name(name).await? {
            Some(config) => Ok(Some(config)),
            None => Ok(None),
        }
    }

    /// Get pool configuration by host:port target (with caching)
    pub async fn get_pool_config_by_target(&self, target: &str) -> Result<Option<PoolConfig>> {
        // Parse target into host and port
        let (host, port) = match Self::parse_target(target) {
            Some((h, p)) => (h, p),
            None => return Ok(None),
        };

        // For target-based lookup, check cached configs first
        if let Some(config) = self.find_in_cache_by_target(&host, port) {
            return Ok(Some(config));
        }

        // Cache miss - load from database
        match self
            .inner
            .database
            .get_pool_config_by_target(&host, port)
            .await?
        {
            Some(config) => {
                self.cache_pool_config(config.id, config.clone());
                Ok(Some(config))
            }
            None => Ok(None),
        }
    }

    /// List all active pool configurations (always from database)
    pub async fn list_active_pool_configs(&self) -> Result<Vec<PoolConfig>> {
        let configs = self.inner.database.list_active_pool_configs().await?;

        // Update cache with fresh data
        for config in &configs {
            self.cache_pool_config(config.id, config.clone());
        }

        Ok(configs)
    }

    /// Invalidate cache for a specific pool
    pub fn invalidate_pool(&self, pool_id: Uuid) {
        if let Ok(mut cache) = self.inner.cache.write() {
            cache.pools.remove(&pool_id);
            if cache.default_pool_id == Some(pool_id) {
                cache.default_pool_id = None;
            }
            info!("Invalidated cache for pool ID: {}", pool_id);
        }
    }

    /// Invalidate entire cache
    pub fn invalidate_all(&self) {
        if let Ok(mut cache) = self.inner.cache.write() {
            cache.pools.clear();
            cache.default_pool_id = None;
            cache.last_refresh = Instant::now();
            info!("Invalidated all pool configuration cache");
        }
    }

    /// Manually refresh cache from database
    pub async fn refresh_cache(&self) -> Result<()> {
        self.inner.refresh_cache().await
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> PoolConfigCacheStats {
        if let Ok(cache) = self.inner.cache.read() {
            PoolConfigCacheStats {
                cached_pools: cache.pools.len(),
                has_default_pool: cache.default_pool_id.is_some(),
                last_refresh: cache.last_refresh,
                cache_age: cache.last_refresh.elapsed(),
            }
        } else {
            PoolConfigCacheStats {
                cached_pools: 0,
                has_default_pool: false,
                last_refresh: Instant::now(),
                cache_age: Duration::from_secs(0),
            }
        }
    }

    // Private helper methods
    fn get_from_cache(&self, pool_id: Uuid) -> Option<PoolConfig> {
        if let Ok(cache) = self.inner.cache.read() {
            if let Some(cached) = cache.pools.get(&pool_id) {
                if cached.cached_at.elapsed() < self.inner.config.cache_ttl {
                    return Some(cached.config.clone());
                }
            }
        }
        None
    }

    fn cache_pool_config(&self, pool_id: Uuid, config: PoolConfig) {
        if let Ok(mut cache) = self.inner.cache.write() {
            cache.pools.insert(
                pool_id,
                CachedPoolConfig {
                    config,
                    cached_at: Instant::now(),
                },
            );
        }
    }

    fn get_default_pool_id(&self) -> Option<Uuid> {
        if let Ok(cache) = self.inner.cache.read() {
            cache.default_pool_id
        } else {
            None
        }
    }

    fn set_default_pool_id(&self, pool_id: Uuid) {
        if let Ok(mut cache) = self.inner.cache.write() {
            cache.default_pool_id = Some(pool_id);
        }
    }

    fn find_in_cache_by_name(&self, name: &str) -> Option<PoolConfig> {
        if let Ok(cache) = self.inner.cache.read() {
            for cached in cache.pools.values() {
                if cached.cached_at.elapsed() < self.inner.config.cache_ttl
                    && cached.config.name == name
                {
                    return Some(cached.config.clone());
                }
            }
        }
        None
    }

    fn find_in_cache_by_target(&self, host: &str, port: u16) -> Option<PoolConfig> {
        let address = format!("{host}:{port}");

        if let Ok(cache) = self.inner.cache.read() {
            for cached in cache.pools.values() {
                if cached.cached_at.elapsed() < self.inner.config.cache_ttl
                    && cached.config.address == address
                {
                    return Some(cached.config.clone());
                }
            }
        }
        None
    }

    /// Check if a target host:port combination is valid for a known pool
    pub async fn is_valid_target(&self, host: &str, port: u16) -> bool {
        let target = format!("{host}:{port}");

        // Check if we have a pool configuration for this target
        match self.get_pool_config_by_target(&target).await {
            Ok(Some(_)) => true,
            Ok(None) => {
                // Also try to validate against active pools
                match self.list_active_pool_configs().await {
                    Ok(configs) => configs.iter().any(|config| {
                        Self::parse_target(&config.address)
                            .map(|(h, p)| h == host && p == port)
                            .unwrap_or(false)
                    }),
                    Err(_) => false,
                }
            }
            Err(_) => false,
        }
    }

    /// Parse target string into host and port components
    fn parse_target(target: &str) -> Option<(String, u16)> {
        if let Some(colon_pos) = target.rfind(':') {
            let (host, port_str) = target.split_at(colon_pos);
            let port_str = &port_str[1..]; // Remove the colon

            if let Ok(port) = port_str.parse::<u16>() {
                if !host.is_empty() {
                    return Some((host.to_string(), port));
                }
            }
        }
        None
    }
}

impl PoolConfigServiceInner {
    async fn refresh_cache(&self) -> Result<()> {
        match self.database.list_active_pool_configs().await {
            Ok(configs) => {
                if let Ok(mut cache) = self.cache.write() {
                    // Clear expired entries and update with fresh data
                    let now = Instant::now();
                    cache.pools.retain(|_, cached| {
                        now.duration_since(cached.cached_at) < self.config.cache_ttl
                    });

                    // Add/update all active pools
                    for config in configs {
                        let pool_id = config.id;
                        cache.pools.insert(
                            pool_id,
                            CachedPoolConfig {
                                config,
                                cached_at: now,
                            },
                        );

                        // Set default pool if we don't have one
                        if cache.default_pool_id.is_none() {
                            cache.default_pool_id = Some(pool_id);
                        }
                    }

                    cache.last_refresh = now;
                    info!(
                        "Pool configuration cache refreshed with {} pools",
                        cache.pools.len()
                    );
                }
                Ok(())
            }
            Err(e) => {
                warn!("Failed to refresh pool config cache: {}", e);
                Err(e)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct PoolConfigCacheStats {
    pub cached_pools: usize,
    pub has_default_pool: bool,
    pub last_refresh: Instant,
    pub cache_age: Duration,
}

impl std::fmt::Display for PoolConfigCacheStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PoolConfigCache(pools: {}, default: {}, age: {:?})",
            self.cached_pools, self.has_default_pool, self.cache_age
        )
    }
}
