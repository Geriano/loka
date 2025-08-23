use std::net::SocketAddr;
use std::sync::Arc;

use tokio::task::JoinHandle;

use crate::Config;
use crate::services::database::DatabaseService;
use crate::{auth, job, submission};

#[derive(Debug, Clone)]
pub struct Manager {
    config: Arc<Config>,
    auth: Arc<auth::Manager>,
    jobs: Arc<job::Manager>,
    submissions: Arc<submission::Manager>,
    database: Arc<DatabaseService>,
    tasks: Vec<Arc<JoinHandle<()>>>,
}

impl Manager {
    pub fn new(
        config: Arc<Config>,
        database: Arc<DatabaseService>,
    ) -> Result<Self, crate::error::StratumError> {
        Ok(Self {
            auth: Arc::new(auth::Manager::default()),
            jobs: Arc::new(job::Manager::new(&config)),
            submissions: Arc::new(submission::Manager::new(database.clone(), config.pool.id)),
            config,
            database,
            tasks: Vec::new(),
        })
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

    pub fn database(&self) -> &Arc<DatabaseService> {
        &self.database
    }

    /// Get the pool ID from config
    pub fn get_pool_id(&self) -> uuid::Uuid {
        self.config.pool.id
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
