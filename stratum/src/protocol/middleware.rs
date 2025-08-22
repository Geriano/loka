use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use dashmap::DashMap;
use tracing::{debug, info, warn};

use crate::config::types::PoolConfig;
use crate::error::{Result, StratumError};
use crate::protocol::messages::StratumMessage;
use crate::protocol::parser::StratumParser;
use crate::protocol::pipeline::{MessageContext, Middleware};

/// Middleware for parsing raw messages into StratumMessage
#[derive(Debug)]
pub struct ParsingMiddleware {
    parser: StratumParser,
}

impl ParsingMiddleware {
    pub fn new() -> Self {
        Self {
            parser: StratumParser::new(),
        }
    }
}

impl Default for ParsingMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Middleware for ParsingMiddleware {
    async fn process(&self, mut context: MessageContext) -> Result<MessageContext> {
        match self.parser.parse_message(&context.raw_message)? {
            Some(message) => {
                context = context.with_parsed_message(message);
                Ok(context)
            }
            None => Err(StratumError::Protocol {
                message: "Failed to parse message".to_string(),
                method: None,
                request_id: None,
            }),
        }
    }
}

/// Middleware for validating parsed messages
#[derive(Debug)]
pub struct ValidationMiddleware;

impl ValidationMiddleware {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ValidationMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Middleware for ValidationMiddleware {
    async fn process(&self, context: MessageContext) -> Result<MessageContext> {
        if let Some(ref message) = context.parsed_message {
            match message {
                StratumMessage::Authenticate { user, worker, .. } => {
                    if user.is_empty() {
                        return Err(StratumError::Protocol {
                            message: "User cannot be empty".to_string(),
                            method: Some("mining.authorize".to_string()),
                            request_id: None,
                        });
                    }
                    if worker.is_empty() {
                        return Err(StratumError::Protocol {
                            message: "Worker cannot be empty".to_string(),
                            method: Some("mining.authorize".to_string()),
                            request_id: None,
                        });
                    }
                }
                StratumMessage::SetDifficulty { difficulty } => {
                    if *difficulty <= 0.0 {
                        return Err(StratumError::Protocol {
                            message: "Difficulty must be positive".to_string(),
                            method: None,
                            request_id: None,
                        });
                    }
                }
                StratumMessage::Submit { job_id, .. } => {
                    if job_id.is_empty() {
                        return Err(StratumError::Protocol {
                            message: "Job ID cannot be empty".to_string(),
                            method: None,
                            request_id: None,
                        });
                    }
                }
                _ => {} // Other messages pass validation
            }
        }
        Ok(context)
    }
}

/// Rate limiting data per client
#[derive(Debug)]
struct RateLimitData {
    requests: AtomicU64,
    window_start: SystemTime,
}

/// Middleware for rate limiting based on client IP/ID
#[derive(Debug)]
pub struct RateLimitingMiddleware {
    max_requests_per_minute: u64,
    client_data: DashMap<String, RateLimitData>,
}

impl RateLimitingMiddleware {
    pub fn new(max_requests_per_minute: u64) -> Self {
        Self {
            max_requests_per_minute,
            client_data: DashMap::new(),
        }
    }

    fn get_client_key(&self, context: &MessageContext) -> String {
        if let Some(ref client_id) = context.client_id {
            format!("client:{}", client_id)
        } else if let Some(client_ip) = context.client_ip {
            format!("ip:{}", client_ip)
        } else {
            "unknown".to_string()
        }
    }

    fn should_rate_limit(&self, client_key: &str) -> bool {
        let now = SystemTime::now();

        // Clean up old entries
        self.client_data.retain(|_, data| {
            now.duration_since(data.window_start)
                .unwrap_or(Duration::from_secs(0))
                < Duration::from_secs(60)
        });

        let mut should_limit = false;

        self.client_data
            .entry(client_key.to_string())
            .and_modify(|data| {
                let window_age = now
                    .duration_since(data.window_start)
                    .unwrap_or(Duration::from_secs(0));

                if window_age >= Duration::from_secs(60) {
                    // Reset window
                    data.window_start = now;
                    data.requests.store(1, Ordering::Relaxed);
                } else {
                    let current_requests = data.requests.fetch_add(1, Ordering::Relaxed) + 1;
                    if current_requests > self.max_requests_per_minute {
                        should_limit = true;
                    }
                }
            })
            .or_insert_with(|| RateLimitData {
                requests: AtomicU64::new(1),
                window_start: now,
            });

        should_limit
    }
}

#[async_trait]
impl Middleware for RateLimitingMiddleware {
    async fn process(&self, context: MessageContext) -> Result<MessageContext> {
        let client_key = self.get_client_key(&context);

        if self.should_rate_limit(&client_key) {
            warn!("Rate limiting client: {}", client_key);
            return Err(StratumError::Protocol {
                message: "Rate limit exceeded".to_string(),
                method: None,
                request_id: None,
            });
        }

        Ok(context)
    }
}

/// Middleware for handling authentication messages
#[derive(Debug)]
pub struct AuthenticationMiddleware {
    authenticated_clients: DashMap<String, bool>,
    pool_config: PoolConfig,
}

impl AuthenticationMiddleware {
    pub fn new(pool_config: PoolConfig) -> Self {
        Self {
            authenticated_clients: DashMap::new(),
            pool_config,
        }
    }

    /// Transform miner username to pool-compatible format
    /// Converts "john.test" to "37vuX2XMqtcrobGwxSZJSwJoYyjiH18SiQ.john_test"
    fn transform_username(&self, user: &str, worker: &str) -> String {
        let (from_separator, to_separator) = &self.pool_config.separator;

        // Combine user and worker to create the full miner identifier
        let full_miner_username = format!("{}.{}", user, worker);

        // Transform the miner username: replace dots with underscores (or configured separator)
        let transformed_worker = full_miner_username.replace(from_separator, to_separator);

        // Combine pool username with transformed miner username
        format!("{}.{}", self.pool_config.username, transformed_worker)
    }
}

impl Default for AuthenticationMiddleware {
    fn default() -> Self {
        // Use default pool config when no config is provided
        Self::new(PoolConfig {
            address: "127.0.0.1:4444".to_string(),
            name: "default".to_string(),
            host: None,
            port: None,
            username: "default_user".to_string(),
            password: None,
            separator: (".".to_string(), "_".to_string()),
            extranonce: false,
        })
    }
}

#[async_trait]
impl Middleware for AuthenticationMiddleware {
    async fn process(&self, mut context: MessageContext) -> Result<MessageContext> {
        if let Some(ref message) = context.parsed_message {
            match message {
                StratumMessage::Authenticate { user, worker, .. } => {
                    let client_key = format!("{}:{}", user, worker);
                    let user_clone = user.clone();
                    let worker_clone = worker.clone();

                    // Transform username to pool-compatible format
                    let pool_username = self.transform_username(user, worker);

                    // Simple authentication - accept all for now
                    // TODO: Add actual authentication logic
                    self.authenticated_clients.insert(client_key.clone(), true);
                    context.set_metadata("authenticated".to_string(), "true".to_string());
                    context.set_metadata("auth_key".to_string(), client_key);
                    context.set_metadata("pool_username".to_string(), pool_username.clone());

                    info!(
                        "Client authenticated: {}.{} (pool format: {})",
                        user_clone, worker_clone, pool_username
                    );
                }
                StratumMessage::Subscribe { .. } => {
                    // Subscribe is allowed without authentication (it's the first message miners send)
                    debug!("Subscribe request received (no authentication required)");
                }
                _ => {
                    // Check if client is authenticated for non-auth/non-subscribe messages
                    if let Some(client_id) = &context.client_id {
                        if !self.authenticated_clients.contains_key(client_id) {
                            return Err(StratumError::Protocol {
                                message: "Client not authenticated".to_string(),
                                method: None,
                                request_id: None,
                            });
                        }
                    }
                }
            }
        }
        Ok(context)
    }
}

/// Middleware for logging message processing
#[derive(Debug)]
pub struct LoggingMiddleware;

impl LoggingMiddleware {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LoggingMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Middleware for LoggingMiddleware {
    async fn process(&self, context: MessageContext) -> Result<MessageContext> {
        if let Some(ref message) = context.parsed_message {
            debug!(
                "Processing message: type={}, client_id={:?}, client_ip={:?}",
                message.message_type(),
                context.client_id,
                context.client_ip
            );

            match message {
                StratumMessage::Authenticate { user, worker, .. } => {
                    info!("Authentication request: user={}, worker={}", user, worker);
                }
                StratumMessage::Submit { id, job_id, .. } => {
                    debug!("Share submission: id={}, job_id={}", id, job_id);
                }
                StratumMessage::SetDifficulty { difficulty } => {
                    debug!("Difficulty update: {}", difficulty);
                }
                _ => {}
            }
        } else {
            warn!(
                "Processing message without parsed content: {}",
                context.raw_message
            );
        }

        Ok(context)
    }
}
