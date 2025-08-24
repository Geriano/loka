use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use tokio::task::JoinHandle;

use crate::Config;
use crate::job::State;
use crate::services::metrics::MetricsService;

#[derive(Debug, Clone)]
pub struct Manager {
    job_expiration: Duration,
    jobs: DashMap<String, Arc<State>>,
    metrics: Option<Arc<MetricsService>>,
    job_notification_times: DashMap<String, Instant>,
}

impl Manager {
    pub fn new(config: &Config) -> Self {
        Self {
            job_expiration: config.limiter.jobs,
            jobs: DashMap::new(),
            metrics: None,
            job_notification_times: DashMap::new(),
        }
    }

    pub fn with_metrics(config: &Config, metrics: Arc<MetricsService>) -> Self {
        Self {
            job_expiration: config.limiter.jobs,
            jobs: DashMap::new(),
            metrics: Some(metrics),
            job_notification_times: DashMap::new(),
        }
    }

    pub fn get(&self, id: &str) -> Option<Arc<State>> {
        self.jobs.get(id).as_deref().cloned()
    }

    pub fn notified(&self, id: &str, difficulty: f64) {
        metrics::counter!("job_notified_total").increment(1);
        metrics::histogram!("job_difficulty_total").record(difficulty);

        // Record job notification time for latency tracking
        let notification_time = Instant::now();
        self.job_notification_times
            .insert(id.to_owned(), notification_time);

        // Record difficulty adjustment if metrics available
        if let Some(ref metrics) = self.metrics {
            metrics.record_difficulty_adjustment_event(difficulty);
        }

        self.jobs
            .insert(id.to_owned(), Arc::new(State::new(difficulty)));
    }

    pub fn job_distributed(&self, id: &str) {
        // Calculate and record job distribution latency
        if let Some(notification_time) = self.job_notification_times.get(id) {
            let latency = notification_time.elapsed();
            let latency_ms = latency.as_secs_f64() * 1000.0;

            if let Some(ref metrics) = self.metrics {
                metrics.record_job_distribution_latency_event(latency_ms);
            }

            // Clean up the notification time record
            self.job_notification_times.remove(id);
        }
    }

    pub fn total(&self) -> usize {
        self.jobs.len()
    }

    pub fn prune(&self, interval: Duration) -> JoinHandle<()> {
        let jobs = self.jobs.clone();
        let job_notification_times = self.job_notification_times.clone();
        let expiration = self.job_expiration;

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);

            loop {
                ticker.tick().await;

                jobs.retain(|id, job| {
                    // clear job when job notified got expired
                    let should_keep = job.elapsed() <= expiration;
                    if !should_keep {
                        // Also clean up notification time if job is being removed
                        job_notification_times.remove(id);
                    }
                    should_keep
                });

                // Clean up orphaned notification times
                job_notification_times.retain(|id, _| jobs.contains_key(id));
            }
        })
    }
}
