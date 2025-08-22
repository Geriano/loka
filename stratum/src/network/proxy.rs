use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, error, trace, warn};

use crate::error::Result;
use crate::network::connection::{Connection, DisconnectReason};
use crate::network::tasks::TaskManager;

/// Message transformation trait for processing messages between client and server
#[async_trait]
pub trait MessageProcessor: Send + Sync {
    /// Process message from client to server
    async fn process_client_message(
        &self,
        message: Value,
        connection: &Arc<Connection>,
    ) -> Result<Value>;

    /// Process message from server to client  
    async fn process_server_message(
        &self,
        message: Value,
        connection: &Arc<Connection>,
    ) -> Result<Value>;
}

/// Pass-through processor that doesn't modify messages
#[derive(Debug, Default)]
pub struct PassThroughProcessor;

#[async_trait]
impl MessageProcessor for PassThroughProcessor {
    async fn process_client_message(
        &self,
        message: Value,
        _connection: &Arc<Connection>,
    ) -> Result<Value> {
        Ok(message)
    }

    async fn process_server_message(
        &self,
        message: Value,
        _connection: &Arc<Connection>,
    ) -> Result<Value> {
        Ok(message)
    }
}

/// Bidirectional proxy for handling client-server message flow
pub struct BidirectionalProxy {
    connection: Arc<Connection>,
    processor: Arc<dyn MessageProcessor>,
    #[allow(unused)]
    task_manager: Option<Arc<TaskManager>>,
}

impl BidirectionalProxy {
    /// Create a new bidirectional proxy
    pub fn new(connection: Arc<Connection>, processor: Arc<dyn MessageProcessor>) -> Self {
        Self {
            connection,
            processor,
            task_manager: None,
        }
    }

    /// Create a new bidirectional proxy with task management
    pub fn with_task_manager(
        connection: Arc<Connection>,
        processor: Arc<dyn MessageProcessor>,
        task_manager: Arc<TaskManager>,
    ) -> Self {
        Self {
            connection,
            processor,
            task_manager: Some(task_manager),
        }
    }

    /// Create a pass-through proxy that doesn't modify messages
    pub fn pass_through(connection: Arc<Connection>) -> Self {
        Self::new(connection, Arc::new(PassThroughProcessor))
    }

    /// Run the proxy with the given streams
    pub async fn run(
        &self,
        client_reader: OwnedReadHalf,
        client_writer: OwnedWriteHalf,
        server_reader: OwnedReadHalf,
        server_writer: OwnedWriteHalf,
    ) -> Result<()> {
        let (client_to_server_tx, client_to_server_rx) = mpsc::unbounded_channel::<Value>();
        let (server_to_client_tx, server_to_client_rx) = mpsc::unbounded_channel::<Value>();

        // Spawn all proxy tasks
        let client_reader_task = self.spawn_client_reader(client_reader, client_to_server_tx);
        let server_reader_task = self.spawn_server_reader(server_reader, server_to_client_tx);
        let client_writer_task = self.spawn_client_writer(client_writer, server_to_client_rx);
        let server_writer_task = self.spawn_server_writer(server_writer, client_to_server_rx);

        // Wait for all tasks to complete
        let (
            client_reader_result,
            server_reader_result,
            client_writer_result,
            server_writer_result,
        ) = tokio::join!(
            client_reader_task,
            server_reader_task,
            client_writer_task,
            server_writer_task
        );

        // Log any errors
        if let Err(e) = client_reader_result {
            error!(
                "Connection {} - Client reader task failed: {}",
                self.connection.id(),
                e
            );
        }
        if let Err(e) = server_reader_result {
            error!(
                "Connection {} - Server reader task failed: {}",
                self.connection.id(),
                e
            );
        }
        if let Err(e) = client_writer_result {
            error!(
                "Connection {} - Client writer task failed: {}",
                self.connection.id(),
                e
            );
        }
        if let Err(e) = server_writer_result {
            error!(
                "Connection {} - Server writer task failed: {}",
                self.connection.id(),
                e
            );
        }

        debug!("Connection {} - Proxy completed", self.connection.id());
        Ok(())
    }

    /// Spawn client reader task (client -> processor -> server)
    fn spawn_client_reader(
        &self,
        client_reader: OwnedReadHalf,
        sender: mpsc::UnboundedSender<Value>,
    ) -> JoinHandle<Result<()>> {
        let connection = self.connection.clone();
        let processor = self.processor.clone();

        tokio::spawn(async move {
            let mut reader = BufReader::new(client_reader);
            let mut buffer = String::new();

            loop {
                buffer.clear();

                // Check for termination
                if connection
                    .should_terminate(std::time::Duration::from_secs(300))
                    .await
                    .is_some()
                {
                    debug!("Connection {} - Client reader terminating", connection.id());
                    break;
                }

                match reader.read_line(&mut buffer).await {
                    Ok(0) => {
                        debug!("Connection {} - Client disconnected", connection.id());
                        connection
                            .disconnect(DisconnectReason::ClientDisconnect)
                            .await;
                        break;
                    }
                    Ok(bytes_read) => {
                        connection.record_message_received(bytes_read).await;

                        match serde_json::from_str::<Value>(buffer.trim()) {
                            Ok(message) => {
                                trace!(
                                    "Connection {} - Client message: {}",
                                    connection.id(),
                                    serde_json::to_string(&message).unwrap_or_default()
                                );

                                match processor.process_client_message(message, &connection).await {
                                    Ok(processed_message) => {
                                        if let Err(e) = sender.send(processed_message) {
                                            error!(
                                                "Connection {} - Failed to send processed client message: {}",
                                                connection.id(),
                                                e
                                            );
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        error!(
                                            "Connection {} - Failed to process client message: {}",
                                            connection.id(),
                                            e
                                        );
                                        continue;
                                    }
                                }
                            }
                            Err(e) => {
                                warn!(
                                    "Connection {} - Invalid JSON from client: {} - {}",
                                    connection.id(),
                                    buffer.trim(),
                                    e
                                );
                                continue;
                            }
                        }
                    }
                    Err(e) => {
                        error!(
                            "Connection {} - Error reading from client: {}",
                            connection.id(),
                            e
                        );
                        connection
                            .disconnect(DisconnectReason::NetworkError {
                                error: e.to_string(),
                            })
                            .await;
                        break;
                    }
                }
            }

            Ok(())
        })
    }

    /// Spawn server reader task (server -> processor -> client)
    fn spawn_server_reader(
        &self,
        server_reader: OwnedReadHalf,
        sender: mpsc::UnboundedSender<Value>,
    ) -> JoinHandle<Result<()>> {
        let connection = self.connection.clone();
        let processor = self.processor.clone();

        tokio::spawn(async move {
            let mut reader = BufReader::new(server_reader);
            let mut buffer = String::new();

            loop {
                buffer.clear();

                // Check for termination
                if connection
                    .should_terminate(std::time::Duration::from_secs(300))
                    .await
                    .is_some()
                {
                    debug!("Connection {} - Server reader terminating", connection.id());
                    break;
                }

                match reader.read_line(&mut buffer).await {
                    Ok(0) => {
                        debug!("Connection {} - Server disconnected", connection.id());
                        connection
                            .disconnect(DisconnectReason::NetworkError {
                                error: "Server connection closed".to_string(),
                            })
                            .await;
                        break;
                    }
                    Ok(bytes_read) => {
                        connection.record_message_received(bytes_read).await;

                        match serde_json::from_str::<Value>(buffer.trim()) {
                            Ok(message) => {
                                trace!(
                                    "Connection {} - Server message: {}",
                                    connection.id(),
                                    serde_json::to_string(&message).unwrap_or_default()
                                );

                                match processor.process_server_message(message, &connection).await {
                                    Ok(processed_message) => {
                                        if let Err(e) = sender.send(processed_message) {
                                            error!(
                                                "Connection {} - Failed to send processed server message: {}",
                                                connection.id(),
                                                e
                                            );
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        error!(
                                            "Connection {} - Failed to process server message: {}",
                                            connection.id(),
                                            e
                                        );
                                        continue;
                                    }
                                }
                            }
                            Err(e) => {
                                warn!(
                                    "Connection {} - Invalid JSON from server: {} - {}",
                                    connection.id(),
                                    buffer.trim(),
                                    e
                                );
                                continue;
                            }
                        }
                    }
                    Err(e) => {
                        error!(
                            "Connection {} - Error reading from server: {}",
                            connection.id(),
                            e
                        );
                        connection
                            .disconnect(DisconnectReason::NetworkError {
                                error: e.to_string(),
                            })
                            .await;
                        break;
                    }
                }
            }

            Ok(())
        })
    }

    /// Spawn client writer task (receives messages to send to client)
    fn spawn_client_writer(
        &self,
        client_writer: OwnedWriteHalf,
        mut receiver: mpsc::UnboundedReceiver<Value>,
    ) -> JoinHandle<Result<()>> {
        let connection = self.connection.clone();

        tokio::spawn(async move {
            let mut writer = BufWriter::new(client_writer);
            let mut count = 0u64;

            while let Some(message) = receiver.recv().await {
                count += 1;

                let message_str =
                    serde_json::to_string(&message).unwrap_or_else(|_| "{}".to_string());
                let message_line = format!("{}\n", message_str);

                trace!(
                    "Connection {} - Sending message #{} to client",
                    connection.id(),
                    count
                );

                if let Err(e) = writer.write_all(message_line.as_bytes()).await {
                    error!(
                        "Connection {} - Failed to write to client: {}",
                        connection.id(),
                        e
                    );
                    break;
                }

                if let Err(e) = writer.flush().await {
                    error!(
                        "Connection {} - Failed to flush client writer: {}",
                        connection.id(),
                        e
                    );
                    break;
                }

                connection.record_message_sent(message_line.len()).await;
            }

            debug!(
                "Connection {} - Client writer completed ({} messages)",
                connection.id(),
                count
            );
            Ok(())
        })
    }

    /// Spawn server writer task (receives messages to send to server)
    fn spawn_server_writer(
        &self,
        server_writer: OwnedWriteHalf,
        mut receiver: mpsc::UnboundedReceiver<Value>,
    ) -> JoinHandle<Result<()>> {
        let connection = self.connection.clone();

        tokio::spawn(async move {
            let mut writer = BufWriter::new(server_writer);
            let mut count = 0u64;

            while let Some(message) = receiver.recv().await {
                count += 1;

                let message_str =
                    serde_json::to_string(&message).unwrap_or_else(|_| "{}".to_string());
                let message_line = format!("{}\n", message_str);

                trace!(
                    "Connection {} - Sending message #{} to server",
                    connection.id(),
                    count
                );

                if let Err(e) = writer.write_all(message_line.as_bytes()).await {
                    error!(
                        "Connection {} - Failed to write to server: {}",
                        connection.id(),
                        e
                    );
                    break;
                }

                if let Err(e) = writer.flush().await {
                    error!(
                        "Connection {} - Failed to flush server writer: {}",
                        connection.id(),
                        e
                    );
                    break;
                }

                connection.record_message_sent(message_line.len()).await;
            }

            debug!(
                "Connection {} - Server writer completed ({} messages)",
                connection.id(),
                count
            );
            Ok(())
        })
    }
}

/// Proxy builder for easy configuration
pub struct ProxyBuilder {
    connection: Arc<Connection>,
    processor: Option<Arc<dyn MessageProcessor>>,
}

impl ProxyBuilder {
    /// Create a new proxy builder
    pub fn new(connection: Arc<Connection>) -> Self {
        Self {
            connection,
            processor: None,
        }
    }

    /// Set the message processor
    pub fn with_processor(mut self, processor: Arc<dyn MessageProcessor>) -> Self {
        self.processor = Some(processor);
        self
    }

    /// Build the proxy
    pub fn build(self) -> BidirectionalProxy {
        let processor = self
            .processor
            .unwrap_or_else(|| Arc::new(PassThroughProcessor));
        BidirectionalProxy::new(self.connection, processor)
    }
}
