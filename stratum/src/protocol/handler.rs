use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::info;
use tracing::{debug, error, trace, warn};

use crate::Manager;
use crate::error::Result;
use crate::protocol::implementations::DefaultProtocolMetrics;
use crate::protocol::messages::StratumMessage;
use crate::protocol::pipeline::{MessageContext, MessagePipeline, PipelineBuilder};
use crate::protocol::traits::ProtocolMetrics;
use crate::services::pool_config::{PoolConfigService, PoolConfigServiceConfig};
use crate::{Config, auth};

/// Modern protocol-aware handler using the new pipeline architecture
#[derive(Debug)]
pub struct ProtocolHandler {
    manager: Arc<Manager>,
    config: Arc<Config>,
    addr: SocketAddr,
    connection_path: Arc<Mutex<Option<String>>>,
    difficulty: Arc<AtomicU64>,
    shutdown: Arc<AtomicBool>,
    pipeline: MessagePipeline,
    metrics: Arc<DefaultProtocolMetrics>,
    pool_service: PoolConfigService,
}

impl ProtocolHandler {
    pub fn new(
        manager: Arc<Manager>,
        config: Arc<Config>,
        addr: SocketAddr,
        connection_path: Option<String>,
    ) -> Self {
        // Build the processing pipeline
        let pipeline = PipelineBuilder::new()
            .with_logging() // Log all messages
            .with_parsing() // Parse raw messages
            .with_validation() // Validate messages
            .with_rate_limiting(60) // Rate limit clients
            .with_authentication(config.pool.clone()) // Handle authentication with pool config
            .build();

        // Create pool configuration service for target validation
        let pool_service = PoolConfigService::new(
            manager.database().clone(),
            PoolConfigServiceConfig::default(),
        );

        // Log the connection path if available
        if let Some(ref path) = connection_path {
            tracing::info!(
                "miner {} - Handler created with connection path: {}",
                addr,
                path
            );
        }

        Self {
            manager,
            config,
            addr,
            connection_path: Arc::new(Mutex::new(connection_path)),
            difficulty: Arc::new(AtomicU64::new(0)),
            shutdown: Arc::new(AtomicBool::new(false)),
            pipeline,
            metrics: Arc::new(DefaultProtocolMetrics::new()),
            pool_service,
        }
    }

    pub fn auth(&self) -> Option<Arc<auth::State>> {
        self.manager.auth().get(&self.addr)
    }

    pub fn difficulty(&self) -> f64 {
        f64::from_bits(self.difficulty.load(Ordering::Relaxed))
    }

    pub fn terminated(&self) -> bool {
        self.shutdown.load(Ordering::Relaxed)
    }

    pub fn metrics(&self) -> &DefaultProtocolMetrics {
        &self.metrics
    }

    pub fn connection_path(&self) -> Option<String> {
        self.connection_path.lock().unwrap().clone()
    }

    pub async fn run(&self, downstream: TcpStream, upstream: TcpStream) -> std::io::Result<()> {
        let (downstream_read, downstream_write) = downstream.into_split();
        let (upstream_read, upstream_write) = upstream.into_split();

        // Create channels for message flow
        let (downstream_to_upstream_tx, downstream_to_upstream_rx) =
            mpsc::unbounded_channel::<Value>();
        let (upstream_to_downstream_tx, upstream_to_downstream_rx) =
            mpsc::unbounded_channel::<Value>();

        // Spawn handlers
        let downstream_handler =
            self.spawn_downstream_handler(downstream_read, downstream_to_upstream_tx);

        let upstream_handler =
            self.spawn_upstream_handler(upstream_read, upstream_to_downstream_tx);

        let proxy_coordinator = self.spawn_proxy_coordinator(
            downstream_to_upstream_rx,
            upstream_to_downstream_rx,
            downstream_write,
            upstream_write,
        );

        // Wait for all handlers to complete
        let (downstream_result, upstream_result, proxy_result) =
            tokio::join!(downstream_handler, upstream_handler, proxy_coordinator);

        // Log results
        if let Err(e) = &downstream_result {
            error!("miner {} - Downstream handler error: {}", self.addr, e);
        }

        if let Err(e) = &upstream_result {
            error!("miner {} - Upstream handler error: {}", self.addr, e);
        }

        if let Err(e) = &proxy_result {
            error!("miner {} - Proxy coordinator error: {}", self.addr, e);
        }

        self.shutdown.store(true, Ordering::Release);
        Ok(())
    }

    fn spawn_downstream_handler(
        &self,
        downstream_read: OwnedReadHalf,
        downstream_to_upstream_tx: mpsc::UnboundedSender<Value>,
    ) -> JoinHandle<std::io::Result<()>> {
        let manager = self.manager.clone();
        let config = self.config.clone();
        let pipeline = self.pipeline.clone();
        let metrics = self.metrics.clone();
        let shutdown = Arc::clone(&self.shutdown);
        let connection_path = Arc::clone(&self.connection_path);
        let pool_service = self.pool_service.clone();
        let addr = self.addr;

        tokio::spawn(async move {
            let mut reader = BufReader::new(downstream_read);
            let mut buffer = String::new();

            while !shutdown.load(Ordering::Relaxed) {
                buffer.clear();

                match reader.read_line(&mut buffer).await {
                    Ok(0) => {
                        debug!("miner {} - Downstream connection closed", addr);
                        break;
                    }
                    Ok(_) => {
                        let start_time = Instant::now();
                        let raw_message = buffer.trim().to_string();

                        // Check if this is an HTTP CONNECT request first
                        if raw_message.trim().starts_with("CONNECT ") {
                            info!(
                                "🔌 HTTP CONNECT DETECTED! Miner {} - Request: {}",
                                addr,
                                raw_message.trim()
                            );

                            // Extract target from CONNECT request
                            let extracted_target =
                                Self::parse_connect_target_from_message(&raw_message);

                            if let Some(ref target) = extracted_target {
                                tracing::info!(
                                    "🎯 TARGET EXTRACTED! Miner {} - HTTP CONNECT Target: {}",
                                    addr,
                                    target
                                );
                                if let Ok(mut path_guard) = connection_path.lock() {
                                    *path_guard = Some(target.clone());
                                }
                            } else {
                                error!(
                                    "❌ HTTP CONNECT detected but NO TARGET found for miner {}",
                                    addr
                                );
                            }

                            // Validate target and determine response
                            let (status_code, status_text) = match extracted_target.as_ref() {
                                Some(target) => {
                                    // Track validation start time for metrics
                                    let validation_start = std::time::Instant::now();

                                    // Validate the target against pool configuration database
                                    let result =
                                        match Self::validate_connect_target(&pool_service, target)
                                            .await
                                        {
                                            Ok(true) => {
                                                metrics.record_connect_validation(
                                                    "success",
                                                    validation_start.elapsed(),
                                                );
                                                (200, "Connection established")
                                            }
                                            Ok(false) => {
                                                metrics.record_connect_validation(
                                                    "not_found",
                                                    validation_start.elapsed(),
                                                );
                                                (404, "Not Found")
                                            }
                                            Err(_) => {
                                                metrics.record_connect_validation(
                                                    "error",
                                                    validation_start.elapsed(),
                                                );
                                                (500, "Internal Server Error")
                                            }
                                        };

                                    result
                                }
                                None => {
                                    metrics.record_connect_validation(
                                        "invalid_format",
                                        std::time::Duration::from_nanos(0),
                                    );
                                    (400, "Bad Request")
                                }
                            };

                            if let Err(e) = downstream_to_upstream_tx.send(serde_json::json!({
                                "__http_connect_response": {
                                    "status_code": status_code,
                                    "status_text": status_text,
                                    "target": extracted_target.unwrap_or_else(|| "unknown".to_string())
                                }
                            })) {
                                error!("miner {} - Failed to queue HTTP response: {}", addr, e);
                                break;
                            }
                            continue;
                        }

                        // Check if this is a direct HTTP request (POST/GET) and handle it specially
                        let trimmed_message = raw_message.trim();
                        if Self::is_direct_http_request(trimmed_message) {
                            tracing::info!(
                                "🌐 DIRECT HTTP REQUEST DETECTED! Miner {} - Request line: {}",
                                addr,
                                trimmed_message
                            );

                            // Extract path from HTTP request for tracking
                            if let Some(extracted_path) =
                                Self::extract_path_from_message(&raw_message)
                            {
                                tracing::info!(
                                    "🎯 HTTP PATH EXTRACTED! Miner {} - Path: {}",
                                    addr,
                                    extracted_path
                                );
                                if let Ok(mut path_guard) = connection_path.lock() {
                                    *path_guard = Some(extracted_path.clone());
                                }
                            }

                            // Track HTTP request detection in metrics
                            metrics.record_message(
                                "direct_http_request",
                                start_time.elapsed(),
                                true,
                            );

                            // Handle complete HTTP request - this will read all headers and body
                            // and prevent individual lines from being processed through JSON pipeline
                            let http_result = Self::handle_complete_http_request(
                                &mut reader,
                                &raw_message,
                                addr,
                                &downstream_to_upstream_tx,
                                &manager,
                                &config,
                                &pipeline,
                                &metrics,
                            )
                            .await;

                            match http_result {
                                Ok(_) => {
                                    // HTTP request handled completely - no further line processing needed
                                    tracing::debug!(
                                        "miner {} - HTTP request processing completed",
                                        addr
                                    );
                                    continue;
                                }
                                Err(e) => {
                                    error!("miner {} - Failed to handle HTTP request: {}", addr, e);
                                    // Send error response and break connection
                                    let _ = Self::send_http_error_response(
                                        &downstream_to_upstream_tx,
                                        500,
                                        &e.to_string(),
                                    )
                                    .await;
                                    break;
                                }
                            }
                        }

                        // Try to extract path information from message if not already captured
                        {
                            let path_guard = connection_path.lock().unwrap();
                            if path_guard.is_none() {
                                drop(path_guard); // Release the lock before calling extract_path_from_message

                                if let Some(extracted_path) =
                                    Self::extract_path_from_message(&raw_message)
                                {
                                    tracing::info!(
                                        "miner {} - Path extracted from message: {}",
                                        addr,
                                        extracted_path
                                    );
                                    match connection_path.lock() {
                                        Ok(mut path_guard) => {
                                            *path_guard = Some(extracted_path.clone());
                                            tracing::debug!(
                                                "miner {} - Path stored in connection context: {}",
                                                addr,
                                                extracted_path
                                            );
                                        }
                                        Err(e) => {
                                            error!(
                                                "miner {} - Failed to acquire path lock for storage: {}",
                                                addr, e
                                            );
                                            metrics.record_error(
                                                "path_lock_failure",
                                                &format!("addr:{}", addr),
                                            );
                                        }
                                    }
                                }
                            }
                        }

                        // Create message context
                        let context = MessageContext::new(raw_message.clone())
                            .with_client_info(Some(format!("client_{}", addr)), Some(addr));

                        // Process message through pipeline
                        match pipeline.process(context).await {
                            Ok(processed_context) => {
                                let processing_time = start_time.elapsed();

                                if let Some(parsed_message) = processed_context.parsed_message {
                                    metrics.record_message(
                                        parsed_message.message_type(),
                                        processing_time,
                                        true,
                                    );

                                    // Handle the processed message
                                    let send_result = Self::handle_parsed_message(
                                        &parsed_message,
                                        &raw_message,
                                        &manager,
                                        &config,
                                        addr,
                                        &downstream_to_upstream_tx,
                                    )
                                    .await;

                                    if let Err(e) = send_result {
                                        error!("miner {} - Failed to handle message: {}", addr, e);
                                        metrics.record_error(
                                            "message_handling",
                                            &format!("addr:{}", addr),
                                        );
                                        break;
                                    }
                                } else {
                                    // Forward unparsed messages as-is
                                    match serde_json::from_str::<Value>(&raw_message) {
                                        Ok(request) => {
                                            trace!(
                                                "miner {} - Forwarding unparsed message: {}",
                                                addr,
                                                serde_json::to_string(&request).unwrap_or_default()
                                            );

                                            if let Err(e) = downstream_to_upstream_tx.send(request)
                                            {
                                                error!(
                                                    "miner {} - Failed to forward message: {}",
                                                    addr, e
                                                );
                                                break;
                                            }
                                        }
                                        Err(e) => {
                                            warn!("miner {} - Invalid JSON: {}", addr, e);
                                            metrics.record_error(
                                                "invalid_json",
                                                &format!("addr:{}", addr),
                                            );
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                let processing_time = start_time.elapsed();
                                metrics.record_message("unknown", processing_time, false);
                                warn!("miner {} - Pipeline processing failed: {}", addr, e);
                                metrics
                                    .record_error("pipeline_processing", &format!("addr:{}", addr));
                            }
                        }
                    }
                    Err(e) => {
                        error!("miner {} - Error reading from downstream: {}", addr, e);
                        metrics.record_error("downstream_read", &format!("addr:{}", addr));
                        break;
                    }
                }
            }

            debug!("miner {} - Downstream handler finished", addr);
            Ok(())
        })
    }

    async fn handle_parsed_message(
        message: &StratumMessage,
        raw_message: &str,
        manager: &Manager,
        config: &Config,
        addr: SocketAddr,
        downstream_to_upstream_tx: &mpsc::UnboundedSender<Value>,
    ) -> Result<()> {
        let send_result = match message {
            StratumMessage::Authenticate { user, worker, .. } => {
                // Handle authentication
                manager
                    .submissions()
                    .authenticated(manager.auth().authenticate(addr, user, worker));

                // Modify message for upstream
                let mut upstream_message = serde_json::from_str::<Value>(raw_message)?;
                let username = &config.pool.username;
                let password = config.pool.password.as_deref().unwrap_or("x");
                let (s1, s2) = &config.pool.separator;
                let current = format!("{}{}{}{}{}", username, s1, user, s2, worker);

                debug!("authenticate {}.{} as {}", user, worker, current);

                upstream_message["params"] = Value::Array(vec![
                    Value::String(current),
                    Value::String(password.to_owned()),
                ]);

                downstream_to_upstream_tx
                    .send(upstream_message)
                    .map_err(|e| crate::error::StratumError::Network {
                        message: format!("Failed to send to upstream: {}", e),
                        source: None,
                    })
            }
            StratumMessage::Submit { id, job_id } => {
                // Handle submission
                let jobs = manager.jobs();
                if let Some(job) = jobs.get(job_id) {
                    if let Some(auth) = manager.auth().get(&addr) {
                        manager.submissions().submit(*id, job, auth);
                    }
                }

                // Forward original message
                let original_value = serde_json::from_str::<Value>(raw_message)?;
                downstream_to_upstream_tx.send(original_value).map_err(|e| {
                    crate::error::StratumError::Network {
                        message: format!("Failed to send original message to upstream: {}", e),
                        source: None,
                    }
                })
            }
            _ => {
                // Forward other messages as-is
                let original_value = serde_json::from_str::<Value>(raw_message)?;
                downstream_to_upstream_tx.send(original_value).map_err(|e| {
                    crate::error::StratumError::Network {
                        message: format!("Failed to forward message to upstream: {}", e),
                        source: None,
                    }
                })
            }
        };

        send_result?;
        Ok(())
    }

    fn spawn_upstream_handler(
        &self,
        upstream_read: OwnedReadHalf,
        upstream_to_downstream_tx: mpsc::UnboundedSender<Value>,
    ) -> JoinHandle<std::io::Result<()>> {
        let shutdown = Arc::clone(&self.shutdown);
        let metrics = self.metrics.clone();
        let addr = self.addr;

        tokio::spawn(async move {
            let mut reader = BufReader::new(upstream_read);
            let mut buffer = String::new();

            while !shutdown.load(Ordering::Relaxed) {
                buffer.clear();

                match reader.read_line(&mut buffer).await {
                    Ok(0) => {
                        debug!("miner {} - Upstream connection closed", addr);
                        break;
                    }
                    Ok(_) => match serde_json::from_str::<Value>(buffer.trim()) {
                        Ok(response) => {
                            trace!(
                                "miner {} - Received from upstream: {}",
                                addr,
                                serde_json::to_string(&response).unwrap_or_default()
                            );

                            if let Err(e) = upstream_to_downstream_tx.send(response) {
                                error!(
                                    "miner {} - Failed to send to downstream channel: {}",
                                    addr, e
                                );
                                break;
                            }
                        }
                        Err(e) => {
                            warn!("miner {} - Invalid upstream JSON: {}", addr, e);
                            metrics
                                .record_error("invalid_upstream_json", &format!("addr:{}", addr));
                        }
                    },
                    Err(e) => {
                        error!("miner {} - Error reading from upstream: {}", addr, e);
                        metrics.record_error("upstream_read", &format!("addr:{}", addr));
                        break;
                    }
                }
            }

            debug!("miner {} - Upstream handler finished", addr);
            Ok(())
        })
    }

    fn spawn_proxy_coordinator(
        &self,
        mut downstream_to_upstream_rx: mpsc::UnboundedReceiver<Value>,
        mut upstream_to_downstream_rx: mpsc::UnboundedReceiver<Value>,
        downstream_write: OwnedWriteHalf,
        upstream_write: OwnedWriteHalf,
    ) -> JoinHandle<std::io::Result<()>> {
        let shutdown = Arc::clone(&self.shutdown);
        let metrics = self.metrics.clone();
        let addr = self.addr;

        tokio::spawn(async move {
            let mut downstream_writer = BufWriter::new(downstream_write);
            let mut upstream_writer = BufWriter::new(upstream_write);
            let mut http_mode = false; // Track if this connection is in HTTP mode

            while !shutdown.load(Ordering::Relaxed) {
                tokio::select! {
                    Some(message) = downstream_to_upstream_rx.recv() => {
                        // Check for special HTTP mode marker
                        if message.get("__http_mode").is_some() {
                            http_mode = true;
                            tracing::debug!("miner {} - Enabled HTTP mode for responses", addr);
                            continue;
                        }

                        // Check for special HTTP response to send directly
                        if let Some(response_str) = message.get("__http_response").and_then(|v| v.as_str()) {
                            if let Err(e) = downstream_writer.write_all(response_str.as_bytes()).await {
                                error!("miner {} - Error sending HTTP response: {}", addr, e);
                                break;
                            }
                            if let Err(e) = downstream_writer.flush().await {
                                error!("miner {} - Error flushing HTTP response: {}", addr, e);
                                break;
                            }
                            continue;
                        }

                        // Check if this is a special HTTP CONNECT response marker
                        if let Some(http_response) = message.get("__http_connect_response") {
                            // Extract response details
                            let status_code = http_response.get("status_code")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(500) as u16;
                            let status_text = http_response.get("status_text")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Internal Server Error");
                            let target = http_response.get("target")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");

                            // Format HTTP response based on status code
                            let response = Self::format_http_response(status_code, status_text);

                            // Send HTTP response directly to downstream client
                            if let Err(e) = downstream_writer.write_all(response.as_bytes()).await {
                                error!("miner {} - Error sending HTTP CONNECT response: {}", addr, e);
                                break;
                            }
                            if let Err(e) = downstream_writer.flush().await {
                                error!("miner {} - Error flushing HTTP response: {}", addr, e);
                                break;
                            }

                            // Log the response
                            if status_code == 200 {
                                tracing::info!("✅ HTTP TUNNEL ESTABLISHED! Miner {} - Target: {} - Ready for Stratum protocol", addr, target);
                            } else {
                                tracing::warn!("❌ HTTP CONNECT REJECTED! Miner {} - Target: {} - Status: {} {}", addr, target, status_code, status_text);
                            }
                            continue;
                        }

                        // Skip internal control messages
                        if message.get("__request_processed").is_some() {
                            continue;
                        }

                        let json = serde_json::to_string(&message).unwrap_or_default();
                        trace!("miner {} - Sending to upstream: {}", addr, json);

                        if let Err(e) = upstream_writer.write_all((json + "\n").as_bytes()).await {
                            error!("miner {} - Error writing to upstream: {}", addr, e);
                            metrics.record_error("upstream_write", &format!("addr:{}", addr));
                            break;
                        }

                        if let Err(e) = upstream_writer.flush().await {
                            error!("miner {} - Error flushing upstream: {}", addr, e);
                            break;
                        }
                    }
                    Some(message) = upstream_to_downstream_rx.recv() => {
                        let json = serde_json::to_string(&message).unwrap_or_default();
                        trace!("miner {} - Sending to downstream: {}", addr, json);

                        // If in HTTP mode, wrap the JSON-RPC response in HTTP
                        if http_mode {
                            let response_body = json.clone();
                            let http_response = format!(
                                "HTTP/1.1 200 OK\r\n\
                                 Content-Type: application/json\r\n\
                                 Content-Length: {}\r\n\
                                 Connection: close\r\n\
                                 \r\n\
                                 {}",
                                response_body.len(),
                                response_body
                            );

                            tracing::debug!("miner {} - Sending HTTP-wrapped response", addr);

                            if let Err(e) = downstream_writer.write_all(http_response.as_bytes()).await {
                                error!("miner {} - Error writing HTTP response to downstream: {}", addr, e);
                                metrics.record_error("downstream_write", &format!("addr:{}", addr));
                                break;
                            }
                        } else {
                            // Normal Stratum mode - send JSON with newline
                            if let Err(e) = downstream_writer.write_all((json + "\n").as_bytes()).await {
                                error!("miner {} - Error writing to downstream: {}", addr, e);
                                metrics.record_error("downstream_write", &format!("addr:{}", addr));
                                break;
                            }
                        }

                        if let Err(e) = downstream_writer.flush().await {
                            error!("miner {} - Error flushing downstream: {}", addr, e);
                            break;
                        }
                    }
                    else => break,
                }
            }

            debug!("miner {} - Proxy coordinator finished", addr);
            Ok(())
        })
    }

    /// Extract path information from HTTP requests or JSON messages
    /// First checks for direct HTTP requests (POST/GET), then looks for URL patterns in mining.subscribe
    pub fn extract_path_from_message(raw_message: &str) -> Option<String> {
        let trimmed = raw_message.trim();

        // First check for direct HTTP requests (POST, GET, etc.)
        if let Some(path) = Self::parse_http_request_path(trimmed) {
            return Some(path);
        }

        // If not HTTP request, try parsing as JSON
        if let Ok(json) = serde_json::from_str::<Value>(raw_message) {
            // Check if it's a mining.subscribe message with URL in params
            if let Some(method) = json.get("method").and_then(|m| m.as_str()) {
                if method == "mining.subscribe" {
                    if let Some(params) = json.get("params").and_then(|p| p.as_array()) {
                        // Look for URL in the first parameter (user agent)
                        if let Some(user_agent) = params.get(0).and_then(|p| p.as_str()) {
                            // Check if user agent contains URL with path
                            if let Some(path) = Self::parse_url_path_from_string(user_agent) {
                                return Some(path);
                            }
                        }

                        // Look for URL in any parameter
                        for param in params {
                            if let Some(param_str) = param.as_str() {
                                if let Some(path) = Self::parse_url_path_from_string(param_str) {
                                    return Some(path);
                                }
                            }
                        }
                    }
                }
            }

            // Check for URL in any field of the JSON
            Self::search_json_for_url_path(&json)
        } else {
            None
        }
    }

    /// Check if a message is a direct HTTP request (not CONNECT)
    /// Identifies requests like "POST /path HTTP/1.1" or "GET /status HTTP/1.1"
    pub fn is_direct_http_request(line: &str) -> bool {
        let parts: Vec<&str> = line.split_whitespace().collect();

        // HTTP request format: METHOD PATH HTTP/VERSION
        if parts.len() >= 3 {
            let method = parts[0];
            let protocol = parts[2];

            // Check for HTTP methods (excluding CONNECT which is handled separately)
            (method == "POST" || method == "GET" || method == "PUT" || method == "DELETE")
                && (protocol.starts_with("HTTP/1.") || protocol.starts_with("HTTP/2"))
        } else {
            false
        }
    }

    /// Parse path from direct HTTP request lines
    /// Supports POST and GET requests like "POST /order-1 HTTP/1.1" or "GET /status HTTP/1.1"
    pub fn parse_http_request_path(line: &str) -> Option<String> {
        // Check if this is an HTTP request line
        let parts: Vec<&str> = line.split_whitespace().collect();

        // HTTP request format: METHOD PATH HTTP/VERSION
        if parts.len() >= 3 {
            let method = parts[0];
            let path = parts[1];
            let protocol = parts[2];

            // Check for supported HTTP methods and HTTP/1.1 protocol
            if (method == "POST" || method == "GET" || method == "PUT" || method == "DELETE")
                && (protocol.starts_with("HTTP/1.") || protocol.starts_with("HTTP/2"))
            {
                // Validate path starts with /
                if path.starts_with('/') && path.len() > 1 {
                    // Extract path without query params or fragments
                    let clean_path = if let Some(query_pos) = path.find('?') {
                        &path[1..query_pos] // Remove leading / and query params
                    } else if let Some(fragment_pos) = path.find('#') {
                        &path[1..fragment_pos] // Remove leading / and fragment
                    } else {
                        &path[1..] // Remove leading /
                    };

                    // Basic validation: path should contain only alphanumeric chars, hyphens, underscores, and slashes
                    if !clean_path.is_empty()
                        && clean_path
                            .chars()
                            .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '/')
                    {
                        return Some(clean_path.to_string());
                    }
                }
            }
        }

        None
    }

    /// Parse URL path from a string that may contain stratum+tcp://host:port/path
    fn parse_url_path_from_string(text: &str) -> Option<String> {
        // Look for stratum+tcp:// URLs with paths
        if let Some(start) = text.find("stratum+tcp://") {
            let url_part = &text[start..];
            // Find the first slash after the protocol and host
            if let Some(proto_end) = url_part.find("://") {
                let after_proto = &url_part[proto_end + 3..];
                if let Some(path_start) = after_proto.find('/') {
                    let path_part = &after_proto[path_start + 1..];
                    // Extract until whitespace, query params, or fragment
                    let path_end = path_part
                        .find(|c: char| c.is_whitespace() || c == '?' || c == '#')
                        .unwrap_or(path_part.len());
                    let path = &path_part[..path_end];
                    if !path.is_empty() && path != "/" {
                        return Some(path.to_string());
                    }
                }
            }
        }

        // Look for paths that start with / (like /order-1)
        if let Some(start) = text.find('/') {
            let path_part = &text[start + 1..];
            let path_end = path_part
                .find(|c: char| c.is_whitespace() || c == '?' || c == '#')
                .unwrap_or(path_part.len());
            let path = &path_part[..path_end];
            if !path.is_empty()
                && path
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
            {
                return Some(path.to_string());
            }
        }

        None
    }

    /// Recursively search JSON for URL paths
    fn search_json_for_url_path(value: &Value) -> Option<String> {
        match value {
            Value::String(s) => Self::parse_url_path_from_string(s),
            Value::Array(arr) => {
                for item in arr {
                    if let Some(path) = Self::search_json_for_url_path(item) {
                        return Some(path);
                    }
                }
                None
            }
            Value::Object(obj) => {
                for (_, v) in obj {
                    if let Some(path) = Self::search_json_for_url_path(v) {
                        return Some(path);
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Parse target from HTTP CONNECT message
    /// Format: "CONNECT host:port HTTP/1.1"
    pub fn parse_connect_target_from_message(message: &str) -> Option<String> {
        let line = message.trim();
        if line.starts_with("CONNECT ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let target = parts[1];

                // HTTP CONNECT target should be host:port (no path)
                // Validate format and return the host:port target
                if target.contains(':') && !target.contains('/') {
                    // Basic validation: should have format host:port
                    if let Some(colon_pos) = target.rfind(':') {
                        let (host, port_str) = target.split_at(colon_pos);
                        let port_str = &port_str[1..]; // Remove the colon

                        // Validate port is numeric
                        if port_str.parse::<u16>().is_ok() && !host.is_empty() {
                            return Some(target.to_string());
                        }
                    }
                }
            }
        }
        None
    }

    /// Format HTTP response for different status codes
    fn format_http_response(status_code: u16, status_text: &str) -> String {
        match status_code {
            200 => format!("HTTP/1.1 200 Connection established\r\n\r\n"),
            400 => format!("HTTP/1.1 400 Bad Request\r\n\r\nInvalid CONNECT target format"),
            404 => format!("HTTP/1.1 404 Not Found\r\n\r\nPool target not found"),
            403 => format!("HTTP/1.1 403 Forbidden\r\n\r\nPool target is inactive"),
            500 => format!(
                "HTTP/1.1 500 Internal Server Error\r\n\r\nDatabase error during pool lookup"
            ),
            502 => format!("HTTP/1.1 502 Bad Gateway\r\n\r\nUnable to connect to upstream pool"),
            _ => format!(
                "HTTP/1.1 {} {}\r\n\r\n{}",
                status_code, status_text, status_text
            ),
        }
    }

    /// Validate HTTP CONNECT target against pool configuration database
    async fn validate_connect_target(
        pool_service: &PoolConfigService,
        target: &str,
    ) -> std::result::Result<bool, crate::error::StratumError> {
        match pool_service.get_pool_config_by_target(target).await {
            Ok(Some(_config)) => {
                tracing::debug!("CONNECT target {} validated successfully", target);
                Ok(true)
            }
            Ok(None) => {
                tracing::warn!("CONNECT target {} not found in pool configurations", target);
                Ok(false)
            }
            Err(e) => {
                tracing::error!("Database error validating CONNECT target {}: {}", target, e);
                Err(e)
            }
        }
    }

    /// Handle complete HTTP requests by reading all headers and body in one go,
    /// preventing individual HTTP lines from going through the JSON pipeline
    async fn handle_complete_http_request(
        reader: &mut BufReader<OwnedReadHalf>,
        request_line: &str,
        addr: SocketAddr,
        downstream_to_upstream_tx: &mpsc::UnboundedSender<Value>,
        manager: &Manager,
        config: &Config,
        pipeline: &MessagePipeline,
        metrics: &DefaultProtocolMetrics,
    ) -> Result<()> {
        use std::collections::HashMap;

        tracing::debug!("miner {} - Handling HTTP request: {}", addr, request_line);

        // Parse HTTP headers
        let mut headers = HashMap::new();
        let mut content_length: Option<usize> = None;
        let mut buffer = String::new();

        // Read headers until we find empty line
        loop {
            buffer.clear();
            match reader.read_line(&mut buffer).await {
                Ok(0) => {
                    return Err(crate::error::StratumError::Network {
                        message: "Connection closed while reading HTTP headers".to_string(),
                        source: None,
                    });
                }
                Ok(_) => {
                    let line = buffer.trim();
                    if line.is_empty() {
                        // Empty line marks end of headers
                        break;
                    }

                    // Parse header
                    if let Some(colon_pos) = line.find(':') {
                        let header_name = line[..colon_pos].trim().to_lowercase();
                        let header_value = line[colon_pos + 1..].trim();

                        if header_name == "content-length" {
                            content_length = header_value.parse::<usize>().ok();
                        }

                        tracing::trace!(
                            "miner {} - HTTP header: {}: {}",
                            addr,
                            header_name,
                            header_value
                        );
                        headers.insert(header_name, header_value.to_string());
                    }
                }
                Err(e) => {
                    return Err(crate::error::StratumError::Network {
                        message: format!("Error reading HTTP headers: {}", e),
                        source: None,
                    });
                }
            }
        }

        // Read HTTP body if Content-Length is present
        let json_payload = if let Some(length) = content_length {
            tracing::debug!(
                "miner {} - Reading HTTP body with Content-Length: {}",
                addr,
                length
            );

            let mut body = vec![0u8; length];
            use tokio::io::AsyncReadExt;

            match reader.read_exact(&mut body).await {
                Ok(_) => {
                    let body_str = String::from_utf8_lossy(&body);
                    tracing::debug!("miner {} - HTTP body: {}", addr, body_str);
                    body_str.to_string()
                }
                Err(e) => {
                    return Err(crate::error::StratumError::Network {
                        message: format!("Error reading HTTP body: {}", e),
                        source: None,
                    });
                }
            }
        } else {
            // No body or chunked encoding not supported
            tracing::warn!(
                "miner {} - No Content-Length header, assuming no body",
                addr
            );
            "{}".to_string()
        };

        // Check if this is an HTTP-specific mining method that needs special handling
        let is_http_mining_method =
            if let Ok(json_obj) = serde_json::from_str::<Value>(&json_payload) {
                matches!(
                    json_obj.get("method").and_then(|m| m.as_str()),
                    Some("getblocktemplate") | Some("getwork") | Some("submitblock")
                )
            } else {
                false
            };

        if is_http_mining_method {
            // Handle HTTP mining methods - send proper 501 response
            tracing::info!(
                "miner {} - HTTP mining method detected: getblocktemplate/getwork/submitblock",
                addr
            );

            let error_body = r#"{"error":{"code":-32601,"message":"Method not supported via HTTP. Please use Stratum protocol."},"id":0}"#;
            let error_response = format!(
                "HTTP/1.1 501 Not Implemented\r\n\
                 Content-Type: application/json\r\n\
                 Content-Length: {}\r\n\
                 Connection: close\r\n\
                 \r\n\
                 {}",
                error_body.len(),
                error_body
            );

            return Self::send_direct_http_response(downstream_to_upstream_tx, &error_response)
                .await;
        }

        // Process non-mining JSON-RPC payload through the Stratum pipeline
        let context = MessageContext::new(json_payload.clone())
            .with_client_info(Some(format!("http_client_{}", addr)), Some(addr));

        // Track start time for metrics
        let start_time = Instant::now();

        match pipeline.process(context).await {
            Ok(processed_context) => {
                let processing_time = start_time.elapsed();

                if let Some(parsed_message) = processed_context.parsed_message {
                    metrics.record_message(parsed_message.message_type(), processing_time, true);

                    // Handle the parsed message and get response
                    let send_result = Self::handle_parsed_message(
                        &parsed_message,
                        &json_payload,
                        manager,
                        config,
                        addr,
                        downstream_to_upstream_tx,
                    )
                    .await;

                    if let Err(e) = send_result {
                        // Send HTTP error response
                        return Self::send_http_error_response(
                            &downstream_to_upstream_tx,
                            500,
                            &e.to_string(),
                        )
                        .await;
                    }

                    // For HTTP requests, enable HTTP response mode
                    // This will cause responses to be wrapped in HTTP format
                    let _ = downstream_to_upstream_tx.send(serde_json::json!({
                        "__http_mode": true
                    }));
                } else {
                    // Try to forward as raw JSON-RPC
                    match serde_json::from_str::<Value>(&json_payload) {
                        Ok(request) => {
                            tracing::debug!("miner {} - Forwarding HTTP JSON-RPC request", addr);

                            // Forward the request and enable HTTP mode for responses
                            if let Err(e) = downstream_to_upstream_tx.send(request) {
                                return Err(crate::error::StratumError::Network {
                                    message: format!("Failed to forward JSON-RPC: {}", e),
                                    source: None,
                                });
                            }
                            let _ = downstream_to_upstream_tx.send(serde_json::json!({
                                "__http_mode": true
                            }));
                        }
                        Err(e) => {
                            // Send HTTP error response for invalid JSON
                            let error_msg = format!("Invalid JSON-RPC: {}", e);
                            return Self::send_http_error_response(
                                &downstream_to_upstream_tx,
                                400,
                                &error_msg,
                            )
                            .await;
                        }
                    }
                }
            }
            Err(e) => {
                let processing_time = start_time.elapsed();
                metrics.record_message("unknown", processing_time, false);

                // Send HTTP error response for pipeline failure
                let error_msg = format!("Pipeline processing failed: {}", e);
                return Self::send_http_error_response(&downstream_to_upstream_tx, 500, &error_msg)
                    .await;
            }
        }

        Ok(())
    }

    /// Send HTTP error response directly to client
    async fn send_http_error_response(
        downstream_to_upstream_tx: &mpsc::UnboundedSender<Value>,
        status_code: u16,
        error_message: &str,
    ) -> Result<()> {
        let status_text = match status_code {
            400 => "Bad Request",
            404 => "Not Found",
            500 => "Internal Server Error",
            501 => "Not Implemented",
            _ => "Error",
        };

        let error_body = format!("{{\"error\":\"{}\"}}", error_message.replace('"', "\\\""));

        let response = format!(
            "HTTP/1.1 {} {}\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\
             \r\n\
             {}",
            status_code,
            status_text,
            error_body.len(),
            error_body
        );

        Self::send_direct_http_response(downstream_to_upstream_tx, &response).await
    }

    /// Send HTTP response directly to client without complex JSON markers
    async fn send_direct_http_response(
        downstream_to_upstream_tx: &mpsc::UnboundedSender<Value>,
        response: &str,
    ) -> Result<()> {
        downstream_to_upstream_tx
            .send(serde_json::json!({
                "__http_response": response
            }))
            .map_err(|e| crate::error::StratumError::Network {
                message: format!("Failed to send HTTP response: {}", e),
                source: None,
            })?;
        Ok(())
    }
}
