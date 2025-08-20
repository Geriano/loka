use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Default)]
pub struct Counter {
    value: AtomicU64,
}

impl Counter {
    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }
}

impl metrics::CounterFn for Counter {
    fn absolute(&self, value: u64) {
        self.value.store(value, Ordering::Relaxed);
    }

    fn increment(&self, value: u64) {
        self.value.fetch_add(value, Ordering::Release);
    }
}
