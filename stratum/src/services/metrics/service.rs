//! Main metrics service coordinator and aggregation.
//!
//! This module provides the central metrics service that coordinates between
//! different metric collection modules and provides unified access to all metrics.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, error, info};

use super::{
    AtomicMetrics, DatabaseMetrics, MetricsSnapshot,
    TimeSeriesMetrics, UserMetrics,
};
use crate::services::database::DatabaseService;
use crate::services::performance::ResourceMonitor;

/// Configuration for the metrics service.
#[derive(Debug, Clone)]
pub struct MetricsConfig {
    /// How often to capture snapshots for time series
    pub snapshot_interval: Duration,
    /// How long to retain time series data
    pub retention_duration: Duration,
    /// Maximum number of samples per time series
    pub max_samples: usize,
    /// Whether to persist metrics to database
    pub persist_to_database: bool,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            snapshot_interval: Duration::from_secs(30),
            retention_duration: Duration::from_secs(3600), // 1 hour
            max_samples: 120, // 120 samples at 30s = 1 hour
            persist_to_database: true,
        }
    }
}

/// Main metrics service that coordinates all metrics collection and analysis.
///
/// The `MetricsService` acts as the central coordinator for all metrics-related
/// functionality, aggregating data from atomic metrics, user metrics, database
/// metrics, and time series analysis.
///
/// # Examples
///
/// ```rust
/// use loka_stratum::services::metrics::{MetricsService, MetricsConfig};
///
/// let config = MetricsConfig::default();
/// let service = MetricsService::new(config);
/// 
/// // Note: capturing snapshots requires async runtime
/// // tokio::runtime::Runtime::new().unwrap().block_on(async {
/// //     let snapshot = service.capture_snapshot().await;
/// //     println!("Active connections: {}", snapshot.active_connections);
/// // });
/// ```
#[derive(Debug)]
pub struct MetricsService {
    /// Atomic metrics for high-performance counters
    atomic_metrics: Arc<AtomicMetrics>,
    /// Per-user metrics tracking
    user_metrics: Arc<UserMetrics>,
    /// Database operation metrics
    database_metrics: Arc<RwLock<DatabaseMetrics>>,
    /// Time series data collection
    time_series: Arc<RwLock<TimeSeriesMetrics>>,
    /// Service configuration
    config: MetricsConfig,
    /// Optional database service for persistence
    database_service: Option<Arc<DatabaseService>>,
    /// Resource monitor for system metrics
    resource_monitor: Option<Arc<ResourceMonitor>>,
}

impl MetricsService {
    /// Create a new metrics service with provided atomic metrics.
    ///
    /// # Arguments
    ///
    /// * `atomic_metrics` - Shared atomic metrics instance
    ///
    /// # Examples
    ///
    /// ```rust
    /// use loka_stratum::services::metrics::{MetricsService, AtomicMetrics};
    /// use std::sync::Arc;
    ///
    /// let atomic_metrics = Arc::new(AtomicMetrics::new());
    /// let service = MetricsService::with_atomic_metrics(atomic_metrics);
    /// ```
    pub fn with_atomic_metrics(atomic_metrics: Arc<AtomicMetrics>) -> Self {
        let config = MetricsConfig::default();
        let time_series = Arc::new(RwLock::new(TimeSeriesMetrics::new(
            config.retention_duration,
            config.max_samples,
        )));

        Self {
            atomic_metrics,
            user_metrics: Arc::new(UserMetrics::new()),
            database_metrics: Arc::new(RwLock::new(DatabaseMetrics::new())),
            time_series,
            config,
            database_service: None,
            resource_monitor: None,
        }
    }

    /// Create a new metrics service with just configuration.
    ///
    /// This creates a new atomic metrics instance internally.
    /// 
    /// # Arguments
    ///
    /// * `config` - Custom metrics configuration
    pub fn new(config: MetricsConfig) -> Self {
        let atomic_metrics = Arc::new(AtomicMetrics::new());
        Self::with_config(atomic_metrics, config)
    }

    /// Create a new metrics service with custom configuration.
    ///
    /// # Arguments
    ///
    /// * `atomic_metrics` - Shared atomic metrics instance
    /// * `config` - Custom metrics configuration
    pub fn with_config(atomic_metrics: Arc<AtomicMetrics>, config: MetricsConfig) -> Self {
        let time_series = Arc::new(RwLock::new(TimeSeriesMetrics::new(
            config.retention_duration,
            config.max_samples,
        )));

        Self {
            atomic_metrics,
            user_metrics: Arc::new(UserMetrics::new()),
            database_metrics: Arc::new(RwLock::new(DatabaseMetrics::new())),
            time_series,
            config,
            database_service: None,
            resource_monitor: None,
        }
    }

    /// Set the database service for metrics persistence.
    ///
    /// # Arguments
    ///
    /// * `database_service` - Database service instance
    pub fn with_database_service(mut self, database_service: Arc<DatabaseService>) -> Self {
        self.database_service = Some(database_service);
        self
    }

    /// Set the resource monitor for system metrics.
    ///
    /// # Arguments
    ///
    /// * `resource_monitor` - Resource monitor instance
    pub fn with_resource_monitor(mut self, resource_monitor: Arc<ResourceMonitor>) -> Self {
        self.resource_monitor = Some(resource_monitor);
        self
    }

    /// Start the metrics service background tasks.
    ///
    /// This starts periodic snapshot collection and optional database persistence.
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Starting metrics service");

        // Start periodic snapshot collection
        let service_clone = self.clone_for_background();
        tokio::spawn(async move {
            service_clone.run_snapshot_collection().await;
        });

        // Start database persistence if configured
        if self.config.persist_to_database && self.database_service.is_some() {
            let service_clone = self.clone_for_background();
            tokio::spawn(async move {
                service_clone.run_database_persistence().await;
            });
        }

        Ok(())
    }

    /// Start metrics collection and return a handle.
    ///
    /// This is an alias for start() for compatibility with tests.
    pub async fn start_collection(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.start().await
    }

    /// Capture a complete metrics snapshot.
    ///
    /// # Returns
    ///
    /// Current metrics snapshot with all collected data
    pub async fn capture_snapshot(&self) -> MetricsSnapshot {
        let mut snapshot = self.atomic_metrics.snapshot();

        // Add resource utilization if available
        if let Some(resource_monitor) = &self.resource_monitor {
            let resource_summary = resource_monitor.get_utilization_summary().await;
            snapshot.memory_usage_bytes = (resource_summary.current_memory_mb * 1024.0 * 1024.0) as u64;
            snapshot.cpu_usage_percent = resource_summary.current_cpu_percent;
        }

        // Add database metrics
        let db_metrics = self.database_metrics.read().await;
        // Database metrics are included in the comprehensive snapshot structure
        
        snapshot
    }

    /// Get the atomic metrics reference.
    pub fn atomic_metrics(&self) -> &Arc<AtomicMetrics> {
        &self.atomic_metrics
    }

    /// Get the user metrics reference.
    pub fn user_metrics(&self) -> &Arc<UserMetrics> {
        &self.user_metrics
    }

    /// Get the database metrics (requires async read lock).
    pub async fn database_metrics(&self) -> tokio::sync::RwLockReadGuard<DatabaseMetrics> {
        self.database_metrics.read().await
    }

    /// Get a mutable reference to database metrics (requires async write lock).
    pub async fn database_metrics_mut(&self) -> tokio::sync::RwLockWriteGuard<DatabaseMetrics> {
        self.database_metrics.write().await
    }

    /// Get time series metrics for a specific metric.
    ///
    /// # Arguments
    ///
    /// * `metric_name` - Name of the metric
    /// * `sample_count` - Number of recent samples
    ///
    /// # Returns
    ///
    /// Moving average if enough data exists
    pub async fn get_time_series_average(&self, metric_name: &str, sample_count: usize) -> Option<f64> {
        let ts = self.time_series.read().await;
        ts.get_moving_average(metric_name, sample_count)
    }

    /// Get trend analysis for a metric.
    ///
    /// # Arguments
    ///
    /// * `metric_name` - Name of the metric
    /// * `sample_count` - Number of samples to analyze
    ///
    /// # Returns
    ///
    /// Trend direction (positive = increasing, negative = decreasing)
    pub async fn get_metric_trend(&self, metric_name: &str, sample_count: usize) -> Option<f64> {
        let ts = self.time_series.read().await;
        ts.get_trend(metric_name, sample_count)
    }

    /// Get all available time series metric names.
    pub async fn get_available_metrics(&self) -> Vec<String> {
        let ts = self.time_series.read().await;
        ts.metric_names()
    }

    /// Clone service for background tasks (lightweight clone of Arc references).
    fn clone_for_background(&self) -> MetricsServiceBackground {
        MetricsServiceBackground {
            atomic_metrics: self.atomic_metrics.clone(),
            user_metrics: self.user_metrics.clone(),
            database_metrics: self.database_metrics.clone(),
            time_series: self.time_series.clone(),
            config: self.config.clone(),
            database_service: self.database_service.clone(),
            resource_monitor: self.resource_monitor.clone(),
        }
    }

    /// Run periodic snapshot collection.
    async fn run_snapshot_collection(&self) {
        let mut interval = interval(self.config.snapshot_interval);
        
        loop {
            interval.tick().await;
            
            match self.capture_and_store_snapshot().await {
                Ok(_) => {
                    debug!("Captured metrics snapshot");
                }
                Err(e) => {
                    error!("Failed to capture metrics snapshot: {}", e);
                }
            }
        }
    }

    /// Capture snapshot and store in time series.
    async fn capture_and_store_snapshot(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let snapshot = self.capture_snapshot().await;
        let mut ts = self.time_series.write().await;

        // Store key metrics in time series
        ts.record_metric("active_connections", snapshot.active_connections as f64);
        ts.record_metric("messages_per_second", snapshot.messages_received as f64);
        ts.record_metric("bytes_per_second", (snapshot.bytes_received + snapshot.bytes_sent) as f64);
        ts.record_metric("cpu_usage_percent", snapshot.cpu_usage_percent);
        ts.record_metric("memory_usage_mb", (snapshot.memory_usage_bytes / (1024 * 1024)) as f64);

        Ok(())
    }

    /// Run database persistence (if configured).
    async fn run_database_persistence(&self) {
        if self.database_service.is_none() {
            return;
        }

        let mut interval = interval(Duration::from_secs(300)); // Every 5 minutes
        
        loop {
            interval.tick().await;
            
            match self.persist_metrics_to_database().await {
                Ok(_) => {
                    debug!("Persisted metrics to database");
                }
                Err(e) => {
                    error!("Failed to persist metrics to database: {}", e);
                }
            }
        }
    }

    /// Persist current metrics to database.
    async fn persist_metrics_to_database(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(db_service) = &self.database_service {
            let snapshot = self.capture_snapshot().await;
            // Implementation would depend on database schema for metrics storage
            // This is a placeholder for actual database persistence logic
            debug!("Would persist metrics snapshot to database");
        }
        Ok(())
    }

    // ===== DELEGATION METHODS TO ATOMIC METRICS =====
    
    /// Record a new connection.
    pub fn record_connection(&self) {
        self.atomic_metrics.increment_connection();
    }

    /// Record a disconnection.
    pub fn record_disconnection(&self) {
        self.atomic_metrics.decrement_connection();
    }

    /// Record bytes received.
    pub fn record_bytes_received(&self, bytes: u64) {
        self.atomic_metrics.record_bytes_received(bytes);
    }

    /// Record bytes sent.
    pub fn record_bytes_sent(&self, bytes: u64) {
        self.atomic_metrics.record_bytes_sent(bytes);
    }

    /// Record messages received.
    pub fn record_messages_received(&self, count: u64) {
        self.atomic_metrics.record_messages_received(count);
    }

    /// Record messages sent.
    pub fn record_messages_sent(&self, count: u64) {
        self.atomic_metrics.record_messages_sent(count);
    }

    /// Record a job received from pool.
    pub fn record_job_received(&self) {
        self.atomic_metrics.record_job_received();
    }

    /// Record a job processed and distributed.
    pub fn record_job_processed(&self) {
        self.atomic_metrics.record_job_processed();
    }

    /// Record a submission received.
    pub fn record_submission_received(&self) {
        self.atomic_metrics.record_submission_received();
    }

    /// Record a submission accepted.
    pub fn record_submission_accepted(&self) {
        self.atomic_metrics.record_submission_accepted();
    }

    /// Record a submission rejected.
    pub fn record_submission_rejected(&self) {
        self.atomic_metrics.record_submission_rejected();
    }

    /// Record an authentication attempt.
    pub fn record_auth_attempt(&self, success: bool) {
        self.atomic_metrics.record_auth_attempt(success);
    }

    /// Record a successful authentication.
    pub fn record_auth_success(&self) {
        self.atomic_metrics.record_auth_success();
    }

    /// Record a failed authentication.
    pub fn record_auth_failure(&self) {
        self.atomic_metrics.record_auth_failure();
    }

    /// Record a protocol error.
    pub fn record_protocol_error(&self) {
        self.atomic_metrics.record_protocol_error_simple();
    }

    /// Record a connection error.
    pub fn record_connection_error(&self) {
        self.atomic_metrics.record_connection_error();
    }

    /// Record a connection timeout.
    pub fn record_connection_timeout(&self) {
        self.atomic_metrics.record_connection_timeout();
    }

    /// Record a security violation.
    pub fn record_security_violation(&self) {
        self.atomic_metrics.record_security_violation();
    }

    /// Record a rate limit hit.
    pub fn record_rate_limit_hit(&self) {
        self.atomic_metrics.record_rate_limit_hit();
    }

    /// Update response time metrics.
    pub fn update_response_time(&self, time_ms: f64) {
        self.atomic_metrics.update_response_time(time_ms);
    }

    /// Update resource utilization.
    pub fn update_resource_utilization(&self, memory_mb: f64, cpu_percent: f64) {
        self.atomic_metrics.update_resource_utilization(memory_mb, cpu_percent);
    }

    /// Record a difficulty adjustment event.
    pub fn record_difficulty_adjustment_event(&self, difficulty: f64) {
        self.atomic_metrics.update_difficulty(difficulty);
    }

    /// Record a job distribution latency event.
    pub fn record_job_distribution_latency_event(&self, latency_ms: f64) {
        self.atomic_metrics.record_job_distribution_latency(latency_ms);
    }

    // ===== MINING-SPECIFIC EVENT RECORDING METHODS =====
    
    /// Record connection established event with duration.
    pub fn record_connection_established_event(&self, duration_ms: f64) {
        self.atomic_metrics.record_connection_duration(duration_ms);
    }

    /// Record connection closed event with duration.
    pub fn record_connection_closed_event(&self, duration_ms: f64) {
        self.atomic_metrics.record_connection_duration(duration_ms);
    }

    /// Update idle time gauge.
    pub fn update_idle_time(&self, idle_time_ms: f64) {
        self.atomic_metrics.store_f64(&self.atomic_metrics.idle_time_gauge_ms_bits, idle_time_ms);
    }

    /// Record HTTP request event.
    pub fn record_http_request_event(&self) {
        self.atomic_metrics.http_requests_total.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Record Stratum request event.
    pub fn record_stratum_request_event(&self) {
        self.atomic_metrics.stratum_requests_total.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Record protocol detection success event.
    pub fn record_protocol_detection_success_event(&self) {
        self.atomic_metrics.protocol_detection_successes.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Record protocol detection failure event.
    pub fn record_protocol_detection_failure_event(&self) {
        self.atomic_metrics.protocol_detection_failures.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Record HTTP CONNECT request event.
    pub fn record_http_connect_request_event(&self) {
        self.atomic_metrics.http_connect_requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Record direct Stratum connection event.
    pub fn record_direct_stratum_connection_event(&self) {
        self.atomic_metrics.direct_stratum_connections.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Record protocol conversion success event with rate.
    pub fn record_protocol_conversion_success_event(&self, success_rate: f64) {
        self.atomic_metrics.record_protocol_conversion_success(success_rate);
    }

    /// Record protocol conversion error event.
    pub fn record_protocol_conversion_error_event(&self) {
        self.atomic_metrics.protocol_conversion_errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Record share submission event.
    pub fn record_share_submission_event(&self) {
        self.atomic_metrics.share_submissions_total.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Record duplicate share event.
    pub fn record_duplicate_share_event(&self) {
        self.atomic_metrics.duplicate_shares_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Record stale share event.
    pub fn record_stale_share_event(&self) {
        self.atomic_metrics.record_stale_share();
    }

    /// Record share acceptance event.
    pub fn record_share_acceptance_event(&self, accepted: bool) {
        if accepted {
            self.atomic_metrics.record_submission_accepted();
        } else {
            self.atomic_metrics.record_submission_rejected();
        }
        // Also record acceptance rate
        let rate = if accepted { 1.0 } else { 0.0 };
        self.atomic_metrics.record_share_acceptance_rate(rate);
    }

    /// Get global metrics snapshot.
    pub fn get_global_snapshot(&self) -> MetricsSnapshot {
        self.atomic_metrics.snapshot()
    }

    /// Get all user metrics.
    pub fn get_all_user_metrics(&self) -> Vec<super::UserMetricsSnapshot> {
        let users = self.user_metrics.get_all_users();
        users.iter().map(|user_id| self.user_metrics.get_user_snapshot(user_id)).collect()
    }

    /// Record a categorized error event by type.
    pub fn record_categorized_error_event(&self, error_type: &str) {
        match error_type {
            "network" => self.atomic_metrics.record_connection_error(),
            "auth" | "authentication" => self.atomic_metrics.record_auth_failure(),
            "timeout" => self.atomic_metrics.record_connection_timeout(),
            "protocol" => self.atomic_metrics.record_protocol_error_simple(),
            "security" => self.atomic_metrics.record_security_violation(),
            _ => {
                // Generic internal error
                self.atomic_metrics.internal_errors_total.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
    }

    /// Record a reconnection attempt event.
    pub fn record_reconnection_attempt_event(&self) {
        // For now, treat this as a regular connection event
        // We can extend this later with more specific tracking if needed
        self.atomic_metrics.increment_connection();
    }
}

/// Lightweight background service struct for tokio tasks.
#[derive(Debug, Clone)]
struct MetricsServiceBackground {
    atomic_metrics: Arc<AtomicMetrics>,
    user_metrics: Arc<UserMetrics>,
    database_metrics: Arc<RwLock<DatabaseMetrics>>,
    time_series: Arc<RwLock<TimeSeriesMetrics>>,
    config: MetricsConfig,
    database_service: Option<Arc<DatabaseService>>,
    resource_monitor: Option<Arc<ResourceMonitor>>,
}

impl MetricsServiceBackground {
    async fn run_snapshot_collection(&self) {
        let mut interval = interval(self.config.snapshot_interval);
        
        loop {
            interval.tick().await;
            
            match self.capture_and_store_snapshot().await {
                Ok(_) => {
                    debug!("Captured metrics snapshot");
                }
                Err(e) => {
                    error!("Failed to capture metrics snapshot: {}", e);
                }
            }
        }
    }

    async fn capture_and_store_snapshot(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let snapshot = self.atomic_metrics.snapshot();
        let mut ts = self.time_series.write().await;

        // Store key metrics in time series
        ts.record_metric("active_connections", snapshot.active_connections as f64);
        ts.record_metric("messages_received", snapshot.messages_received as f64);
        ts.record_metric("bytes_received", snapshot.bytes_received as f64);

        Ok(())
    }

    async fn run_database_persistence(&self) {
        if self.database_service.is_none() {
            return;
        }

        let mut interval = interval(Duration::from_secs(300));
        
        loop {
            interval.tick().await;
            
            if let Err(e) = self.persist_metrics_to_database().await {
                error!("Failed to persist metrics to database: {}", e);
            }
        }
    }

    async fn persist_metrics_to_database(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(_db_service) = &self.database_service {
            let _snapshot = self.atomic_metrics.snapshot();
            // Database persistence implementation
            debug!("Would persist metrics to database");
        }
        Ok(())
    }

    // Delegate methods to atomic metrics for common operations
    
    /// Record a new connection.
    pub fn record_connection(&self) {
        self.atomic_metrics.increment_connection();
    }

    /// Record a disconnection.
    pub fn record_disconnection(&self) {
        self.atomic_metrics.decrement_connection();
    }

    /// Record bytes received.
    pub fn record_bytes_received(&self, bytes: u64) {
        self.atomic_metrics.record_bytes_received(bytes);
    }

    /// Record bytes sent.
    pub fn record_bytes_sent(&self, bytes: u64) {
        self.atomic_metrics.record_bytes_sent(bytes);
    }

    /// Record messages received.
    pub fn record_messages_received(&self, count: u64) {
        self.atomic_metrics.record_messages_received(count);
    }

    /// Record messages sent.
    pub fn record_messages_sent(&self, count: u64) {
        self.atomic_metrics.record_messages_sent(count);
    }

    /// Record a job received from pool.
    pub fn record_job_received(&self) {
        self.atomic_metrics.record_job_received();
    }

    /// Record a job processed and distributed.
    pub fn record_job_processed(&self) {
        self.atomic_metrics.record_job_processed();
    }

    /// Record a submission received.
    pub fn record_submission_received(&self) {
        self.atomic_metrics.record_submission_received();
    }

    /// Record a submission accepted.
    pub fn record_submission_accepted(&self) {
        self.atomic_metrics.record_submission_accepted();
    }

    /// Record a submission rejected.
    pub fn record_submission_rejected(&self) {
        self.atomic_metrics.record_submission_rejected();
    }

    /// Record an authentication attempt.
    pub fn record_auth_attempt(&self, success: bool) {
        self.atomic_metrics.record_auth_attempt(success);
    }

    /// Record a successful authentication.
    pub fn record_auth_success(&self) {
        self.atomic_metrics.record_auth_success();
    }

    /// Record a failed authentication.
    pub fn record_auth_failure(&self) {
        self.atomic_metrics.record_auth_failure();
    }

    /// Record a protocol error.
    pub fn record_protocol_error(&self) {
        self.atomic_metrics.record_protocol_error_simple();
    }

    /// Record a connection error.
    pub fn record_connection_error(&self) {
        self.atomic_metrics.record_connection_error();
    }

    /// Record a connection timeout.
    pub fn record_connection_timeout(&self) {
        self.atomic_metrics.record_connection_timeout();
    }

    /// Record a security violation.
    pub fn record_security_violation(&self) {
        self.atomic_metrics.record_security_violation();
    }

    /// Record a rate limit hit.
    pub fn record_rate_limit_hit(&self) {
        self.atomic_metrics.record_rate_limit_hit();
    }

    /// Update response time metrics.
    pub fn update_response_time(&self, time_ms: f64) {
        self.atomic_metrics.update_response_time(time_ms);
    }

    /// Update resource utilization.
    pub fn update_resource_utilization(&self, memory_mb: f64, cpu_percent: f64) {
        self.atomic_metrics.update_resource_utilization(memory_mb, cpu_percent);
    }

    // Mining-specific event recording methods
    
    /// Record connection established event with duration.
    pub fn record_connection_established_event(&self, duration_ms: f64) {
        self.atomic_metrics.record_connection_duration(duration_ms);
    }

    /// Record connection closed event with duration.
    pub fn record_connection_closed_event(&self, duration_ms: f64) {
        self.atomic_metrics.record_connection_duration(duration_ms);
    }

    /// Update idle time gauge.
    pub fn update_idle_time(&self, idle_time_ms: f64) {
        self.atomic_metrics.store_f64(&self.atomic_metrics.idle_time_gauge_ms_bits, idle_time_ms);
    }

    /// Record HTTP request event.
    pub fn record_http_request_event(&self) {
        self.atomic_metrics.http_requests_total.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Record Stratum request event.
    pub fn record_stratum_request_event(&self) {
        self.atomic_metrics.stratum_requests_total.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Record protocol detection success event.
    pub fn record_protocol_detection_success_event(&self) {
        self.atomic_metrics.protocol_detection_successes.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Record protocol detection failure event.
    pub fn record_protocol_detection_failure_event(&self) {
        self.atomic_metrics.protocol_detection_failures.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Record HTTP CONNECT request event.
    pub fn record_http_connect_request_event(&self) {
        self.atomic_metrics.http_connect_requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Record direct Stratum connection event.
    pub fn record_direct_stratum_connection_event(&self) {
        self.atomic_metrics.direct_stratum_connections.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Record protocol conversion success event with rate.
    pub fn record_protocol_conversion_success_event(&self, success_rate: f64) {
        self.atomic_metrics.record_protocol_conversion_success(success_rate);
    }

    /// Record protocol conversion error event.
    pub fn record_protocol_conversion_error_event(&self) {
        self.atomic_metrics.protocol_conversion_errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Record share submission event.
    pub fn record_share_submission_event(&self) {
        self.atomic_metrics.share_submissions_total.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Record duplicate share event.
    pub fn record_duplicate_share_event(&self) {
        self.atomic_metrics.duplicate_shares_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Record stale share event.
    pub fn record_stale_share_event(&self) {
        self.atomic_metrics.record_stale_share();
    }

    /// Record share acceptance event.
    pub fn record_share_acceptance_event(&self, accepted: bool) {
        if accepted {
            self.atomic_metrics.record_submission_accepted();
        } else {
            self.atomic_metrics.record_submission_rejected();
        }
        // Also record acceptance rate
        let rate = if accepted { 1.0 } else { 0.0 };
        self.atomic_metrics.record_share_acceptance_rate(rate);
    }

    /// Get global metrics snapshot.
    pub fn get_global_snapshot(&self) -> MetricsSnapshot {
        self.atomic_metrics.snapshot()
    }

    /// Get all user metrics.
    pub fn get_all_user_metrics(&self) -> Vec<super::UserMetricsSnapshot> {
        let users = self.user_metrics.get_all_users();
        users.iter().map(|user_id| self.user_metrics.get_user_snapshot(user_id)).collect()
    }

    /// Record a categorized error event by type.
    pub fn record_categorized_error_event(&self, error_type: &str) {
        match error_type {
            "network" => self.atomic_metrics.record_connection_error(),
            "auth" | "authentication" => self.atomic_metrics.record_auth_failure(),
            "timeout" => self.atomic_metrics.record_connection_timeout(),
            "protocol" => self.atomic_metrics.record_protocol_error_simple(),
            "security" => self.atomic_metrics.record_security_violation(),
            _ => {
                // Generic internal error
                self.atomic_metrics.internal_errors_total.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
    }
}

impl Default for MetricsService {
    fn default() -> Self {
        Self::new(MetricsConfig::default())
    }
}