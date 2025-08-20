use std::sync::Arc;
use std::time::Instant;
use tokio::net::{TcpListener, TcpStream};
use tracing;

use crate::handler::Handler;
use crate::{Config, Manager};

pub struct Listener {
    listener: TcpListener,
    manager: Arc<Manager>,
    handler_factory: Handler,
}

impl Listener {
    pub async fn new(config: Arc<Config>) -> anyhow::Result<Self> {
        let listener = TcpListener::bind(config.server.bind_address).await?;

        tracing::info!(
            "Stratum proxy listening on {} (forwarding to {})",
            config.server.bind_address,
            config.pool.address
        );

        let manager = Arc::new(Manager::new(config.clone()));

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

                    metrics::counter!("network_connected_total").increment(1);

                    // Parse the pool address from the configuration
                    let upstream_addr = self.manager.config().pool.address.parse::<std::net::SocketAddr>()?;

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

                            let handler = self.handler_factory.create(addr);

                            tokio::spawn(async move {
                                if let Err(e) = handler.run(downstream, upstream).await {
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
}
