use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::{broadcast, RwLock};
use tokio::time::{interval, sleep};
use tracing::{debug, error, info, warn};

use super::config::MinerConfig;
use super::hashrate::HashrateSimulator;
use super::messages::{MiningJob, StratumMessages};

#[derive(Debug, Clone)]
pub struct WorkerStats {
    pub worker_id: usize,
    pub worker_name: String,
    pub connected: bool,
    pub authorized: bool,
    pub current_difficulty: f64,
    pub shares_submitted: u64,
    pub shares_accepted: u64,
    pub shares_rejected: u64,
    pub last_submit_time: Option<Instant>,
    pub connection_time: Option<Instant>,
    pub total_runtime: Duration,
}

impl WorkerStats {
    pub fn acceptance_rate(&self) -> f64 {
        if self.shares_submitted > 0 {
            (self.shares_accepted as f64 / self.shares_submitted as f64) * 100.0
        } else {
            0.0
        }
    }
    
    pub fn shares_per_minute(&self) -> f64 {
        if let Some(start_time) = self.connection_time {
            let runtime_minutes = start_time.elapsed().as_secs_f64() / 60.0;
            if runtime_minutes > 0.0 {
                self.shares_submitted as f64 / runtime_minutes
            } else {
                0.0
            }
        } else {
            0.0
        }
    }
}

pub struct MockWorker {
    worker_id: usize,
    worker_name: String,
    config: Arc<MinerConfig>,
    hashrate_sim: HashrateSimulator,
    stats: Arc<RwLock<WorkerStats>>,
    shutdown_rx: broadcast::Receiver<()>,
    current_job: Arc<RwLock<Option<MiningJob>>>,
    current_difficulty: Arc<RwLock<f64>>,
    extranonce1: Arc<RwLock<Option<String>>>,
    extranonce2_size: Arc<RwLock<usize>>,
}

impl MockWorker {
    pub fn new(
        worker_id: usize,
        config: Arc<MinerConfig>,
        shutdown_rx: broadcast::Receiver<()>,
    ) -> Self {
        let worker_name = config.worker_name(worker_id);
        let hashrate_sim = HashrateSimulator::new(
            config.hashrate_mhs,
            config.difficulty_variance,
        );
        
        let stats = WorkerStats {
            worker_id,
            worker_name: worker_name.clone(),
            connected: false,
            authorized: false,
            current_difficulty: 1.0,
            shares_submitted: 0,
            shares_accepted: 0,
            shares_rejected: 0,
            last_submit_time: None,
            connection_time: None,
            total_runtime: Duration::ZERO,
        };
        
        Self {
            worker_id,
            worker_name,
            config,
            hashrate_sim,
            stats: Arc::new(RwLock::new(stats)),
            shutdown_rx,
            current_job: Arc::new(RwLock::new(None)),
            current_difficulty: Arc::new(RwLock::new(1.0)),
            extranonce1: Arc::new(RwLock::new(None)),
            extranonce2_size: Arc::new(RwLock::new(4)),
        }
    }
    
    pub async fn start(self) -> Result<()> {
        info!("Starting worker {}", self.worker_name);
        
        let mut reconnect_attempts = 0;
        let mut shutdown_rx = self.shutdown_rx.resubscribe();
        
        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    info!("Worker {} shutting down", self.worker_name);
                    break;
                }
                result = self.run_connection() => {
                    match result {
                        Ok(_) => {
                            info!("Worker {} connection ended normally", self.worker_name);
                            break;
                        }
                        Err(e) => {
                            error!("Worker {} connection error: {}", self.worker_name, e);
                            
                            {
                                let mut stats = self.stats.write().await;
                                stats.connected = false;
                                stats.authorized = false;
                            }
                            
                            reconnect_attempts += 1;
                            if reconnect_attempts >= self.config.reconnect_attempts {
                                error!("Worker {} max reconnection attempts reached", self.worker_name);
                                break;
                            }
                            
                            warn!("Worker {} reconnecting in {} seconds (attempt {})", 
                                  self.worker_name, self.config.reconnect_delay_secs, reconnect_attempts);
                            sleep(self.config.reconnect_delay()).await;
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    async fn run_connection(&self) -> Result<()> {
        // Connect to pool
        let stream = tokio::time::timeout(
            self.config.connection_timeout(),
            TcpStream::connect(&self.config.pool_address),
        ).await??;
        
        info!("Worker {} connected to {}", self.worker_name, self.config.pool_address);
        
        {
            let mut stats = self.stats.write().await;
            stats.connected = true;
            stats.connection_time = Some(Instant::now());
        }
        
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        
        // Subscribe to mining
        self.send_message(&mut writer, &StratumMessages::mining_subscribe()).await?;
        
        // Create a shared message sender
        let (message_tx, mut message_rx) = tokio::sync::mpsc::channel(100);
        let message_tx = Arc::new(message_tx);
        
        // Start writer task
        let writer_handle = tokio::spawn(async move {
            while let Some(msg) = message_rx.recv().await {
                if let Err(e) = Self::send_message_static(&mut writer, &msg).await {
                    error!("Writer error: {}", e);
                    break;
                }
            }
        });
        
        // Start keepalive task
        let keepalive_handle = self.start_keepalive_task(Arc::clone(&message_tx));
        
        // Start mining task
        let mining_handle = self.start_mining_task(Arc::clone(&message_tx));
        
        // Read responses
        let mut line = String::new();
        let mut shutdown_rx = self.shutdown_rx.resubscribe();
        
        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    break;
                }
                read_result = reader.read_line(&mut line) => {
                    match read_result {
                        Ok(0) => {
                            warn!("Worker {} connection closed by server", self.worker_name);
                            break;
                        }
                        Ok(_) => {
                            if let Err(e) = self.handle_message(&line, &message_tx).await {
                                error!("Worker {} message handling error: {}", self.worker_name, e);
                            }
                            line.clear();
                        }
                        Err(e) => {
                            error!("Worker {} read error: {}", self.worker_name, e);
                            break;
                        }
                    }
                }
            }
        }
        
        // Cleanup
        keepalive_handle.abort();
        mining_handle.abort();
        writer_handle.abort();
        
        Ok(())
    }
    
    async fn handle_message(&self, message: &str, message_tx: &Arc<tokio::sync::mpsc::Sender<serde_json::Value>>) -> Result<()> {
        let msg: Value = serde_json::from_str(message)?;
        
        if let Some(method) = msg["method"].as_str() {
            match method {
                "mining.notify" => {
                    if let Some(params) = msg["params"].as_array() {
                        let difficulty = *self.current_difficulty.read().await;
                        if let Some(job) = MiningJob::from_notify_params(params, difficulty) {
                            debug!("Worker {} received new job: {}", self.worker_name, job.job_id);
                            *self.current_job.write().await = Some(job);
                        }
                    }
                }
                "mining.set_difficulty" => {
                    if let Some(params) = msg["params"].as_array() {
                        if let Some(difficulty) = params.get(0).and_then(|v| v.as_f64()) {
                            debug!("Worker {} difficulty set to: {}", self.worker_name, difficulty);
                            *self.current_difficulty.write().await = difficulty;
                            
                            let mut stats = self.stats.write().await;
                            stats.current_difficulty = difficulty;
                        }
                    }
                }
                _ => {
                    debug!("Worker {} received method: {}", self.worker_name, method);
                }
            }
        } else if let Some(_id) = msg["id"].as_u64() {
            // Handle response
            if let Some(result) = msg.get("result") {
                if let Some(subscribe_result) = result.as_array() {
                    // mining.subscribe response
                    if subscribe_result.len() >= 3 {
                        if let Some(extranonce1) = subscribe_result[1].as_str() {
                            *self.extranonce1.write().await = Some(extranonce1.to_string());
                        }
                        if let Some(extranonce2_size) = subscribe_result[2].as_u64() {
                            *self.extranonce2_size.write().await = extranonce2_size as usize;
                        }
                        
                        // Now authorize
                        let auth_msg = StratumMessages::mining_authorize(&self.config.username, &self.config.password);
                        message_tx.send(auth_msg).await?;
                    }
                } else if result.as_bool() == Some(true) {
                    // Successful authorization or share submission
                    let mut stats = self.stats.write().await;
                    if !stats.authorized {
                        stats.authorized = true;
                        info!("Worker {} authorized successfully", self.worker_name);
                    } else {
                        stats.shares_accepted += 1;
                        if self.config.log_mining {
                            debug!("Worker {} share accepted", self.worker_name);
                        }
                    }
                }
            } else if msg.get("error").is_some() {
                // Handle error
                if let Some(error) = msg["error"].as_array() {
                    warn!("Worker {} error: {:?}", self.worker_name, error);
                    
                    let mut stats = self.stats.write().await;
                    stats.shares_rejected += 1;
                }
            }
        }
        
        Ok(())
    }
    
    async fn send_message(&self, writer: &mut tokio::net::tcp::OwnedWriteHalf, msg: &Value) -> Result<()> {
        let mut msg_str = msg.to_string();
        msg_str.push('\n');
        writer.write_all(msg_str.as_bytes()).await?;
        writer.flush().await?;
        Ok(())
    }
    
    fn start_keepalive_task(&self, message_tx: Arc<tokio::sync::mpsc::Sender<serde_json::Value>>) -> tokio::task::JoinHandle<()> {
        let config = Arc::clone(&self.config);
        let worker_name = self.worker_name.clone();
        
        tokio::spawn(async move {
            let mut interval = interval(config.keepalive_interval());
            
            loop {
                interval.tick().await;
                
                let ping_msg = StratumMessages::mining_ping();
                if let Err(e) = message_tx.send(ping_msg).await {
                    error!("Worker {} keepalive error: {}", worker_name, e);
                    break;
                }
            }
        })
    }
    
    fn start_mining_task(&self, message_tx: Arc<tokio::sync::mpsc::Sender<serde_json::Value>>) -> tokio::task::JoinHandle<()> {
        let config = Arc::clone(&self.config);
        let worker_name = self.worker_name.clone();
        let stats = Arc::clone(&self.stats);
        let current_job = Arc::clone(&self.current_job);
        let current_difficulty = Arc::clone(&self.current_difficulty);
        let extranonce1 = Arc::clone(&self.extranonce1);
        let extranonce2_size = Arc::clone(&self.extranonce2_size);
        let hashrate_sim = self.hashrate_sim.clone();
        
        tokio::spawn(async move {
            let mut hashrate_sim = hashrate_sim;
            
            loop {
                // Wait for authorization and job
                let (job, difficulty, _en1, en2_size) = {
                    let stats_guard = stats.read().await;
                    if !stats_guard.authorized {
                        sleep(Duration::from_millis(100)).await;
                        continue;
                    }
                    
                    let job_guard = current_job.read().await;
                    let job = match job_guard.as_ref() {
                        Some(j) => j.clone(),
                        None => {
                            sleep(Duration::from_millis(100)).await;
                            continue;
                        }
                    };
                    
                    let difficulty = *current_difficulty.read().await;
                    let en1 = extranonce1.read().await.clone();
                    let en2_size = *extranonce2_size.read().await;
                    
                    (job, difficulty, en1, en2_size)
                };
                
                // Calculate time to next share based on difficulty and hashrate
                let share_interval = hashrate_sim.calculate_share_interval(difficulty);
                sleep(share_interval).await;
                
                // Generate share
                let extranonce2 = hashrate_sim.generate_extranonce2(en2_size);
                let nonce = hashrate_sim.generate_realistic_nonce();
                let ntime = job.ntime.clone(); // Use job's ntime or modify slightly
                
                // Determine if share should be stale or invalid
                let is_stale = config.should_submit_stale();
                let is_invalid = config.should_submit_invalid();
                
                let (final_job_id, final_nonce) = if is_stale {
                    // Use old job ID to simulate stale share
                    (format!("stale_{}", job.job_id), nonce)
                } else if is_invalid {
                    // Use invalid nonce
                    ("invalid_job".to_string(), "invalid".to_string())
                } else {
                    (job.job_id.clone(), nonce)
                };
                
                // Submit share
                let submit_msg = StratumMessages::mining_submit(
                    &worker_name,
                    &final_job_id,
                    &extranonce2,
                    &ntime,
                    &final_nonce,
                );
                
                if let Err(e) = message_tx.send(submit_msg).await {
                    error!("Worker {} submit error: {}", worker_name, e);
                    break;
                }
                
                // Update stats
                {
                    let mut stats_guard = stats.write().await;
                    stats_guard.shares_submitted += 1;
                    stats_guard.last_submit_time = Some(Instant::now());
                }
                
                // Update hashrate simulation
                hashrate_sim.update_hashrate_sample(difficulty, share_interval);
                
                if config.log_mining {
                    debug!("Worker {} submitted share for job {}", worker_name, final_job_id);
                }
            }
        })
    }
    
    async fn send_message_static(writer: &mut tokio::net::tcp::OwnedWriteHalf, msg: &Value) -> Result<()> {
        let mut msg_str = msg.to_string();
        msg_str.push('\n');
        writer.write_all(msg_str.as_bytes()).await?;
        writer.flush().await?;
        Ok(())
    }
    
    pub async fn get_stats(&self) -> WorkerStats {
        self.stats.read().await.clone()
    }
}