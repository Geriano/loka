use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::debug;

/// High-performance atomic metrics collector
#[derive(Debug)]
pub struct AtomicMetrics {
    // Connection metrics
    pub total_connections: AtomicU64,
    pub active_connections: AtomicU64,
    pub connection_errors: AtomicU64,
    pub connection_timeouts: AtomicU64,

    // Protocol metrics
    pub messages_received: AtomicU64,
    pub messages_sent: AtomicU64,
    pub bytes_received: AtomicU64,
    pub bytes_sent: AtomicU64,
    pub protocol_errors: AtomicU64,

    // Authentication metrics
    pub auth_attempts: AtomicU64,
    pub auth_successes: AtomicU64,
    pub auth_failures: AtomicU64,

    // Job metrics
    pub jobs_received: AtomicU64,
    pub jobs_processed: AtomicU64,
    pub job_processing_errors: AtomicU64,

    // Submission metrics
    pub submissions_received: AtomicU64,
    pub submissions_accepted: AtomicU64,
    pub submissions_rejected: AtomicU64,
    pub duplicate_submissions: AtomicU64,

    // Performance metrics (stored as atomic u64 bits for lock-free access)
    pub avg_response_time_ms_bits: AtomicU64,
    pub max_response_time_ms_bits: AtomicU64,
    pub min_response_time_ms_bits: AtomicU64,

    // Security metrics
    pub security_violations: AtomicU64,
    pub rate_limit_hits: AtomicU64,
    pub blocked_ips: AtomicU64,

    // Resource metrics
    pub memory_usage_bytes: AtomicU64,
    pub cpu_usage_percent_bits: AtomicU64,
    pub goroutine_count: AtomicU64,

    // Business metrics
    pub hashrate_estimate_bits: AtomicU64,
    pub difficulty_adjustments: AtomicU64,
    pub pool_switches: AtomicU64,
}

impl AtomicMetrics {
    pub fn new() -> Self {
        Self {
            total_connections: AtomicU64::new(0),
            active_connections: AtomicU64::new(0),
            connection_errors: AtomicU64::new(0),
            connection_timeouts: AtomicU64::new(0),

            messages_received: AtomicU64::new(0),
            messages_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
            protocol_errors: AtomicU64::new(0),

            auth_attempts: AtomicU64::new(0),
            auth_successes: AtomicU64::new(0),
            auth_failures: AtomicU64::new(0),

            jobs_received: AtomicU64::new(0),
            jobs_processed: AtomicU64::new(0),
            job_processing_errors: AtomicU64::new(0),

            submissions_received: AtomicU64::new(0),
            submissions_accepted: AtomicU64::new(0),
            submissions_rejected: AtomicU64::new(0),
            duplicate_submissions: AtomicU64::new(0),

            avg_response_time_ms_bits: AtomicU64::new((0.0_f64).to_bits()),
            max_response_time_ms_bits: AtomicU64::new((0.0_f64).to_bits()),
            min_response_time_ms_bits: AtomicU64::new(f64::MAX.to_bits()),

            security_violations: AtomicU64::new(0),
            rate_limit_hits: AtomicU64::new(0),
            blocked_ips: AtomicU64::new(0),

            memory_usage_bytes: AtomicU64::new(0),
            cpu_usage_percent_bits: AtomicU64::new((0.0_f64).to_bits()),
            goroutine_count: AtomicU64::new(0),

            hashrate_estimate_bits: AtomicU64::new((0.0_f64).to_bits()),
            difficulty_adjustments: AtomicU64::new(0),
            pool_switches: AtomicU64::new(0),
        }
    }

    // Helper methods for atomic float operations
    #[allow(unused)]
    fn store_f64(&self, atomic: &AtomicU64, value: f64) {
        atomic.store(value.to_bits(), Ordering::Relaxed);
    }

    fn load_f64(&self, atomic: &AtomicU64) -> f64 {
        f64::from_bits(atomic.load(Ordering::Relaxed))
    }

    fn compare_and_swap_f64(&self, atomic: &AtomicU64, current: f64, new: f64) -> Result<f64, f64> {
        match atomic.compare_exchange_weak(
            current.to_bits(),
            new.to_bits(),
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => Ok(new),
            Err(actual_bits) => Err(f64::from_bits(actual_bits)),
        }
    }

    /// Record response time and update running averages
    pub fn record_response_time(&self, duration_ms: f64) {
        // Update min using compare-and-swap loop
        loop {
            let current_min = self.load_f64(&self.min_response_time_ms_bits);
            if duration_ms < current_min {
                match self.compare_and_swap_f64(
                    &self.min_response_time_ms_bits,
                    current_min,
                    duration_ms,
                ) {
                    Ok(_) => break,
                    Err(_) => continue, // Retry if another thread updated it
                }
            } else {
                break;
            }
        }

        // Update max using compare-and-swap loop
        loop {
            let current_max = self.load_f64(&self.max_response_time_ms_bits);
            if duration_ms > current_max {
                match self.compare_and_swap_f64(
                    &self.max_response_time_ms_bits,
                    current_max,
                    duration_ms,
                ) {
                    Ok(_) => break,
                    Err(_) => continue, // Retry if another thread updated it
                }
            } else {
                break;
            }
        }

        // Update average using exponential moving average with compare-and-swap loop
        loop {
            let current_avg = self.load_f64(&self.avg_response_time_ms_bits);
            let alpha = 0.1; // Smoothing factor
            let new_avg = alpha * duration_ms + (1.0 - alpha) * current_avg;
            match self.compare_and_swap_f64(&self.avg_response_time_ms_bits, current_avg, new_avg) {
                Ok(_) => break,
                Err(_) => continue, // Retry if another thread updated it
            }
        }
    }

    /// Get current snapshot of metrics
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            timestamp: SystemTime::now(),

            // Connection metrics
            total_connections: self.total_connections.load(Ordering::Relaxed),
            active_connections: self.active_connections.load(Ordering::Relaxed),
            connection_errors: self.connection_errors.load(Ordering::Relaxed),
            connection_timeouts: self.connection_timeouts.load(Ordering::Relaxed),

            // Protocol metrics
            messages_received: self.messages_received.load(Ordering::Relaxed),
            messages_sent: self.messages_sent.load(Ordering::Relaxed),
            bytes_received: self.bytes_received.load(Ordering::Relaxed),
            bytes_sent: self.bytes_sent.load(Ordering::Relaxed),
            protocol_errors: self.protocol_errors.load(Ordering::Relaxed),

            // Authentication metrics
            auth_attempts: self.auth_attempts.load(Ordering::Relaxed),
            auth_successes: self.auth_successes.load(Ordering::Relaxed),
            auth_failures: self.auth_failures.load(Ordering::Relaxed),

            // Job metrics
            jobs_received: self.jobs_received.load(Ordering::Relaxed),
            jobs_processed: self.jobs_processed.load(Ordering::Relaxed),
            job_processing_errors: self.job_processing_errors.load(Ordering::Relaxed),

            // Submission metrics
            submissions_received: self.submissions_received.load(Ordering::Relaxed),
            submissions_accepted: self.submissions_accepted.load(Ordering::Relaxed),
            submissions_rejected: self.submissions_rejected.load(Ordering::Relaxed),
            duplicate_submissions: self.duplicate_submissions.load(Ordering::Relaxed),

            // Performance metrics
            avg_response_time_ms: self.load_f64(&self.avg_response_time_ms_bits),
            max_response_time_ms: self.load_f64(&self.max_response_time_ms_bits),
            min_response_time_ms: self.load_f64(&self.min_response_time_ms_bits),

            // Security metrics
            security_violations: self.security_violations.load(Ordering::Relaxed),
            rate_limit_hits: self.rate_limit_hits.load(Ordering::Relaxed),
            blocked_ips: self.blocked_ips.load(Ordering::Relaxed),

            // Resource metrics
            memory_usage_bytes: self.memory_usage_bytes.load(Ordering::Relaxed),
            cpu_usage_percent: self.load_f64(&self.cpu_usage_percent_bits),
            goroutine_count: self.goroutine_count.load(Ordering::Relaxed),

            // Business metrics
            hashrate_estimate: self.load_f64(&self.hashrate_estimate_bits),
            difficulty_adjustments: self.difficulty_adjustments.load(Ordering::Relaxed),
            pool_switches: self.pool_switches.load(Ordering::Relaxed),
        }
    }
}

impl Default for AtomicMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Point-in-time metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub timestamp: SystemTime,

    // Connection metrics
    pub total_connections: u64,
    pub active_connections: u64,
    pub connection_errors: u64,
    pub connection_timeouts: u64,

    // Protocol metrics
    pub messages_received: u64,
    pub messages_sent: u64,
    pub bytes_received: u64,
    pub bytes_sent: u64,
    pub protocol_errors: u64,

    // Authentication metrics
    pub auth_attempts: u64,
    pub auth_successes: u64,
    pub auth_failures: u64,

    // Job metrics
    pub jobs_received: u64,
    pub jobs_processed: u64,
    pub job_processing_errors: u64,

    // Submission metrics
    pub submissions_received: u64,
    pub submissions_accepted: u64,
    pub submissions_rejected: u64,
    pub duplicate_submissions: u64,

    // Performance metrics
    pub avg_response_time_ms: f64,
    pub max_response_time_ms: f64,
    pub min_response_time_ms: f64,

    // Security metrics
    pub security_violations: u64,
    pub rate_limit_hits: u64,
    pub blocked_ips: u64,

    // Resource metrics
    pub memory_usage_bytes: u64,
    pub cpu_usage_percent: f64,
    pub goroutine_count: u64,

    // Business metrics
    pub hashrate_estimate: f64,
    pub difficulty_adjustments: u64,
    pub pool_switches: u64,
}

impl MetricsSnapshot {
    /// Calculate derived metrics
    pub fn calculated_metrics(&self) -> CalculatedMetrics {
        CalculatedMetrics {
            // Connection metrics
            connection_success_rate: if self.total_connections > 0 {
                (self.total_connections - self.connection_errors) as f64
                    / self.total_connections as f64
            } else {
                0.0
            },

            // Authentication metrics
            auth_success_rate: if self.auth_attempts > 0 {
                self.auth_successes as f64 / self.auth_attempts as f64
            } else {
                0.0
            },

            // Submission metrics
            submission_acceptance_rate: if self.submissions_received > 0 {
                self.submissions_accepted as f64 / self.submissions_received as f64
            } else {
                0.0
            },

            // Protocol metrics
            avg_message_size: if self.messages_received > 0 {
                self.bytes_received as f64 / self.messages_received as f64
            } else {
                0.0
            },

            protocol_error_rate: if self.messages_received > 0 {
                self.protocol_errors as f64 / self.messages_received as f64
            } else {
                0.0
            },

            // Job metrics
            job_processing_success_rate: if self.jobs_received > 0 {
                self.jobs_processed as f64 / self.jobs_received as f64
            } else {
                0.0
            },
        }
    }
}

/// Calculated metrics derived from raw metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalculatedMetrics {
    pub connection_success_rate: f64,
    pub auth_success_rate: f64,
    pub submission_acceptance_rate: f64,
    pub avg_message_size: f64,
    pub protocol_error_rate: f64,
    pub job_processing_success_rate: f64,
}

/// Time-series metrics for historical data
#[derive(Debug)]
pub struct TimeSeriesMetrics {
    pub snapshots: RwLock<Vec<MetricsSnapshot>>,
    pub max_snapshots: usize,
}

impl TimeSeriesMetrics {
    pub fn new(max_snapshots: usize) -> Self {
        Self {
            snapshots: RwLock::new(Vec::with_capacity(max_snapshots)),
            max_snapshots,
        }
    }

    pub async fn add_snapshot(&self, snapshot: MetricsSnapshot) {
        let mut snapshots = self.snapshots.write().await;
        snapshots.push(snapshot);

        // Keep only the most recent snapshots
        if snapshots.len() > self.max_snapshots {
            snapshots.remove(0);
        }
    }

    pub async fn get_snapshots(&self, limit: Option<usize>) -> Vec<MetricsSnapshot> {
        let snapshots = self.snapshots.read().await;
        match limit {
            Some(n) => snapshots.iter().rev().take(n).rev().cloned().collect(),
            None => snapshots.clone(),
        }
    }

    pub async fn get_latest(&self) -> Option<MetricsSnapshot> {
        let snapshots = self.snapshots.read().await;
        snapshots.last().cloned()
    }
}

/// Per-user metrics tracking
#[derive(Debug)]
pub struct UserMetrics {
    pub user_id: String,
    pub connections: AtomicU64,
    pub messages_received: AtomicU64,
    pub messages_sent: AtomicU64,
    pub submissions_received: AtomicU64,
    pub submissions_accepted: AtomicU64,
    pub submissions_rejected: AtomicU64,
    pub hashrate_estimate_bits: AtomicU64,
    pub first_seen: SystemTime,
    pub last_seen: AtomicU64, // Unix timestamp
}

impl UserMetrics {
    pub fn new(user_id: String) -> Self {
        Self {
            user_id,
            connections: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
            messages_sent: AtomicU64::new(0),
            submissions_received: AtomicU64::new(0),
            submissions_accepted: AtomicU64::new(0),
            submissions_rejected: AtomicU64::new(0),
            hashrate_estimate_bits: AtomicU64::new((0.0_f64).to_bits()),
            first_seen: SystemTime::now(),
            last_seen: AtomicU64::new(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            ),
        }
    }

    pub fn update_last_seen(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.last_seen.store(now, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> UserMetricsSnapshot {
        UserMetricsSnapshot {
            user_id: self.user_id.clone(),
            connections: self.connections.load(Ordering::Relaxed),
            messages_received: self.messages_received.load(Ordering::Relaxed),
            messages_sent: self.messages_sent.load(Ordering::Relaxed),
            submissions_received: self.submissions_received.load(Ordering::Relaxed),
            submissions_accepted: self.submissions_accepted.load(Ordering::Relaxed),
            submissions_rejected: self.submissions_rejected.load(Ordering::Relaxed),
            hashrate_estimate: f64::from_bits(self.hashrate_estimate_bits.load(Ordering::Relaxed)),
            first_seen: self.first_seen,
            last_seen_timestamp: self.last_seen.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMetricsSnapshot {
    pub user_id: String,
    pub connections: u64,
    pub messages_received: u64,
    pub messages_sent: u64,
    pub submissions_received: u64,
    pub submissions_accepted: u64,
    pub submissions_rejected: u64,
    pub hashrate_estimate: f64,
    pub first_seen: SystemTime,
    pub last_seen_timestamp: u64,
}

impl UserMetricsSnapshot {
    pub fn acceptance_rate(&self) -> f64 {
        if self.submissions_received > 0 {
            self.submissions_accepted as f64 / self.submissions_received as f64
        } else {
            0.0
        }
    }
}

/// High-performance metrics service
#[derive(Debug)]
pub struct MetricsService {
    pub global_metrics: Arc<AtomicMetrics>,
    pub time_series: Arc<TimeSeriesMetrics>,
    pub user_metrics: Arc<DashMap<String, Arc<UserMetrics>>>,
    pub config: MetricsConfig,
}

#[derive(Debug, Clone)]
pub struct MetricsConfig {
    pub collection_interval: Duration,
    pub max_snapshots: usize,
    pub user_metrics_ttl: Duration,
    pub enable_time_series: bool,
    pub enable_user_metrics: bool,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            collection_interval: Duration::from_secs(30),
            max_snapshots: 2880, // 24 hours at 30s intervals
            user_metrics_ttl: Duration::from_secs(86400), // 24 hours
            enable_time_series: true,
            enable_user_metrics: true,
        }
    }
}

impl MetricsService {
    pub fn new(config: MetricsConfig) -> Self {
        Self {
            global_metrics: Arc::new(AtomicMetrics::new()),
            time_series: Arc::new(TimeSeriesMetrics::new(config.max_snapshots)),
            user_metrics: Arc::new(DashMap::new()),
            config,
        }
    }

    /// Start background metrics collection
    pub async fn start_collection(&self) -> tokio::task::JoinHandle<()> {
        let global_metrics = self.global_metrics.clone();
        let time_series = self.time_series.clone();
        let user_metrics = self.user_metrics.clone();
        let interval_duration = self.config.collection_interval;
        let ttl = self.config.user_metrics_ttl;
        let enable_time_series = self.config.enable_time_series;

        tokio::spawn(async move {
            let mut interval = interval(interval_duration);

            loop {
                interval.tick().await;

                // Take snapshot
                let snapshot = global_metrics.snapshot();

                // Add to time series if enabled
                if enable_time_series {
                    time_series.add_snapshot(snapshot.clone()).await;
                }

                // Log key metrics
                debug!(
                    active_connections = snapshot.active_connections,
                    messages_per_sec = snapshot.messages_received,
                    avg_response_time = snapshot.avg_response_time_ms,
                    hashrate = snapshot.hashrate_estimate,
                    "Metrics snapshot collected"
                );

                // Cleanup old user metrics
                Self::cleanup_user_metrics(&user_metrics, ttl).await;
            }
        })
    }

    async fn cleanup_user_metrics(user_metrics: &DashMap<String, Arc<UserMetrics>>, ttl: Duration) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let ttl_secs = ttl.as_secs();
        let mut to_remove = Vec::new();

        for entry in user_metrics.iter() {
            let last_seen = entry.value().last_seen.load(Ordering::Relaxed);
            if now - last_seen > ttl_secs {
                to_remove.push(entry.key().clone());
            }
        }

        for user_id in to_remove {
            user_metrics.remove(&user_id);
            debug!(user_id = user_id, "Cleaned up expired user metrics");
        }
    }

    /// Get or create user metrics
    pub fn get_user_metrics(&self, user_id: &str) -> Arc<UserMetrics> {
        self.user_metrics
            .entry(user_id.to_string())
            .or_insert_with(|| Arc::new(UserMetrics::new(user_id.to_string())))
            .clone()
    }

    /// Get current global metrics snapshot
    pub fn get_global_snapshot(&self) -> MetricsSnapshot {
        self.global_metrics.snapshot()
    }

    /// Get historical metrics
    pub async fn get_historical_metrics(&self, limit: Option<usize>) -> Vec<MetricsSnapshot> {
        self.time_series.get_snapshots(limit).await
    }

    /// Get all user metrics
    pub fn get_all_user_metrics(&self) -> Vec<UserMetricsSnapshot> {
        self.user_metrics
            .iter()
            .map(|entry| entry.value().snapshot())
            .collect()
    }

    /// Get top users by hashrate
    pub fn get_top_users_by_hashrate(&self, limit: usize) -> Vec<UserMetricsSnapshot> {
        let mut users: Vec<UserMetricsSnapshot> = self
            .user_metrics
            .iter()
            .map(|entry| entry.value().snapshot())
            .collect();

        users.sort_by(|a, b| {
            b.hashrate_estimate
                .partial_cmp(&a.hashrate_estimate)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        users.into_iter().take(limit).collect()
    }

    /// Record various metrics (convenience methods)
    pub fn record_connection(&self, user_id: Option<&str>) {
        self.global_metrics
            .total_connections
            .fetch_add(1, Ordering::Relaxed);
        self.global_metrics
            .active_connections
            .fetch_add(1, Ordering::Relaxed);

        if let Some(user_id) = user_id {
            let user_metrics = self.get_user_metrics(user_id);
            user_metrics.connections.fetch_add(1, Ordering::Relaxed);
            user_metrics.update_last_seen();
        }
    }

    pub fn record_disconnection(&self, user_id: Option<&str>) {
        self.global_metrics
            .active_connections
            .fetch_sub(1, Ordering::Relaxed);

        if let Some(user_id) = user_id {
            let user_metrics = self.get_user_metrics(user_id);
            user_metrics.connections.fetch_sub(1, Ordering::Relaxed);
        }
    }

    pub fn record_message_received(&self, bytes: usize, user_id: Option<&str>) {
        self.global_metrics
            .messages_received
            .fetch_add(1, Ordering::Relaxed);
        self.global_metrics
            .bytes_received
            .fetch_add(bytes as u64, Ordering::Relaxed);

        if let Some(user_id) = user_id {
            let user_metrics = self.get_user_metrics(user_id);
            user_metrics
                .messages_received
                .fetch_add(1, Ordering::Relaxed);
            user_metrics.update_last_seen();
        }
    }

    pub fn record_message_sent(&self, bytes: usize, user_id: Option<&str>) {
        self.global_metrics
            .messages_sent
            .fetch_add(1, Ordering::Relaxed);
        self.global_metrics
            .bytes_sent
            .fetch_add(bytes as u64, Ordering::Relaxed);

        if let Some(user_id) = user_id {
            let user_metrics = self.get_user_metrics(user_id);
            user_metrics.messages_sent.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn record_auth_attempt(&self, success: bool, user_id: Option<&str>) {
        self.global_metrics
            .auth_attempts
            .fetch_add(1, Ordering::Relaxed);
        if success {
            self.global_metrics
                .auth_successes
                .fetch_add(1, Ordering::Relaxed);
        } else {
            self.global_metrics
                .auth_failures
                .fetch_add(1, Ordering::Relaxed);
        }

        if let Some(user_id) = user_id {
            let user_metrics = self.get_user_metrics(user_id);
            user_metrics.update_last_seen();
        }
    }

    pub fn record_submission(&self, accepted: bool, user_id: Option<&str>, hashrate: Option<f64>) {
        self.global_metrics
            .submissions_received
            .fetch_add(1, Ordering::Relaxed);
        if accepted {
            self.global_metrics
                .submissions_accepted
                .fetch_add(1, Ordering::Relaxed);
        } else {
            self.global_metrics
                .submissions_rejected
                .fetch_add(1, Ordering::Relaxed);
        }

        if let Some(user_id) = user_id {
            let user_metrics = self.get_user_metrics(user_id);
            user_metrics
                .submissions_received
                .fetch_add(1, Ordering::Relaxed);
            if accepted {
                user_metrics
                    .submissions_accepted
                    .fetch_add(1, Ordering::Relaxed);
            } else {
                user_metrics
                    .submissions_rejected
                    .fetch_add(1, Ordering::Relaxed);
            }
            user_metrics.update_last_seen();

            if let Some(hashrate) = hashrate {
                user_metrics
                    .hashrate_estimate_bits
                    .store(hashrate.to_bits(), Ordering::Relaxed);
            }
        }
    }

    pub fn record_response_time(&self, duration: Duration) {
        let duration_ms = duration.as_secs_f64() * 1000.0;
        self.global_metrics.record_response_time(duration_ms);
    }

    pub fn record_security_violation(&self) {
        self.global_metrics
            .security_violations
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_rate_limit_hit(&self) {
        self.global_metrics
            .rate_limit_hits
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Get metrics summary as human-readable string
    pub fn get_summary(&self) -> String {
        let snapshot = self.get_global_snapshot();
        let calculated = snapshot.calculated_metrics();

        format!(
            "Metrics Summary:\n\
             Active Connections: {}\n\
             Messages: {} received, {} sent\n\
             Auth Success Rate: {:.1}%\n\
             Submission Acceptance Rate: {:.1}%\n\
             Avg Response Time: {:.1}ms\n\
             Hashrate Estimate: {:.2} MH/s\n\
             Security Violations: {}",
            snapshot.active_connections,
            snapshot.messages_received,
            snapshot.messages_sent,
            calculated.auth_success_rate * 100.0,
            calculated.submission_acceptance_rate * 100.0,
            snapshot.avg_response_time_ms,
            snapshot.hashrate_estimate / 1_000_000.0, // Convert to MH/s
            snapshot.security_violations
        )
    }
}

impl Default for MetricsService {
    fn default() -> Self {
        Self::new(MetricsConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[test]
    fn test_atomic_metrics() {
        let metrics = AtomicMetrics::new();

        // Test basic operations
        metrics.total_connections.fetch_add(10, Ordering::Relaxed);
        metrics.messages_received.fetch_add(100, Ordering::Relaxed);
        metrics.record_response_time(50.0);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.total_connections, 10);
        assert_eq!(snapshot.messages_received, 100);
        // Exponential moving average: alpha * new_value + (1-alpha) * old_value
        // With alpha=0.1, first value: 0.1 * 50.0 + 0.9 * 0.0 = 5.0
        assert_eq!(snapshot.avg_response_time_ms, 5.0);
    }

    #[tokio::test]
    async fn test_time_series_metrics() {
        let time_series = TimeSeriesMetrics::new(3);

        // Add snapshots
        for i in 1..=5 {
            let mut snapshot = MetricsSnapshot {
                timestamp: SystemTime::now(),
                total_connections: i,
                active_connections: i,
                ..Default::default()
            };
            time_series.add_snapshot(snapshot).await;
        }

        let snapshots = time_series.get_snapshots(None).await;
        assert_eq!(snapshots.len(), 3); // Should keep only last 3
        assert_eq!(snapshots[0].total_connections, 3);
        assert_eq!(snapshots[2].total_connections, 5);
    }

    #[tokio::test]
    async fn test_metrics_service() {
        let config = MetricsConfig {
            collection_interval: Duration::from_millis(100),
            ..Default::default()
        };
        let service = MetricsService::new(config);

        // Record some metrics
        service.record_connection(Some("user1"));
        service.record_message_received(100, Some("user1"));
        service.record_submission(true, Some("user1"), Some(1_000_000.0));

        let snapshot = service.get_global_snapshot();
        assert_eq!(snapshot.active_connections, 1);
        assert_eq!(snapshot.messages_received, 1);
        assert_eq!(snapshot.submissions_accepted, 1);

        // Check user metrics
        let user_metrics = service.get_user_metrics("user1");
        let user_snapshot = user_metrics.snapshot();
        assert_eq!(user_snapshot.connections, 1);
        assert_eq!(user_snapshot.messages_received, 1);
        assert_eq!(user_snapshot.hashrate_estimate, 1_000_000.0);
    }

    #[test]
    fn test_calculated_metrics() {
        let snapshot = MetricsSnapshot {
            timestamp: SystemTime::now(),
            total_connections: 100,
            connection_errors: 5,
            auth_attempts: 80,
            auth_successes: 76,
            submissions_received: 1000,
            submissions_accepted: 950,
            messages_received: 5000,
            bytes_received: 50000,
            protocol_errors: 10,
            jobs_received: 200,
            jobs_processed: 195,
            ..Default::default()
        };

        let calculated = snapshot.calculated_metrics();

        assert_eq!(calculated.connection_success_rate, 0.95);
        assert_eq!(calculated.auth_success_rate, 0.95);
        assert_eq!(calculated.submission_acceptance_rate, 0.95);
        assert_eq!(calculated.avg_message_size, 10.0);
        assert_eq!(calculated.protocol_error_rate, 0.002);
        assert_eq!(calculated.job_processing_success_rate, 0.975);
    }
}

// Add Default implementation for MetricsSnapshot
impl Default for MetricsSnapshot {
    fn default() -> Self {
        Self {
            timestamp: SystemTime::now(),
            total_connections: 0,
            active_connections: 0,
            connection_errors: 0,
            connection_timeouts: 0,
            messages_received: 0,
            messages_sent: 0,
            bytes_received: 0,
            bytes_sent: 0,
            protocol_errors: 0,
            auth_attempts: 0,
            auth_successes: 0,
            auth_failures: 0,
            jobs_received: 0,
            jobs_processed: 0,
            job_processing_errors: 0,
            submissions_received: 0,
            submissions_accepted: 0,
            submissions_rejected: 0,
            duplicate_submissions: 0,
            avg_response_time_ms: 0.0,
            max_response_time_ms: 0.0,
            min_response_time_ms: 0.0,
            security_violations: 0,
            rate_limit_hits: 0,
            blocked_ips: 0,
            memory_usage_bytes: 0,
            cpu_usage_percent: 0.0,
            goroutine_count: 0,
            hashrate_estimate: 0.0,
            difficulty_adjustments: 0,
            pool_switches: 0,
        }
    }
}
