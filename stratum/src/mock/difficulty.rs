use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct WorkerDifficulty {
    pub current: u64,
    pub shares_count: u64,
    pub last_share_time: i64,
    pub window_start: i64,
    pub shares_in_window: u64,
}

impl WorkerDifficulty {
    pub fn new(initial_difficulty: u64) -> Self {
        let now = Utc::now().timestamp();
        Self {
            current: initial_difficulty,
            shares_count: 0,
            last_share_time: now,
            window_start: now,
            shares_in_window: 0,
        }
    }

    pub fn record_share(&mut self) {
        self.shares_count += 1;
        self.shares_in_window += 1;
        self.last_share_time = Utc::now().timestamp();
    }

    pub fn reset_window(&mut self) {
        self.window_start = Utc::now().timestamp();
        self.shares_in_window = 0;
    }
}

pub struct MockDifficultyManager {
    workers: Arc<RwLock<HashMap<String, WorkerDifficulty>>>,
    initial_difficulty: u64,
    target_time_secs: u64,
    min_difficulty: u64,
    max_difficulty: u64,
    vardiff_enabled: bool,
}

impl MockDifficultyManager {
    pub fn new(initial_difficulty: u64, target_time_secs: u64, vardiff_enabled: bool) -> Self {
        Self {
            workers: Arc::new(RwLock::new(HashMap::new())),
            initial_difficulty,
            target_time_secs,
            min_difficulty: 16,
            max_difficulty: 65536,
            vardiff_enabled,
        }
    }

    pub async fn get_or_create_worker(&self, worker_id: &str) -> WorkerDifficulty {
        let mut workers = self.workers.write().await;
        workers
            .entry(worker_id.to_string())
            .or_insert_with(|| WorkerDifficulty::new(self.initial_difficulty))
            .clone()
    }

    pub async fn record_share(&self, worker_id: &str) -> u64 {
        let mut workers = self.workers.write().await;
        let worker = workers
            .entry(worker_id.to_string())
            .or_insert_with(|| WorkerDifficulty::new(self.initial_difficulty));

        worker.record_share();

        if !self.vardiff_enabled {
            return worker.current;
        }

        let now = Utc::now().timestamp();
        let window_duration = now - worker.window_start;

        if window_duration >= self.target_time_secs as i64 * 2 {
            let shares_per_second = worker.shares_in_window as f64 / window_duration as f64;
            let target_shares_per_second = 1.0 / self.target_time_secs as f64;

            let adjustment_factor = shares_per_second / target_shares_per_second;

            let new_difficulty = if adjustment_factor > 1.5 {
                (worker.current as f64 * 2.0).min(self.max_difficulty as f64) as u64
            } else if adjustment_factor < 0.5 {
                (worker.current as f64 / 2.0).max(self.min_difficulty as f64) as u64
            } else {
                worker.current
            };

            if new_difficulty != worker.current {
                worker.current = new_difficulty;
                worker.reset_window();
            }
        }

        worker.current
    }

    pub async fn get_worker_difficulty(&self, worker_id: &str) -> Option<u64> {
        let workers = self.workers.read().await;
        workers.get(worker_id).map(|w| w.current)
    }

    pub async fn remove_worker(&self, worker_id: &str) {
        let mut workers = self.workers.write().await;
        workers.remove(worker_id);
    }
}
