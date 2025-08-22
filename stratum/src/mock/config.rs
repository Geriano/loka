use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockConfig {
    #[serde(default = "default_port")]
    pub port: u16,

    #[serde(default = "default_accept_rate")]
    pub accept_rate: f64,

    #[serde(default = "default_job_interval")]
    pub job_interval_secs: u64,

    #[serde(default = "default_initial_difficulty")]
    pub initial_difficulty: u64,

    #[serde(default = "default_vardiff_enabled")]
    pub vardiff_enabled: bool,

    #[serde(default = "default_vardiff_target_time")]
    pub vardiff_target_time_secs: u64,

    #[serde(default = "default_latency_ms")]
    pub latency_ms: u64,

    #[serde(default = "default_error_rate")]
    pub error_rate: f64,

    #[serde(default = "default_max_connections")]
    pub max_connections: usize,

    #[serde(default = "default_extranonce_size")]
    pub extranonce_size: usize,
}

impl Default for MockConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
            accept_rate: default_accept_rate(),
            job_interval_secs: default_job_interval(),
            initial_difficulty: default_initial_difficulty(),
            vardiff_enabled: default_vardiff_enabled(),
            vardiff_target_time_secs: default_vardiff_target_time(),
            latency_ms: default_latency_ms(),
            error_rate: default_error_rate(),
            max_connections: default_max_connections(),
            extranonce_size: default_extranonce_size(),
        }
    }
}

impl MockConfig {
    pub fn job_interval(&self) -> Duration {
        Duration::from_secs(self.job_interval_secs)
    }

    pub fn latency(&self) -> Duration {
        Duration::from_millis(self.latency_ms)
    }

    pub fn vardiff_target_time(&self) -> Duration {
        Duration::from_secs(self.vardiff_target_time_secs)
    }

    pub fn should_accept_share(&self) -> bool {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        rng.gen_bool(self.accept_rate)
    }

    pub fn should_inject_error(&self) -> bool {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        rng.gen_bool(self.error_rate)
    }
}

fn default_port() -> u16 {
    13333
}

fn default_accept_rate() -> f64 {
    0.95
}

fn default_job_interval() -> u64 {
    30
}

fn default_initial_difficulty() -> u64 {
    1024
}

fn default_vardiff_enabled() -> bool {
    true
}

fn default_vardiff_target_time() -> u64 {
    10
}

fn default_latency_ms() -> u64 {
    50
}

fn default_error_rate() -> f64 {
    0.01
}

fn default_max_connections() -> usize {
    100
}

fn default_extranonce_size() -> usize {
    4
}
