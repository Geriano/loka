use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio::time::interval;
use tracing::{debug, info, warn};

use crate::error::Result;
use crate::network::connection::{Connection, ConnectionId, DisconnectReason};

/// Configuration for connection management
#[derive(Debug, Clone)]
pub struct ConnectionManagerConfig {
    /// Maximum number of concurrent connections
    pub max_connections: usize,
    /// Idle timeout before connections are closed
    pub idle_timeout: Duration,
    /// Interval for connection cleanup tasks
    pub cleanup_interval: Duration,
    /// Interval for connection statistics logging
    pub stats_interval: Duration,
}

impl Default for ConnectionManagerConfig {
    fn default() -> Self {
        Self {
            max_connections: 1000,
            idle_timeout: Duration::from_secs(300), // 5 minutes
            cleanup_interval: Duration::from_secs(60), // 1 minute
            stats_interval: Duration::from_secs(300), // 5 minutes
        }
    }
}

/// Centralized connection lifecycle management
#[derive(Debug)]
pub struct ConnectionManager {
    /// Active connections indexed by connection ID
    connections: DashMap<ConnectionId, Arc<Connection>>,
    /// Connections indexed by remote address for lookup
    addr_to_connection: DashMap<SocketAddr, ConnectionId>,
    /// Manager configuration
    config: ConnectionManagerConfig,
    /// Background task handles
    tasks: RwLock<Vec<JoinHandle<()>>>,
}

impl ConnectionManager {
    /// Create a new connection manager
    pub fn new(config: ConnectionManagerConfig) -> Self {
        Self {
            connections: DashMap::new(),
            addr_to_connection: DashMap::new(),
            config,
            tasks: RwLock::new(Vec::new()),
        }
    }

    /// Start background tasks for connection management
    pub async fn start_background_tasks(&self) -> Result<()> {
        let mut tasks = self.tasks.write().await;

        // Start cleanup task
        let cleanup_task = self.start_cleanup_task();
        tasks.push(cleanup_task);

        // Start statistics task
        let stats_task = self.start_stats_task();
        tasks.push(stats_task);

        info!("Connection manager background tasks started");
        Ok(())
    }

    /// Stop all background tasks
    pub async fn stop_background_tasks(&self) {
        let mut tasks = self.tasks.write().await;

        for task in tasks.drain(..) {
            task.abort();
        }

        info!("Connection manager background tasks stopped");
    }

    /// Register a new connection
    pub async fn register_connection(&self, remote_addr: SocketAddr) -> Result<Arc<Connection>> {
        // Check connection limit
        if self.connections.len() >= self.config.max_connections {
            warn!(
                "Connection limit reached: {}/{}",
                self.connections.len(),
                self.config.max_connections
            );
            return Err(crate::error::StratumError::ConnectionLimitExceeded {
                current: self.connections.len(),
                max: self.config.max_connections,
            });
        }

        // Check for existing connection from same address
        if let Some(existing_id) = self.addr_to_connection.get(&remote_addr) {
            if let Some(existing_conn) = self.connections.get(&existing_id) {
                warn!(
                    "Duplicate connection attempt from {}, closing existing",
                    remote_addr
                );
                existing_conn
                    .disconnect(DisconnectReason::ClientDisconnect)
                    .await;
                self.unregister_connection(existing_conn.id()).await;
            }
        }

        // Create new connection
        let connection = Arc::new(Connection::new(remote_addr));
        let connection_id = connection.id();

        // Register in maps
        self.connections.insert(connection_id, connection.clone());
        self.addr_to_connection.insert(remote_addr, connection_id);

        info!(
            "Registered new connection: {} from {}",
            connection_id, remote_addr
        );
        Ok(connection)
    }

    /// Unregister a connection
    pub async fn unregister_connection(&self, connection_id: ConnectionId) {
        if let Some((_, connection)) = self.connections.remove(&connection_id) {
            let remote_addr = connection.remote_addr();
            self.addr_to_connection.remove(&remote_addr);

            // Mark as disconnected if not already
            connection
                .mark_disconnected(DisconnectReason::ClientDisconnect)
                .await;

            debug!(
                "Unregistered connection: {} from {}",
                connection_id, remote_addr
            );
        }
    }

    /// Get a connection by ID
    pub fn get_connection(&self, connection_id: ConnectionId) -> Option<Arc<Connection>> {
        self.connections
            .get(&connection_id)
            .map(|entry| entry.clone())
    }

    /// Get a connection by remote address
    pub fn get_connection_by_addr(&self, remote_addr: SocketAddr) -> Option<Arc<Connection>> {
        self.addr_to_connection
            .get(&remote_addr)
            .and_then(|entry| self.connections.get(&entry).map(|conn| conn.clone()))
    }

    /// Get all active connections
    pub fn get_all_connections(&self) -> Vec<Arc<Connection>> {
        self.connections.iter().map(|entry| entry.clone()).collect()
    }

    /// Get connection count
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    /// Get connections by state
    pub async fn get_authenticated_connections(&self) -> Vec<Arc<Connection>> {
        let mut authenticated = Vec::new();

        for connection in self.connections.iter() {
            if connection.is_authenticated().await {
                authenticated.push(connection.clone());
            }
        }

        authenticated
    }

    /// Force disconnect all connections
    pub async fn disconnect_all(&self, reason: DisconnectReason) {
        let connections: Vec<_> = self.connections.iter().map(|entry| entry.clone()).collect();

        info!(
            "Disconnecting all {} connections: {}",
            connections.len(),
            reason
        );

        for connection in connections {
            connection.disconnect(reason.clone()).await;
        }
    }

    /// Cleanup idle and terminated connections
    pub async fn cleanup_connections(&self) -> usize {
        let mut to_remove = Vec::new();

        for entry in self.connections.iter() {
            let connection = entry.value();

            if let Some(reason) = connection.should_terminate(self.config.idle_timeout).await {
                to_remove.push((connection.id(), reason));
            }
        }

        let cleanup_count = to_remove.len();

        for (connection_id, reason) in to_remove {
            if let Some(connection) = self.get_connection(connection_id) {
                connection.mark_disconnected(reason).await;
                self.unregister_connection(connection_id).await;
            }
        }

        if cleanup_count > 0 {
            debug!("Cleaned up {} idle/terminated connections", cleanup_count);
        }

        cleanup_count
    }

    /// Get manager statistics
    pub async fn get_stats(&self) -> ConnectionManagerStats {
        let mut stats = ConnectionManagerStats::default();

        for connection in self.connections.iter() {
            let connection_metrics = connection.metrics().await;

            stats.total_connections += 1;
            stats.total_messages_received += connection_metrics.messages_received;
            stats.total_messages_sent += connection_metrics.messages_sent;
            stats.total_bytes_received += connection_metrics.bytes_received;
            stats.total_bytes_sent += connection_metrics.bytes_sent;
            stats.total_auth_attempts += connection_metrics.auth_attempts;
            stats.total_auth_successes += connection_metrics.auth_successes;
            stats.total_share_submissions += connection_metrics.share_submissions;
            stats.total_accepted_shares += connection_metrics.accepted_shares;
            stats.total_rejected_shares += connection_metrics.rejected_shares;

            if connection.is_authenticated().await {
                stats.authenticated_connections += 1;
            }
        }

        stats.max_connections = self.config.max_connections;
        stats.utilization_percentage = if self.config.max_connections > 0 {
            (stats.total_connections as f64 / self.config.max_connections as f64) * 100.0
        } else {
            0.0
        };

        stats.acceptance_rate = if stats.total_share_submissions > 0 {
            (stats.total_accepted_shares as f64 / stats.total_share_submissions as f64) * 100.0
        } else {
            0.0
        };

        stats.auth_success_rate = if stats.total_auth_attempts > 0 {
            (stats.total_auth_successes as f64 / stats.total_auth_attempts as f64) * 100.0
        } else {
            0.0
        };

        stats
    }

    /// Start cleanup task
    fn start_cleanup_task(&self) -> JoinHandle<()> {
        let connections = self.connections.clone();
        let addr_to_connection = self.addr_to_connection.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut cleanup_interval = interval(config.cleanup_interval);

            loop {
                cleanup_interval.tick().await;

                let mut to_remove = Vec::new();

                for entry in connections.iter() {
                    let connection = entry.value();

                    if let Some(reason) = connection.should_terminate(config.idle_timeout).await {
                        to_remove.push((connection.id(), connection.remote_addr(), reason));
                    }
                }

                for (connection_id, remote_addr, reason) in to_remove {
                    if let Some((_, connection)) = connections.remove(&connection_id) {
                        addr_to_connection.remove(&remote_addr);
                        let reason_str = reason.to_string();
                        connection.mark_disconnected(reason).await;
                        debug!("Cleaned up connection: {} ({})", connection_id, reason_str);
                    }
                }
            }
        })
    }

    /// Start statistics logging task
    fn start_stats_task(&self) -> JoinHandle<()> {
        let manager = ConnectionManager {
            connections: self.connections.clone(),
            addr_to_connection: self.addr_to_connection.clone(),
            config: self.config.clone(),
            tasks: RwLock::new(Vec::new()),
        };

        tokio::spawn(async move {
            let mut stats_interval = interval(manager.config.stats_interval);

            loop {
                stats_interval.tick().await;

                let stats = manager.get_stats().await;

                info!(
                    "Connection Stats: {} active ({:.1}% capacity), {} authenticated, {:.1}% acceptance rate",
                    stats.total_connections,
                    stats.utilization_percentage,
                    stats.authenticated_connections,
                    stats.acceptance_rate
                );

                debug!("Detailed stats: {:?}", stats);
            }
        })
    }
}

impl Drop for ConnectionManager {
    fn drop(&mut self) {
        // Note: Can't use async in Drop, so we just log
        info!(
            "Connection manager dropping with {} active connections",
            self.connections.len()
        );
    }
}

/// Statistics about the connection manager
#[derive(Debug, Clone, Default)]
pub struct ConnectionManagerStats {
    pub total_connections: usize,
    pub authenticated_connections: usize,
    pub max_connections: usize,
    pub utilization_percentage: f64,
    pub total_messages_received: u64,
    pub total_messages_sent: u64,
    pub total_bytes_received: u64,
    pub total_bytes_sent: u64,
    pub total_auth_attempts: u64,
    pub total_auth_successes: u64,
    pub auth_success_rate: f64,
    pub total_share_submissions: u64,
    pub total_accepted_shares: u64,
    pub total_rejected_shares: u64,
    pub acceptance_rate: f64,
}

impl std::fmt::Display for ConnectionManagerStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Connections: {}/{} ({:.1}%), Auth: {} ({:.1}%), Shares: {}/{} ({:.1}%)",
            self.total_connections,
            self.max_connections,
            self.utilization_percentage,
            self.authenticated_connections,
            self.auth_success_rate,
            self.total_accepted_shares,
            self.total_share_submissions,
            self.acceptance_rate
        )
    }
}
