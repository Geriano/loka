use std::sync::atomic::{AtomicU64, Ordering};

use crate::job::State;

#[derive(Debug, Default)]
pub struct Submit {
    accepted: AtomicU64,
    rejected: AtomicU64,
}

#[allow(dead_code)]
impl Submit {
    pub fn new() -> Self {
        Self {
            accepted: AtomicU64::new(0),
            rejected: AtomicU64::new(0),
        }
    }

    pub fn accept(&self, job: &State) -> u64 {
        let hashes = job.hashes();

        self.accepted.fetch_add(hashes, Ordering::Relaxed);

        hashes
    }

    pub fn reject(&self, job: &State) -> u64 {
        let hashes = job.hashes();

        self.rejected.fetch_add(hashes, Ordering::Relaxed);

        hashes
    }

    pub fn accepted(&self) -> u64 {
        self.accepted.load(Ordering::Relaxed)
    }

    pub fn rejected(&self) -> u64 {
        self.rejected.load(Ordering::Relaxed)
    }
}
