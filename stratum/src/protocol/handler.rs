use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, error, trace, warn};

use crate::error::Result;
use crate::Manager;
use crate::protocol::implementations::DefaultProtocolMetrics;
use crate::protocol::messages::StratumMessage;
use crate::protocol::pipeline::{MessageContext, MessagePipeline, PipelineBuilder};
use crate::protocol::traits::ProtocolMetrics;
use crate::{auth, Config};

/// Modern protocol-aware handler using the new pipeline architecture
#[derive(Debug)]
pub struct ProtocolHandler {
    manager: Arc<Manager>,
    config: Arc<Config>,
    addr: SocketAddr,
    difficulty: Arc<AtomicU64>,
    shutdown: Arc<AtomicBool>,
    pipeline: MessagePipeline,
    metrics: Arc<DefaultProtocolMetrics>,
}

impl ProtocolHandler {
    pub fn new(manager: Arc<Manager>, config: Arc<Config>, addr: SocketAddr) -> Self {
        // Build the processing pipeline
        let pipeline = PipelineBuilder::new()
            .with_logging()           // Log all messages
            .with_parsing()           // Parse raw messages
            .with_validation()        // Validate messages
            .with_rate_limiting(60)   // Rate limit clients
            .with_authentication()    // Handle authentication
            .build();

        Self {
            manager,
            config,
            addr,
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

    pub async fn run(&self, downstream: TcpStream, upstream: TcpStream) -> std::io::Result<()> {
        let (downstream_read, downstream_write) = downstream.into_split();
        let (upstream_read, upstream_write) = upstream.into_split();

        // Create channels for message flow
        let (downstream_to_upstream_tx, downstream_to_upstream_rx) =
            mpsc::unbounded_channel::<Value>();
        let (upstream_to_downstream_tx, upstream_to_downstream_rx) =
            mpsc::unbounded_channel::<Value>();

        // Spawn handlers
        let downstream_handler = self.spawn_downstream_handler(
            downstream_read,
            downstream_to_upstream_tx,
        );

        let upstream_handler = self.spawn_upstream_handler(
            upstream_read,
            upstream_to_downstream_tx,
        );

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
                        
                        // Create message context
                        let context = MessageContext::new(raw_message.clone())
                            .with_client_info(
                                Some(format!("client_{}", addr)),
                                Some(addr)
                            );

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
                                    ).await;

                                    if let Err(e) = send_result {
                                        error!(
                                            "miner {} - Failed to handle message: {}",
                                            addr, e
                                        );
                                        metrics.record_error("message_handling", &format!("addr:{}", addr));
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

                                            if let Err(e) = downstream_to_upstream_tx.send(request) {
                                                error!(
                                                    "miner {} - Failed to forward message: {}",
                                                    addr, e
                                                );
                                                break;
                                            }
                                        }
                                        Err(e) => {
                                            warn!("miner {} - Invalid JSON: {}", addr, e);
                                            metrics.record_error("invalid_json", &format!("addr:{}", addr));
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                let processing_time = start_time.elapsed();
                                metrics.record_message("unknown", processing_time, false);
                                warn!("miner {} - Pipeline processing failed: {}", addr, e);
                                metrics.record_error("pipeline_processing", &format!("addr:{}", addr));
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
                manager.submissions().authenticated(
                    manager.auth().authenticate(addr, user, worker),
                );

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

                downstream_to_upstream_tx.send(upstream_message)
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
                downstream_to_upstream_tx.send(original_value)
                    .map_err(|e| crate::error::StratumError::Network {
                        message: format!("Failed to send original message to upstream: {}", e),
                        source: None,
                    })
            }
            _ => {
                // Forward other messages as-is
                let original_value = serde_json::from_str::<Value>(raw_message)?;
                downstream_to_upstream_tx.send(original_value)
                    .map_err(|e| crate::error::StratumError::Network {
                        message: format!("Failed to forward message to upstream: {}", e),
                        source: None,
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
                    Ok(_) => {
                        match serde_json::from_str::<Value>(buffer.trim()) {
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
                                metrics.record_error("invalid_upstream_json", &format!("addr:{}", addr));
                            }
                        }
                    }
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
}