use std::collections::HashSet;
use std::sync::Arc;

// Note: Chrono imports removed as timestamp handling moved to SystemTime
use dashmap::DashMap;

use crate::auth;
use crate::job::State;
use crate::services::database::DatabaseService;
use crate::services::metrics::MetricsService;

use super::Auth;

#[derive(Debug, Clone)]
pub struct Manager {
    auth: DashMap<Arc<auth::State>, Arc<Auth>>,
    database: Arc<DatabaseService>,
    pool_id: uuid::Uuid,
    metrics: Option<Arc<MetricsService>>,
    // Track recent share hashes to detect duplicates
    recent_shares: Arc<DashMap<String, HashSet<String>>>,
}

impl Manager {
    pub fn new(database: Arc<DatabaseService>, pool_id: uuid::Uuid) -> Self {
        Self {
            auth: DashMap::new(),
            database,
            pool_id,
            metrics: None,
            recent_shares: Arc::new(DashMap::new()),
        }
    }

    pub fn with_metrics(
        database: Arc<DatabaseService>,
        pool_id: uuid::Uuid,
        metrics: Arc<MetricsService>,
    ) -> Self {
        Self {
            auth: DashMap::new(),
            database,
            pool_id,
            metrics: Some(metrics),
            recent_shares: Arc::new(DashMap::new()),
        }
    }

    pub fn authenticated(&self, auth: Arc<auth::State>) {
        self.auth.entry(auth).or_default();
    }

    pub fn submit(&self, id: u64, job: Arc<State>, auth: Arc<auth::State>) {
        metrics::counter!("submission_request_user", "auth" => auth.to_string()).increment(1);

        // Record share submission to metrics
        if let Some(ref metrics) = self.metrics {
            metrics.record_share_submission_event();
        }

        // Create a simple share identifier based on submission ID and user
        // In a real implementation, this would include nonce, extranonce2, etc.
        let share_id = format!("{}:{}", auth.to_string(), id);
        let user_key = auth.to_string();

        // Check for duplicate share
        let mut is_duplicate = false;
        self.recent_shares
            .entry(user_key.clone())
            .and_modify(|shares| {
                if shares.contains(&share_id) {
                    is_duplicate = true;
                    if let Some(ref metrics) = self.metrics {
                        metrics.record_duplicate_share_event();
                    }
                } else {
                    shares.insert(share_id.clone());
                    // Keep only recent shares (last 1000)
                    if shares.len() > 1000 {
                        // Remove oldest shares - this is simplified
                        shares.clear();
                        shares.insert(share_id.clone());
                    }
                }
            })
            .or_insert_with(|| {
                let mut set = HashSet::new();
                set.insert(share_id);
                set
            });

        if !is_duplicate {
            self.auth.entry(auth).or_default().submit(id, job);
        }
    }

    pub fn submitted(&self, id: &u64, valid: bool, auth: Arc<auth::State>) {
        metrics::counter!("submission_response_user", "auth" => auth.to_string(), "valid" => valid.to_string())
            .increment(1);

        // Record share acceptance to metrics
        if let Some(ref metrics) = self.metrics {
            metrics.record_share_acceptance_event(valid);
        }

        let result = self
            .auth
            .entry(auth.clone())
            .or_default()
            .submitted(id, valid);

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
                    )
                    .await
                    {
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
                            )
                            .await
                            {
                                tracing::error!("Failed to store submission to database: {}", e);
                            } else {
                                tracing::debug!(
                                    "Stored submission: valid={}, hashes={}",
                                    valid,
                                    hashes
                                );
                            }
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to create/get worker for submission storage: {}",
                                e
                            );
                        }
                    }
                }
            });
        }
    }
}
