//! Connection state management and lifecycle coordination.
//!
//! This module provides the main ProtocolHandler that coordinates between
//! all the specialized handlers and manages the connection lifecycle,
//! state, and resource cleanup.

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use serde_json;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, trace, warn};

use crate::Manager;
use crate::error::Result;
use crate::protocol::implementations::DefaultProtocolMetrics;
use crate::protocol::pipeline::{MessagePipeline, PipelineBuilder};
use crate::services::pool_config::{PoolConfigService, PoolConfigServiceConfig};
use crate::{Config, auth};

use super::http_handler::HttpHandler;
use super::stratum_handler::StratumHandler;
use super::message_processor::MessageProcessor;

/// Main protocol handler that coordinates connection lifecycle and message routing.
///
/// The `ProtocolHandler` serves as the central coordinator for handling mining
/// connections. It manages:
/// - Connection lifecycle and resource cleanup
/// - Message routing between HTTP and Stratum handlers  
/// - Connection state and authentication
/// - Metrics collection and reporting
/// - Upstream and downstream proxy coordination
///
/// # Examples
///
/// ```rust
/// use loka_stratum::protocol::handler::ProtocolHandler;
/// use std::sync::Arc;
///
/// let handler = ProtocolHandler::new(manager, config, addr, None);
/// // Handler automatically routes messages based on protocol detection
/// ```
#[derive(Debug)]
pub struct ProtocolHandler {
    /// Main manager instance
    manager: Arc<Manager>,
    /// Server configuration
    config: Arc<Config>,
    /// Client socket address
    addr: SocketAddr,
    /// Connection path information (for tracking/routing)
    connection_path: Arc<Mutex<Option<String>>>,
    /// Current difficulty target
    difficulty: Arc<AtomicU64>,
    /// Shutdown flag for clean termination
    shutdown: Arc<AtomicBool>,
    /// Message processing pipeline
    #[allow(dead_code)]
    pipeline: MessagePipeline,
    /// Protocol-specific metrics
    metrics: Arc<DefaultProtocolMetrics>,
    /// Pool configuration service
    #[allow(dead_code)]
    pool_service: PoolConfigService,
    /// HTTP message handler
    #[allow(dead_code)]
    http_handler: HttpHandler,
    /// Stratum message handler
    #[allow(dead_code)]
    stratum_handler: StratumHandler,
    /// Message processor and router
    message_processor: MessageProcessor,
}

impl ProtocolHandler {
    /// Create a new protocol handler.
    ///
    /// # Arguments
    ///
    /// * `manager` - The main manager instance
    /// * `config` - Server configuration
    /// * `addr` - Client socket address
    /// * `connection_path` - Optional connection path for routing
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
            // NOTE: Validation middleware removed - proxy acts as transparent reformatter
            // All business validation is handled by upstream pool
            .with_rate_limiting(60) // Rate limit clients
            .with_authentication(config.pool.clone()) // Handle authentication with pool config
            .build();

        // Create pool configuration service for target validation
        let pool_service = PoolConfigService::new(
            manager.database().clone(),
            PoolConfigServiceConfig::default(),
        );

        // Create specialized handlers
        let http_handler = HttpHandler::new(Arc::new(pool_service.clone()));
        let stratum_handler = StratumHandler::new();
        
        // Create message processor that coordinates between handlers
        let message_processor = MessageProcessor::new(
            pipeline.clone(),
            http_handler.clone(),
            stratum_handler.clone(),
        );

        // Log the connection path if available
        if let Some(ref path) = connection_path {
            info!(
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
            http_handler,
            stratum_handler,
            message_processor,
        }
    }

    /// Get authentication state for this connection.
    pub fn auth(&self) -> Option<Arc<auth::State>> {
        self.manager.auth().get(&self.addr)
    }

    /// Get current difficulty target.
    pub fn difficulty(&self) -> f64 {
        f64::from_bits(self.difficulty.load(Ordering::Relaxed))
    }

    /// Check if connection has been terminated.
    pub fn terminated(&self) -> bool {
        self.shutdown.load(Ordering::Relaxed)
    }

    /// Get protocol metrics.
    pub fn metrics(&self) -> &DefaultProtocolMetrics {
        &self.metrics
    }

    /// Get connection path.
    pub fn connection_path(&self) -> Option<String> {
        self.connection_path.lock().unwrap().clone()
    }

    /// Set connection path.
    pub fn set_connection_path(&self, path: String) {
        *self.connection_path.lock().unwrap() = Some(path);
    }

    /// Handle a client connection using proxy coordination.
    ///
    /// This is the main entry point for handling client connections.
    /// It sets up bidirectional proxy channels and coordinates message
    /// flow between client and upstream pool.
    pub async fn handle_connection(&self, stream: TcpStream) -> Result<()> {
        info!("miner {} - Starting connection handler", self.addr);

        // Split stream for concurrent read/write operations
        let (downstream_read, downstream_write) = stream.into_split();
        let (upstream_read, upstream_write) = {
            // Connect to upstream pool
            let upstream_stream = self.connect_to_upstream().await?;
            upstream_stream.into_split()
        };

        // Create communication channels
        let (downstream_to_upstream_tx, downstream_to_upstream_rx) = mpsc::unbounded_channel();
        let (upstream_to_downstream_tx, upstream_to_downstream_rx) = mpsc::unbounded_channel();

        // Spawn handlers
        let downstream_handler =
            self.spawn_downstream_handler(downstream_read, downstream_to_upstream_tx.clone());

        let upstream_handler =
            self.spawn_upstream_handler(upstream_read, upstream_to_downstream_tx);

        let proxy_coordinator = self.spawn_proxy_coordinator(
            downstream_write,
            upstream_write,
            downstream_to_upstream_rx,
            upstream_to_downstream_rx,
        );

        // Wait for all handlers to complete
        let (downstream_result, upstream_result, coordinator_result) =
            tokio::join!(downstream_handler, upstream_handler, proxy_coordinator);

        // Handle results and log any errors
        match downstream_result {
            Ok(Ok(())) => {},
            Ok(Err(e)) => error!("miner {} - Downstream handler error: {}", self.addr, e),
            Err(e) => error!("miner {} - Downstream join error: {}", self.addr, e),
        }

        match upstream_result {
            Ok(Ok(())) => {},
            Ok(Err(e)) => error!("miner {} - Upstream handler error: {}", self.addr, e),
            Err(e) => error!("miner {} - Upstream join error: {}", self.addr, e),
        }

        match coordinator_result {
            Ok(Ok(())) => {},
            Ok(Err(e)) => error!("miner {} - Proxy coordinator error: {}", self.addr, e),
            Err(e) => error!("miner {} - Coordinator join error: {}", self.addr, e),
        }

        info!("miner {} - Connection handler completed", self.addr);
        Ok(())
    }

    /// Connect to the upstream mining pool.
    async fn connect_to_upstream(&self) -> Result<TcpStream> {
        let pool_address = &self.config.pool.address; // Already in host:port format
        info!("miner {} - Connecting to upstream pool: {}", self.addr, pool_address);

        let stream = TcpStream::connect(pool_address).await?;
        debug!("miner {} - Connected to upstream pool", self.addr);
        
        Ok(stream)
    }

    /// Spawn handler for downstream (client) messages.
    ///
    /// Reads messages from the client and processes them through the
    /// message processor pipeline.
    fn spawn_downstream_handler(
        &self,
        downstream_read: OwnedReadHalf,
        _downstream_to_upstream_tx: mpsc::UnboundedSender<serde_json::Value>,
    ) -> JoinHandle<Result<()>> {
        let manager = self.manager.clone();
        let config = self.config.clone();
        let addr = self.addr;
        let message_processor = self.message_processor.clone();
        let shutdown = self.shutdown.clone();

        tokio::spawn(async move {
            let mut reader = BufReader::new(downstream_read);
            let mut line = String::new();

            info!("miner {} - Downstream handler started", addr);

            while !shutdown.load(Ordering::Relaxed) {
                line.clear();

                match reader.read_line(&mut line).await {
                    Ok(0) => {
                        debug!("miner {} - Client disconnected", addr);
                        break;
                    }
                    Ok(_) => {
                        let raw_message = line.trim();
                        if !raw_message.is_empty() {
                            trace!("miner {} - Received: {}", addr, raw_message);

                            // Process through the message processor
                            if let Err(e) = message_processor
                                .process_message(raw_message, &manager, &config, addr)
                                .await
                            {
                                warn!("miner {} - Message processing error: {}", addr, e);
                                manager.metrics().record_protocol_error();
                            }
                        }
                    }
                    Err(e) => {
                        warn!("miner {} - Read error: {}", addr, e);
                        manager.metrics().record_connection_error();
                        break;
                    }
                }
            }

            info!("miner {} - Downstream handler completed", addr);
            Ok(())
        })
    }

    /// Spawn handler for upstream (pool) messages.
    ///
    /// Reads messages from the upstream pool and forwards them to the client.
    fn spawn_upstream_handler(
        &self,
        upstream_read: OwnedReadHalf,
        upstream_to_downstream_tx: mpsc::UnboundedSender<serde_json::Value>,
    ) -> JoinHandle<Result<()>> {
        let addr = self.addr;
        let _manager = self.manager.clone();
        let shutdown = self.shutdown.clone();

        tokio::spawn(async move {
            let mut reader = BufReader::new(upstream_read);
            let mut line = String::new();

            info!("miner {} - Upstream handler started", addr);

            while !shutdown.load(Ordering::Relaxed) {
                line.clear();

                match reader.read_line(&mut line).await {
                    Ok(0) => {
                        debug!("miner {} - Upstream disconnected", addr);
                        break;
                    }
                    Ok(_) => {
                        let raw_message = line.trim();
                        if !raw_message.is_empty() {
                            trace!("miner {} - Upstream message: {}", addr, raw_message);

                            // Parse and forward to client
                            if let Ok(json_value) = serde_json::from_str(raw_message) {
                                if let Err(e) = upstream_to_downstream_tx.send(json_value) {
                                    warn!("miner {} - Failed to forward upstream message: {}", addr, e);
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("miner {} - Upstream read error: {}", addr, e);
                        break;
                    }
                }
            }

            info!("miner {} - Upstream handler completed", addr);
            Ok(())
        })
    }

    /// Spawn proxy coordinator for bidirectional message forwarding.
    ///
    /// Coordinates message flow between client and upstream pool,
    /// handling serialization and forwarding in both directions.
    fn spawn_proxy_coordinator(
        &self,
        downstream_write: OwnedWriteHalf,
        upstream_write: OwnedWriteHalf,
        mut downstream_to_upstream_rx: mpsc::UnboundedReceiver<serde_json::Value>,
        mut upstream_to_downstream_rx: mpsc::UnboundedReceiver<serde_json::Value>,
    ) -> JoinHandle<Result<()>> {
        let addr = self.addr;
        let manager = self.manager.clone();
        let shutdown = self.shutdown.clone();

        tokio::spawn(async move {
            let mut downstream_writer = BufWriter::new(downstream_write);
            let mut upstream_writer = BufWriter::new(upstream_write);

            info!("miner {} - Proxy coordinator started", addr);

            loop {
                if shutdown.load(Ordering::Relaxed) {
                    break;
                }

                tokio::select! {
                    // Forward messages from client to upstream
                    Some(message) = downstream_to_upstream_rx.recv() => {
                        let serialized = serde_json::to_string(&message)?;
                        trace!("miner {} - Forwarding to upstream: {}", addr, serialized);

                        upstream_writer.write_all(serialized.as_bytes()).await?;
                        upstream_writer.write_all(b"\n").await?;
                        upstream_writer.flush().await?;

                        manager.metrics().record_messages_sent(1);
                        manager.metrics().record_bytes_sent(serialized.len() as u64);
                    }

                    // Forward messages from upstream to client
                    Some(message) = upstream_to_downstream_rx.recv() => {
                        let serialized = serde_json::to_string(&message)?;
                        trace!("miner {} - Forwarding to client: {}", addr, serialized);

                        downstream_writer.write_all(serialized.as_bytes()).await?;
                        downstream_writer.write_all(b"\n").await?;
                        downstream_writer.flush().await?;

                        manager.metrics().record_messages_sent(1);
                        manager.metrics().record_bytes_sent(serialized.len() as u64);
                    }

                    else => break,
                }
            }

            info!("miner {} - Proxy coordinator completed", addr);
            Ok(())
        })
    }

    /// Initiate graceful shutdown of the connection.
    pub fn shutdown(&self) {
        info!("miner {} - Initiating shutdown", self.addr);
        self.shutdown.store(true, Ordering::Relaxed);
    }

    /// Update connection metrics.
    pub fn update_metrics(&self) {
        // Update connection-specific metrics
        self.manager.metrics().update_resource_utilization(0.0, 0.0); // Would get real values
        
        // Update connection duration, etc.
    }

    /// Handle connection cleanup when the connection ends.
    pub async fn cleanup(&self) {
        info!("miner {} - Cleaning up connection resources", self.addr);
        
        // Clean up authentication state
        if let Some(_auth_state) = self.auth() {
            // Remove from manager's auth tracking
            self.manager.auth().terminated(&self.addr);
        }
        
        // Update metrics
        self.manager.metrics().record_disconnection();
        
        // Log connection statistics
        debug!(
            "miner {} - Connection completed, path: {:?}",
            self.addr,
            self.connection_path()
        );
    }

    /// Run the protocol handler for a connection.
    ///
    /// This is the main entry point that processes downstream and upstream streams,
    /// handling message routing, protocol detection, and proxying.
    ///
    /// # Arguments
    ///
    /// * `downstream` - Client connection stream
    /// * `upstream` - Pool connection stream  
    ///
    /// # Returns
    ///
    /// Result indicating success or failure of the connection handling
    pub async fn run(&self, downstream: tokio::net::TcpStream, upstream: tokio::net::TcpStream) -> crate::error::Result<()> {
        info!("miner {} - Starting protocol handler", self.addr);
        
        // Set up the connection
        self.manager.connection_established(&self.addr);
        
        let (downstream_read, downstream_write) = downstream.into_split();
        let (upstream_read, upstream_write) = upstream.into_split();
        
        // Create buffered readers/writers
        let mut downstream_reader = tokio::io::BufReader::new(downstream_read);
        let mut upstream_reader = tokio::io::BufReader::new(upstream_read);
        let mut downstream_writer = tokio::io::BufWriter::new(downstream_write);
        let mut upstream_writer = tokio::io::BufWriter::new(upstream_write);

        // Message processing loop
        let mut downstream_line = String::new();
        let mut upstream_line = String::new();
        loop {
            tokio::select! {
                // Handle downstream messages (from client)
                result = downstream_reader.read_line(&mut downstream_line) => {
                    match result {
                        Ok(0) => {
                            debug!("miner {} - Client disconnected", self.addr);
                            break;
                        }
                        Ok(_) => {
                            let trimmed_message = downstream_line.trim();
                            
                            // Handle HTTP CONNECT requests locally instead of forwarding them
                            if trimmed_message.starts_with("CONNECT ") {
                                info!("miner {} - Handling HTTP CONNECT request locally", self.addr);
                                
                                // Extract CONNECT target
                                if let Some(target) = HttpHandler::parse_connect_target_from_message(trimmed_message) {
                                    debug!("miner {} - CONNECT target: {}", self.addr, target);
                                    
                                    // Send HTTP 200 Connection Established response to client
                                    let connect_response = "HTTP/1.1 200 Connection established\r\n\r\n";
                                    if let Err(e) = downstream_writer.write_all(connect_response.as_bytes()).await {
                                        warn!("miner {} - Failed to send CONNECT response: {}", self.addr, e);
                                        break;
                                    }
                                    if let Err(e) = downstream_writer.flush().await {
                                        warn!("miner {} - Failed to flush CONNECT response: {}", self.addr, e);
                                        break;
                                    }
                                    
                                    info!("miner {} - Sent HTTP 200 Connection Established", self.addr);
                                    
                                    // Now we're in tunnel mode - continue to next message
                                    downstream_line.clear();
                                    continue;
                                } else {
                                    warn!("miner {} - Invalid CONNECT target format", self.addr);
                                    // Send HTTP 400 Bad Request
                                    let error_response = "HTTP/1.1 400 Bad Request\r\nConnection: close\r\n\r\n";
                                    if let Err(e) = downstream_writer.write_all(error_response.as_bytes()).await {
                                        warn!("miner {} - Failed to send 400 response: {}", self.addr, e);
                                    }
                                    if let Err(e) = downstream_writer.flush().await {
                                        warn!("miner {} - Failed to flush 400 response: {}", self.addr, e);
                                    }
                                    break;
                                }
                            }
                            
                            // Skip other HTTP headers during CONNECT handshake
                            if trimmed_message.starts_with("Host: ") || 
                               trimmed_message.starts_with("Proxy-Connection: ") ||
                               trimmed_message.starts_with("Connection: ") ||
                               trimmed_message.is_empty() {
                                trace!("miner {} - Skipping HTTP header: {}", self.addr, trimmed_message);
                                downstream_line.clear();
                                continue;
                            }
                            
                            // Process message for normalization and metrics only
                            // NO local responses - everything is forwarded to real pool
                            let message_to_forward = match self.message_processor.process_message(&downstream_line, &self.manager, &self.config, self.addr).await {
                                Ok(Some(normalized_message)) => {
                                    // Use normalized message (e.g., fixed mining.subscribe parameters)
                                    format!("{}\n", serde_json::to_string(&normalized_message).unwrap_or_default())
                                }
                                Ok(None) => {
                                    // Use original message if no normalization needed
                                    downstream_line.clone()
                                }
                                Err(e) => {
                                    warn!("miner {} - Error processing message, using original: {}", self.addr, e);
                                    downstream_line.clone()
                                }
                            };
                            
                            // Forward processed message to upstream pool
                            if let Err(e) = upstream_writer.write_all(message_to_forward.as_bytes()).await {
                                warn!("miner {} - Failed to forward to upstream: {}", self.addr, e);
                                break;
                            }
                            if let Err(e) = upstream_writer.flush().await {
                                warn!("miner {} - Failed to flush upstream: {}", self.addr, e);
                                break;
                            }
                            
                            trace!("miner {} - Forwarded to pool: {}", self.addr, message_to_forward.trim());
                            self.manager.metrics().record_messages_sent(1);
                            self.manager.metrics().record_bytes_sent(message_to_forward.len() as u64);
                            
                            downstream_line.clear();
                        }
                        Err(e) => {
                            warn!("miner {} - Error reading from client: {}", self.addr, e);
                            break;
                        }
                    }
                }
                
                // Handle upstream messages (from pool)
                result = upstream_reader.read_line(&mut upstream_line) => {
                    match result {
                        Ok(0) => {
                            debug!("miner {} - Upstream disconnected", self.addr);
                            break;
                        }
                        Ok(_) => {
                            // Forward message from upstream to client
                            if let Err(e) = downstream_writer.write_all(upstream_line.as_bytes()).await {
                                warn!("miner {} - Failed to forward to client: {}", self.addr, e);
                                break;
                            }
                            if let Err(e) = downstream_writer.flush().await {
                                warn!("miner {} - Failed to flush client: {}", self.addr, e);
                                break;
                            }
                            
                            debug!("miner {} - Forwarded upstream message to client: {}", self.addr, upstream_line.trim());
                            self.manager.metrics().record_messages_sent(1);
                            self.manager.metrics().record_bytes_sent(upstream_line.len() as u64);
                            
                            upstream_line.clear();
                        }
                        Err(e) => {
                            warn!("miner {} - Error reading from upstream: {}", self.addr, e);
                            break;
                        }
                    }
                }
                
                // Handle shutdown signal
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                    if self.terminated() {
                        info!("miner {} - Shutdown signal received", self.addr);
                        break;
                    }
                }
            }
        }

        // Cleanup
        self.cleanup().await;
        info!("miner {} - Protocol handler completed", self.addr);
        Ok(())
    }
}

// Note: Some methods from the original ProtocolHandler would be moved here
// and integrated with the new modular structure. This includes methods like:
// - extract_path_from_message (moved to HttpHandler)
// - handle_parsed_message (moved to StratumHandler) 
// - is_direct_http_request (moved to HttpHandler)
// - etc.

impl Drop for ProtocolHandler {
    fn drop(&mut self) {
        trace!("miner {} - ProtocolHandler dropped", self.addr);
    }
}