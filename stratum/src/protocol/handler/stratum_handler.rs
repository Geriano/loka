//! Stratum V1 mining protocol handler implementation.
//!
//! This module provides specialized handling for Bitcoin Stratum V1 mining protocol
//! messages including authentication, job distribution, share submission, and
//! difficulty adjustments.

use serde_json::Value;
use std::net::SocketAddr;
use tracing::{debug, info, warn};

use crate::Config;
use crate::Manager;
use crate::error::Result;
use crate::protocol::messages::StratumMessage;

/// Stratum V1 protocol handler for mining operations.
///
/// Handles the core mining protocol operations including:
/// - Miner authentication and authorization
/// - Job distribution and updates
/// - Share submission and validation
/// - Difficulty target adjustments
/// - Session management
///
/// # Examples
///
/// ```rust
/// use loka_stratum::protocol::handler::StratumHandler;
///
/// let handler = StratumHandler::new();
/// // Handler processes parsed Stratum messages
/// ```
#[derive(Debug, Clone)]
pub struct StratumHandler {
    // Internal handler state if needed (placeholder)
}

impl StratumHandler {
    /// Create a new Stratum protocol handler.
    pub fn new() -> Self {
        Self {}
    }

    /// Handle a parsed Stratum message.
    ///
    /// Processes different types of Stratum messages based on their method
    /// and forwards them to the appropriate handlers.
    ///
    /// # Arguments
    ///
    /// * `message` - The parsed Stratum message
    /// * `raw_message` - The original raw message string  
    /// * `manager` - The main manager instance
    /// * `config` - Server configuration
    /// * `addr` - Client socket address
    /// * `context` - Optional message context containing metadata
    ///
    /// # Returns
    ///
    /// Result with optional response to send back to client
    pub async fn handle_parsed_message(
        &self,
        message: &StratumMessage,
        raw_message: &str,
        manager: &Manager,
        _config: &Config,
        addr: SocketAddr,
        context: Option<&crate::protocol::pipeline::MessageContext>,
    ) -> Result<Option<Value>> {
        debug!(
            "miner {} - Processing Stratum message for forwarding: method={}, id={:?}",
            addr,
            message.message_type(),
            message.id()
        );

        // Record metrics for the message
        manager.metrics().record_stratum_request_event();

        // Apply parameter normalization and prepare for forwarding
        // NO mock responses - everything is forwarded to real pool
        match message {
            StratumMessage::Subscribe {
                id: _,
                user_agent: _,
            } => {
                self.normalize_mining_subscribe(raw_message, manager, addr)
                    .await
            }
            StratumMessage::Authenticate {
                id: _,
                user: _,
                worker: _,
                password: _,
            } => {
                self.prepare_mining_authorize_forwarding(raw_message, manager, addr, context)
                    .await
            }
            StratumMessage::Submit { id: _, job_id: _ } => {
                self.prepare_mining_submit_forwarding(raw_message, manager, addr)
                    .await
            }
            _ => {
                // For other message types, forward as-is
                debug!(
                    "miner {} - Forwarding message as-is: {}",
                    addr,
                    message.message_type()
                );
                Ok(self.parse_raw_json(raw_message))
            }
        }
    }

    /// Normalize mining.subscribe parameters for forwarding to real pool.
    ///
    /// Fixes the parameter issue: ["cpuminer/2.5.1", "1"] → ["cpuminer/2.5.1"]
    /// by removing the second parameter when forwarding to upstream pool.
    async fn normalize_mining_subscribe(
        &self,
        raw_message: &str,
        manager: &Manager,
        addr: SocketAddr,
    ) -> Result<Option<Value>> {
        info!(
            "miner {} - Normalizing mining.subscribe for forwarding",
            addr
        );

        // Parse the original message
        match self.parse_raw_json(raw_message) {
            Some(mut json_value) => {
                // Fix the parameter issue for mining.subscribe
                if let Some(obj) = json_value.as_object_mut() {
                    if let Some(params) = obj.get_mut("params") {
                        if let Some(params_array) = params.as_array_mut() {
                            // Remove the second parameter if present (extranonce1 placeholder)
                            // This fixes: ["cpuminer/2.5.1", "1"] → ["cpuminer/2.5.1"]
                            if params_array.len() > 1 {
                                params_array.remove(1);
                                debug!(
                                    "miner {} - Normalized mining.subscribe parameters: {:?}",
                                    addr, params_array
                                );
                            }
                        }
                    }
                }

                // Record protocol detection success
                manager.metrics().record_protocol_detection_success_event();

                debug!(
                    "miner {} - Mining.subscribe normalized for pool forwarding",
                    addr
                );
                Ok(Some(json_value))
            }
            None => {
                warn!("miner {} - Failed to parse mining.subscribe message", addr);
                manager.metrics().record_protocol_error();
                Ok(None)
            }
        }
    }

    /// Prepare mining.authorize for forwarding to real pool.
    ///
    /// Records authentication metrics but forwards the request to the upstream
    /// pool for real authentication. No mock validation is performed.
    /// Uses pool format from context metadata if available.
    async fn prepare_mining_authorize_forwarding(
        &self,
        raw_message: &str,
        manager: &Manager,
        addr: SocketAddr,
        context: Option<&crate::protocol::pipeline::MessageContext>,
    ) -> Result<Option<Value>> {
        info!(
            "miner {} - Preparing mining.authorize for pool forwarding",
            addr
        );

        // Parse the original message to extract username for metrics
        if let Some(mut json_value) = self.parse_raw_json(raw_message) {
            // Check if we have pool format username from middleware context
            let pool_username = context
                .and_then(|ctx| ctx.metadata.get("pool_username"))
                .cloned();

            if let Some(obj) = json_value.as_object_mut() {
                if let Some(params) = obj.get_mut("params") {
                    if let Some(params_array) = params.as_array_mut() {
                        // Extract original username first
                        let original_username = params_array
                            .first()
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());

                        if let Some(ref pool_user) = pool_username {
                            if let Some(ref orig_user) = original_username {
                                // Replace original username with pool format
                                params_array[0] = serde_json::Value::String(pool_user.clone());
                                info!(
                                    "miner {} - Transformed authorize: {} -> {}",
                                    addr, orig_user, pool_user
                                );
                            }
                        } else if let Some(ref orig_user) = original_username {
                            debug!(
                                "miner {} - Forwarding authorize for user: {}",
                                addr, orig_user
                            );
                        }
                    }
                }
            }

            // Record that we're attempting authentication (real result will come from pool)
            manager.metrics().record_auth_attempt(true); // We don't know the result yet

            debug!(
                "miner {} - Mining.authorize prepared for pool forwarding",
                addr
            );
            Ok(Some(json_value))
        } else {
            warn!("miner {} - Failed to parse mining.authorize message", addr);
            manager.metrics().record_auth_failure();
            manager.metrics().record_protocol_error();
            Ok(None)
        }
    }

    /// Prepare mining.submit for forwarding to real pool.
    ///
    /// Records share submission metrics but forwards the actual share to the upstream
    /// pool for real validation. No mock validation is performed.
    async fn prepare_mining_submit_forwarding(
        &self,
        raw_message: &str,
        manager: &Manager,
        addr: SocketAddr,
    ) -> Result<Option<Value>> {
        info!(
            "miner {} - Preparing mining.submit for pool forwarding",
            addr
        );

        // Parse the original message to extract job_id and other details for metrics
        if let Some(json_value) = self.parse_raw_json(raw_message) {
            if let Some(obj) = json_value.as_object() {
                if let Some(params) = obj.get("params") {
                    if let Some(params_array) = params.as_array() {
                        // Extract job_id (typically at index 1) for logging
                        if params_array.len() >= 2 {
                            if let Some(job_id) = params_array.get(1).and_then(|v| v.as_str()) {
                                debug!("miner {} - Forwarding share for job: {}", addr, job_id);
                            }
                        }
                    }
                }
            }

            // Record the submission attempt (real result will come from pool)
            manager.metrics().record_share_submission_event();
            manager.metrics().record_submission_received();

            debug!(
                "miner {} - Mining.submit prepared for pool forwarding",
                addr
            );
            Ok(Some(json_value))
        } else {
            warn!("miner {} - Failed to parse mining.submit message", addr);
            manager.metrics().record_protocol_error();
            Ok(None)
        }
    }

    /// Parse raw JSON message for forwarding.
    ///
    /// Helper method to parse raw JSON messages while preserving structure
    /// for forwarding to upstream pools.
    fn parse_raw_json(&self, raw_message: &str) -> Option<Value> {
        serde_json::from_str::<Value>(raw_message.trim()).ok()
    }
}

impl Default for StratumHandler {
    fn default() -> Self {
        Self::new()
    }
}
