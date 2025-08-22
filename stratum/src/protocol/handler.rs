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
use tracing::{debug, error, trace, warn};

use crate::Manager;
use crate::error::Result;
use crate::protocol::implementations::DefaultProtocolMetrics;
use crate::protocol::messages::StratumMessage;
use crate::protocol::pipeline::{MessageContext, MessagePipeline, PipelineBuilder};
use crate::protocol::traits::ProtocolMetrics;
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
                            error!(
                                "🔌 HTTP CONNECT DETECTED! Miner {} - Request: {}",
                                addr,
                                raw_message.trim()
                            );

                            // Extract path from CONNECT request
                            if let Some(extracted_path) =
                                Self::parse_connect_path_from_message(&raw_message)
                            {
                                error!(
                                    "🎯 PATH EXTRACTED! Miner {} - HTTP CONNECT Path: {}",
                                    addr, extracted_path
                                );
                                if let Ok(mut path_guard) = connection_path.lock() {
                                    *path_guard = Some(extracted_path);
                                }
                            } else {
                                error!(
                                    "❌ HTTP CONNECT detected but NO PATH found for miner {}",
                                    addr
                                );
                            }

                            // Send special marker to proxy coordinator to send HTTP response
                            if let Err(e) = downstream_to_upstream_tx.send(serde_json::json!({
                                "__http_connect_response": true
                            })) {
                                error!("miner {} - Failed to queue HTTP response: {}", addr, e);
                                break;
                            }
                            continue;
                        }

                        // Try to extract path information from message if not already captured
                        {
                            let path_guard = connection_path.lock().unwrap();
                            if path_guard.is_none() {
                                drop(path_guard); // Release the lock before calling extract_path_from_message
                                tracing::debug!(
                                    "miner {} - Analyzing message for path info: {}",
                                    addr,
                                    raw_message
                                );
                                if let Some(extracted_path) =
                                    Self::extract_path_from_message(&raw_message)
                                {
                                    tracing::info!(
                                        "miner {} - Path extracted from message: {}",
                                        addr,
                                        extracted_path
                                    );
                                    if let Ok(mut path_guard) = connection_path.lock() {
                                        *path_guard = Some(extracted_path);
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

            while !shutdown.load(Ordering::Relaxed) {
                tokio::select! {
                    Some(message) = downstream_to_upstream_rx.recv() => {
                        // Check if this is a special HTTP CONNECT response marker
                        if message.get("__http_connect_response").is_some() {
                            // Send HTTP response directly to downstream client
                            if let Err(e) = downstream_writer.write_all(b"HTTP/1.1 200 Connection established\r\n\r\n").await {
                                error!("miner {} - Error sending HTTP CONNECT response: {}", addr, e);
                                break;
                            }
                            if let Err(e) = downstream_writer.flush().await {
                                error!("miner {} - Error flushing HTTP response: {}", addr, e);
                                break;
                            }
                            error!("✅ HTTP TUNNEL ESTABLISHED! Miner {} - Ready for Stratum protocol", addr);
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

                        if let Err(e) = downstream_writer.write_all((json + "\n").as_bytes()).await {
                            error!("miner {} - Error writing to downstream: {}", addr, e);
                            metrics.record_error("downstream_write", &format!("addr:{}", addr));
                            break;
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

    /// Extract path information from JSON messages
    /// Looks for URL patterns in mining.subscribe or other connection messages
    fn extract_path_from_message(raw_message: &str) -> Option<String> {
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

    /// Parse path from HTTP CONNECT message
    /// Format: "CONNECT host:port/path HTTP/1.1"  
    fn parse_connect_path_from_message(message: &str) -> Option<String> {
        let line = message.trim();
        if line.starts_with("CONNECT ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let target = parts[1];

                // Look for path after the port
                if let Some(path_start) = target.find('/') {
                    let path = &target[path_start + 1..];
                    if !path.is_empty() {
                        return Some(path.to_string());
                    }
                }
            }
        }
        None
    }
}
