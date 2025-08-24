use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub struct State {
    difficulty: f64,
    notified: Instant,
}

impl State {
    pub fn new(difficulty: f64) -> Self {
        Self {
            difficulty,
            notified: Instant::now(),
        }
    }

    pub fn hashes(&self) -> u64 {
        // Each share represents difficulty * 2^32 hashes according to Bitcoin standard
        // This is NOT time-based - each share submission represents a fixed amount of work
        (self.difficulty * 2.0f64.powf(32.0f64)) as u64
    }

    pub fn elapsed(&self) -> Duration {
        self.notified.elapsed()
    }
}
