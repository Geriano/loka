//! Per-user metrics tracking and aggregation.
//!
//! This module provides user-specific metrics collection to track individual
//! miner performance, behavior patterns, and resource usage.

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::SystemTime;
use tracing::debug;

/// Per-user metrics tracking for individual miner performance analysis.
///
/// Tracks metrics on a per-user basis to provide insights into individual
/// miner behavior, performance patterns, and resource utilization.
///
/// # Examples
///
/// ```rust
/// use loka_stratum::services::metrics::UserMetrics;
/// use std::sync::Arc;
///
/// let user_metrics = Arc::new(UserMetrics::new());
/// 
/// // Record user activity
/// user_metrics.record_connection("miner1");
/// user_metrics.record_submission("miner1", true);
/// user_metrics.record_bytes("miner1", 1024, 2048);
/// 
/// // Get user snapshot
/// let snapshot = user_metrics.get_user_snapshot("miner1");
/// println!("Miner1 connections: {}", snapshot.total_connections);
/// ```
#[derive(Debug)]
pub struct UserMetrics {
    /// Per-user metrics data indexed by user identifier
    user_data: DashMap<String, Arc<UserMetricsData>>,
}

/// Individual user's metrics data with atomic counters.
#[derive(Debug)]
struct UserMetricsData {
    /// Total connections from this user
    pub total_connections: AtomicU64,
    /// Currently active connections from this user
    pub active_connections: AtomicU64,
    /// Total bytes received from this user
    pub bytes_received: AtomicU64,
    /// Total bytes sent to this user
    pub bytes_sent: AtomicU64,
    /// Total submissions from this user
    pub total_submissions: AtomicU64,
    /// Accepted submissions from this user
    pub accepted_submissions: AtomicU64,
    /// Rejected submissions from this user
    pub rejected_submissions: AtomicU64,
    /// Authentication attempts by this user
    pub auth_attempts: AtomicU64,
    /// Successful authentications by this user
    pub auth_successes: AtomicU64,
    /// Protocol errors from this user
    pub protocol_errors: AtomicU64,
    /// Jobs sent to this user
    pub jobs_sent: AtomicU64,
    /// First connection timestamp (as seconds since epoch)
    pub first_seen: AtomicU64,
    /// Last activity timestamp (as seconds since epoch)
    pub last_seen: AtomicU64,
}

impl UserMetrics {
    /// Create a new user metrics tracker.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use loka_stratum::services::metrics::UserMetrics;
    /// 
    /// let user_metrics = UserMetrics::new();
    /// ```
    pub fn new() -> Self {
        Self {
            user_data: DashMap::new(),
        }
    }

    /// Record a connection from a user.
    ///
    /// # Arguments
    ///
    /// * `user_id` - User identifier (typically miner address or username)
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use loka_stratum::services::metrics::UserMetrics;
    /// # let user_metrics = UserMetrics::new();
    /// user_metrics.record_connection("miner_001");
    /// ```
    pub fn record_connection(&self, user_id: &str) {
        let data = self.get_or_create_user_data(user_id);
        data.total_connections.fetch_add(1, Ordering::Relaxed);
        data.active_connections.fetch_add(1, Ordering::Relaxed);
        self.update_last_seen(user_id);
        
        debug!(user_id = user_id, "Recorded user connection");
    }

    /// Record a disconnection from a user.
    ///
    /// # Arguments
    ///
    /// * `user_id` - User identifier
    pub fn record_disconnection(&self, user_id: &str) {
        let data = self.get_or_create_user_data(user_id);
        data.active_connections.fetch_sub(1, Ordering::Relaxed);
        self.update_last_seen(user_id);
        
        debug!(user_id = user_id, "Recorded user disconnection");
    }

    /// Record bytes transferred for a user.
    ///
    /// # Arguments
    ///
    /// * `user_id` - User identifier
    /// * `bytes_rx` - Bytes received from user
    /// * `bytes_tx` - Bytes sent to user
    pub fn record_bytes(&self, user_id: &str, bytes_rx: u64, bytes_tx: u64) {
        let data = self.get_or_create_user_data(user_id);
        data.bytes_received.fetch_add(bytes_rx, Ordering::Relaxed);
        data.bytes_sent.fetch_add(bytes_tx, Ordering::Relaxed);
        self.update_last_seen(user_id);
    }

    /// Record a share submission from a user.
    ///
    /// # Arguments
    ///
    /// * `user_id` - User identifier
    /// * `accepted` - Whether the submission was accepted
    pub fn record_submission(&self, user_id: &str, accepted: bool) {
        let data = self.get_or_create_user_data(user_id);
        data.total_submissions.fetch_add(1, Ordering::Relaxed);
        
        if accepted {
            data.accepted_submissions.fetch_add(1, Ordering::Relaxed);
        } else {
            data.rejected_submissions.fetch_add(1, Ordering::Relaxed);
        }
        
        self.update_last_seen(user_id);
        
        debug!(
            user_id = user_id,
            accepted = accepted,
            "Recorded user submission"
        );
    }

    /// Record an authentication attempt from a user.
    ///
    /// # Arguments
    ///
    /// * `user_id` - User identifier
    /// * `success` - Whether authentication succeeded
    pub fn record_auth(&self, user_id: &str, success: bool) {
        let data = self.get_or_create_user_data(user_id);
        data.auth_attempts.fetch_add(1, Ordering::Relaxed);
        
        if success {
            data.auth_successes.fetch_add(1, Ordering::Relaxed);
        }
        
        self.update_last_seen(user_id);
    }

    /// Record a protocol error from a user.
    ///
    /// # Arguments
    ///
    /// * `user_id` - User identifier
    pub fn record_protocol_error(&self, user_id: &str) {
        let data = self.get_or_create_user_data(user_id);
        data.protocol_errors.fetch_add(1, Ordering::Relaxed);
        self.update_last_seen(user_id);
    }

    /// Record a job sent to a user.
    ///
    /// # Arguments
    ///
    /// * `user_id` - User identifier
    pub fn record_job_sent(&self, user_id: &str) {
        let data = self.get_or_create_user_data(user_id);
        data.jobs_sent.fetch_add(1, Ordering::Relaxed);
        self.update_last_seen(user_id);
    }

    /// Get a snapshot of metrics for a specific user.
    ///
    /// # Arguments
    ///
    /// * `user_id` - User identifier
    ///
    /// # Returns
    ///
    /// User metrics snapshot with current values
    pub fn get_user_snapshot(&self, user_id: &str) -> UserMetricsSnapshot {
        if let Some(data) = self.user_data.get(user_id) {
            UserMetricsSnapshot {
                user_id: user_id.to_string(),
                total_connections: data.total_connections.load(Ordering::Relaxed),
                active_connections: data.active_connections.load(Ordering::Relaxed),
                bytes_received: data.bytes_received.load(Ordering::Relaxed),
                bytes_sent: data.bytes_sent.load(Ordering::Relaxed),
                total_submissions: data.total_submissions.load(Ordering::Relaxed),
                accepted_submissions: data.accepted_submissions.load(Ordering::Relaxed),
                rejected_submissions: data.rejected_submissions.load(Ordering::Relaxed),
                auth_attempts: data.auth_attempts.load(Ordering::Relaxed),
                auth_successes: data.auth_successes.load(Ordering::Relaxed),
                protocol_errors: data.protocol_errors.load(Ordering::Relaxed),
                jobs_sent: data.jobs_sent.load(Ordering::Relaxed),
                first_seen_timestamp: data.first_seen.load(Ordering::Relaxed),
                last_seen_timestamp: data.last_seen.load(Ordering::Relaxed),
            }
        } else {
            UserMetricsSnapshot::default_for_user(user_id)
        }
    }

    /// Get a list of all tracked user IDs.
    pub fn get_all_users(&self) -> Vec<String> {
        self.user_data.iter().map(|entry| entry.key().clone()).collect()
    }

    /// Get the number of currently tracked users.
    pub fn user_count(&self) -> usize {
        self.user_data.len()
    }

    /// Get or create user data entry.
    fn get_or_create_user_data(&self, user_id: &str) -> Arc<UserMetricsData> {
        self.user_data
            .entry(user_id.to_string())
            .or_insert_with(|| {
                let now = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                    
                Arc::new(UserMetricsData {
                    total_connections: AtomicU64::new(0),
                    active_connections: AtomicU64::new(0),
                    bytes_received: AtomicU64::new(0),
                    bytes_sent: AtomicU64::new(0),
                    total_submissions: AtomicU64::new(0),
                    accepted_submissions: AtomicU64::new(0),
                    rejected_submissions: AtomicU64::new(0),
                    auth_attempts: AtomicU64::new(0),
                    auth_successes: AtomicU64::new(0),
                    protocol_errors: AtomicU64::new(0),
                    jobs_sent: AtomicU64::new(0),
                    first_seen: AtomicU64::new(now),
                    last_seen: AtomicU64::new(now),
                })
            })
            .clone()
    }

    /// Update the last seen timestamp for a user.
    fn update_last_seen(&self, user_id: &str) {
        if let Some(data) = self.user_data.get(user_id) {
            let now = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            data.last_seen.store(now, Ordering::Relaxed);
        }
    }
}

impl Default for UserMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of user-specific metrics at a point in time.
///
/// Contains all user metrics as simple values for serialization and analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMetricsSnapshot {
    /// User identifier
    pub user_id: String,
    /// Total connections from this user
    pub total_connections: u64,
    /// Currently active connections from this user
    pub active_connections: u64,
    /// Total bytes received from this user
    pub bytes_received: u64,
    /// Total bytes sent to this user
    pub bytes_sent: u64,
    /// Total submissions from this user
    pub total_submissions: u64,
    /// Accepted submissions from this user
    pub accepted_submissions: u64,
    /// Rejected submissions from this user
    pub rejected_submissions: u64,
    /// Authentication attempts by this user
    pub auth_attempts: u64,
    /// Successful authentications by this user
    pub auth_successes: u64,
    /// Protocol errors from this user
    pub protocol_errors: u64,
    /// Jobs sent to this user
    pub jobs_sent: u64,
    /// First connection timestamp (seconds since epoch)
    pub first_seen_timestamp: u64,
    /// Last activity timestamp (seconds since epoch)
    pub last_seen_timestamp: u64,
}

impl UserMetricsSnapshot {
    /// Create a default snapshot for a user ID.
    fn default_for_user(user_id: &str) -> Self {
        Self {
            user_id: user_id.to_string(),
            total_connections: 0,
            active_connections: 0,
            bytes_received: 0,
            bytes_sent: 0,
            total_submissions: 0,
            accepted_submissions: 0,
            rejected_submissions: 0,
            auth_attempts: 0,
            auth_successes: 0,
            protocol_errors: 0,
            jobs_sent: 0,
            first_seen_timestamp: 0,
            last_seen_timestamp: 0,
        }
    }

    /// Calculate the acceptance rate for this user's submissions.
    pub fn acceptance_rate(&self) -> f64 {
        if self.total_submissions == 0 {
            0.0
        } else {
            self.accepted_submissions as f64 / self.total_submissions as f64
        }
    }

    /// Calculate authentication success rate for this user.
    pub fn auth_success_rate(&self) -> f64 {
        if self.auth_attempts == 0 {
            0.0
        } else {
            self.auth_successes as f64 / self.auth_attempts as f64
        }
    }

    /// Calculate total bytes transferred (rx + tx).
    pub fn total_bytes(&self) -> u64 {
        self.bytes_received + self.bytes_sent
    }
}