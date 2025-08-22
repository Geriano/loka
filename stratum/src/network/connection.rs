use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::auth;
use crate::error::Result;

/// Unique identifier for connections
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConnectionId(u64);

impl ConnectionId {
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "conn-{}", self.0)
    }
}

/// Connection state tracking
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    /// Just connected, not yet authenticated
    Connected,
    /// Successfully authenticated with session
    Authenticated { session_id: String },
    /// Connection is being terminated
    Terminating { reason: DisconnectReason },
    /// Connection fully closed
    Disconnected { reason: DisconnectReason },
}

/// Reasons for connection termination
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DisconnectReason {
    /// Client disconnected normally
    ClientDisconnect,
    /// Server initiated disconnect
    ServerShutdown,
    /// Connection timeout (idle)
    Timeout,
    /// Protocol error
    ProtocolError { message: String },
    /// Authentication failure
    AuthenticationFailed,
    /// Rate limiting triggered
    RateLimited,
    /// Network error
    NetworkError { error: String },
}

impl std::fmt::Display for DisconnectReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClientDisconnect => write!(f, "client disconnect"),
            Self::ServerShutdown => write!(f, "server shutdown"),
            Self::Timeout => write!(f, "connection timeout"),
            Self::ProtocolError { message } => write!(f, "protocol error: {}", message),
            Self::AuthenticationFailed => write!(f, "authentication failed"),
            Self::RateLimited => write!(f, "rate limited"),
            Self::NetworkError { error } => write!(f, "network error: {}", error),
        }
    }
}

/// Connection-level metrics
#[derive(Debug, Clone)]
pub struct ConnectionMetrics {
    /// Total messages received from client
    pub messages_received: u64,
    /// Total messages sent to client
    pub messages_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Authentication attempts
    pub auth_attempts: u64,
    /// Successful authentications
    pub auth_successes: u64,
    /// Share submissions
    pub share_submissions: u64,
    /// Accepted shares
    pub accepted_shares: u64,
    /// Rejected shares
    pub rejected_shares: u64,
    /// Last activity timestamp
    pub last_activity: Instant,
}

impl Default for ConnectionMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionMetrics {
    pub fn new() -> Self {
        Self {
            messages_received: 0,
            messages_sent: 0,
            bytes_received: 0,
            bytes_sent: 0,
            auth_attempts: 0,
            auth_successes: 0,
            share_submissions: 0,
            accepted_shares: 0,
            rejected_shares: 0,
            last_activity: Instant::now(),
        }
    }

    pub fn record_message_received(&mut self, bytes: usize) {
        self.messages_received += 1;
        self.bytes_received += bytes as u64;
        self.update_activity();
    }

    pub fn record_message_sent(&mut self, bytes: usize) {
        self.messages_sent += 1;
        self.bytes_sent += bytes as u64;
        self.update_activity();
    }

    pub fn record_auth_attempt(&mut self, success: bool) {
        self.auth_attempts += 1;
        if success {
            self.auth_successes += 1;
        }
        self.update_activity();
    }

    pub fn record_share_submission(&mut self, accepted: bool) {
        self.share_submissions += 1;
        if accepted {
            self.accepted_shares += 1;
        } else {
            self.rejected_shares += 1;
        }
        self.update_activity();
    }

    pub fn update_activity(&mut self) {
        self.last_activity = Instant::now();
    }

    pub fn is_idle(&self, timeout: Duration) -> bool {
        self.last_activity.elapsed() > timeout
    }

    /// Calculate acceptance rate as percentage
    pub fn acceptance_rate(&self) -> f64 {
        if self.share_submissions == 0 {
            0.0
        } else {
            (self.accepted_shares as f64 / self.share_submissions as f64) * 100.0
        }
    }

    /// Calculate throughput in messages per second
    pub fn message_throughput(&self, duration: Duration) -> f64 {
        if duration.is_zero() {
            0.0
        } else {
            (self.messages_received + self.messages_sent) as f64 / duration.as_secs_f64()
        }
    }
}

/// Comprehensive connection information and lifecycle management
#[derive(Debug)]
pub struct Connection {
    /// Unique connection identifier
    id: ConnectionId,
    /// Remote client address
    remote_addr: SocketAddr,
    /// Connection establishment time
    created_at: Instant,
    /// Current connection state
    state: RwLock<ConnectionState>,
    /// Authentication session (if authenticated)
    auth_session: RwLock<Option<auth::State>>,
    /// Connection-level metrics
    metrics: RwLock<ConnectionMetrics>,
}

impl Connection {
    /// Create a new connection
    pub fn new(remote_addr: SocketAddr) -> Self {
        Self {
            id: ConnectionId::new(),
            remote_addr,
            created_at: Instant::now(),
            state: RwLock::new(ConnectionState::Connected),
            auth_session: RwLock::new(None),
            metrics: RwLock::new(ConnectionMetrics::new()),
        }
    }

    /// Get connection ID
    pub fn id(&self) -> ConnectionId {
        self.id
    }

    /// Get remote address
    pub fn remote_addr(&self) -> SocketAddr {
        self.remote_addr
    }

    /// Get connection age
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }

    /// Get current state
    pub async fn state(&self) -> ConnectionState {
        self.state.read().await.clone()
    }

    /// Check if connection is authenticated
    pub async fn is_authenticated(&self) -> bool {
        matches!(
            *self.state.read().await,
            ConnectionState::Authenticated { .. }
        )
    }

    /// Get authentication session
    pub async fn auth_session(&self) -> Option<auth::State> {
        self.auth_session.read().await.clone()
    }

    /// Authenticate the connection
    pub async fn authenticate(&self, session: auth::State) -> Result<()> {
        let session_id = format!("{}:{}", session.user(), session.worker());

        {
            let mut state = self.state.write().await;
            *state = ConnectionState::Authenticated { session_id };
        }

        {
            let mut auth = self.auth_session.write().await;
            *auth = Some(session);
        }

        {
            let mut metrics = self.metrics.write().await;
            metrics.record_auth_attempt(true);
        }

        tracing::info!("Connection {} authenticated successfully", self.id);
        Ok(())
    }

    /// Record authentication failure
    pub async fn record_auth_failure(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.record_auth_attempt(false);
    }

    /// Update activity timestamp
    pub async fn update_activity(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.update_activity();
    }

    /// Check if connection is idle
    pub async fn is_idle(&self, timeout: Duration) -> bool {
        let metrics = self.metrics.read().await;
        metrics.is_idle(timeout)
    }

    /// Record message received from client
    pub async fn record_message_received(&self, message_size: usize) {
        let mut metrics = self.metrics.write().await;
        metrics.record_message_received(message_size);
    }

    /// Record message sent to client
    pub async fn record_message_sent(&self, message_size: usize) {
        let mut metrics = self.metrics.write().await;
        metrics.record_message_sent(message_size);
    }

    /// Record share submission
    pub async fn record_share_submission(&self, accepted: bool) {
        let mut metrics = self.metrics.write().await;
        metrics.record_share_submission(accepted);
    }

    /// Get connection metrics snapshot
    pub async fn metrics(&self) -> ConnectionMetrics {
        self.metrics.read().await.clone()
    }

    /// Initiate graceful disconnection
    pub async fn disconnect(&self, reason: DisconnectReason) {
        let mut state = self.state.write().await;

        match &*state {
            ConnectionState::Disconnected { .. } => {
                // Already disconnected
                return;
            }
            _ => {
                *state = ConnectionState::Terminating {
                    reason: reason.clone(),
                };
            }
        }

        tracing::info!("Connection {} disconnecting: {}", self.id, reason);
    }

    /// Mark connection as fully disconnected
    pub async fn mark_disconnected(&self, reason: DisconnectReason) {
        let mut state = self.state.write().await;
        *state = ConnectionState::Disconnected {
            reason: reason.clone(),
        };

        tracing::debug!("Connection {} disconnected: {}", self.id, reason);
    }

    /// Check if connection should be terminated
    pub async fn should_terminate(&self, idle_timeout: Duration) -> Option<DisconnectReason> {
        let state = self.state.read().await;

        match &*state {
            ConnectionState::Terminating { reason } => Some(reason.clone()),
            ConnectionState::Disconnected { reason } => Some(reason.clone()),
            _ => {
                drop(state);

                if self.is_idle(idle_timeout).await {
                    Some(DisconnectReason::Timeout)
                } else {
                    None
                }
            }
        }
    }

    /// Get connection summary for logging
    pub async fn summary(&self) -> ConnectionSummary {
        let state = self.state.read().await.clone();
        let metrics = self.metrics.read().await.clone();
        let auth_session = self.auth_session.read().await.clone();

        ConnectionSummary {
            id: self.id,
            remote_addr: self.remote_addr,
            age: self.age(),
            state,
            authenticated_user: auth_session.map(|s| format!("{}:{}", s.user(), s.worker())),
            metrics,
        }
    }
}

/// Connection summary for logging and monitoring
#[derive(Debug, Clone)]
pub struct ConnectionSummary {
    pub id: ConnectionId,
    pub remote_addr: SocketAddr,
    pub age: Duration,
    pub state: ConnectionState,
    pub authenticated_user: Option<String>,
    pub metrics: ConnectionMetrics,
}

impl std::fmt::Display for ConnectionSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} [{}] age={:.1}s state={:?} user={} msgs={}/{} shares={}/{} rate={:.1}%",
            self.id,
            self.remote_addr,
            self.age.as_secs_f64(),
            self.state,
            self.authenticated_user.as_deref().unwrap_or("none"),
            self.metrics.messages_received,
            self.metrics.messages_sent,
            self.metrics.accepted_shares,
            self.metrics.rejected_shares,
            self.metrics.acceptance_rate(),
        )
    }
}
