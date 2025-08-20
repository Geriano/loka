use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use dashmap::DashMap;

use crate::auth::State;

#[derive(Debug, Clone, Default)]
pub struct Manager {
    authenticated: DashMap<SocketAddr, Arc<State>>,
    addresses: DashMap<Arc<State>, SocketAddr>,
    last_seen: DashMap<Arc<State>, Instant>,
}

impl Manager {
    pub fn get(&self, addr: &SocketAddr) -> Option<Arc<State>> {
        self.authenticated.get(addr).as_deref().cloned()
    }

    pub fn address(&self, user: &str, worker: &str) -> Option<SocketAddr> {
        let state = State::new(user, worker);

        self.addresses.get(&state).as_deref().cloned()
    }

    pub fn authenticate(&self, addr: SocketAddr, user: &str, worker: &str) -> Arc<State> {
        let state = Arc::new(State::new(user, worker));

        self.authenticated.insert(addr, state.clone());
        self.addresses.insert(state.clone(), addr);
        self.update_last_seen(&state);

        metrics::counter!("auth_logged_in_total").increment(1);
        metrics::counter!("auth_logged_in", "auth" => state.to_string()).increment(1);

        state
    }

    pub fn update_last_seen(&self, state: &Arc<State>) {
        self.last_seen.insert(state.clone(), Instant::now());
    }

    pub fn last_seen(&self, state: &Arc<State>) -> Option<Duration> {
        self.last_seen.get(state).as_deref().map(|i| i.elapsed())
    }

    pub fn terminated(&self, addr: &SocketAddr) {
        if let Some((_, state)) = self.authenticated.remove(addr) {
            metrics::counter!("auth_logged_out_total").increment(1);
            metrics::counter!("auth_logged_out", "auth" => state.to_string()).increment(1);

            self.addresses.remove(&state);
        }
    }
}
