use std::sync::Arc;

// Note: Chrono imports removed as timestamp handling moved to SystemTime
use dashmap::DashMap;

use crate::auth;
use crate::job::State;

use super::Auth;

#[derive(Debug, Clone, Default)]
pub struct Manager {
    auth: DashMap<Arc<auth::State>, Arc<Auth>>,
}

impl Manager {
    pub fn authenticated(&self, auth: Arc<auth::State>) {
        self.auth.entry(auth).or_default();
    }

    pub fn submit(&self, id: u64, job: Arc<State>, auth: Arc<auth::State>) {
        metrics::counter!("submission_request_user", "auth" => auth.to_string()).increment(1);

        self.auth.entry(auth).or_default().submit(id, job);
    }

    pub fn submitted(&self, id: &u64, valid: bool, auth: Arc<auth::State>) {
        metrics::counter!("submission_response_user", "auth" => auth.to_string(), "valid" => valid.to_string())
            .increment(1);

        self.auth.entry(auth).or_default().submitted(id, valid);
    }
}
