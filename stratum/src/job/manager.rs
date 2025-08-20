use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use tokio::task::JoinHandle;

use crate::Config;
use crate::job::State;

#[derive(Debug, Clone)]
pub struct Manager {
    job_expiration: Duration,
    jobs: DashMap<String, Arc<State>>,
}

impl Manager {
    pub fn new(config: &Config) -> Self {
        Self {
            job_expiration: config.limiter.jobs,
            jobs: DashMap::new(),
        }
    }

    pub fn get(&self, id: &str) -> Option<Arc<State>> {
        self.jobs.get(id).as_deref().cloned()
    }

    pub fn notified(&self, id: &str, difficulty: f64) {
        metrics::counter!("job_notified_total").increment(1);
        metrics::histogram!("job_difficulty_total").record(difficulty);

        self.jobs
            .insert(id.to_owned(), Arc::new(State::new(difficulty)));
    }

    pub fn total(&self) -> usize {
        self.jobs.len()
    }

    pub fn prune(&self, interval: Duration) -> JoinHandle<()> {
        let jobs = self.jobs.clone();
        let expiration = self.job_expiration;

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);

            loop {
                ticker.tick().await;

                (&jobs).retain(|_, job| {
                    // clear job when job notified got expired
                    job.elapsed() > expiration
                });
            }
        })
    }
}
