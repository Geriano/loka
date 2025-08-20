use std::sync::Arc;

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, error, trace, warn};

use crate::error::Result;
use crate::network::connection::{Connection, DisconnectReason};
use crate::network::manager::ConnectionManager;
use crate::processor::Message;
use crate::protocol::types::Response;
use crate::{auth, processor, Manager};

/// Network handler that integrates with the new Connection model
#[derive(Debug)]
pub struct NetworkHandler {
    /// Central manager for legacy compatibility
    manager: Arc<Manager>,
    /// Connection manager for the new connection model
    connection_manager: Arc<ConnectionManager>,
    /// Message processor
    processor: Arc<processor::Manager>,
    /// Connection abstraction
    connection: Arc<Connection>,
}

impl NetworkHandler {
    /// Create a new network handler for a connection
    pub fn new(
        manager: Arc<Manager>,
        connection_manager: Arc<ConnectionManager>,
        connection: Arc<Connection>,
    ) -> Self {
        Self {
            manager,
            connection_manager,
            processor: Arc::new(processor::Manager::default()),
            connection,
        }
    }

    /// Get the connection abstraction
    pub fn connection(&self) -> Arc<Connection> {
        self.connection.clone()
    }

    /// Check if the connection is authenticated
    pub async fn is_authenticated(&self) -> bool {
        self.connection.is_authenticated().await
    }

    /// Get authentication state
    pub async fn auth_state(&self) -> Option<auth::State> {
        self.connection.auth_session().await
    }

    /// Get connection metrics
    pub async fn metrics(&self) -> crate::network::connection::ConnectionMetrics {
        self.connection.metrics().await
    }

    /// Main handler entry point - manages bidirectional proxy
    pub async fn run(&self, downstream: TcpStream, upstream: TcpStream) -> Result<()> {
        let (downstream_read, downstream_write) = downstream.into_split();
        let (upstream_read, upstream_write) = upstream.into_split();

        // Create channels for message flow
        let (downstream_to_upstream_tx, downstream_to_upstream_rx) =
            mpsc::unbounded_channel::<Value>();
        let (upstream_to_downstream_tx, upstream_to_downstream_rx) =
            mpsc::unbounded_channel::<Value>();

        // Spawn handlers for each direction
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

        // Handle results and log errors
        if let Err(e) = downstream_result.map_err(|e| format!("Task join error: {}", e)) {
            error!("Connection {} - Downstream handler failed: {}", self.connection.id(), e);
        }

        if let Err(e) = upstream_result.map_err(|e| format!("Task join error: {}", e)) {
            error!("Connection {} - Upstream handler failed: {}", self.connection.id(), e);
        }

        if let Err(e) = proxy_result.map_err(|e| format!("Task join error: {}", e)) {
            error!("Connection {} - Proxy coordinator failed: {}", self.connection.id(), e);
        }

        // Mark connection as disconnected
        self.connection.disconnect(DisconnectReason::ClientDisconnect).await;

        debug!("Connection {} - Handler completed", self.connection.id());
        Ok(())
    }

    /// Spawn downstream handler (client -> proxy -> upstream)
    fn spawn_downstream_handler(
        &self,
        downstream_read: OwnedReadHalf,
        downstream_to_upstream_tx: mpsc::UnboundedSender<Value>,
    ) -> JoinHandle<Result<()>> {
        let manager = self.manager.clone();
        let processor = self.processor.clone();
        let connection = self.connection.clone();

        tokio::spawn(async move {
            let mut reader = BufReader::new(downstream_read);
            let mut buffer = String::new();

            loop {
                buffer.clear();

                // Check if connection should terminate
                if let Some(_reason) = connection.should_terminate(std::time::Duration::from_secs(300)).await {
                    debug!("Connection {} - Downstream handler terminating", connection.id());
                    break;
                }

                match reader.read_line(&mut buffer).await {
                    Ok(0) => {
                        debug!("Connection {} - Downstream connection closed", connection.id());
                        connection.disconnect(DisconnectReason::ClientDisconnect).await;
                        break;
                    }
                    Ok(bytes_read) => {
                        // Record message received
                        connection.record_message_received(bytes_read).await;

                        // Try to parse as a known message type first
                        if let Some(message) = processor.parse(buffer.trim()) {
                            let send_result = match message {
                                Message::Authenticate { user, worker, .. } => {
                                    Self::handle_authentication(
                                        &connection,
                                        &manager,
                                        &user,
                                        &worker,
                                        &buffer,
                                        &downstream_to_upstream_tx,
                                    ).await
                                }
                                Message::Submit { id, job_id } => {
                                    Self::handle_submission(
                                        &connection,
                                        &manager,
                                        id,
                                        &job_id,
                                        &buffer,
                                        &downstream_to_upstream_tx,
                                    ).await
                                }
                                _ => {
                                    // Forward other messages directly
                                    match serde_json::from_str::<Value>(buffer.trim()) {
                                        Ok(json) => downstream_to_upstream_tx.send(json).map_err(|e| e.to_string()),
                                        Err(e) => Err(format!("JSON parse error: {}", e)),
                                    }
                                }
                            };

                            if let Err(e) = send_result {
                                error!("Connection {} - Failed to process message: {}", connection.id(), e);
                                break;
                            }
                        } else {
                            // Forward as generic JSON
                            match serde_json::from_str::<Value>(buffer.trim()) {
                                Ok(request) => {
                                    trace!("Connection {} - Forwarding downstream message: {}",
                                        connection.id(),
                                        serde_json::to_string(&request).unwrap_or_default()
                                    );

                                    if let Err(e) = downstream_to_upstream_tx.send(request) {
                                        error!("Connection {} - Failed to forward message: {}", 
                                            connection.id(), e);
                                        break;
                                    }
                                }
                                Err(e) => {
                                    warn!("Connection {} - Invalid JSON from client: {} - {}", 
                                        connection.id(), buffer.trim(), e);
                                    continue;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Connection {} - Error reading from downstream: {}", connection.id(), e);
                        connection.disconnect(DisconnectReason::NetworkError { 
                            error: e.to_string() 
                        }).await;
                        break;
                    }
                }
            }

            Ok(())
        })
    }

    /// Spawn upstream handler (upstream -> proxy -> client)
    fn spawn_upstream_handler(
        &self,
        upstream_read: OwnedReadHalf,
        upstream_to_downstream_tx: mpsc::UnboundedSender<Value>,
    ) -> JoinHandle<Result<()>> {
        let manager = self.manager.clone();
        let processor = self.processor.clone();
        let connection = self.connection.clone();

        tokio::spawn(async move {
            let mut reader = BufReader::new(upstream_read);
            let mut buffer = String::new();

            loop {
                buffer.clear();

                // Check if connection should terminate
                if let Some(_reason) = connection.should_terminate(std::time::Duration::from_secs(300)).await {
                    debug!("Connection {} - Upstream handler terminating", connection.id());
                    break;
                }

                match reader.read_line(&mut buffer).await {
                    Ok(0) => {
                        debug!("Connection {} - Upstream connection closed", connection.id());
                        connection.disconnect(DisconnectReason::NetworkError { 
                            error: "Upstream connection closed".to_string() 
                        }).await;
                        break;
                    }
                    Ok(bytes_read) => {
                        // Record message received from upstream
                        connection.record_message_received(bytes_read).await;

                        // Try to parse as known message types
                        if let Some(message) = processor.parse(buffer.trim()) {
                            let send_result = match message {
                                Message::SetDifficulty { difficulty } => {
                                    debug!("Connection {} - Set difficulty to {}", connection.id(), difficulty);
                                    
                                    match serde_json::from_str::<Value>(buffer.trim()) {
                                        Ok(json) => upstream_to_downstream_tx.send(json).map_err(|e| e.to_string()),
                                        Err(e) => Err(format!("JSON parse error: {}", e)),
                                    }
                                }
                                Message::Notify { job_id } => {
                                    let jobs = manager.jobs();
                                    
                                    // Use a default difficulty for now - this could be improved
                                    jobs.notified(&job_id, 1.0);

                                    debug!("Connection {} - Notified new job {} (total jobs: {})",
                                        connection.id(), job_id, jobs.total());

                                    match serde_json::from_str::<Value>(buffer.trim()) {
                                        Ok(json) => upstream_to_downstream_tx.send(json).map_err(|e| e.to_string()),
                                        Err(e) => Err(format!("JSON parse error: {}", e)),
                                    }
                                }
                                _ => {
                                    // Forward other messages
                                    match serde_json::from_str::<Value>(buffer.trim()) {
                                        Ok(json) => upstream_to_downstream_tx.send(json).map_err(|e| e.to_string()),
                                        Err(e) => Err(format!("JSON parse error: {}", e)),
                                    }
                                }
                            };

                            if let Err(e) = send_result {
                                error!("Connection {} - Failed to process upstream message: {}", connection.id(), e);
                                break;
                            }
                        } else if let Ok(response) = serde_json::from_str::<Response>(buffer.trim()) {
                            // Handle submission responses
                            Self::handle_submission_response(&connection, &manager, &response).await;

                            // Forward the response
                            if let Ok(response_value) = serde_json::to_value(response) {
                                if let Err(e) = upstream_to_downstream_tx.send(response_value) {
                                    error!("Connection {} - Failed to forward response: {}", connection.id(), e);
                                    break;
                                }
                            }
                        } else {
                            // Forward as generic JSON
                            match serde_json::from_str::<Value>(buffer.trim()) {
                                Ok(response) => {
                                    trace!("Connection {} - Forwarding upstream message: {}",
                                        connection.id(),
                                        serde_json::to_string(&response).unwrap_or_default()
                                    );

                                    if let Err(e) = upstream_to_downstream_tx.send(response) {
                                        error!("Connection {} - Failed to forward upstream message: {}", 
                                            connection.id(), e);
                                        break;
                                    }
                                }
                                Err(e) => {
                                    warn!("Connection {} - Invalid JSON from upstream: {} - {}", 
                                        connection.id(), buffer.trim(), e);
                                    continue;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Connection {} - Error reading from upstream: {}", connection.id(), e);
                        connection.disconnect(DisconnectReason::NetworkError { 
                            error: e.to_string() 
                        }).await;
                        break;
                    }
                }
            }

            Ok(())
        })
    }

    /// Spawn proxy coordinator (manages message flow between channels and sockets)
    fn spawn_proxy_coordinator(
        &self,
        mut downstream_to_upstream_rx: mpsc::UnboundedReceiver<Value>,
        mut upstream_to_downstream_rx: mpsc::UnboundedReceiver<Value>,
        downstream_writer: OwnedWriteHalf,
        upstream_writer: OwnedWriteHalf,
    ) -> JoinHandle<Result<()>> {
        let connection = self.connection.clone();

        tokio::spawn(async move {
            let mut downstream_writer = BufWriter::new(downstream_writer);
            let mut upstream_writer = BufWriter::new(upstream_writer);

            trace!("Connection {} - Proxy coordinator started", connection.id());

            // Spawn tasks for each direction
            let downstream_to_upstream_task = {
                let connection = connection.clone();
                tokio::spawn(async move {
                    let mut count = 0u64;
                    
                    while let Some(request) = downstream_to_upstream_rx.recv().await {
                        count += 1;
                        
                        let request_str = serde_json::to_string(&request)
                            .unwrap_or_else(|_| "{}".to_string());
                        let request_line = format!("{}\n", request_str);
                        
                        trace!("Connection {} - Forwarding request #{} to upstream", 
                            connection.id(), count);

                        if let Err(e) = upstream_writer.write_all(request_line.as_bytes()).await {
                            error!("Connection {} - Failed to write to upstream: {}", connection.id(), e);
                            break;
                        }

                        if let Err(e) = upstream_writer.flush().await {
                            error!("Connection {} - Failed to flush upstream: {}", connection.id(), e);
                            break;
                        }

                        // Record message sent
                        connection.record_message_sent(request_line.len()).await;
                    }
                    
                    count
                })
            };

            let upstream_to_downstream_task = {
                let connection = connection.clone();
                tokio::spawn(async move {
                    let mut count = 0u64;
                    
                    while let Some(response) = upstream_to_downstream_rx.recv().await {
                        count += 1;
                        
                        let response_str = serde_json::to_string(&response)
                            .unwrap_or_else(|_| "{}".to_string());
                        let response_line = format!("{}\n", response_str);
                        
                        trace!("Connection {} - Forwarding response #{} to downstream", 
                            connection.id(), count);

                        if let Err(e) = downstream_writer.write_all(response_line.as_bytes()).await {
                            error!("Connection {} - Failed to write to downstream: {}", connection.id(), e);
                            break;
                        }

                        if let Err(e) = downstream_writer.flush().await {
                            error!("Connection {} - Failed to flush downstream: {}", connection.id(), e);
                            break;
                        }

                        // Record message sent
                        connection.record_message_sent(response_line.len()).await;
                    }
                    
                    count
                })
            };

            // Wait for both tasks to complete
            let (upstream_count, downstream_count) = 
                tokio::join!(downstream_to_upstream_task, upstream_to_downstream_task);
            
            let upstream_count = upstream_count.unwrap_or(0);
            let downstream_count = downstream_count.unwrap_or(0);

            trace!("Connection {} - Proxy coordinator completed. {} upstream, {} downstream messages", 
                connection.id(), upstream_count, downstream_count);

            Ok(())
        })
    }

    /// Handle authentication messages
    async fn handle_authentication(
        connection: &Arc<Connection>,
        manager: &Arc<Manager>,
        user: &str,
        worker: &str,
        buffer: &str,
        sender: &mpsc::UnboundedSender<Value>,
    ) -> std::result::Result<(), String> {
        debug!("Connection {} - Authenticating user: {}, worker: {}", 
            connection.id(), user, worker);

        // Create authentication state and register with connection
        let auth_state = manager.auth().authenticate(connection.remote_addr(), user, worker);
        connection.authenticate((*auth_state).clone()).await
            .map_err(|e| format!("Authentication failed: {}", e))?;

        // Register with submissions manager
        manager.submissions().authenticated(auth_state);

        // Modify the authentication message for the pool
        let mut message = serde_json::from_str::<Value>(buffer.trim())
            .map_err(|e| format!("JSON parse error: {}", e))?;

        let config = manager.config();
        let pool_username = config.pool.username.as_str();
        let pool_password = config.pool.password.as_deref().unwrap_or("x");
        let (s1, s2) = &config.pool.separator;
        let combined_username = format!("{}{}{}{}{}", pool_username, s1, user, s2, worker);

        debug!("Connection {} - Combined username: {}", connection.id(), combined_username);

        message["params"] = Value::Array(vec![
            Value::String(combined_username),
            Value::String(pool_password.to_owned()),
        ]);

        sender.send(message).map_err(|e| e.to_string())
    }

    /// Handle submission messages
    async fn handle_submission(
        connection: &Arc<Connection>,
        manager: &Arc<Manager>,
        id: u64,
        job_id: &str,
        buffer: &str,
        sender: &mpsc::UnboundedSender<Value>,
    ) -> std::result::Result<(), String> {
        debug!("Connection {} - Share submission: id={}, job_id={}", 
            connection.id(), id, job_id);

        // Get job and auth state
        let jobs = manager.jobs();
        if let Some(job) = jobs.get(job_id) {
            if let Some(auth_state) = connection.auth_session().await {
                manager.submissions().submit(id, job, auth_state.into());
                debug!("Connection {} - Recorded share submission", connection.id());
            } else {
                warn!("Connection {} - Share submission without authentication", connection.id());
            }
        } else {
            warn!("Connection {} - Share submission for unknown job: {}", connection.id(), job_id);
        }

        // Forward the submission
        let message = serde_json::from_str::<Value>(buffer.trim())
            .map_err(|e| format!("JSON parse error: {}", e))?;

        sender.send(message).map_err(|e| e.to_string())
    }

    /// Handle submission responses from the pool
    async fn handle_submission_response(
        connection: &Arc<Connection>,
        manager: &Arc<Manager>,
        response: &Response,
    ) {
        if let Some(auth_state) = connection.auth_session().await {
            if let Some(id) = response.id {
                if let Some(Some(result)) = response.result.as_ref().map(|r| r.as_bool()) {
                    manager.submissions().submitted(&id, result, auth_state.into());
                    connection.record_share_submission(result).await;
                    
                    debug!("Connection {} - Share submission result: id={}, accepted={}", 
                        connection.id(), id, result);
                }
            }
        }
    }
}

impl Drop for NetworkHandler {
    fn drop(&mut self) {
        // The connection will be cleaned up by the ConnectionManager
        debug!("Connection {} - NetworkHandler dropped", self.connection.id());
    }
}