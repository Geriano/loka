use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;

use dashmap::DashMap;
use metrics::Key;

use crate::HistogramSummary;
use crate::types::{Counter, Gauge, Histogram};

static RECORDER: OnceLock<Recorder> = OnceLock::new();

#[derive(Debug, Default)]
pub struct Recorder {
    counters: DashMap<Key, Arc<Counter>>,
    gauges: DashMap<Key, Arc<Gauge>>,
    histograms: DashMap<Key, Arc<Histogram>>,
}

impl Recorder {
    pub fn new() -> Self {
        Self {
            counters: DashMap::new(),
            gauges: DashMap::new(),
            histograms: DashMap::new(),
        }
    }

    pub fn init() -> Result<&'static Self, metrics::SetRecorderError<&'static Self>> {
        let recorder = RECORDER.get_or_init(|| Self::new());

        metrics::set_global_recorder(recorder).map(|_| recorder)
    }

    pub fn current() -> &'static Self {
        RECORDER.get().expect("Metrics recorder not initialized")
    }

    pub fn counters(&self) -> HashMap<String, u64> {
        self.counters
            .iter()
            .map(|entry| (crate::key::to_string(entry.key()), entry.value().get()))
            .collect()
    }

    pub fn gauges(&self) -> HashMap<String, f64> {
        self.gauges
            .iter()
            .map(|entry| (crate::key::to_string(entry.key()), entry.value().get()))
            .collect()
    }

    pub fn histograms(&self) -> HashMap<String, HistogramSummary> {
        self.histograms
            .iter()
            .map(|entry| (crate::key::to_string(entry.key()), entry.value().summary()))
            .collect()
    }
}

impl metrics::Recorder for Recorder {
    fn describe_counter(
        &self,
        _key: metrics::KeyName,
        _unit: Option<metrics::Unit>,
        _description: metrics::SharedString,
    ) {
        //
    }

    fn describe_gauge(
        &self,
        _key: metrics::KeyName,
        _unit: Option<metrics::Unit>,
        _description: metrics::SharedString,
    ) {
        //
    }

    fn describe_histogram(
        &self,
        _key: metrics::KeyName,
        _unit: Option<metrics::Unit>,
        _description: metrics::SharedString,
    ) {
        //
    }

    fn register_counter(&self, key: &Key, _metadata: &metrics::Metadata<'_>) -> metrics::Counter {
        let counter = self
            .counters
            .entry(key.clone())
            .or_insert_with(|| Arc::new(Counter::default()))
            .value()
            .clone();

        metrics::Counter::from_arc(counter)
    }

    fn register_gauge(&self, key: &Key, _metadata: &metrics::Metadata<'_>) -> metrics::Gauge {
        let gauge = self
            .gauges
            .entry(key.clone())
            .or_insert_with(|| Arc::new(Gauge::default()))
            .value()
            .clone();

        metrics::Gauge::from_arc(gauge)
    }

    fn register_histogram(
        &self,
        key: &Key,
        _metadata: &metrics::Metadata<'_>,
    ) -> metrics::Histogram {
        let histogram = self
            .histograms
            .entry(key.clone())
            .or_insert_with(|| Arc::new(Histogram::default()))
            .value()
            .clone();

        metrics::Histogram::from_arc(histogram)
    }
}
