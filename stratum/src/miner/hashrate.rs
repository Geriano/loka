use rand::Rng;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct HashrateSimulator {
    target_hashrate_mhs: f64,
    variance_factor: f64,
    last_calculation: Instant,
    samples: Vec<f64>,
    max_samples: usize,
}

impl HashrateSimulator {
    pub fn new(target_hashrate_mhs: f64, variance_factor: f64) -> Self {
        Self {
            target_hashrate_mhs,
            variance_factor,
            last_calculation: Instant::now(),
            samples: Vec::new(),
            max_samples: 60, // Keep 60 samples for averaging
        }
    }

    pub fn calculate_share_interval(&mut self, difficulty: f64) -> Duration {
        // Calculate expected time to find a share at this difficulty
        // Hashrate in H/s = hashrate_mhs * 1_000_000
        // Expected time = difficulty / hashrate_hs
        let hashrate_hs = self.target_hashrate_mhs * 1_000_000.0;
        let expected_seconds = difficulty / hashrate_hs;

        // Add variance to make it more realistic
        let mut rng = rand::thread_rng();
        let variance = rng.gen_range(-self.variance_factor..=self.variance_factor);
        let actual_seconds = expected_seconds * (1.0 + variance);

        // Ensure minimum interval to avoid spam
        let min_seconds = 0.1;
        let clamped_seconds = actual_seconds.max(min_seconds);

        Duration::from_secs_f64(clamped_seconds)
    }

    pub fn update_hashrate_sample(&mut self, difficulty: f64, time_taken: Duration) {
        // Calculate actual hashrate from this share
        let time_seconds = time_taken.as_secs_f64();
        if time_seconds > 0.0 {
            let actual_hashrate_hs = difficulty / time_seconds;
            let actual_hashrate_mhs = actual_hashrate_hs / 1_000_000.0;

            self.samples.push(actual_hashrate_mhs);

            // Keep only recent samples
            if self.samples.len() > self.max_samples {
                self.samples.remove(0);
            }
        }

        self.last_calculation = Instant::now();
    }

    pub fn current_hashrate_mhs(&self) -> f64 {
        if self.samples.is_empty() {
            self.target_hashrate_mhs
        } else {
            // Calculate average of recent samples
            let sum: f64 = self.samples.iter().sum();
            sum / self.samples.len() as f64
        }
    }

    pub fn efficiency_percent(&self) -> f64 {
        let current = self.current_hashrate_mhs();
        if self.target_hashrate_mhs > 0.0 {
            (current / self.target_hashrate_mhs) * 100.0
        } else {
            100.0
        }
    }

    pub fn generate_realistic_nonce(&self) -> String {
        use rand::RngCore;
        let mut rng = rand::thread_rng();
        let nonce = rng.next_u32();
        format!("{:08x}", nonce)
    }

    pub fn generate_extranonce2(&self, size: usize) -> String {
        use rand::RngCore;
        let mut rng = rand::thread_rng();
        let mut bytes = vec![0u8; size];
        rng.fill_bytes(&mut bytes);
        hex::encode(bytes)
    }

    pub fn should_be_valid_share(&self, difficulty: f64) -> bool {
        // Simulate finding a valid share based on hashrate and difficulty
        // This is a simplified model - real mining involves more complex probability
        let mut rng = rand::thread_rng();
        let random_value: f64 = rng.r#gen();

        // Calculate probability of finding a valid share
        // Higher hashrate and lower difficulty = higher probability
        let hashrate_factor = (self.current_hashrate_mhs() / 100.0).min(1.0);
        let difficulty_factor = (1000.0 / difficulty).min(1.0);
        let base_probability = 0.7; // 70% base chance for valid shares

        let probability = base_probability * hashrate_factor * difficulty_factor;

        random_value < probability
    }

    pub fn stats(&self) -> HashrateStats {
        HashrateStats {
            target_hashrate_mhs: self.target_hashrate_mhs,
            current_hashrate_mhs: self.current_hashrate_mhs(),
            efficiency_percent: self.efficiency_percent(),
            samples_count: self.samples.len(),
            variance_factor: self.variance_factor,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HashrateStats {
    pub target_hashrate_mhs: f64,
    pub current_hashrate_mhs: f64,
    pub efficiency_percent: f64,
    pub samples_count: usize,
    pub variance_factor: f64,
}

impl std::fmt::Display for HashrateStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Hashrate: {:.2} MH/s (target: {:.2} MH/s, efficiency: {:.1}%, samples: {})",
            self.current_hashrate_mhs,
            self.target_hashrate_mhs,
            self.efficiency_percent,
            self.samples_count
        )
    }
}
