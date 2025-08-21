use anyhow::Result;
use chrono::Utc;
use rand::Rng;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct MockJob {
    pub job_id: String,
    pub prev_hash: String,
    pub coinbase1: String,
    pub coinbase2: String,
    pub merkle_branches: Vec<String>,
    pub version: String,
    pub nbits: String,
    pub ntime: String,
    pub clean_jobs: bool,
    pub created_at: i64,
}

impl MockJob {
    pub fn new(clean_jobs: bool) -> Self {
        let mut rng = rand::thread_rng();
        
        Self {
            job_id: Uuid::new_v4().to_string().replace("-", "")[..8].to_string(),
            prev_hash: generate_random_hex(64),
            coinbase1: generate_random_hex(80),
            coinbase2: generate_random_hex(80),
            merkle_branches: (0..rng.gen_range(5..15))
                .map(|_| generate_random_hex(64))
                .collect(),
            version: "20000000".to_string(),
            nbits: "1a0fffff".to_string(),
            ntime: format!("{:08x}", Utc::now().timestamp() as u32),
            clean_jobs,
            created_at: Utc::now().timestamp(),
        }
    }
    
    pub fn to_notify_params(&self) -> Vec<serde_json::Value> {
        vec![
            json!(self.job_id),
            json!(self.prev_hash),
            json!(self.coinbase1),
            json!(self.coinbase2),
            json!(self.merkle_branches),
            json!(self.version),
            json!(self.nbits),
            json!(self.ntime),
            json!(self.clean_jobs),
        ]
    }
}

pub struct MockJobManager {
    current_job: Arc<RwLock<MockJob>>,
    job_history: Arc<RwLock<Vec<MockJob>>>,
    max_history: usize,
}

impl MockJobManager {
    pub fn new() -> Self {
        Self {
            current_job: Arc::new(RwLock::new(MockJob::new(true))),
            job_history: Arc::new(RwLock::new(Vec::new())),
            max_history: 100,
        }
    }
    
    pub async fn rotate_job(&self, clean_jobs: bool) -> Result<MockJob> {
        let new_job = MockJob::new(clean_jobs);
        
        let mut current = self.current_job.write().await;
        let old_job = current.clone();
        *current = new_job.clone();
        
        let mut history = self.job_history.write().await;
        history.push(old_job);
        
        if history.len() > self.max_history {
            history.remove(0);
        }
        
        Ok(new_job)
    }
    
    pub async fn get_current_job(&self) -> MockJob {
        self.current_job.read().await.clone()
    }
    
    pub async fn is_valid_job(&self, job_id: &str) -> bool {
        let current = self.current_job.read().await;
        if current.job_id == job_id {
            return true;
        }
        
        let history = self.job_history.read().await;
        history.iter().any(|job| job.job_id == job_id)
    }
    
    pub async fn is_stale_job(&self, job_id: &str) -> bool {
        let current = self.current_job.read().await;
        if current.job_id == job_id {
            return false;
        }
        
        let history = self.job_history.read().await;
        history.iter().any(|job| job.job_id == job_id)
    }
}

fn generate_random_hex(len: usize) -> String {
    use hex;
    use rand::RngCore;
    
    let mut rng = rand::thread_rng();
    let mut bytes = vec![0u8; len / 2];
    rng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}