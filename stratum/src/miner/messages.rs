use serde_json::{Value, json};
use std::sync::atomic::{AtomicU64, Ordering};

static MESSAGE_ID: AtomicU64 = AtomicU64::new(1);

pub struct StratumMessages;

impl StratumMessages {
    fn next_id() -> u64 {
        MESSAGE_ID.fetch_add(1, Ordering::SeqCst)
    }

    pub fn mining_subscribe() -> Value {
        json!({
            "id": Self::next_id(),
            "method": "mining.subscribe",
            "params": ["MockMiner/1.0.0", null]
        })
    }

    pub fn mining_authorize(username: &str, password: &str) -> Value {
        json!({
            "id": Self::next_id(),
            "method": "mining.authorize",
            "params": [username, password]
        })
    }

    pub fn mining_submit(
        worker: &str,
        job_id: &str,
        extranonce2: &str,
        ntime: &str,
        nonce: &str,
    ) -> Value {
        json!({
            "id": Self::next_id(),
            "method": "mining.submit",
            "params": [worker, job_id, extranonce2, ntime, nonce]
        })
    }

    pub fn mining_extranonce_subscribe() -> Value {
        json!({
            "id": Self::next_id(),
            "method": "mining.extranonce.subscribe",
            "params": []
        })
    }

    pub fn mining_ping() -> Value {
        json!({
            "id": Self::next_id(),
            "method": "mining.ping",
            "params": []
        })
    }

    pub fn mining_get_version() -> Value {
        json!({
            "id": Self::next_id(),
            "method": "mining.get_version",
            "params": []
        })
    }
}

#[derive(Debug, Clone)]
pub struct MiningJob {
    pub job_id: String,
    pub prev_hash: String,
    pub coinbase1: String,
    pub coinbase2: String,
    pub merkle_branches: Vec<String>,
    pub version: String,
    pub nbits: String,
    pub ntime: String,
    pub clean_jobs: bool,
    pub difficulty: f64,
}

impl MiningJob {
    pub fn from_notify_params(params: &[Value], current_difficulty: f64) -> Option<Self> {
        if params.len() < 9 {
            return None;
        }

        Some(Self {
            job_id: params[0].as_str()?.to_string(),
            prev_hash: params[1].as_str()?.to_string(),
            coinbase1: params[2].as_str()?.to_string(),
            coinbase2: params[3].as_str()?.to_string(),
            merkle_branches: params[4]
                .as_array()?
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
            version: params[5].as_str()?.to_string(),
            nbits: params[6].as_str()?.to_string(),
            ntime: params[7].as_str()?.to_string(),
            clean_jobs: params[8].as_bool().unwrap_or(false),
            difficulty: current_difficulty,
        })
    }

    pub fn is_valid_for_difficulty(&self, nonce: &str, difficulty: f64) -> bool {
        // Simple mock validation - in reality this would involve SHA256 hashing
        // For testing purposes, we'll use a simple hash of the nonce
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        format!("{}:{}", self.job_id, nonce).hash(&mut hasher);
        let hash_value = hasher.finish();

        // Calculate target from difficulty
        let target = (u64::MAX as f64 / difficulty) as u64;

        hash_value <= target
    }
}
