use std::net::SocketAddr;
use std::sync::Arc;

use tokio::task::JoinHandle;

use crate::Config;
use crate::{auth, job, submission};

#[derive(Debug, Clone)]
pub struct Manager {
    config: Arc<Config>,
    auth: Arc<auth::Manager>,
    jobs: Arc<job::Manager>,
    submissions: Arc<submission::Manager>,
    tasks: Vec<Arc<JoinHandle<()>>>,
}

impl Manager {
    pub fn new(config: Arc<Config>) -> Self {
        Self {
            auth: Arc::new(auth::Manager::default()),
            jobs: Arc::new(job::Manager::new(&config)),
            submissions: Arc::new(submission::Manager::default()),
            config,
            tasks: Vec::new(),
        }
    }

    pub fn config(&self) -> &Arc<Config> {
        &self.config
    }

    pub fn auth(&self) -> &Arc<auth::Manager> {
        &self.auth
    }

    pub fn jobs(&self) -> &Arc<job::Manager> {
        &self.jobs
    }

    pub fn submissions(&self) -> &Arc<submission::Manager> {
        &self.submissions
    }

    pub fn terminated(&self, addr: &SocketAddr) {
        self.auth.terminated(addr);
    }
}

impl Drop for Manager {
    fn drop(&mut self) {
        for task in self.tasks.drain(..) {
            task.abort();
        }
    }
}
