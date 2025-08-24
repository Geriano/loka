use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use dashmap::DashMap;
use tracing::{debug, info, warn};

use crate::error::{Result, StratumError};
use crate::protocol::messages::StratumMessage;
use crate::protocol::parser::StratumParser;
use crate::protocol::pipeline::MessageContext;
use crate::protocol::traits::{
    MessageHandler, MessageParser, MessageRouter, ProtocolMetrics, ProtocolMetricsSnapshot,
    SessionHandle, SessionInfo, SessionManager, SessionUpdate,
};

/// Default implementation of MessageParser using StratumParser
#[derive(Debug, Clone)]
pub struct DefaultMessageParser {
    parser: StratumParser,
}

impl DefaultMessageParser {
    pub fn new() -> Self {
        Self {
            parser: StratumParser::new(),
        }
    }
}

impl Default for DefaultMessageParser {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MessageParser for DefaultMessageParser {
    async fn parse(&self, raw_message: &str) -> Result<Option<StratumMessage>> {
        self.parser.parse_message(raw_message)
    }

    async fn serialize(&self, message: &StratumMessage) -> Result<String> {
        serde_json::to_string(message).map_err(|e| StratumError::Protocol {
            message: format!("Failed to serialize message: {e}"),
            method: None,
            request_id: None,
        })
    }

    fn protocol_version(&self) -> &'static str {
        "stratum-v1"
    }
}

/// Authentication handler for mining.authorize messages
#[derive(Debug)]
pub struct AuthenticationHandler;

impl AuthenticationHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AuthenticationHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MessageHandler for AuthenticationHandler {
    async fn handle(&self, context: MessageContext) -> Result<Option<StratumMessage>> {
        if let Some(StratumMessage::Authenticate { user, worker, .. }) = &context.parsed_message {
            info!("Handling authentication for {}:{}", user, worker);

            // For now, accept all authentication requests
            // TODO: Add actual authentication logic
            Ok(Some(StratumMessage::Submitted {
                id: context
                    .parsed_message
                    .as_ref()
                    .and_then(|m| m.id())
                    .unwrap_or(0),
                valid: true,
            }))
        } else {
            Ok(None)
        }
    }

    fn supports(&self, message: &StratumMessage) -> bool {
        matches!(message, StratumMessage::Authenticate { .. })
    }

    fn priority(&self) -> u32 {
        10 // High priority for authentication
    }
}

/// Submit handler for mining.submit messages
#[derive(Debug)]
pub struct SubmitHandler;

impl SubmitHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SubmitHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MessageHandler for SubmitHandler {
    async fn handle(&self, context: MessageContext) -> Result<Option<StratumMessage>> {
        if let Some(StratumMessage::Submit { id, job_id, .. }) = &context.parsed_message {
            debug!("Handling submit for job: {}", job_id);

            // TODO: Add actual share validation logic
            // For now, accept all submissions
            Ok(Some(StratumMessage::Submitted {
                id: *id,
                valid: true,
            }))
        } else {
            Ok(None)
        }
    }

    fn supports(&self, message: &StratumMessage) -> bool {
        matches!(message, StratumMessage::Submit { .. })
    }

    fn priority(&self) -> u32 {
        20 // Medium priority for submissions
    }
}

/// Default message router implementation
#[derive(Debug)]
pub struct DefaultMessageRouter {
    handlers: DashMap<String, Arc<dyn MessageHandler>>,
}

impl DefaultMessageRouter {
    pub fn new() -> Self {
        let router = Self {
            handlers: DashMap::new(),
        };

        // Register default handlers
        let auth_handler = Arc::new(AuthenticationHandler::new());
        let submit_handler = Arc::new(SubmitHandler::new());

        router.handlers.insert("auth".to_string(), auth_handler);
        router.handlers.insert("submit".to_string(), submit_handler);

        router
    }
}

impl Default for DefaultMessageRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MessageRouter for DefaultMessageRouter {
    async fn route(&self, context: MessageContext) -> Result<Option<StratumMessage>> {
        if let Some(ref message) = context.parsed_message {
            // Find a handler that supports this message
            let handlers: Vec<_> = self
                .handlers
                .iter()
                .map(|entry| entry.value().clone())
                .collect();
            for handler in handlers {
                if handler.supports(message) {
                    return handler.handle(context.clone()).await;
                }
            }
        }

        warn!("No handler found for message: {:?}", context.parsed_message);
        Ok(None)
    }

    async fn register_handler<H>(&self, handler: H) -> Result<()>
    where
        H: MessageHandler + 'static,
    {
        let handler_id = format!("handler_{}", self.handlers.len());
        self.handlers.insert(handler_id, Arc::new(handler));
        Ok(())
    }

    async fn unregister_handler(&self, handler_id: &str) -> Result<()> {
        self.handlers.remove(handler_id);
        Ok(())
    }

    fn handlers_count(&self) -> usize {
        self.handlers.len()
    }
}

/// In-memory session manager implementation
#[derive(Debug)]
pub struct InMemorySessionManager {
    sessions: DashMap<String, SessionInfo>,
    session_counter: AtomicU64,
}

impl InMemorySessionManager {
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
            session_counter: AtomicU64::new(0),
        }
    }

    fn generate_session_id(&self) -> String {
        let counter = self.session_counter.fetch_add(1, Ordering::Relaxed);
        format!("session_{counter}")
    }
}

impl Default for InMemorySessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SessionManager for InMemorySessionManager {
    async fn start_session(
        &self,
        client_id: String,
        mut session_info: SessionInfo,
    ) -> Result<SessionHandle> {
        let session_id = self.generate_session_id();
        session_info.session_id = session_id.clone();
        session_info.client_id = client_id.clone();

        let handle = SessionHandle::new(session_id.clone(), client_id);
        self.sessions.insert(session_id, session_info);

        info!("Started session: {}", handle.session_id);
        Ok(handle)
    }

    async fn end_session(&self, session_id: &str) -> Result<()> {
        if let Some((_, session)) = self.sessions.remove(session_id) {
            info!(
                "Ended session: {} for client: {}",
                session_id, session.client_id
            );
            Ok(())
        } else {
            Err(StratumError::Protocol {
                message: format!("Session not found: {session_id}"),
                method: None,
                request_id: None,
            })
        }
    }

    async fn get_session(&self, session_id: &str) -> Result<Option<SessionInfo>> {
        Ok(self.sessions.get(session_id).map(|entry| entry.clone()))
    }

    async fn update_session(&self, session_id: &str, update: SessionUpdate) -> Result<()> {
        if let Some(mut session) = self.sessions.get_mut(session_id) {
            match update {
                SessionUpdate::Authenticate { user, worker } => {
                    session.authenticated = true;
                    session.user = Some(user);
                    session.worker = Some(worker);
                }
                SessionUpdate::SetDifficulty { difficulty } => {
                    session.difficulty = difficulty;
                }
                SessionUpdate::UpdateActivity => {
                    session.last_activity = chrono::Utc::now();
                }
                SessionUpdate::IncrementShares { accepted } => {
                    session.total_shares += 1;
                    if accepted {
                        session.accepted_shares += 1;
                    } else {
                        session.rejected_shares += 1;
                    }
                }
                SessionUpdate::Disconnect => {
                    // Session will be removed by end_session
                }
            }
            Ok(())
        } else {
            Err(StratumError::Protocol {
                message: format!("Session not found: {session_id}"),
                method: None,
                request_id: None,
            })
        }
    }

    async fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        Ok(self.sessions.iter().map(|entry| entry.clone()).collect())
    }
}

/// Metrics collector for protocol statistics
#[derive(Debug)]
pub struct DefaultProtocolMetrics {
    message_counts: DashMap<String, AtomicU64>,
    success_count: AtomicU64,
    error_count: AtomicU64,
    total_processing_time: AtomicU64, // in microseconds
    session_count: AtomicU64,
    connect_validation_counts: DashMap<String, AtomicU64>,
    connect_validation_time: AtomicU64, // in microseconds
    #[allow(unused)]
    start_time: Instant,
}

impl DefaultProtocolMetrics {
    pub fn new() -> Self {
        Self {
            message_counts: DashMap::new(),
            success_count: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
            total_processing_time: AtomicU64::new(0),
            session_count: AtomicU64::new(0),
            connect_validation_counts: DashMap::new(),
            connect_validation_time: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    /// Record HTTP CONNECT validation metrics
    pub fn record_connect_validation(&self, result_type: &str, duration: Duration) {
        // Update validation result count
        self.connect_validation_counts
            .entry(result_type.to_string())
            .and_modify(|counter| {
                counter.fetch_add(1, Ordering::Relaxed);
            })
            .or_insert_with(|| AtomicU64::new(1));

        // Update total validation time
        let validation_micros = duration.as_micros() as u64;
        self.connect_validation_time
            .fetch_add(validation_micros, Ordering::Relaxed);

        tracing::debug!("CONNECT validation: {} took {:?}", result_type, duration);
    }

    /// Get CONNECT validation statistics
    pub fn get_connect_validation_stats(
        &self,
    ) -> (std::collections::HashMap<String, u64>, Duration) {
        let mut stats = std::collections::HashMap::new();
        for entry in self.connect_validation_counts.iter() {
            stats.insert(entry.key().clone(), entry.value().load(Ordering::Relaxed));
        }

        let total_validations: u64 = stats.values().sum();
        let total_time_micros = self.connect_validation_time.load(Ordering::Relaxed);
        let avg_validation_time = if total_validations > 0 {
            Duration::from_micros(total_time_micros / total_validations)
        } else {
            Duration::from_micros(0)
        };

        (stats, avg_validation_time)
    }
}

impl Default for DefaultProtocolMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl ProtocolMetrics for DefaultProtocolMetrics {
    fn record_message(&self, message_type: &str, processing_time: Duration, success: bool) {
        // Update message type count
        self.message_counts
            .entry(message_type.to_string())
            .and_modify(|counter| {
                counter.fetch_add(1, Ordering::Relaxed);
            })
            .or_insert_with(|| AtomicU64::new(1));

        // Update success/error counts
        if success {
            self.success_count.fetch_add(1, Ordering::Relaxed);
        } else {
            self.error_count.fetch_add(1, Ordering::Relaxed);
        }

        // Update processing time
        let processing_micros = processing_time.as_micros() as u64;
        self.total_processing_time
            .fetch_add(processing_micros, Ordering::Relaxed);
    }

    fn record_session_event(&self, event_type: &str, _session_id: &str) {
        debug!("Session event: {}", event_type);

        if event_type == "session_start" {
            self.session_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn record_error(&self, error_type: &str, context: &str) {
        warn!("Protocol error: {} in context: {}", error_type, context);
        self.error_count.fetch_add(1, Ordering::Relaxed);
    }

    fn get_metrics(&self) -> ProtocolMetricsSnapshot {
        let total_messages =
            self.success_count.load(Ordering::Relaxed) + self.error_count.load(Ordering::Relaxed);
        let successful_messages = self.success_count.load(Ordering::Relaxed);
        let failed_messages = self.error_count.load(Ordering::Relaxed);

        let total_processing_micros = self.total_processing_time.load(Ordering::Relaxed);
        let average_processing_time = if total_messages > 0 {
            Duration::from_micros(total_processing_micros / total_messages)
        } else {
            Duration::from_micros(0)
        };

        let error_rate = if total_messages > 0 {
            failed_messages as f64 / total_messages as f64 * 100.0
        } else {
            0.0
        };

        ProtocolMetricsSnapshot {
            total_messages,
            successful_messages,
            failed_messages,
            average_processing_time,
            active_sessions: self.session_count.load(Ordering::Relaxed),
            total_sessions: self.session_count.load(Ordering::Relaxed),
            error_rate,
            timestamp: chrono::Utc::now(),
        }
    }
}
