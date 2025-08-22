use anyhow::Result;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::signal;
use tokio::sync::{RwLock, broadcast};
use tokio::time::{interval, sleep};
use tracing::info;

use super::client::MockMiner;
use super::config::MinerConfig;
use super::worker::WorkerStats;

#[derive(Debug, Clone)]
pub struct SimulationStats {
    pub start_time: Instant,
    pub total_runtime: Duration,
    pub total_workers: usize,
    pub connected_workers: usize,
    pub authorized_workers: usize,
    pub total_shares_submitted: u64,
    pub total_shares_accepted: u64,
    pub total_shares_rejected: u64,
    pub average_hashrate_mhs: f64,
    pub acceptance_rate: f64,
    pub shares_per_minute: f64,
}

impl SimulationStats {
    pub fn new(total_workers: usize) -> Self {
        Self {
            start_time: Instant::now(),
            total_runtime: Duration::ZERO,
            total_workers,
            connected_workers: 0,
            authorized_workers: 0,
            total_shares_submitted: 0,
            total_shares_accepted: 0,
            total_shares_rejected: 0,
            average_hashrate_mhs: 0.0,
            acceptance_rate: 0.0,
            shares_per_minute: 0.0,
        }
    }

    pub fn update_from_workers(&mut self, worker_stats: &[WorkerStats]) {
        self.total_runtime = self.start_time.elapsed();
        self.connected_workers = worker_stats.iter().filter(|w| w.connected).count();
        self.authorized_workers = worker_stats.iter().filter(|w| w.authorized).count();

        self.total_shares_submitted = worker_stats.iter().map(|w| w.shares_submitted).sum();
        self.total_shares_accepted = worker_stats.iter().map(|w| w.shares_accepted).sum();
        self.total_shares_rejected = worker_stats.iter().map(|w| w.shares_rejected).sum();

        self.acceptance_rate = if self.total_shares_submitted > 0 {
            (self.total_shares_accepted as f64 / self.total_shares_submitted as f64) * 100.0
        } else {
            0.0
        };

        let runtime_minutes = self.total_runtime.as_secs_f64() / 60.0;
        self.shares_per_minute = if runtime_minutes > 0.0 {
            self.total_shares_submitted as f64 / runtime_minutes
        } else {
            0.0
        };
    }
}

impl std::fmt::Display for SimulationStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Runtime: {:.1}s | Workers: {}/{} connected, {}/{} authorized | Shares: {} submitted, {} accepted ({:.1}%) | Rate: {:.1}/min",
            self.total_runtime.as_secs_f64(),
            self.connected_workers,
            self.total_workers,
            self.authorized_workers,
            self.total_workers,
            self.total_shares_submitted,
            self.total_shares_accepted,
            self.acceptance_rate,
            self.shares_per_minute
        )
    }
}

pub struct MinerSimulator {
    config: MinerConfig,
    stats: Arc<RwLock<SimulationStats>>,
    shutdown_tx: broadcast::Sender<()>,
    shutdown_rx: broadcast::Receiver<()>,
}

impl MinerSimulator {
    pub fn new(config: MinerConfig) -> Self {
        let stats = SimulationStats::new(config.workers);
        let (shutdown_tx, shutdown_rx) = broadcast::channel(1);

        Self {
            config,
            stats: Arc::new(RwLock::new(stats)),
            shutdown_tx,
            shutdown_rx,
        }
    }

    pub async fn run(self) -> Result<()> {
        info!(
            "Starting mining simulation with {} workers",
            self.config.workers
        );
        info!("Target pool: {}", self.config.pool_address);
        info!("Total hashrate: {:.2} MH/s", self.config.total_hashrate());

        if let Some(duration) = self.config.simulation_duration() {
            info!("Simulation duration: {:.1} seconds", duration.as_secs_f64());
        } else {
            info!("Simulation duration: infinite (Ctrl+C to stop)");
        }

        // Start mock miner
        let mut miner = MockMiner::new(self.config.clone());
        miner.start().await?;

        // Start statistics reporting
        let stats_handle = self.start_stats_reporting();

        // Wait for shutdown condition
        let duration = self.config.simulation_duration();
        let mut shutdown_rx = self.shutdown_rx;

        tokio::select! {
            _ = signal::ctrl_c() => {
                info!("Received Ctrl+C, shutting down...");
            }
            _ = async {
                if let Some(duration) = duration {
                    sleep(duration).await;
                } else {
                    tokio::time::sleep(Duration::from_secs(u64::MAX)).await;
                }
            } => {
                info!("Simulation duration reached, shutting down...");
            }
            _ = shutdown_rx.recv() => {
                info!("Received shutdown signal");
            }
        }

        // Cleanup
        stats_handle.abort();
        let _ = self.shutdown_tx.send(());

        // Shutdown miner
        miner.shutdown().await?;

        // Print final statistics
        let final_stats = self.stats.read().await;
        info!("Final statistics: {}", *final_stats);

        Ok(())
    }

    fn start_stats_reporting(&self) -> tokio::task::JoinHandle<()> {
        let stats = Arc::clone(&self.stats);
        let _config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(10)); // Report every 10 seconds

            loop {
                interval.tick().await;

                // Note: In a real implementation, we would collect actual worker stats
                // For now, we'll just report the current stats
                let stats_guard = stats.read().await;
                info!("Stats: {}", *stats_guard);
            }
        })
    }

    pub async fn get_stats(&self) -> SimulationStats {
        self.stats.read().await.clone()
    }

    pub async fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }

    pub fn config(&self) -> &MinerConfig {
        &self.config
    }
}
