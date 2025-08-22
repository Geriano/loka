use anyhow::Result;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tracing::info;

use super::config::MinerConfig;
use super::worker::MockWorker;

pub struct MockMiner {
    config: Arc<MinerConfig>,
    workers: Vec<JoinHandle<Result<()>>>,
    shutdown_tx: broadcast::Sender<()>,
}

impl MockMiner {
    pub fn new(config: MinerConfig) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);

        Self {
            config: Arc::new(config),
            workers: Vec::new(),
            shutdown_tx,
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        info!(
            "Starting {} mock miners targeting {}",
            self.config.workers, self.config.pool_address
        );
        info!("Total hashrate: {:.2} MH/s", self.config.total_hashrate());

        // Start all workers
        for worker_id in 0..self.config.workers {
            let worker = MockWorker::new(
                worker_id,
                Arc::clone(&self.config),
                self.shutdown_tx.subscribe(),
            );

            let handle = tokio::spawn(async move { worker.start().await });

            self.workers.push(handle);
        }

        info!("All {} workers started", self.config.workers);
        Ok(())
    }

    pub async fn shutdown(self) -> Result<()> {
        info!("Shutting down mock miner...");

        // Send shutdown signal
        let _ = self.shutdown_tx.send(());

        // Wait for all workers to finish
        for (i, handle) in self.workers.into_iter().enumerate() {
            if let Err(e) = handle.await? {
                tracing::error!("Worker {} shutdown error: {}", i, e);
            }
        }

        info!("Mock miner shutdown complete");
        Ok(())
    }

    pub fn worker_count(&self) -> usize {
        self.config.workers
    }

    pub fn config(&self) -> &MinerConfig {
        &self.config
    }
}
