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
        (self.difficulty * 2.0f64.powf(32.0f64)) as u64 * self.elapsed().as_secs()
    }

    pub fn elapsed(&self) -> Duration {
        self.notified.elapsed()
    }
}
