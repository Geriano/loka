//! Point-in-time snapshots of metrics for analysis and reporting.
//!
//! This module provides functionality to capture and analyze metrics snapshots,
//! including derived calculations and time-based comparisons.

use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::atomic::AtomicMetrics;
use super::calculated::CalculatedMetrics;

/// A point-in-time snapshot of all metrics.
///
/// This structure captures the current state of all metrics at a specific moment,
/// allowing for historical comparison, trend analysis, and derived calculations.
///
/// # Fields Organization
///
/// The snapshot contains both raw metric values and calculated/derived metrics:
/// - Raw counters and gauges from [`AtomicMetrics`]
/// - Calculated rates and ratios
/// - Timestamp for temporal analysis
///
/// # Example
///
/// ```rust
/// use loka_stratum::services::metrics::{AtomicMetrics, MetricsSnapshot};
/// use std::sync::Arc;
///
/// let metrics = Arc::new(AtomicMetrics::new());
/// let snapshot = MetricsSnapshot::from_atomic(&metrics);
///
/// println!("Active connections: {}", snapshot.active_connections);
/// println!("Connection success rate: {:.2}%",
///          snapshot.calculate_metrics().connection_success_rate * 100.0);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    /// Timestamp when this snapshot was taken
    pub timestamp: SystemTime,

    // === CONNECTION METRICS ===
    /// Total connections since startup
    pub total_connections: u64,
    /// Currently active connections
    pub active_connections: u64,
    /// Total connection errors
    pub connection_errors: u64,
    /// Connection timeout count
    pub connection_timeouts: u64,

    // === PROTOCOL METRICS ===
    /// Messages received from miners
    pub messages_received: u64,
    /// Messages sent to miners
    pub messages_sent: u64,
    /// Bytes received from network
    pub bytes_received: u64,
    /// Bytes sent to network
    pub bytes_sent: u64,
    /// Protocol errors encountered
    pub protocol_errors: u64,

    // === AUTHENTICATION METRICS ===
    /// Total authentication attempts
    pub auth_attempts: u64,
    /// Successful authentications
    pub auth_successes: u64,
    /// Failed authentications
    pub auth_failures: u64,

    // === JOB METRICS ===
    /// Jobs received from pool
    pub jobs_received: u64,
    /// Jobs successfully processed
    pub jobs_processed: u64,
    /// Job processing errors
    pub job_processing_errors: u64,

    // === SUBMISSION METRICS ===
    /// Submissions received from miners
    pub submissions_received: u64,
    /// Submissions accepted by pool
    pub submissions_accepted: u64,
    /// Submissions rejected by pool
    pub submissions_rejected: u64,
    /// Duplicate submissions detected
    pub duplicate_submissions: u64,

    // === PERFORMANCE METRICS ===
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
    /// Maximum response time in milliseconds
    pub max_response_time_ms: f64,
    /// Minimum response time in milliseconds
    pub min_response_time_ms: f64,

    // === SECURITY METRICS ===
    /// Security violations detected
    pub security_violations: u64,
    /// Rate limit violations
    pub rate_limit_hits: u64,
    /// Blocked IP addresses count
    pub blocked_ips: u64,

    // === RESOURCE METRICS ===
    /// Current memory usage in bytes
    pub memory_usage_bytes: u64,
    /// CPU usage percentage
    pub cpu_usage_percent: f64,
    /// Active goroutines/tasks
    pub goroutine_count: u64,

    // === BUSINESS METRICS ===
    /// Estimated network hashrate
    pub hashrate_estimate: f64,
    /// Difficulty adjustment count
    pub difficulty_adjustments: u64,
    /// Pool switch count
    pub pool_switches: u64,

    // === CONNECTION LIFECYCLE METRICS ===
    /// Average connection duration in milliseconds
    pub avg_connection_duration_ms: f64,
    /// Maximum connection duration in milliseconds
    pub max_connection_duration_ms: f64,
    /// Minimum connection duration in milliseconds
    pub min_connection_duration_ms: f64,
    /// Current idle time in milliseconds
    pub idle_time_ms: f64,
    /// Reconnection attempts
    pub reconnection_attempts: u64,
    /// Connections established
    pub connections_established: u64,
    /// Connections closed
    pub connections_closed: u64,

    // === PROTOCOL DETECTION METRICS ===
    /// HTTP requests received
    pub http_requests: u64,
    /// Stratum requests received
    pub stratum_requests: u64,
    /// Protocol detection failures
    pub protocol_detection_failures: u64,
    /// Protocol detection successes
    pub protocol_detection_successes: u64,
    /// Protocol conversion success rate
    pub protocol_conversion_success_rate: f64,
    /// Protocol conversion errors
    pub protocol_conversion_errors: u64,
    /// HTTP CONNECT requests
    pub http_connect_requests: u64,
    /// Direct Stratum connections
    pub direct_stratum_connections: u64,

    // === MINING OPERATION METRICS ===
    /// Total share submissions
    pub share_submissions: u64,
    /// Share acceptance rate
    pub share_acceptance_rate: f64,
    /// Difficulty adjustments
    pub difficulty_adjustments_count: u64,
    /// Average job distribution latency in milliseconds
    pub avg_job_distribution_latency_ms: f64,
    /// Maximum job distribution latency in milliseconds
    pub max_job_distribution_latency_ms: f64,
    /// Minimum job distribution latency in milliseconds
    pub min_job_distribution_latency_ms: f64,
    /// Current mining difficulty
    pub current_difficulty: f64,
    /// Shares per minute
    pub shares_per_minute: u64,
    /// Stale shares count
    pub stale_shares: u64,
    /// Duplicate shares count
    pub duplicate_shares: u64,

    // Alias fields for test compatibility
    /// Total share submissions (alias)
    pub share_submissions_total: u64,
    /// Share acceptance average rate (alias)  
    pub share_acceptance_avg_rate: f64,
    /// Difficulty adjustments counter (alias)
    pub difficulty_adjustments_counter: u64,
    /// Job distribution latency average ms (alias)
    pub job_distribution_latency_avg_ms: f64,
    /// Job distribution latency minimum ms (alias)
    pub job_distribution_latency_min_ms: f64,
    /// Job distribution latency maximum ms (alias)
    pub job_distribution_latency_max_ms: f64,
    /// Stale shares counter (alias)
    pub stale_shares_counter: u64,
    /// Duplicate shares counter (alias)
    pub duplicate_shares_counter: u64,
    /// Shares per minute gauge (alias)
    pub shares_per_minute_gauge: u64,

    // === ERROR CATEGORIZATION METRICS ===
    /// Network errors
    pub network_errors: u64,
    /// Authentication failures
    pub authentication_failures: u64,
    /// Timeout errors
    pub timeout_errors: u64,
    /// Protocol parse errors
    pub protocol_parse_errors: u64,
    /// Protocol version errors
    pub protocol_version_errors: u64,
    /// Protocol message errors
    pub protocol_message_errors: u64,
    /// Validation errors
    pub validation_errors: u64,
    /// Security violation errors
    pub security_violation_errors: u64,
    /// Resource exhaustion errors
    pub resource_exhaustion_errors: u64,
    /// Internal errors
    pub internal_errors: u64,

    // === RESOURCE UTILIZATION METRICS ===
    /// Average memory per connection in MB
    pub avg_memory_per_connection_mb: f64,
    /// Maximum memory per connection in MB
    pub max_memory_per_connection_mb: f64,
    /// Minimum memory per connection in MB
    pub min_memory_per_connection_mb: f64,
    /// CPU utilization percentage
    pub cpu_utilization: f64,
    /// Network RX bandwidth in bytes/sec
    pub network_bandwidth_rx_bps: u64,
    /// Network TX bandwidth in bytes/sec
    pub network_bandwidth_tx_bps: u64,
    /// Total connection memory in bytes
    pub connection_memory_total: u64,
    /// Peak connection memory in bytes
    pub connection_memory_peak: u64,
    /// Resource pressure events
    pub resource_pressure_events: u64,
    /// Memory efficiency ratio
    pub memory_efficiency_ratio: f64,
}

impl MetricsSnapshot {
    /// Create a snapshot from atomic metrics.
    ///
    /// This captures the current state of all atomic metrics and converts
    /// float values from their bit representations.
    pub fn from_atomic(metrics: &AtomicMetrics) -> Self {
        // Helper to load f64 from atomic bits
        let load_f64 = |atomic: &std::sync::atomic::AtomicU64| -> f64 {
            f64::from_bits(atomic.load(Ordering::Relaxed))
        };

        // Calculate averages for metrics with sum and count
        let avg_connection_duration = {
            let sum = load_f64(&metrics.connection_duration_sum_ms_bits);
            let count = metrics.connection_duration_count.load(Ordering::Relaxed);
            if count > 0 { sum / count as f64 } else { 0.0 }
        };

        let avg_job_latency = {
            let sum = load_f64(&metrics.job_distribution_latency_sum_ms_bits);
            let count = metrics
                .job_distribution_latency_count
                .load(Ordering::Relaxed);
            if count > 0 { sum / count as f64 } else { 0.0 }
        };

        let share_acceptance_rate = {
            let sum = load_f64(&metrics.share_acceptance_rate_sum_bits);
            let count = metrics.share_acceptance_rate_count.load(Ordering::Relaxed);
            if count > 0 { sum / count as f64 } else { 0.0 }
        };

        let protocol_conversion_rate = {
            let sum = load_f64(&metrics.protocol_conversion_success_rate_sum_bits);
            let count = metrics
                .protocol_conversion_success_count
                .load(Ordering::Relaxed);
            if count > 0 { sum / count as f64 } else { 0.0 }
        };

        let avg_memory_per_conn = {
            let sum = load_f64(&metrics.memory_usage_per_connection_sum_bits);
            let count = metrics
                .memory_usage_per_connection_count
                .load(Ordering::Relaxed);
            if count > 0 { sum / count as f64 } else { 0.0 }
        };

        Self {
            timestamp: SystemTime::now(),

            // Connection metrics
            total_connections: metrics.total_connections.load(Ordering::Relaxed),
            active_connections: metrics.active_connections.load(Ordering::Relaxed),
            connection_errors: metrics.connection_errors.load(Ordering::Relaxed),
            connection_timeouts: metrics.connection_timeouts.load(Ordering::Relaxed),

            // Protocol metrics
            messages_received: metrics.messages_received.load(Ordering::Relaxed),
            messages_sent: metrics.messages_sent.load(Ordering::Relaxed),
            bytes_received: metrics.bytes_received.load(Ordering::Relaxed),
            bytes_sent: metrics.bytes_sent.load(Ordering::Relaxed),
            protocol_errors: metrics.protocol_errors.load(Ordering::Relaxed),

            // Authentication metrics
            auth_attempts: metrics.auth_attempts.load(Ordering::Relaxed),
            auth_successes: metrics.auth_successes.load(Ordering::Relaxed),
            auth_failures: metrics.auth_failures.load(Ordering::Relaxed),

            // Job metrics
            jobs_received: metrics.jobs_received.load(Ordering::Relaxed),
            jobs_processed: metrics.jobs_processed.load(Ordering::Relaxed),
            job_processing_errors: metrics.job_processing_errors.load(Ordering::Relaxed),

            // Submission metrics
            submissions_received: metrics.submissions_received.load(Ordering::Relaxed),
            submissions_accepted: metrics.submissions_accepted.load(Ordering::Relaxed),
            submissions_rejected: metrics.submissions_rejected.load(Ordering::Relaxed),
            duplicate_submissions: metrics.duplicate_submissions.load(Ordering::Relaxed),

            // Performance metrics
            avg_response_time_ms: load_f64(&metrics.avg_response_time_ms_bits),
            max_response_time_ms: load_f64(&metrics.max_response_time_ms_bits),
            min_response_time_ms: load_f64(&metrics.min_response_time_ms_bits),

            // Security metrics
            security_violations: metrics.security_violations.load(Ordering::Relaxed),
            rate_limit_hits: metrics.rate_limit_hits.load(Ordering::Relaxed),
            blocked_ips: metrics.blocked_ips.load(Ordering::Relaxed),

            // Resource metrics
            memory_usage_bytes: metrics.memory_usage_bytes.load(Ordering::Relaxed),
            cpu_usage_percent: load_f64(&metrics.cpu_usage_percent_bits),
            goroutine_count: metrics.goroutine_count.load(Ordering::Relaxed),

            // Business metrics
            hashrate_estimate: load_f64(&metrics.hashrate_estimate_bits),
            difficulty_adjustments: metrics.difficulty_adjustments.load(Ordering::Relaxed),
            pool_switches: metrics.pool_switches.load(Ordering::Relaxed),

            // Connection lifecycle metrics
            avg_connection_duration_ms: avg_connection_duration,
            max_connection_duration_ms: load_f64(&metrics.connection_duration_max_ms_bits),
            min_connection_duration_ms: {
                let min = load_f64(&metrics.connection_duration_min_ms_bits);
                if min == f64::MAX { 0.0 } else { min }
            },
            idle_time_ms: load_f64(&metrics.idle_time_gauge_ms_bits),
            reconnection_attempts: metrics
                .reconnection_attempts_counter
                .load(Ordering::Relaxed),
            connections_established: metrics
                .connection_established_counter
                .load(Ordering::Relaxed),
            connections_closed: metrics.connection_closed_counter.load(Ordering::Relaxed),

            // Protocol detection metrics
            http_requests: metrics.http_requests_total.load(Ordering::Relaxed),
            stratum_requests: metrics.stratum_requests_total.load(Ordering::Relaxed),
            protocol_detection_failures: metrics
                .protocol_detection_failures
                .load(Ordering::Relaxed),
            protocol_detection_successes: metrics
                .protocol_detection_successes
                .load(Ordering::Relaxed),
            protocol_conversion_success_rate: protocol_conversion_rate,
            protocol_conversion_errors: metrics.protocol_conversion_errors.load(Ordering::Relaxed),
            http_connect_requests: metrics.http_connect_requests.load(Ordering::Relaxed),
            direct_stratum_connections: metrics.direct_stratum_connections.load(Ordering::Relaxed),

            // Mining operation metrics
            share_submissions: metrics.share_submissions_total.load(Ordering::Relaxed),
            share_acceptance_rate,
            difficulty_adjustments_count: metrics
                .difficulty_adjustments_counter
                .load(Ordering::Relaxed),
            avg_job_distribution_latency_ms: avg_job_latency,
            max_job_distribution_latency_ms: load_f64(
                &metrics.job_distribution_latency_max_ms_bits,
            ),
            min_job_distribution_latency_ms: {
                let min = load_f64(&metrics.job_distribution_latency_min_ms_bits);
                if min == f64::MAX { 0.0 } else { min }
            },
            current_difficulty: load_f64(&metrics.current_difficulty_bits),
            shares_per_minute: metrics.shares_per_minute_gauge.load(Ordering::Relaxed),
            stale_shares: metrics.stale_shares_counter.load(Ordering::Relaxed),
            duplicate_shares: metrics.duplicate_shares_counter.load(Ordering::Relaxed),

            // Populate alias fields with the same data
            share_submissions_total: metrics.share_submissions_total.load(Ordering::Relaxed),
            share_acceptance_avg_rate: share_acceptance_rate,
            difficulty_adjustments_counter: metrics
                .difficulty_adjustments_counter
                .load(Ordering::Relaxed),
            job_distribution_latency_avg_ms: avg_job_latency,
            job_distribution_latency_min_ms: {
                let min = load_f64(&metrics.job_distribution_latency_min_ms_bits);
                if min == f64::MAX { 0.0 } else { min }
            },
            job_distribution_latency_max_ms: load_f64(
                &metrics.job_distribution_latency_max_ms_bits,
            ),
            stale_shares_counter: metrics.stale_shares_counter.load(Ordering::Relaxed),
            duplicate_shares_counter: metrics.duplicate_shares_counter.load(Ordering::Relaxed),
            shares_per_minute_gauge: metrics.shares_per_minute_gauge.load(Ordering::Relaxed),

            // Error categorization metrics
            network_errors: metrics.network_errors_total.load(Ordering::Relaxed),
            authentication_failures: metrics
                .authentication_failures_total
                .load(Ordering::Relaxed),
            timeout_errors: metrics.timeout_errors_total.load(Ordering::Relaxed),
            protocol_parse_errors: metrics.protocol_parse_errors_total.load(Ordering::Relaxed),
            protocol_version_errors: metrics
                .protocol_version_errors_total
                .load(Ordering::Relaxed),
            protocol_message_errors: metrics
                .protocol_message_errors_total
                .load(Ordering::Relaxed),
            validation_errors: metrics.validation_errors_total.load(Ordering::Relaxed),
            security_violation_errors: metrics
                .security_violation_errors_total
                .load(Ordering::Relaxed),
            resource_exhaustion_errors: metrics
                .resource_exhaustion_errors_total
                .load(Ordering::Relaxed),
            internal_errors: metrics.internal_errors_total.load(Ordering::Relaxed),

            // Resource utilization metrics
            avg_memory_per_connection_mb: avg_memory_per_conn,
            max_memory_per_connection_mb: load_f64(&metrics.memory_usage_per_connection_max_bits),
            min_memory_per_connection_mb: {
                let min = load_f64(&metrics.memory_usage_per_connection_min_bits);
                if min == f64::MAX { 0.0 } else { min }
            },
            cpu_utilization: load_f64(&metrics.cpu_utilization_sample_bits),
            network_bandwidth_rx_bps: metrics
                .network_bandwidth_rx_bytes_per_sec
                .load(Ordering::Relaxed),
            network_bandwidth_tx_bps: metrics
                .network_bandwidth_tx_bytes_per_sec
                .load(Ordering::Relaxed),
            connection_memory_total: metrics
                .connection_memory_total_bytes
                .load(Ordering::Relaxed),
            connection_memory_peak: metrics.connection_memory_peak_bytes.load(Ordering::Relaxed),
            resource_pressure_events: metrics.resource_pressure_events.load(Ordering::Relaxed),
            memory_efficiency_ratio: load_f64(&metrics.memory_efficiency_ratio_bits),
        }
    }

    /// Calculate derived metrics from the snapshot.
    ///
    /// This computes various rates, ratios, and percentages from the raw metrics.
    pub fn calculate_metrics(&self) -> CalculatedMetrics {
        CalculatedMetrics::from_snapshot(self)
    }

    /// Calculate the time elapsed since this snapshot was taken.
    pub fn age(&self) -> Duration {
        SystemTime::now()
            .duration_since(self.timestamp)
            .unwrap_or(Duration::ZERO)
    }

    /// Get timestamp as seconds since UNIX epoch.
    pub fn timestamp_secs(&self) -> u64 {
        self.timestamp
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs()
    }

    /// Compare this snapshot with another to calculate deltas.
    ///
    /// Returns a new snapshot containing the differences between this snapshot
    /// and the provided baseline.
    pub fn delta_from(&self, baseline: &MetricsSnapshot) -> MetricsDelta {
        MetricsDelta {
            duration: self
                .timestamp
                .duration_since(baseline.timestamp)
                .unwrap_or(Duration::ZERO),

            connections_delta: self.total_connections as i64 - baseline.total_connections as i64,
            messages_delta: self.messages_received as i64 - baseline.messages_received as i64,
            bytes_received_delta: self.bytes_received as i64 - baseline.bytes_received as i64,
            bytes_sent_delta: self.bytes_sent as i64 - baseline.bytes_sent as i64,
            submissions_delta: self.submissions_received as i64
                - baseline.submissions_received as i64,
            accepted_delta: self.submissions_accepted as i64 - baseline.submissions_accepted as i64,
            rejected_delta: self.submissions_rejected as i64 - baseline.submissions_rejected as i64,
            errors_delta: self.protocol_errors as i64 - baseline.protocol_errors as i64,
        }
    }

    /// Export metrics in Prometheus format.
    pub fn to_prometheus_format(&self) -> String {
        let mut output = String::with_capacity(4096);

        // Connection metrics
        output.push_str("# HELP stratum_connections_total Total number of connections\n");
        output.push_str("# TYPE stratum_connections_total counter\n");
        output.push_str(&format!(
            "stratum_connections_total {}\n",
            self.total_connections
        ));

        output.push_str("# HELP stratum_connections_active Current active connections\n");
        output.push_str("# TYPE stratum_connections_active gauge\n");
        output.push_str(&format!(
            "stratum_connections_active {}\n",
            self.active_connections
        ));

        // Protocol metrics
        output.push_str("# HELP stratum_messages_received_total Messages received\n");
        output.push_str("# TYPE stratum_messages_received_total counter\n");
        output.push_str(&format!(
            "stratum_messages_received_total {}\n",
            self.messages_received
        ));

        output.push_str("# HELP stratum_bytes_received_total Bytes received\n");
        output.push_str("# TYPE stratum_bytes_received_total counter\n");
        output.push_str(&format!(
            "stratum_bytes_received_total {}\n",
            self.bytes_received
        ));

        // Mining metrics
        output.push_str("# HELP stratum_shares_submitted_total Shares submitted\n");
        output.push_str("# TYPE stratum_shares_submitted_total counter\n");
        output.push_str(&format!(
            "stratum_shares_submitted_total {}\n",
            self.share_submissions
        ));

        output.push_str("# HELP stratum_shares_accepted_total Shares accepted\n");
        output.push_str("# TYPE stratum_shares_accepted_total counter\n");
        output.push_str(&format!(
            "stratum_shares_accepted_total {}\n",
            self.submissions_accepted
        ));

        output.push_str("# HELP stratum_share_acceptance_rate Share acceptance rate\n");
        output.push_str("# TYPE stratum_share_acceptance_rate gauge\n");
        output.push_str(&format!(
            "stratum_share_acceptance_rate {:.4}\n",
            self.share_acceptance_rate
        ));

        // Hashrate metrics
        output.push_str("# HELP stratum_hashrate_estimate_hs Estimated hashrate in H/s\n");
        output.push_str("# TYPE stratum_hashrate_estimate_hs gauge\n");
        output.push_str(&format!(
            "stratum_hashrate_estimate_hs {:.2}\n",
            self.hashrate_estimate
        ));

        output.push_str("# HELP stratum_hashrate_estimate_mhs Estimated hashrate in MH/s\n");
        output.push_str("# TYPE stratum_hashrate_estimate_mhs gauge\n");
        output.push_str(&format!(
            "stratum_hashrate_estimate_mhs {:.4}\n",
            self.hashrate_estimate / 1_000_000.0
        ));

        output.push_str("# HELP stratum_difficulty_current Current mining difficulty\n");
        output.push_str("# TYPE stratum_difficulty_current gauge\n");
        output.push_str(&format!(
            "stratum_difficulty_current {:.6}\n",
            self.current_difficulty
        ));

        // Performance metrics
        output.push_str("# HELP stratum_response_time_ms Response time in milliseconds\n");
        output.push_str("# TYPE stratum_response_time_ms summary\n");
        output.push_str(&format!(
            "stratum_response_time_ms{{quantile=\"0.5\"}} {:.2}\n",
            self.avg_response_time_ms
        ));
        output.push_str(&format!(
            "stratum_response_time_ms{{quantile=\"1.0\"}} {:.2}\n",
            self.max_response_time_ms
        ));

        // Resource metrics
        output.push_str("# HELP stratum_memory_usage_bytes Memory usage\n");
        output.push_str("# TYPE stratum_memory_usage_bytes gauge\n");
        output.push_str(&format!(
            "stratum_memory_usage_bytes {}\n",
            self.memory_usage_bytes
        ));

        output.push_str("# HELP stratum_cpu_usage_percent CPU usage percentage\n");
        output.push_str("# TYPE stratum_cpu_usage_percent gauge\n");
        output.push_str(&format!(
            "stratum_cpu_usage_percent {:.2}\n",
            self.cpu_usage_percent
        ));

        output
    }
}

/// Represents the difference between two metric snapshots.
#[derive(Debug, Clone)]
pub struct MetricsDelta {
    /// Time duration between snapshots
    pub duration: Duration,

    /// Change in total connections
    pub connections_delta: i64,
    /// Change in messages received
    pub messages_delta: i64,
    /// Change in bytes received
    pub bytes_received_delta: i64,
    /// Change in bytes sent
    pub bytes_sent_delta: i64,
    /// Change in submissions
    pub submissions_delta: i64,
    /// Change in accepted submissions
    pub accepted_delta: i64,
    /// Change in rejected submissions
    pub rejected_delta: i64,
    /// Change in errors
    pub errors_delta: i64,
}

impl MetricsDelta {
    /// Calculate rates per second from the deltas.
    pub fn rates_per_second(&self) -> DeltaRates {
        let secs = self.duration.as_secs_f64();
        if secs == 0.0 {
            return DeltaRates::default();
        }

        DeltaRates {
            connections_per_sec: self.connections_delta as f64 / secs,
            messages_per_sec: self.messages_delta as f64 / secs,
            bytes_received_per_sec: self.bytes_received_delta as f64 / secs,
            bytes_sent_per_sec: self.bytes_sent_delta as f64 / secs,
            submissions_per_sec: self.submissions_delta as f64 / secs,
            accepts_per_sec: self.accepted_delta as f64 / secs,
            rejects_per_sec: self.rejected_delta as f64 / secs,
            errors_per_sec: self.errors_delta as f64 / secs,
        }
    }
}

/// Calculated rates from metric deltas.
#[derive(Debug, Clone, Default)]
pub struct DeltaRates {
    pub connections_per_sec: f64,
    pub messages_per_sec: f64,
    pub bytes_received_per_sec: f64,
    pub bytes_sent_per_sec: f64,
    pub submissions_per_sec: f64,
    pub accepts_per_sec: f64,
    pub rejects_per_sec: f64,
    pub errors_per_sec: f64,
}

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
            avg_connection_duration_ms: 0.0,
            max_connection_duration_ms: 0.0,
            min_connection_duration_ms: 0.0,
            idle_time_ms: 0.0,
            reconnection_attempts: 0,
            connections_established: 0,
            connections_closed: 0,
            http_requests: 0,
            stratum_requests: 0,
            protocol_detection_failures: 0,
            protocol_detection_successes: 0,
            protocol_conversion_success_rate: 0.0,
            protocol_conversion_errors: 0,
            http_connect_requests: 0,
            direct_stratum_connections: 0,
            share_submissions: 0,
            share_acceptance_rate: 0.0,
            difficulty_adjustments_count: 0,
            avg_job_distribution_latency_ms: 0.0,
            max_job_distribution_latency_ms: 0.0,
            min_job_distribution_latency_ms: 0.0,
            current_difficulty: 0.0,
            shares_per_minute: 0,
            stale_shares: 0,
            duplicate_shares: 0,
            network_errors: 0,
            authentication_failures: 0,
            timeout_errors: 0,
            protocol_parse_errors: 0,
            protocol_version_errors: 0,
            protocol_message_errors: 0,
            validation_errors: 0,
            security_violation_errors: 0,
            resource_exhaustion_errors: 0,
            internal_errors: 0,
            avg_memory_per_connection_mb: 0.0,
            max_memory_per_connection_mb: 0.0,
            min_memory_per_connection_mb: 0.0,
            cpu_utilization: 0.0,
            network_bandwidth_rx_bps: 0,
            network_bandwidth_tx_bps: 0,
            connection_memory_total: 0,
            connection_memory_peak: 0,
            resource_pressure_events: 0,
            memory_efficiency_ratio: 1.0,

            // Alias fields with same default values
            share_submissions_total: 0,
            share_acceptance_avg_rate: 0.0,
            difficulty_adjustments_counter: 0,
            job_distribution_latency_avg_ms: 0.0,
            job_distribution_latency_min_ms: 0.0,
            job_distribution_latency_max_ms: 0.0,
            stale_shares_counter: 0,
            duplicate_shares_counter: 0,
            shares_per_minute_gauge: 0,
        }
    }
}
