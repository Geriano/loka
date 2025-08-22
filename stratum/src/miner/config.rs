use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerConfig {
    /// Pool address to connect to
    #[serde(default = "default_pool_address")]
    pub pool_address: String,

    /// Number of worker connections
    #[serde(default = "default_workers")]
    pub workers: usize,

    /// Hashrate per worker in MH/s
    #[serde(default = "default_hashrate")]
    pub hashrate_mhs: f64,

    /// Username for authentication
    #[serde(default = "default_username")]
    pub username: String,

    /// Password for authentication
    #[serde(default = "default_password")]
    pub password: String,

    /// Worker name prefix
    #[serde(default = "default_worker_prefix")]
    pub worker_prefix: String,

    /// Share submission interval in seconds
    #[serde(default = "default_share_interval")]
    pub share_interval_secs: f64,

    /// Connection timeout in seconds
    #[serde(default = "default_connection_timeout")]
    pub connection_timeout_secs: u64,

    /// Keep-alive interval in seconds
    #[serde(default = "default_keepalive_interval")]
    pub keepalive_interval_secs: u64,

    /// Difficulty variance factor (0.0-1.0)
    #[serde(default = "default_difficulty_variance")]
    pub difficulty_variance: f64,

    /// Stale share rate (0.0-1.0)
    #[serde(default = "default_stale_rate")]
    pub stale_rate: f64,

    /// Invalid share rate (0.0-1.0)
    #[serde(default = "default_invalid_rate")]
    pub invalid_rate: f64,

    /// Reconnection attempts
    #[serde(default = "default_reconnect_attempts")]
    pub reconnect_attempts: u32,

    /// Reconnection delay in seconds
    #[serde(default = "default_reconnect_delay")]
    pub reconnect_delay_secs: u64,

    /// Enable mining notifications logging
    #[serde(default = "default_log_mining")]
    pub log_mining: bool,

    /// Simulation duration in seconds (0 = infinite)
    #[serde(default = "default_duration")]
    pub duration_secs: u64,
}

impl Default for MinerConfig {
    fn default() -> Self {
        Self {
            pool_address: default_pool_address(),
            workers: default_workers(),
            hashrate_mhs: default_hashrate(),
            username: default_username(),
            password: default_password(),
            worker_prefix: default_worker_prefix(),
            share_interval_secs: default_share_interval(),
            connection_timeout_secs: default_connection_timeout(),
            keepalive_interval_secs: default_keepalive_interval(),
            difficulty_variance: default_difficulty_variance(),
            stale_rate: default_stale_rate(),
            invalid_rate: default_invalid_rate(),
            reconnect_attempts: default_reconnect_attempts(),
            reconnect_delay_secs: default_reconnect_delay(),
            log_mining: default_log_mining(),
            duration_secs: default_duration(),
        }
    }
}

impl MinerConfig {
    pub fn connection_timeout(&self) -> Duration {
        Duration::from_secs(self.connection_timeout_secs)
    }

    pub fn keepalive_interval(&self) -> Duration {
        Duration::from_secs(self.keepalive_interval_secs)
    }

    pub fn share_interval(&self) -> Duration {
        Duration::from_secs_f64(self.share_interval_secs)
    }

    pub fn reconnect_delay(&self) -> Duration {
        Duration::from_secs(self.reconnect_delay_secs)
    }

    pub fn simulation_duration(&self) -> Option<Duration> {
        if self.duration_secs > 0 {
            Some(Duration::from_secs(self.duration_secs))
        } else {
            None
        }
    }

    pub fn worker_name(&self, worker_id: usize) -> String {
        format!("{}.{}", self.worker_prefix, worker_id)
    }

    pub fn total_hashrate(&self) -> f64 {
        self.hashrate_mhs * self.workers as f64
    }

    pub fn should_submit_stale(&self) -> bool {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        rng.gen_bool(self.stale_rate)
    }

    pub fn should_submit_invalid(&self) -> bool {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        rng.gen_bool(self.invalid_rate)
    }
}

fn default_pool_address() -> String {
    "127.0.0.1:3333".to_string()
}

fn default_workers() -> usize {
    1
}

fn default_hashrate() -> f64 {
    100.0 // 100 MH/s per worker
}

fn default_username() -> String {
    "testuser".to_string()
}

fn default_password() -> String {
    "testpass".to_string()
}

fn default_worker_prefix() -> String {
    "worker".to_string()
}

fn default_share_interval() -> f64 {
    10.0 // 10 seconds between shares
}

fn default_connection_timeout() -> u64 {
    30
}

fn default_keepalive_interval() -> u64 {
    60
}

fn default_difficulty_variance() -> f64 {
    0.1 // 10% variance
}

fn default_stale_rate() -> f64 {
    0.02 // 2% stale shares
}

fn default_invalid_rate() -> f64 {
    0.01 // 1% invalid shares
}

fn default_reconnect_attempts() -> u32 {
    3
}

fn default_reconnect_delay() -> u64 {
    5
}

fn default_log_mining() -> bool {
    true
}

fn default_duration() -> u64 {
    0 // Infinite
}
