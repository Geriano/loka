use std::sync::Arc;

// Note: Chrono imports removed as timestamp handling moved to SystemTime
use dashmap::DashMap;

use crate::auth;
use crate::job::State;
use crate::services::database::DatabaseService;

use super::Auth;

#[derive(Debug, Clone)]
pub struct Manager {
    auth: DashMap<Arc<auth::State>, Arc<Auth>>,
    database: Arc<DatabaseService>,
    pool_id: uuid::Uuid,
}

impl Manager {
    pub fn new(database: Arc<DatabaseService>, pool_id: uuid::Uuid) -> Self {
        Self {
            auth: DashMap::new(),
            database,
            pool_id,
        }
    }

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

        let result = self.auth.entry(auth.clone()).or_default().submitted(id, valid);
        
        // Store completed submission to database (async, non-blocking)
        if let Some(submit) = result {
            tokio::spawn({
                let db = self.database.clone();
                let auth = auth.clone();
                let submit = submit.clone();
                let pool_id = self.pool_id;
                async move {
                    
                    // Create or get miner and worker
                    match loka_model::entities::workers::Model::first_or_create(
                        &db.connection,
                        pool_id,
                        auth.user(),
                        auth.worker(),
                    ).await {
                        Ok((_miner, worker)) => {
                            // Get hashes from the completed submission result
                            let hashes = if valid {
                                submit.accepted() as f64
                            } else {
                                submit.rejected() as f64
                            };
                            
                            // Store final submission with known validity
                            if let Err(e) = loka_model::entities::submissions::Model::store(
                                &db.connection,
                                &worker,
                                hashes,
                                valid,
                            ).await {
                                tracing::error!("Failed to store submission to database: {}", e);
                            } else {
                                tracing::debug!("Stored submission: valid={}, hashes={}", valid, hashes);
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to create/get worker for submission storage: {}", e);
                        }
                    }
                }
            });
        }
    }
}
