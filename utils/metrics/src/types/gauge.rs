use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Default)]
pub struct Gauge {
    value: AtomicU64,
}

impl Gauge {
    pub fn get(&self) -> f64 {
        f64::from_bits(self.value.load(Ordering::Relaxed))
    }
}

impl metrics::GaugeFn for Gauge {
    fn set(&self, value: f64) {
        self.value.store(value.to_bits(), Ordering::Relaxed);
    }

    fn increment(&self, value: f64) {
        self.set(self.get() + value);
    }

    fn decrement(&self, value: f64) {
        self.set(self.get() - value);
    }
}
