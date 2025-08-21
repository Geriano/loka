use anyhow::{anyhow, Result};
use rand::Rng;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct ShareSubmission {
    pub worker: String,
    pub job_id: String,
    pub extranonce2: String,
    pub ntime: String,
    pub nonce: String,
}

pub struct MockShareValidator {
    submitted_shares: Arc<RwLock<HashSet<String>>>,
    share_stats: Arc<RwLock<HashMap<String, ShareStats>>>,
    accept_rate: f64,
}

#[derive(Debug, Clone, Default)]
pub struct ShareStats {
    pub accepted: u64,
    pub rejected: u64,
    pub stale: u64,
    pub duplicate: u64,
}

impl MockShareValidator {
    pub fn new(accept_rate: f64) -> Self {
        Self {
            submitted_shares: Arc::new(RwLock::new(HashSet::new())),
            share_stats: Arc::new(RwLock::new(HashMap::new())),
            accept_rate,
        }
    }
    
    pub async fn validate_share(
        &self,
        submission: &ShareSubmission,
        is_stale: bool,
    ) -> Result<bool> {
        let share_id = self.generate_share_id(submission);
        
        let mut submitted = self.submitted_shares.write().await;
        let mut stats = self.share_stats.write().await;
        let worker_stats = stats.entry(submission.worker.clone())
            .or_insert_with(ShareStats::default);
        
        if submitted.contains(&share_id) {
            worker_stats.duplicate += 1;
            return Err(anyhow!("Duplicate share"));
        }
        
        if is_stale {
            worker_stats.stale += 1;
            return Err(anyhow!("Stale share"));
        }
        
        if !self.validate_format(submission)? {
            worker_stats.rejected += 1;
            return Err(anyhow!("Invalid share format"));
        }
        
        let mut rng = rand::thread_rng();
        let should_accept = rng.gen_bool(self.accept_rate);
        
        if should_accept {
            submitted.insert(share_id);
            worker_stats.accepted += 1;
            Ok(true)
        } else {
            worker_stats.rejected += 1;
            Err(anyhow!("Share below target"))
        }
    }
    
    fn generate_share_id(&self, submission: &ShareSubmission) -> String {
        format!(
            "{}:{}:{}:{}:{}",
            submission.worker,
            submission.job_id,
            submission.extranonce2,
            submission.ntime,
            submission.nonce
        )
    }
    
    fn validate_format(&self, submission: &ShareSubmission) -> Result<bool> {
        if submission.extranonce2.len() % 2 != 0 {
            return Ok(false);
        }
        
        if submission.ntime.len() != 8 {
            return Ok(false);
        }
        
        if submission.nonce.len() != 8 {
            return Ok(false);
        }
        
        if !is_hex(&submission.extranonce2) 
            || !is_hex(&submission.ntime) 
            || !is_hex(&submission.nonce) {
            return Ok(false);
        }
        
        Ok(true)
    }
    
    pub async fn get_stats(&self, worker: &str) -> Option<ShareStats> {
        let stats = self.share_stats.read().await;
        stats.get(worker).cloned()
    }
    
    pub async fn clear_old_shares(&self, keep_recent: usize) {
        let mut submitted = self.submitted_shares.write().await;
        if submitted.len() > keep_recent * 2 {
            let to_remove = submitted.len() - keep_recent;
            let shares: Vec<String> = submitted.iter().take(to_remove).cloned().collect();
            for share in shares {
                submitted.remove(&share);
            }
        }
    }
}

fn is_hex(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii_hexdigit())
}