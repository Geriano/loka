use anyhow::{Result, anyhow};
use serde_json::Value;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{RwLock, broadcast};
use tokio::task::JoinHandle;
use tokio::time::{interval, sleep};
use tracing::{debug, error, info};
use uuid::Uuid;

use super::config::MockConfig;
use super::difficulty::MockDifficultyManager;
use super::job_manager::MockJobManager;
use super::responses::MockResponses;
use super::validator::{MockShareValidator, ShareSubmission};

#[derive(Clone)]
struct ClientSession {
    id: String,
    worker: Option<String>,
    extranonce1: String,
    authorized: bool,
}

pub struct MockPool {
    config: MockConfig,
    job_manager: Arc<MockJobManager>,
    difficulty_manager: Arc<MockDifficultyManager>,
    validator: Arc<MockShareValidator>,
    sessions: Arc<RwLock<HashMap<String, ClientSession>>>,
    shutdown_tx: broadcast::Sender<()>,
    shutdown_rx: broadcast::Receiver<()>,
}

impl MockPool {
    pub fn new(config: MockConfig) -> Self {
        let (shutdown_tx, shutdown_rx) = broadcast::channel(1);

        let difficulty_manager = Arc::new(MockDifficultyManager::new(
            config.initial_difficulty,
            config.vardiff_target_time_secs,
            config.vardiff_enabled,
        ));

        Self {
            job_manager: Arc::new(MockJobManager::new()),
            difficulty_manager,
            validator: Arc::new(MockShareValidator::new(config.accept_rate)),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            shutdown_tx,
            shutdown_rx,
            config,
        }
    }

    pub async fn start(self, addr: &str) -> Result<MockPoolHandle> {
        let listener = TcpListener::bind(addr).await?;
        let local_addr = listener.local_addr()?;
        info!("Mock pool listening on {}", local_addr);

        let pool = Arc::new(self);
        let accept_handle = tokio::spawn(Self::accept_loop(Arc::clone(&pool), listener));
        let job_rotation_handle = tokio::spawn(Self::job_rotation_loop(Arc::clone(&pool)));

        Ok(MockPoolHandle {
            accept_handle,
            job_rotation_handle,
            shutdown_tx: pool.shutdown_tx.clone(),
            local_addr,
        })
    }

    async fn accept_loop(pool: Arc<Self>, listener: TcpListener) {
        let mut shutdown_rx = pool.shutdown_rx.resubscribe();

        loop {
            tokio::select! {
                accept_result = listener.accept() => {
                    match accept_result {
                        Ok((stream, addr)) => {
                            let pool = Arc::clone(&pool);
                            tokio::spawn(async move {
                                if let Err(e) = pool.handle_client(stream, addr).await {
                                    error!("Client handler error: {}", e);
                                }
                            });
                        }
                        Err(e) => {
                            error!("Accept error: {}", e);
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("Mock pool accept loop shutting down");
                    break;
                }
            }
        }
    }

    async fn job_rotation_loop(pool: Arc<Self>) {
        let mut interval = interval(pool.config.job_interval());
        let mut shutdown_rx = pool.shutdown_rx.resubscribe();

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = pool.broadcast_new_job(false).await {
                        error!("Failed to broadcast job: {}", e);
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("Job rotation loop shutting down");
                    break;
                }
            }
        }
    }

    async fn handle_client(&self, stream: TcpStream, addr: SocketAddr) -> Result<()> {
        info!("New connection from {}", addr);

        let session_id = Uuid::new_v4().to_string();
        let extranonce1 = generate_extranonce1();

        let session = ClientSession {
            id: session_id.clone(),
            worker: None,
            extranonce1: extranonce1.clone(),
            authorized: false,
        };

        self.sessions
            .write()
            .await
            .insert(session_id.clone(), session.clone());

        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        let mut line = String::new();

        let mut shutdown_rx = self.shutdown_rx.resubscribe();

        loop {
            tokio::select! {
                read_result = reader.read_line(&mut line) => {
                    match read_result {
                        Ok(0) => {
                            info!("Client {} disconnected", addr);
                            break;
                        }
                        Ok(_) => {
                            if let Err(e) = self.handle_message(&session_id, &line, &mut writer).await {
                                error!("Message handling error: {}", e);
                            }
                            line.clear();
                        }
                        Err(e) => {
                            error!("Read error from {}: {}", addr, e);
                            break;
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("Client handler for {} shutting down", addr);
                    break;
                }
            }
        }

        self.sessions.write().await.remove(&session_id);
        if let Some(worker) = session.worker {
            self.difficulty_manager.remove_worker(&worker).await;
        }

        Ok(())
    }

    async fn handle_message(
        &self,
        session_id: &str,
        message: &str,
        writer: &mut tokio::net::tcp::OwnedWriteHalf,
    ) -> Result<()> {
        let msg: Value = serde_json::from_str(message)?;

        if self.config.should_inject_error() {
            let error_response =
                MockResponses::error_response(msg["id"].as_u64(), "Random injected error", 99);
            self.send_response(writer, &error_response).await?;
            return Ok(());
        }

        if self.config.latency_ms > 0 {
            sleep(self.config.latency()).await;
        }

        let method = msg["method"].as_str().unwrap_or("");
        let id = msg["id"].as_u64();
        let params = &msg["params"];

        debug!("Received method: {} with params: {:?}", method, params);

        let response = match method {
            "mining.subscribe" => self.handle_subscribe(session_id).await?,
            "mining.authorize" => {
                self.handle_authorize(session_id, params, id.unwrap_or(0))
                    .await?
            }
            "mining.submit" => {
                self.handle_submit(session_id, params, id.unwrap_or(0))
                    .await?
            }
            "mining.extranonce.subscribe" => {
                MockResponses::extranonce_subscribe_response(id.unwrap_or(0))
            }
            "mining.ping" => MockResponses::ping_response(id.unwrap_or(0)),
            "mining.get_version" => MockResponses::get_version_response(id.unwrap_or(0)),
            _ => MockResponses::unknown_method_response(id.unwrap_or(0), method),
        };

        self.send_response(writer, &response).await
    }

    async fn handle_subscribe(&self, session_id: &str) -> Result<Value> {
        let sessions = self.sessions.read().await;
        let session = sessions
            .get(session_id)
            .ok_or_else(|| anyhow!("Session not found"))?;

        Ok(MockResponses::subscribe_response(
            &session.extranonce1,
            self.config.extranonce_size,
        ))
    }

    async fn handle_authorize(&self, session_id: &str, params: &Value, id: u64) -> Result<Value> {
        let worker = params[0].as_str().unwrap_or("unknown");
        let _password = params[1].as_str().unwrap_or("");

        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.worker = Some(worker.to_string());
            session.authorized = true;
        }

        let _ = self.difficulty_manager.get_or_create_worker(worker).await;

        Ok(MockResponses::authorize_response(id, true))
    }

    async fn handle_submit(&self, session_id: &str, params: &Value, id: u64) -> Result<Value> {
        let sessions = self.sessions.read().await;
        let session = sessions
            .get(session_id)
            .ok_or_else(|| anyhow!("Session not found"))?;

        if !session.authorized {
            return Ok(MockResponses::submit_response(
                id,
                false,
                Some("Not authorized".to_string()),
            ));
        }

        let worker = params[0].as_str().unwrap_or("unknown");
        let job_id = params[1].as_str().unwrap_or("");
        let extranonce2 = params[2].as_str().unwrap_or("");
        let ntime = params[3].as_str().unwrap_or("");
        let nonce = params[4].as_str().unwrap_or("");

        let submission = ShareSubmission {
            worker: worker.to_string(),
            job_id: job_id.to_string(),
            extranonce2: extranonce2.to_string(),
            ntime: ntime.to_string(),
            nonce: nonce.to_string(),
        };

        let is_stale = self.job_manager.is_stale_job(job_id).await;

        let _new_difficulty = self.difficulty_manager.record_share(worker).await;

        match self.validator.validate_share(&submission, is_stale).await {
            Ok(true) => Ok(MockResponses::submit_response(id, true, None)),
            Ok(false) => Ok(MockResponses::submit_response(
                id,
                false,
                Some("Share validation failed".to_string()),
            )),
            Err(e) => Ok(MockResponses::submit_response(
                id,
                false,
                Some(e.to_string()),
            )),
        }
    }

    async fn broadcast_new_job(&self, clean_jobs: bool) -> Result<()> {
        let job = self.job_manager.rotate_job(clean_jobs).await?;
        let _notify_msg = MockResponses::notify_message(job.to_notify_params());

        let sessions = self.sessions.read().await;
        for session in sessions.values() {
            if session.authorized {
                debug!("Broadcasting job to worker: {:?}", session.worker);
            }
        }

        Ok(())
    }

    async fn send_response(
        &self,
        writer: &mut tokio::net::tcp::OwnedWriteHalf,
        response: &Value,
    ) -> Result<()> {
        let mut response_str = response.to_string();
        response_str.push('\n');
        writer.write_all(response_str.as_bytes()).await?;
        writer.flush().await?;
        Ok(())
    }
}

pub struct MockPoolHandle {
    accept_handle: JoinHandle<()>,
    job_rotation_handle: JoinHandle<()>,
    shutdown_tx: broadcast::Sender<()>,
    pub local_addr: std::net::SocketAddr,
}

impl MockPoolHandle {
    pub async fn shutdown(self) -> Result<()> {
        let _ = self.shutdown_tx.send(());
        self.accept_handle.await?;
        self.job_rotation_handle.await?;
        Ok(())
    }
}

fn generate_extranonce1() -> String {
    use hex;
    use rand::RngCore;

    let mut rng = rand::thread_rng();
    let mut bytes = [0u8; 4];
    rng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}
