//! Message processing pipeline for parsing, validation, and routing.
//!
//! This module provides the message processing pipeline that handles:
//! - Raw message parsing and validation
//! - Protocol detection (HTTP vs Stratum)
//! - Message routing to appropriate handlers
//! - Rate limiting and security filtering

use std::net::SocketAddr;

use serde_json::Value;
use tracing::{debug, trace, warn};

use crate::Config;
use crate::Manager;
use crate::error::Result;
use crate::protocol::messages::StratumMessage;
use crate::protocol::pipeline::{MessageContext, MessagePipeline};

use super::http_handler::HttpHandler;
use super::stratum_handler::StratumHandler;

/// Message processor handling parsing, validation, and routing of protocol messages.
///
/// Acts as the central coordinator for processing incoming messages from miners,
/// determining their type (HTTP/Stratum), and routing them to the appropriate
/// specialized handlers.
///
/// # Examples
///
/// ```rust,no_run
/// use loka_stratum::protocol::handler::message_processor::MessageProcessor;
/// use loka_stratum::protocol::handler::http_handler::HttpHandler;
/// use loka_stratum::protocol::handler::stratum_handler::StratumHandler;
/// use loka_stratum::protocol::pipeline::MessagePipeline;
/// use loka_stratum::services::pool_config::{PoolConfigService, PoolConfigServiceConfig};
/// use loka_stratum::services::database::DatabaseService;
/// use std::sync::Arc;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let database = Arc::new(DatabaseService::new("sqlite::memory:").await?);
/// let config = PoolConfigServiceConfig::default();
/// let pool_service = Arc::new(PoolConfigService::new(database, config));
/// let pipeline = MessagePipeline::default();
/// let http_handler = HttpHandler::new(pool_service);
/// let stratum_handler = StratumHandler::new();
/// let processor = MessageProcessor::new(pipeline, http_handler, stratum_handler);
/// // Process raw messages from network connection
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct MessageProcessor {
    /// Message processing pipeline for validation and transformation
    pipeline: MessagePipeline,
    /// HTTP request handler
    http_handler: HttpHandler,
    /// Stratum protocol handler  
    stratum_handler: StratumHandler,
}

impl MessageProcessor {
    /// Create a new message processor.
    ///
    /// # Arguments
    ///
    /// * `pipeline` - The message processing pipeline
    /// * `http_handler` - Handler for HTTP protocol messages
    /// * `stratum_handler` - Handler for Stratum protocol messages
    pub fn new(
        pipeline: MessagePipeline,
        http_handler: HttpHandler,
        stratum_handler: StratumHandler,
    ) -> Self {
        Self {
            pipeline,
            http_handler,
            stratum_handler,
        }
    }

    /// Process a raw message from a client connection.
    ///
    /// Determines the message type, parses it appropriately, and routes
    /// it to the correct protocol handler.
    ///
    /// # Arguments
    ///
    /// * `raw_message` - The raw message string from the network
    /// * `manager` - The main manager instance
    /// * `config` - Server configuration  
    /// * `addr` - Client socket address
    ///
    /// # Returns
    ///
    /// Result with optional response to send back to client
    pub async fn process_message(
        &self,
        raw_message: &str,
        manager: &Manager,
        config: &Config,
        addr: SocketAddr,
    ) -> Result<Option<Value>> {
        let trimmed_message = raw_message.trim();

        if trimmed_message.is_empty() {
            trace!("miner {} - Ignoring empty message", addr);
            return Ok(None);
        }

        trace!("miner {} - Processing message: {}", addr, trimmed_message);

        // Record message receipt
        manager.metrics().record_messages_received(1);
        manager
            .metrics()
            .record_bytes_received(raw_message.len() as u64);

        // Detect protocol type
        let protocol_type = self.detect_protocol_type(trimmed_message);

        match protocol_type {
            ProtocolType::Http => {
                self.process_http_message(trimmed_message, manager, config, addr)
                    .await
            }
            ProtocolType::Stratum => {
                self.process_stratum_message(trimmed_message, manager, config, addr)
                    .await
            }
            ProtocolType::Unknown => {
                warn!(
                    "miner {} - Unknown protocol for message: {}",
                    addr, trimmed_message
                );
                manager.metrics().record_protocol_detection_failure_event();
                manager.metrics().record_protocol_error();
                Ok(None)
            }
        }
    }

    /// Detect the protocol type of a raw message.
    ///
    /// Analyzes the message format to determine if it's HTTP or Stratum protocol.
    fn detect_protocol_type(&self, message: &str) -> ProtocolType {
        // Check for HTTP request patterns
        if HttpHandler::is_direct_http_request(message) {
            return ProtocolType::Http;
        }

        // Check for CONNECT method (HTTP tunneling)
        if message.starts_with("CONNECT ") {
            return ProtocolType::Http;
        }

        // Check for JSON-RPC patterns (Stratum)
        if message.starts_with('{') && message.contains("\"method\"") {
            return ProtocolType::Stratum;
        }

        // Check for other JSON patterns
        if message.starts_with('{') || message.starts_with('[') {
            // Try parsing as JSON to confirm
            if serde_json::from_str::<Value>(message).is_ok() {
                return ProtocolType::Stratum;
            }
        }

        ProtocolType::Unknown
    }

    /// Process an HTTP protocol message.
    ///
    /// Routes HTTP messages to the HTTP handler for specialized processing.
    async fn process_http_message(
        &self,
        message: &str,
        manager: &Manager,
        _config: &Config,
        addr: SocketAddr,
    ) -> Result<Option<Value>> {
        debug!("miner {} - Processing as HTTP message", addr);

        // Record protocol detection
        manager.metrics().record_http_request_event();
        manager.metrics().record_protocol_detection_success_event();

        // Extract any path information
        if let Some(path) = HttpHandler::extract_path_from_message(message) {
            debug!("miner {} - Extracted HTTP path: {}", addr, path);
        }

        // For HTTP CONNECT messages, extract target
        if message.starts_with("CONNECT ") {
            if let Some(target) = HttpHandler::parse_connect_target_from_message(message) {
                debug!("miner {} - CONNECT target: {}", addr, target);
                manager.metrics().record_http_connect_request_event();

                // Validate the target
                let is_valid = self
                    .http_handler
                    .validate_connect_target(&target, addr)
                    .await?;
                if !is_valid {
                    manager.metrics().record_protocol_conversion_error_event();
                    return Ok(None);
                }
            }
        }

        // Note: Full HTTP handling would require access to the connection streams
        // This is a simplified version for the refactored structure
        Ok(None)
    }

    /// Process a Stratum protocol message.
    ///
    /// Parses JSON-RPC messages and routes them to the Stratum handler.
    async fn process_stratum_message(
        &self,
        message: &str,
        manager: &Manager,
        config: &Config,
        addr: SocketAddr,
    ) -> Result<Option<Value>> {
        debug!("miner {} - Processing as Stratum message", addr);

        // Record protocol detection
        manager.metrics().record_stratum_request_event();
        manager.metrics().record_protocol_detection_success_event();

        // Parse the message through the pipeline
        let context = MessageContext::new(message.to_string()).with_client_info(None, Some(addr));
        let processed_context = match self.pipeline.process(context).await {
            Ok(ctx) => ctx,
            Err(e) => {
                warn!("miner {} - Pipeline processing failed: {}", addr, e);
                manager.metrics().record_protocol_error();
                return Ok(None);
            }
        };

        // Extract the parsed message
        if let Some(stratum_message) = &processed_context.parsed_message {
            // Route to Stratum handler and return response
            return self
                .stratum_handler
                .handle_parsed_message(
                    stratum_message,
                    message,
                    manager,
                    config,
                    addr,
                    Some(&processed_context),
                )
                .await;
        } else {
            // Try manual parsing as fallback
            match serde_json::from_str::<StratumMessage>(message) {
                Ok(parsed_message) => {
                    return self
                        .stratum_handler
                        .handle_parsed_message(
                            &parsed_message,
                            message,
                            manager,
                            config,
                            addr,
                            None,
                        )
                        .await;
                }
                Err(e) => {
                    warn!("miner {} - Failed to parse Stratum message: {}", addr, e);
                    manager.metrics().record_protocol_error();
                }
            }
        }

        Ok(None)
    }

    /// Apply rate limiting to message processing.
    ///
    /// Checks if the client is within allowed message rate limits.
    pub fn should_rate_limit(&self, _addr: SocketAddr, _manager: &Manager) -> bool {
        // This would typically check against a rate limiter service
        // For now, always allow (no rate limiting)
        false
    }

    /// Record protocol detection metrics.
    ///
    /// Updates metrics based on successful or failed protocol detection.
    pub fn record_protocol_detection(
        &self,
        protocol_type: ProtocolType,
        success: bool,
        manager: &Manager,
    ) {
        match protocol_type {
            ProtocolType::Http => {
                manager.metrics().record_http_request_event();
            }
            ProtocolType::Stratum => {
                manager.metrics().record_stratum_request_event();
            }
            ProtocolType::Unknown => {
                // Unknown protocol
            }
        }

        if success {
            manager.metrics().record_protocol_detection_success_event();
            manager
                .metrics()
                .record_protocol_conversion_success_event(1.0);
        } else {
            manager.metrics().record_protocol_detection_failure_event();
            manager.metrics().record_protocol_conversion_error_event();
        }
    }

    /// Validate message format and structure.
    ///
    /// Performs basic validation on message structure before processing.
    pub fn validate_message_format(&self, message: &str) -> Result<bool> {
        // Check message length limits
        if message.len() > 16384 {
            // 16KB limit
            return Ok(false);
        }

        // Check for basic structural validity
        let trimmed = message.trim();
        if trimmed.is_empty() {
            return Ok(false);
        }

        // For JSON messages, ensure they can be parsed
        if trimmed.starts_with('{') {
            return Ok(serde_json::from_str::<Value>(trimmed).is_ok());
        }

        // For HTTP messages, check for basic structure
        if HttpHandler::is_direct_http_request(trimmed) {
            return Ok(trimmed.contains(" HTTP/"));
        }

        // Allow other formats through
        Ok(true)
    }
}

/// Protocol types that can be detected and handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolType {
    /// HTTP protocol (including CONNECT)
    Http,
    /// Stratum mining protocol (JSON-RPC)
    Stratum,
    /// Unknown or unsupported protocol
    Unknown,
}

impl Default for MessageProcessor {
    fn default() -> Self {
        // This would need to be updated with proper initialization
        // when integrated with the main codebase
        todo!("MessageProcessor::default() needs proper pipeline initialization")
    }
}
