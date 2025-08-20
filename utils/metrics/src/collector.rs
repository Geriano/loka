use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use crate::HistogramSummary;
use crate::Recorder;

#[derive(Debug)]
pub struct Collector {
    interval: Duration,
    counters: Arc<RwLock<BTreeMap<String, u64>>>,
    gauges: Arc<RwLock<BTreeMap<String, f64>>>,
    histograms: Arc<RwLock<BTreeMap<String, HistogramSummary>>>,
    task: Option<JoinHandle<()>>,
}

impl Collector {
    pub fn new(interval: Duration) -> Self {
        let mut s = Self {
            interval,
            counters: Arc::new(RwLock::new(BTreeMap::new())),
            gauges: Arc::new(RwLock::new(BTreeMap::new())),
            histograms: Arc::new(RwLock::new(BTreeMap::new())),
            task: None,
        };

        s.task = Some(s.load());
        s
    }

    pub fn load(&self) -> JoinHandle<()> {
        let interval = self.interval;
        let counters = self.counters.clone();
        let gauges = self.gauges.clone();
        let histograms = self.histograms.clone();

        tokio::spawn(async move {
            let recorder = Recorder::current();
            let mut ticker = tokio::time::interval(interval);

            loop {
                ticker.tick().await;

                if let Ok(mut counters) = counters.try_write() {
                    for (key, value) in recorder.counters() {
                        counters.insert(key, value);
                    }
                }

                if let Ok(mut gauges) = gauges.try_write() {
                    for (key, value) in recorder.gauges() {
                        gauges.insert(key, value);
                    }
                }

                if let Ok(mut histograms) = histograms.try_write() {
                    for (key, value) in recorder.histograms() {
                        histograms.insert(key, value);
                    }
                }
            }
        })
    }
}

impl Drop for Collector {
    fn drop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}
