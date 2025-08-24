use std::sync::Arc;
use std::time::Instant;
use tokio::net::{TcpListener, TcpStream};

use crate::handler::Handler;
use crate::services::database::DatabaseService;
use crate::{Config, Manager};

pub struct Listener {
    listener: TcpListener,
    manager: Arc<Manager>,
    handler_factory: Handler,
}

impl Listener {
    pub async fn new(config: Config) -> anyhow::Result<Self> {
        let config = Arc::new(config);
        let listener = TcpListener::bind(config.server.bind_address).await?;

        tracing::info!(
            "Stratum proxy listening on {} (forwarding to {})",
            config.server.bind_address,
            config.pool.address
        );

        // Initialize database service (always required)
        tracing::info!(
            "Initializing database connection to {}",
            config.database.url
        );
        let database = DatabaseService::new(&config.database.url)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to initialize database: {}", e))?;
        tracing::info!("Database connected successfully");
        let database = Arc::new(database);

        let manager = Arc::new(
            Manager::new(config.clone(), database)
                .map_err(|e| anyhow::anyhow!("Failed to initialize manager: {}", e))?,
        );

        // Create handler factory with protocol handler
        let handler_factory = Handler::new(manager.clone(), config.clone());

        tracing::info!("Using Protocol handler implementation");

        Ok(Self {
            listener,
            manager,
            handler_factory,
        })
    }

    pub async fn accept(&self) -> anyhow::Result<()> {
        loop {
            let manager = Arc::clone(&self.manager);

            match self.listener.accept().await {
                Ok((downstream, addr)) => {
                    tracing::info!("new miner connection from {}", addr);

                    // Track connection establishment (Task 8.1)
                    manager.connection_established(&addr);

                    metrics::counter!("network_connected_total").increment(1);

                    let connection_path: Option<String> = None;
                    let processed_downstream = downstream;

                    // Parse the pool address from the configuration
                    let upstream_addr = self
                        .manager
                        .config()
                        .pool
                        .address
                        .parse::<std::net::SocketAddr>()?;

                    let start = Instant::now();

                    match TcpStream::connect(&upstream_addr).await {
                        Ok(upstream) => {
                            let latency = start.elapsed();

                            metrics::histogram!("network_pool_latency")
                                .record(latency.as_secs_f64());

                            tracing::info!(
                                "miner {} - Successfully connected to upstream {} in {:#.3?}",
                                addr,
                                upstream_addr,
                                latency
                            );

                            let handler = self.handler_factory.create(addr, connection_path);

                            tokio::spawn(async move {
                                if let Err(e) = handler.run(processed_downstream, upstream).await {
                                    tracing::error!("miner {} - Error: {}", addr, e);
                                }

                                manager.terminated(&addr);

                                metrics::counter!("network_disconnected_total").increment(1);
                            });
                        }
                        Err(e) => {
                            let latency = start.elapsed();

                            metrics::histogram!("network_pool_latency")
                                .record(latency.as_secs_f64());

                            tracing::error!(
                                "miner {} - Failed to connect to upstream {}: {} in {:#.3?}",
                                addr,
                                upstream_addr,
                                e,
                                latency
                            );

                            manager.terminated(&addr);

                            metrics::counter!("network_disconnected_total").increment(1);
                            metrics::counter!("network_connection_failed_total").increment(1);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to accept connection: {}", e);
                }
            }
        }
    }

    // /// Handle HTTP CONNECT request and extract path (NiceHash-style)
    // /// Returns the processed stream and any extracted path
    // async fn handle_http_connect(
    //     stream: TcpStream,
    //     addr: std::net::SocketAddr,
    // ) -> (TcpStream, Option<String>) {
    //     tracing::debug!("miner {} - Checking for HTTP CONNECT request", addr);

    //     // Try to read the first line to check if it's HTTP CONNECT
    //     let result = Self::process_potential_connect(stream, addr).await;
    //     let (processed_stream, path) = result;

    //     if let Some(ref p) = path {
    //         tracing::info!("miner {} - Path extracted from HTTP CONNECT: {}", addr, p);
    //     } else {
    //         tracing::debug!(
    //             "miner {} - Direct Stratum connection (no HTTP CONNECT)",
    //             addr
    //         );
    //     }

    //     (processed_stream, path)
    // }

    // /// Process potential HTTP CONNECT request
    // async fn process_potential_connect(
    //     stream: TcpStream,
    //     addr: std::net::SocketAddr,
    // ) -> (TcpStream, Option<String>) {
    //     // Use a different approach - peek at the data without consuming it
    //     let mut buf = [0u8; 1024];

    //     // Try to peek at the data first
    //     match stream.try_read(&mut buf) {
    //         Ok(n) if n > 0 => {
    //             let data = String::from_utf8_lossy(&buf[..n]);
    //             let first_line = data.lines().next().unwrap_or("");

    //             tracing::debug!("miner {} - First line detected: {}", addr, first_line);

    //             if first_line.starts_with("CONNECT ") {
    //                 // It's an HTTP CONNECT request - we need to consume it properly
    //                 let mut reader = BufReader::new(stream);
    //                 let mut line = String::new();
    //                 let _ = reader.read_line(&mut line).await;

    //                 let path = Self::parse_connect_path(&line);

    //                 // Send HTTP response
    //                 let mut stream = reader.into_inner();
    //                 if let Err(e) = stream
    //                     .write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
    //                     .await
    //                 {
    //                     tracing::error!("miner {} - Failed to send HTTP response: {}", addr, e);
    //                 }

    //                 tracing::info!(
    //                     "miner {} - HTTP CONNECT tunnel established with path: {:?}",
    //                     addr,
    //                     path
    //                 );
    //                 (stream, path)
    //             } else {
    //                 // It's direct Stratum - return the stream as-is
    //                 tracing::debug!(
    //                     "miner {} - Direct Stratum connection (no HTTP CONNECT)",
    //                     addr
    //                 );
    //                 (stream, None)
    //             }
    //         }
    //         Ok(0) => {
    //             tracing::debug!("miner {} - No data available", addr);
    //             (stream, None)
    //         }
    //         Ok(_) => {
    //             // Handle any other successful read size
    //             tracing::debug!("miner {} - Data available, assuming direct Stratum", addr);
    //             (stream, None)
    //         }
    //         Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
    //             // No data ready yet - assume direct Stratum
    //             tracing::debug!(
    //                 "miner {} - No immediate data, assuming direct Stratum",
    //                 addr
    //             );
    //             (stream, None)
    //         }
    //         Err(e) => {
    //             tracing::debug!("miner {} - Error peeking at stream: {}", addr, e);
    //             (stream, None)
    //         }
    //     }
    // }

    // /// Parse the path from HTTP CONNECT request line
    // /// Format: "CONNECT host:port/path HTTP/1.1"
    // fn parse_connect_path(connect_line: &str) -> Option<String> {
    //     let parts: Vec<&str> = connect_line.split_whitespace().collect();
    //     if parts.len() >= 2 && parts[0] == "CONNECT" {
    //         let target = parts[1];

    //         // Look for path after the port
    //         // Format: host:port/path or host:port/path/subpath
    //         if let Some(path_start) = target.find('/') {
    //             let path = &target[path_start + 1..];
    //             if !path.is_empty() {
    //                 tracing::debug!("Extracted path from CONNECT: {}", path);
    //                 return Some(path.to_string());
    //             }
    //         }
    //     }
    //     None
    // }
}
