use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use serde_json::Value;
use tracing::{debug, trace};

use crate::error::Result;
use crate::network::connection::Connection;
use crate::network::proxy::MessageProcessor;
use crate::processor::Message;
use crate::protocol::types::Response;
use crate::{processor, Manager};

/// Pass-through processor that logs messages but doesn't modify them
#[derive(Debug, Default)]
pub struct LoggingProcessor {
    log_level: LogLevel,
}

#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Off,
}

impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::Debug
    }
}

impl LoggingProcessor {
    pub fn new(log_level: LogLevel) -> Self {
        Self { log_level }
    }

    fn log_message(&self, direction: &str, connection_id: &str, message: &Value) {
        let message_str = serde_json::to_string(message).unwrap_or_default();
        
        match self.log_level {
            LogLevel::Trace => trace!("{} {} - Message: {}", direction, connection_id, message_str),
            LogLevel::Debug => debug!("{} {} - Message: {}", direction, connection_id, message_str),
            LogLevel::Info => debug!("{} {} - Message size: {} bytes", direction, connection_id, message_str.len()),
            LogLevel::Warn | LogLevel::Error | LogLevel::Off => {} // No logging for higher levels
        }
    }
}

#[async_trait]
impl MessageProcessor for LoggingProcessor {
    async fn process_client_message(&self, message: Value, connection: &Arc<Connection>) -> Result<Value> {
        self.log_message("CLIENT->SERVER", &connection.id().to_string(), &message);
        connection.update_activity().await;
        connection.record_message_received(serde_json::to_string(&message)?.len()).await;
        Ok(message)
    }

    async fn process_server_message(&self, message: Value, connection: &Arc<Connection>) -> Result<Value> {
        self.log_message("SERVER->CLIENT", &connection.id().to_string(), &message);
        connection.update_activity().await;
        connection.record_message_received(serde_json::to_string(&message)?.len()).await;
        Ok(message)
    }
}

/// Stratum-aware processor that handles protocol-specific logic
#[derive(Debug)]
pub struct StratumProcessor {
    manager: Arc<Manager>,
    processor: Arc<processor::Manager>,
    difficulty: Arc<AtomicU64>,
}

impl StratumProcessor {
    pub fn new(
        manager: Arc<Manager>,
        processor: Arc<processor::Manager>,
        difficulty: Arc<AtomicU64>,
    ) -> Self {
        Self {
            manager,
            processor,
            difficulty,
        }
    }

    async fn handle_authentication(
        &self,
        message: Value,
        connection: &Arc<Connection>,
        user: String,
        worker: String,
    ) -> Result<Value> {
        let addr = connection.remote_addr();
        
        // Handle authentication
        let auth_state = self.manager.auth().authenticate(addr, &user, &worker);
        connection.authenticate((*auth_state).clone()).await?;
        self.manager.submissions().authenticated(auth_state);

        // Transform the authentication message for upstream
        let mut transformed_message = message.clone();
        let config = self.manager.config();
        let username = config.pool.username.as_str();
        let password = config.pool.password.as_deref().unwrap_or("x");
        let (s1, s2) = &config.pool.separator;
        let current = format!("{}{}{}{}{}", username, s1, user, s2, worker);

        debug!("authenticate {}.{} as {}", user, worker, current);

        transformed_message["params"] = Value::Array(vec![
            Value::String(current),
            Value::String(password.to_owned()),
        ]);

        Ok(transformed_message)
    }

    async fn handle_submission(
        &self,
        message: Value,
        connection: &Arc<Connection>,
        id: u64,
        job_id: String,
    ) -> Result<Value> {
        // Handle share submission
        let jobs = self.manager.jobs();
        if let Some(job) = jobs.get(&job_id) {
            if let Some(auth) = connection.auth_session().await {
                self.manager.submissions().submit(id, job, Arc::new(auth));
                connection.record_share_submission(true).await; // Assume valid for now
            }
        }
        
        Ok(message)
    }

    async fn handle_difficulty_update(
        &self,
        message: Value,
        connection: &Arc<Connection>,
        difficulty: f64,
    ) -> Result<Value> {
        self.difficulty.store(difficulty.to_bits(), Ordering::Relaxed);
        debug!("Connection {} - Set difficulty to {}", connection.id(), difficulty);
        Ok(message)
    }

    async fn handle_job_notification(
        &self,
        message: Value,
        connection: &Arc<Connection>,
        job_id: String,
    ) -> Result<Value> {
        let jobs = self.manager.jobs();
        jobs.notified(
            &job_id,
            f64::from_bits(self.difficulty.load(Ordering::Relaxed)),
        );

        debug!(
            "Connection {} - Notified new job {} total {}",
            connection.id(),
            job_id,
            jobs.total()
        );
        Ok(message)
    }

    async fn handle_submission_response(
        &self,
        message: Value,
        connection: &Arc<Connection>,
        response: Response,
    ) -> Result<Value> {
        let addr = connection.remote_addr();
        let submission = self.manager.submissions();
        
        if let Some(auth) = self.manager.auth().get(&addr) {
            if let Some(id) = response.id {
                if let Some(Some(result)) = response.result.as_ref().map(|r| r.as_bool()) {
                    submission.submitted(&id, result, auth);
                    connection.record_share_submission(result).await;
                }
            }
        }
        Ok(message)
    }
}

#[async_trait]
impl MessageProcessor for StratumProcessor {
    async fn process_client_message(&self, message: Value, connection: &Arc<Connection>) -> Result<Value> {
        // Update connection activity
        connection.update_activity().await;
        connection.record_message_received(serde_json::to_string(&message)?.len()).await;

        // Parse message using the existing processor
        if let Some(parsed_message) = self.processor.parse(&serde_json::to_string(&message)?) {
            match parsed_message {
                Message::Authenticate { user, worker, .. } => {
                    self.handle_authentication(message, connection, user, worker).await
                }
                Message::Submit { id, job_id } => {
                    self.handle_submission(message, connection, id, job_id).await
                }
                _ => Ok(message),
            }
        } else {
            // Forward message as-is if not parseable
            Ok(message)
        }
    }

    async fn process_server_message(&self, message: Value, connection: &Arc<Connection>) -> Result<Value> {
        // Update connection activity
        connection.update_activity().await;
        connection.record_message_received(serde_json::to_string(&message)?.len()).await;

        // Parse message using the existing processor
        if let Some(parsed_message) = self.processor.parse(&serde_json::to_string(&message)?) {
            match parsed_message {
                Message::SetDifficulty { difficulty: d } => {
                    self.handle_difficulty_update(message, connection, d).await
                }
                Message::Notify { job_id } => {
                    self.handle_job_notification(message, connection, job_id).await
                }
                _ => Ok(message),
            }
        } else if let Ok(response) = serde_json::from_value::<Response>(message.clone()) {
            // Handle submission responses
            self.handle_submission_response(message, connection, response).await
        } else {
            // Forward message as-is
            Ok(message)
        }
    }
}

/// Composite processor that chains multiple processors together
pub struct CompositeProcessor {
    processors: Vec<Arc<dyn MessageProcessor>>,
}

impl std::fmt::Debug for CompositeProcessor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompositeProcessor")
            .field("processor_count", &self.processors.len())
            .finish()
    }
}

impl CompositeProcessor {
    pub fn new() -> Self {
        Self {
            processors: Vec::new(),
        }
    }

    pub fn add_processor(mut self, processor: Arc<dyn MessageProcessor>) -> Self {
        self.processors.push(processor);
        self
    }

    pub fn with_logging(self, log_level: LogLevel) -> Self {
        self.add_processor(Arc::new(LoggingProcessor::new(log_level)))
    }

    pub fn with_stratum_processing(
        self,
        manager: Arc<Manager>,
        processor: Arc<processor::Manager>,
        difficulty: Arc<AtomicU64>,
    ) -> Self {
        self.add_processor(Arc::new(StratumProcessor::new(manager, processor, difficulty)))
    }
}

#[async_trait]
impl MessageProcessor for CompositeProcessor {
    async fn process_client_message(&self, mut message: Value, connection: &Arc<Connection>) -> Result<Value> {
        for processor in &self.processors {
            message = processor.process_client_message(message, connection).await?;
        }
        Ok(message)
    }

    async fn process_server_message(&self, mut message: Value, connection: &Arc<Connection>) -> Result<Value> {
        for processor in &self.processors {
            message = processor.process_server_message(message, connection).await?;
        }
        Ok(message)
    }
}

/// Rate limiting processor
#[derive(Debug)]
pub struct RateLimitProcessor {
    max_messages_per_minute: u64,
    // TODO: Add rate limiting implementation using a token bucket or sliding window
}

impl RateLimitProcessor {
    pub fn new(max_messages_per_minute: u64) -> Self {
        Self {
            max_messages_per_minute,
        }
    }
}

#[async_trait]
impl MessageProcessor for RateLimitProcessor {
    async fn process_client_message(&self, message: Value, connection: &Arc<Connection>) -> Result<Value> {
        // TODO: Implement rate limiting logic
        // For now, just pass through
        connection.update_activity().await;
        Ok(message)
    }

    async fn process_server_message(&self, message: Value, connection: &Arc<Connection>) -> Result<Value> {
        // Server messages are generally not rate limited
        connection.update_activity().await;
        Ok(message)
    }
}

/// Metrics collection processor
#[derive(Debug)]
pub struct MetricsProcessor {
    // TODO: Add metrics collection implementation
}

impl MetricsProcessor {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl MessageProcessor for MetricsProcessor {
    async fn process_client_message(&self, message: Value, connection: &Arc<Connection>) -> Result<Value> {
        // TODO: Collect metrics
        connection.update_activity().await;
        Ok(message)
    }

    async fn process_server_message(&self, message: Value, connection: &Arc<Connection>) -> Result<Value> {
        // TODO: Collect metrics
        connection.update_activity().await;
        Ok(message)
    }
}