use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use tokio::net::TcpStream;
use tokio::sync::{Mutex, RwLock, Semaphore};
use tokio::time::{sleep, timeout};
use tracing::{debug, info, warn};

use crate::error::{Result, StratumError};

/// Configuration for connection pool
#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub min_connections: usize,
    pub max_connections: usize,
    pub connection_timeout: Duration,
    pub idle_timeout: Duration,
    pub validation_interval: Duration,
    pub retry_attempts: u32,
    pub retry_delay: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            min_connections: 2,
            max_connections: 10,
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(300), // 5 minutes
            validation_interval: Duration::from_secs(60), // 1 minute
            retry_attempts: 3,
            retry_delay: Duration::from_secs(1),
        }
    }
}

/// Pooled connection wrapper
#[derive(Debug)]
pub struct PooledConnection {
    connection: Option<TcpStream>,
    created_at: Instant,
    last_used: Instant,
    use_count: AtomicU64,
    is_valid: bool,
    remote_addr: SocketAddr,
}

impl PooledConnection {
    pub fn new(connection: TcpStream, remote_addr: SocketAddr) -> Self {
        let now = Instant::now();
        Self {
            connection: Some(connection),
            created_at: now,
            last_used: now,
            use_count: AtomicU64::new(0),
            is_valid: true,
            remote_addr,
        }
    }

    pub fn take_connection(&mut self) -> Option<TcpStream> {
        self.last_used = Instant::now();
        self.use_count.fetch_add(1, Ordering::Relaxed);
        self.connection.take()
    }

    pub fn return_connection(&mut self, connection: TcpStream) {
        self.connection = Some(connection);
        self.last_used = Instant::now();
    }

    pub fn is_expired(&self, idle_timeout: Duration) -> bool {
        self.last_used.elapsed() > idle_timeout
    }

    pub fn is_available(&self) -> bool {
        self.connection.is_some() && self.is_valid
    }

    pub fn invalidate(&mut self) {
        self.is_valid = false;
        if let Some(conn) = self.connection.take() {
            drop(conn);
        }
    }

    pub fn get_stats(&self) -> ConnectionStats {
        ConnectionStats {
            created_at: self.created_at,
            last_used: self.last_used,
            use_count: self.use_count.load(Ordering::Relaxed),
            age: self.created_at.elapsed(),
            idle_time: self.last_used.elapsed(),
            is_valid: self.is_valid,
            remote_addr: self.remote_addr,
        }
    }
}

/// Connection pool for managing upstream connections
pub struct ConnectionPool {
    config: PoolConfig,
    target_addr: SocketAddr,
    connections: Arc<Mutex<VecDeque<PooledConnection>>>,
    active_connections: AtomicUsize,
    total_created: AtomicU64,
    total_reused: AtomicU64,
    total_errors: AtomicU64,
    semaphore: Arc<Semaphore>,
    is_running: Arc<AtomicBool>,
}

impl ConnectionPool {
    pub fn new(target_addr: SocketAddr, config: PoolConfig) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.max_connections));

        Self {
            config: config.clone(),
            target_addr,
            connections: Arc::new(Mutex::new(VecDeque::with_capacity(config.max_connections))),
            active_connections: AtomicUsize::new(0),
            total_created: AtomicU64::new(0),
            total_reused: AtomicU64::new(0),
            total_errors: AtomicU64::new(0),
            semaphore,
            is_running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start the connection pool with background maintenance
    pub async fn start(&self) -> Result<()> {
        if self
            .is_running
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            return Err(StratumError::Internal {
                message: "Connection pool is already running".to_string(),
            });
        }

        // Pre-populate with minimum connections
        self.ensure_minimum_connections().await?;

        // Start background maintenance task
        let pool_clone = Arc::new(self.clone_for_maintenance());
        tokio::spawn(async move {
            pool_clone.maintenance_loop().await;
        });

        info!("Connection pool started for {}", self.target_addr);
        Ok(())
    }

    /// Stop the connection pool
    pub async fn stop(&self) {
        self.is_running.store(false, Ordering::Release);

        // Close all connections
        let mut connections = self.connections.lock().await;
        while let Some(mut conn) = connections.pop_front() {
            conn.invalidate();
        }

        info!("Connection pool stopped for {}", self.target_addr);
    }

    /// Get a connection from the pool
    pub async fn get_connection(&self) -> Result<ManagedConnection> {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|_| StratumError::Internal {
                message: "Failed to acquire connection permit".to_string(),
            })?;

        // Try to get existing connection first
        if let Some(connection) = self.try_get_existing_connection().await? {
            self.total_reused.fetch_add(1, Ordering::Relaxed);
            return Ok(ManagedConnection::new(
                connection,
                Arc::clone(&self.connections),
            ));
        }

        // Create new connection
        match self.create_new_connection().await {
            Ok(connection) => {
                self.total_created.fetch_add(1, Ordering::Relaxed);
                self.active_connections.fetch_add(1, Ordering::Relaxed);
                Ok(ManagedConnection::new(
                    connection,
                    Arc::clone(&self.connections),
                ))
            }
            Err(e) => {
                self.total_errors.fetch_add(1, Ordering::Relaxed);
                Err(e)
            }
        }
    }

    /// Try to get an existing connection from the pool
    async fn try_get_existing_connection(&self) -> Result<Option<TcpStream>> {
        let mut connections = self.connections.lock().await;

        while let Some(mut pooled_conn) = connections.pop_front() {
            if pooled_conn.is_available() && !pooled_conn.is_expired(self.config.idle_timeout) {
                if let Some(connection) = pooled_conn.take_connection() {
                    // Validate connection is still usable
                    if self.validate_connection(&connection).await {
                        return Ok(Some(connection));
                    }
                }
            }
            // Connection is invalid or expired, discard it
            pooled_conn.invalidate();
            self.active_connections.fetch_sub(1, Ordering::Relaxed);
        }

        Ok(None)
    }

    /// Create a new connection
    async fn create_new_connection(&self) -> Result<TcpStream> {
        let mut _last_error = None;

        for attempt in 1..=self.config.retry_attempts {
            match timeout(
                self.config.connection_timeout,
                TcpStream::connect(self.target_addr),
            )
            .await
            {
                Ok(Ok(stream)) => {
                    debug!(
                        "Created new connection to {} (attempt {})",
                        self.target_addr, attempt
                    );
                    return Ok(stream);
                }
                Ok(Err(e)) => {
                    warn!(
                        "Failed to connect to {} (attempt {}): {}",
                        self.target_addr, attempt, e
                    );
                    _last_error = Some(e);
                }
                Err(_) => {
                    warn!(
                        "Connection timeout to {} (attempt {})",
                        self.target_addr, attempt
                    );
                    _last_error = Some(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "Connection timeout",
                    ));
                }
            }

            if attempt < self.config.retry_attempts {
                sleep(self.config.retry_delay * attempt).await;
            }
        }

        Err(StratumError::Network {
            message: format!(
                "Failed to connect to {} after {} attempts",
                self.target_addr, self.config.retry_attempts
            ),
            source: None, // Remove source for now to fix compilation
        })
    }

    /// Validate that a connection is still usable
    async fn validate_connection(&self, _connection: &TcpStream) -> bool {
        // Simple validation - in production you might want to send a ping
        true
    }

    // /// Return a connection to the pool
    // async fn return_connection(&self, connection: TcpStream) {
    //     let mut connections = self.connections.lock().await;

    //     if connections.len() < self.config.max_connections {
    //         let pooled = PooledConnection::new(connection, self.target_addr);
    //         connections.push_back(pooled);
    //     } else {
    //         // Pool is full, just drop the connection
    //         drop(connection);
    //         self.active_connections.fetch_sub(1, Ordering::Relaxed);
    //     }
    // }

    /// Ensure minimum number of connections are available
    async fn ensure_minimum_connections(&self) -> Result<()> {
        let current_count = {
            let connections = self.connections.lock().await;
            connections.len()
        };

        let needed = self.config.min_connections.saturating_sub(current_count);

        for _ in 0..needed {
            match self.create_new_connection().await {
                Ok(connection) => {
                    let pooled = PooledConnection::new(connection, self.target_addr);
                    let mut connections = self.connections.lock().await;
                    connections.push_back(pooled);
                    self.active_connections.fetch_add(1, Ordering::Relaxed);
                }
                Err(e) => {
                    warn!("Failed to create minimum connection: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    // /// Background maintenance loop
    // async fn maintenance_loop(&self) {
    //     while self.is_running.load(Ordering::Acquire) {
    //         self.cleanup_expired_connections().await;
    //         self.ensure_minimum_connections().await.ok();
    //         sleep(self.config.validation_interval).await;
    //     }
    // }

    /// Clean up expired connections
    #[allow(unused)]
    async fn cleanup_expired_connections(&self) {
        let mut connections = self.connections.lock().await;
        let mut to_remove = Vec::new();

        for (index, connection) in connections.iter().enumerate() {
            if connection.is_expired(self.config.idle_timeout) || !connection.is_valid {
                to_remove.push(index);
            }
        }

        // Remove expired connections (in reverse order to maintain indices)
        for &index in to_remove.iter().rev() {
            if let Some(mut connection) = connections.remove(index) {
                connection.invalidate();
                self.active_connections.fetch_sub(1, Ordering::Relaxed);
                debug!("Removed expired connection from pool");
            }
        }
    }

    /// Get pool statistics
    pub async fn get_stats(&self) -> PoolStats {
        let connections = self.connections.lock().await;
        let available_connections = connections.iter().filter(|c| c.is_available()).count();
        let expired_connections = connections
            .iter()
            .filter(|c| c.is_expired(self.config.idle_timeout))
            .count();

        PoolStats {
            total_connections: connections.len(),
            available_connections,
            active_connections: self.active_connections.load(Ordering::Relaxed),
            expired_connections,
            total_created: self.total_created.load(Ordering::Relaxed),
            total_reused: self.total_reused.load(Ordering::Relaxed),
            total_errors: self.total_errors.load(Ordering::Relaxed),
            target_addr: self.target_addr,
            config: self.config.clone(),
        }
    }

    /// Clone for maintenance tasks
    fn clone_for_maintenance(&self) -> ConnectionPoolMaintenance {
        ConnectionPoolMaintenance {
            config: self.config.clone(),
            connections: Arc::clone(&self.connections),
            active_connections: Arc::new(AtomicUsize::new(
                self.active_connections.load(Ordering::Relaxed),
            )),
            is_running: Arc::clone(&self.is_running),
            target_addr: self.target_addr,
        }
    }
}

/// Simplified pool struct for maintenance tasks
#[derive(Debug)]
struct ConnectionPoolMaintenance {
    config: PoolConfig,
    connections: Arc<Mutex<VecDeque<PooledConnection>>>,
    active_connections: Arc<AtomicUsize>,
    is_running: Arc<AtomicBool>,
    #[allow(unused)]
    target_addr: SocketAddr,
}

impl ConnectionPoolMaintenance {
    async fn maintenance_loop(&self) {
        while self.is_running.load(Ordering::Acquire) {
            self.cleanup_expired_connections().await;
            sleep(self.config.validation_interval).await;
        }
    }

    async fn cleanup_expired_connections(&self) {
        let mut connections = self.connections.lock().await;
        let mut to_remove = Vec::new();

        for (index, connection) in connections.iter().enumerate() {
            if connection.is_expired(self.config.idle_timeout) || !connection.is_valid {
                to_remove.push(index);
            }
        }

        for &index in to_remove.iter().rev() {
            if let Some(mut connection) = connections.remove(index) {
                connection.invalidate();
                self.active_connections.fetch_sub(1, Ordering::Relaxed);
                debug!("Removed expired connection from pool");
            }
        }
    }
}

/// Managed connection that returns to pool when dropped
pub struct ManagedConnection {
    connection: Option<TcpStream>,
    pool: Arc<Mutex<VecDeque<PooledConnection>>>,
}

impl ManagedConnection {
    fn new(connection: TcpStream, pool: Arc<Mutex<VecDeque<PooledConnection>>>) -> Self {
        Self {
            connection: Some(connection),
            pool,
        }
    }

    /// Take ownership of the connection
    pub fn take(mut self) -> TcpStream {
        self.connection.take().expect("Connection already taken")
    }

    /// Get a reference to the connection
    pub fn as_ref(&self) -> &TcpStream {
        self.connection.as_ref().expect("Connection already taken")
    }

    /// Get a mutable reference to the connection
    pub fn as_mut(&mut self) -> &mut TcpStream {
        self.connection.as_mut().expect("Connection already taken")
    }
}

impl Drop for ManagedConnection {
    fn drop(&mut self) {
        if let Some(connection) = self.connection.take() {
            let pool = Arc::clone(&self.pool);
            let peer_addr = connection
                .peer_addr()
                .unwrap_or_else(|_| "0.0.0.0:0".parse().unwrap());
            tokio::spawn(async move {
                let mut connections = pool.lock().await;
                let pooled = PooledConnection::new(connection, peer_addr);
                connections.push_back(pooled);
            });
        }
    }
}

impl std::ops::Deref for ManagedConnection {
    type Target = TcpStream;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl std::ops::DerefMut for ManagedConnection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

/// Resource manager for optimizing system resource usage
pub struct ResourceManager {
    memory_pressure_threshold: f64,
    cpu_pressure_threshold: f64,
    cleanup_interval: Duration,
    is_running: Arc<AtomicBool>,
    cleanup_callbacks: Arc<RwLock<Vec<Box<dyn Fn() + Send + Sync>>>>,
}

impl ResourceManager {
    pub fn new() -> Self {
        Self {
            memory_pressure_threshold: 80.0, // 80% memory usage
            cpu_pressure_threshold: 90.0,    // 90% CPU usage
            cleanup_interval: Duration::from_secs(30),
            is_running: Arc::new(AtomicBool::new(false)),
            cleanup_callbacks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Start the resource manager
    pub async fn start(&self) -> Result<()> {
        if self
            .is_running
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            return Err(StratumError::Internal {
                message: "Resource manager is already running".to_string(),
            });
        }

        let manager = self.clone_for_monitoring();
        tokio::spawn(async move {
            manager.monitoring_loop().await;
        });

        info!("Resource manager started");
        Ok(())
    }

    /// Stop the resource manager
    pub async fn stop(&self) {
        self.is_running.store(false, Ordering::Release);
        info!("Resource manager stopped");
    }

    /// Register cleanup callback for resource pressure
    pub async fn register_cleanup_callback<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        let mut callbacks = self.cleanup_callbacks.write().await;
        callbacks.push(Box::new(callback));
    }

    /// Check current resource pressure
    pub async fn check_resource_pressure(&self) -> ResourcePressure {
        // This is a simplified implementation
        // In production, you would use proper system monitoring
        ResourcePressure {
            memory_pressure: false,
            cpu_pressure: false,
            disk_pressure: false,
            network_pressure: false,
        }
    }

    /// Force cleanup of resources
    pub async fn force_cleanup(&self) {
        let callbacks = self.cleanup_callbacks.read().await;
        for callback in callbacks.iter() {
            callback();
        }
        info!("Forced resource cleanup executed");
    }

    /// Clone for monitoring
    fn clone_for_monitoring(&self) -> ResourceManagerMonitor {
        ResourceManagerMonitor {
            cleanup_interval: self.cleanup_interval,
            is_running: Arc::clone(&self.is_running),
            cleanup_callbacks: Arc::clone(&self.cleanup_callbacks),
            memory_pressure_threshold: self.memory_pressure_threshold,
            cpu_pressure_threshold: self.cpu_pressure_threshold,
        }
    }
}

impl Default for ResourceManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Monitor struct for resource management
struct ResourceManagerMonitor {
    cleanup_interval: Duration,
    is_running: Arc<AtomicBool>,
    cleanup_callbacks: Arc<RwLock<Vec<Box<dyn Fn() + Send + Sync>>>>,
    #[allow(unused)]
    memory_pressure_threshold: f64,
    #[allow(unused)]
    cpu_pressure_threshold: f64,
}

impl ResourceManagerMonitor {
    async fn monitoring_loop(&self) {
        while self.is_running.load(Ordering::Acquire) {
            if self.should_cleanup().await {
                let callbacks = self.cleanup_callbacks.read().await;
                for callback in callbacks.iter() {
                    callback();
                }
                info!("Resource cleanup triggered by pressure detection");
            }

            sleep(self.cleanup_interval).await;
        }
    }

    async fn should_cleanup(&self) -> bool {
        // Simplified pressure detection
        // In production, implement proper system monitoring
        false
    }
}

// Data structures

#[derive(Debug, Clone)]
pub struct ConnectionStats {
    pub created_at: Instant,
    pub last_used: Instant,
    pub use_count: u64,
    pub age: Duration,
    pub idle_time: Duration,
    pub is_valid: bool,
    pub remote_addr: SocketAddr,
}

#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total_connections: usize,
    pub available_connections: usize,
    pub active_connections: usize,
    pub expired_connections: usize,
    pub total_created: u64,
    pub total_reused: u64,
    pub total_errors: u64,
    pub target_addr: SocketAddr,
    pub config: PoolConfig,
}

#[derive(Debug, Clone)]
pub struct ResourcePressure {
    pub memory_pressure: bool,
    pub cpu_pressure: bool,
    pub disk_pressure: bool,
    pub network_pressure: bool,
}
