use std::fmt::Debug;

use async_trait::async_trait;

use crate::error::Result;
use crate::protocol::messages::StratumMessage;
use crate::protocol::pipeline::MessageContext;

/// Trait for parsing protocol messages
#[async_trait]
pub trait MessageParser: Send + Sync + Debug {
    /// Parse raw message string into a StratumMessage
    async fn parse(&self, raw_message: &str) -> Result<Option<StratumMessage>>;

    /// Serialize a StratumMessage back to string
    async fn serialize(&self, message: &StratumMessage) -> Result<String>;

    /// Get supported protocol version
    fn protocol_version(&self) -> &'static str;
}

/// Trait for handling protocol messages
#[async_trait]
pub trait MessageHandler: Send + Sync + Debug {
    /// Handle a parsed message and return response
    async fn handle(&self, context: MessageContext) -> Result<Option<StratumMessage>>;

    /// Check if handler supports this message type
    fn supports(&self, message: &StratumMessage) -> bool;

    /// Get handler priority (lower numbers = higher priority)
    fn priority(&self) -> u32 {
        100
    }
}

/// Trait for protocol-specific validation
#[async_trait]
pub trait MessageValidator: Send + Sync + Debug {
    /// Validate a message according to protocol rules
    async fn validate(&self, message: &StratumMessage, context: &MessageContext) -> Result<()>;

    /// Get validation rules description
    fn rules_description(&self) -> &'static str;
}

/// Trait for protocol session management
#[async_trait]
pub trait SessionManager: Send + Sync + Debug {
    /// Start a new session for a client
    async fn start_session(
        &self,
        client_id: String,
        client_info: SessionInfo,
    ) -> Result<SessionHandle>;

    /// End a session
    async fn end_session(&self, session_id: &str) -> Result<()>;

    /// Get session information
    async fn get_session(&self, session_id: &str) -> Result<Option<SessionInfo>>;

    /// Update session state
    async fn update_session(&self, session_id: &str, update: SessionUpdate) -> Result<()>;

    /// List active sessions
    async fn list_sessions(&self) -> Result<Vec<SessionInfo>>;
}

/// Information about a client session
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub session_id: String,
    pub client_id: String,
    pub client_ip: Option<std::net::SocketAddr>,
    pub authenticated: bool,
    pub user: Option<String>,
    pub worker: Option<String>,
    pub difficulty: f64,
    pub connected_at: chrono::DateTime<chrono::Utc>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
    pub total_shares: u64,
    pub accepted_shares: u64,
    pub rejected_shares: u64,
}

impl SessionInfo {
    pub fn new(session_id: String, client_id: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            session_id,
            client_id,
            client_ip: None,
            authenticated: false,
            user: None,
            worker: None,
            difficulty: 1.0,
            connected_at: now,
            last_activity: now,
            total_shares: 0,
            accepted_shares: 0,
            rejected_shares: 0,
        }
    }

    pub fn with_client_ip(mut self, ip: std::net::SocketAddr) -> Self {
        self.client_ip = Some(ip);
        self
    }

    pub fn authenticate(mut self, user: String, worker: String) -> Self {
        self.authenticated = true;
        self.user = Some(user);
        self.worker = Some(worker);
        self
    }
}

/// Updates that can be applied to a session
#[derive(Debug, Clone)]
pub enum SessionUpdate {
    Authenticate { user: String, worker: String },
    SetDifficulty { difficulty: f64 },
    UpdateActivity,
    IncrementShares { accepted: bool },
    Disconnect,
}

/// Handle to a session for performing operations
#[derive(Debug, Clone)]
pub struct SessionHandle {
    pub session_id: String,
    pub client_id: String,
}

impl SessionHandle {
    pub fn new(session_id: String, client_id: String) -> Self {
        Self {
            session_id,
            client_id,
        }
    }
}

/// Trait for protocol-specific routing
#[async_trait]
pub trait MessageRouter: Send + Sync + Debug {
    /// Route a message to appropriate handler
    async fn route(&self, context: MessageContext) -> Result<Option<StratumMessage>>;

    /// Register a message handler
    async fn register_handler<H>(&self, handler: H) -> Result<()>
    where
        H: MessageHandler + 'static;

    /// Remove a handler
    async fn unregister_handler(&self, handler_id: &str) -> Result<()>;

    /// Get registered handlers count
    fn handlers_count(&self) -> usize;
}

/// Trait for protocol statistics and metrics
pub trait ProtocolMetrics: Send + Sync + Debug {
    /// Record message processing metrics
    fn record_message(
        &self,
        message_type: &str,
        processing_time: std::time::Duration,
        success: bool,
    );

    /// Record session metrics
    fn record_session_event(&self, event_type: &str, session_id: &str);

    /// Record error metrics
    fn record_error(&self, error_type: &str, context: &str);

    /// Get current metrics snapshot
    fn get_metrics(&self) -> ProtocolMetricsSnapshot;
}

/// Snapshot of protocol metrics
#[derive(Debug, Clone)]
pub struct ProtocolMetricsSnapshot {
    pub total_messages: u64,
    pub successful_messages: u64,
    pub failed_messages: u64,
    pub average_processing_time: std::time::Duration,
    pub active_sessions: u64,
    pub total_sessions: u64,
    pub error_rate: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Default for ProtocolMetricsSnapshot {
    fn default() -> Self {
        Self::new()
    }
}

impl ProtocolMetricsSnapshot {
    pub fn new() -> Self {
        Self {
            total_messages: 0,
            successful_messages: 0,
            failed_messages: 0,
            average_processing_time: std::time::Duration::from_millis(0),
            active_sessions: 0,
            total_sessions: 0,
            error_rate: 0.0,
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_messages == 0 {
            0.0
        } else {
            self.successful_messages as f64 / self.total_messages as f64 * 100.0
        }
    }
}
