use std::sync::Arc;

use chrono::NaiveDateTime;
use chrono::Timelike;
use chrono::Utc;
use dashmap::DashMap;

use crate::job;
use crate::job::State;
use crate::submission::Submit;

#[derive(Debug, Clone, Default)]
pub struct Auth {
    pending: DashMap<u64, Arc<job::State>>,
    submitted: DashMap<NaiveDateTime, Arc<Submit>>,
}

impl Auth {
    pub fn submit(&self, id: u64, job: Arc<State>) {
        self.pending.insert(id, job);
    }

    pub fn submitted(&self, id: &u64, valid: bool) -> Option<Arc<Submit>> {
        if let Some((_, job)) = self.pending.remove(id) {
            let now = Utc::now().with_second(0).unwrap().naive_utc();
            let submit = self.submitted.entry(now).or_default().clone();

            if valid {
                submit.accept(&job);
            } else {
                submit.reject(&job);
            }

            Some(submit)
        } else {
            None
        }
    }
}
