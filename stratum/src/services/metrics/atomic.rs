//! Atomic metrics implementation for lock-free, high-performance metrics collection.
//!
//! This module provides the core [`AtomicMetrics`] structure that uses atomic operations
//! for thread-safe, lock-free metrics collection with sub-microsecond performance.

use std::sync::atomic::{AtomicU64, Ordering};

/// High-performance atomic metrics collector for mining proxy operations.
///
/// This structure provides lock-free, thread-safe metrics collection with
/// sub-microsecond performance characteristics. All metrics are stored as
/// atomic integers for maximum performance in high-throughput scenarios.
///
/// # Performance Characteristics
///
/// - Counter operations: ~4.6ns per operation
/// - Gauge operations: ~3.7ns per operation  
/// - Histogram operations: ~3.6ns per operation
/// - Float values stored as atomic bit patterns for lock-free access
///
/// # Metric Categories
///
/// The metrics are organized into logical groups:
/// - **Connection Metrics**: TCP connection lifecycle and health
/// - **Protocol Metrics**: Message parsing and communication stats
/// - **Authentication Metrics**: Login attempts and session management
/// - **Mining Metrics**: Job distribution and share submissions
/// - **Performance Metrics**: Latency and throughput measurements
/// - **Security Metrics**: Rate limiting and violation tracking
/// - **Resource Metrics**: Memory and CPU utilization
///
/// # Examples
///
/// ```rust
/// use loka_stratum::services::metrics::AtomicMetrics;
/// use std::sync::Arc;
/// use std::sync::atomic::Ordering;
///
/// let metrics = Arc::new(AtomicMetrics::new());
///
/// // Record connection events
/// metrics.total_connections.fetch_add(1, Ordering::Relaxed);
/// metrics.active_connections.fetch_add(1, Ordering::Relaxed);
///
/// // Record protocol activity
/// metrics.messages_received.fetch_add(1, Ordering::Relaxed);
/// metrics.bytes_received.fetch_add(1024, Ordering::Relaxed);
/// ```
#[derive(Debug)]
pub struct AtomicMetrics {
    // === CONNECTION METRICS ===
    /// Total number of connections received since startup
    pub total_connections: AtomicU64,
    /// Current number of active connections
    pub active_connections: AtomicU64,
    /// Total connection errors encountered
    pub connection_errors: AtomicU64,
    /// Number of connection timeouts
    pub connection_timeouts: AtomicU64,

    // === PROTOCOL METRICS ===
    /// Total messages received from miners
    pub messages_received: AtomicU64,
    /// Total messages sent to miners
    pub messages_sent: AtomicU64,
    /// Total bytes received from network
    pub bytes_received: AtomicU64,
    /// Total bytes sent to network
    pub bytes_sent: AtomicU64,
    /// Protocol parsing or handling errors
    pub protocol_errors: AtomicU64,

    // === AUTHENTICATION METRICS ===
    /// Total authentication attempts
    pub auth_attempts: AtomicU64,
    /// Successful authentications
    pub auth_successes: AtomicU64,
    /// Failed authentication attempts
    pub auth_failures: AtomicU64,

    // === JOB METRICS ===
    /// Jobs received from pool
    pub jobs_received: AtomicU64,
    /// Jobs successfully processed and distributed
    pub jobs_processed: AtomicU64,
    /// Job processing errors
    pub job_processing_errors: AtomicU64,

    // === SUBMISSION METRICS ===
    /// Share submissions received from miners
    pub submissions_received: AtomicU64,
    /// Shares accepted by pool
    pub submissions_accepted: AtomicU64,
    /// Shares rejected by pool
    pub submissions_rejected: AtomicU64,
    /// Duplicate share submissions
    pub duplicate_submissions: AtomicU64,

    // === PERFORMANCE METRICS ===
    /// Average response time in milliseconds (stored as f64 bits)
    pub avg_response_time_ms_bits: AtomicU64,
    /// Maximum response time in milliseconds (stored as f64 bits)
    pub max_response_time_ms_bits: AtomicU64,
    /// Minimum response time in milliseconds (stored as f64 bits)
    pub min_response_time_ms_bits: AtomicU64,

    // === SECURITY METRICS ===
    /// Security policy violations detected
    pub security_violations: AtomicU64,
    /// Rate limit violations
    pub rate_limit_hits: AtomicU64,
    /// Number of blocked IP addresses
    pub blocked_ips: AtomicU64,

    // === RESOURCE METRICS ===
    /// Current memory usage in bytes
    pub memory_usage_bytes: AtomicU64,
    /// CPU usage percentage (stored as f64 bits)
    pub cpu_usage_percent_bits: AtomicU64,
    /// Number of active goroutines/tasks
    pub goroutine_count: AtomicU64,

    // === HASHRATE CALCULATION METRICS ===
    /// Estimated network hashrate (stored as f64 bits)
    pub hashrate_estimate_bits: AtomicU64,
    /// Hashrate calculation window start timestamp (stored as f64 bits)
    pub hashrate_window_start_timestamp_bits: AtomicU64,
    /// Share count at window start for hashrate calculation
    pub hashrate_window_start_shares: AtomicU64,
    /// Current difficulty for hashrate calculation (stored as f64 bits) 
    pub hashrate_current_difficulty_bits: AtomicU64,

    // === BUSINESS METRICS ===
    /// Number of difficulty adjustments
    pub difficulty_adjustments: AtomicU64,
    /// Number of pool switches
    pub pool_switches: AtomicU64,

    // === CONNECTION LIFECYCLE METRICS ===
    /// Sum of all connection durations in ms (stored as f64 bits)
    pub connection_duration_sum_ms_bits: AtomicU64,
    /// Number of connection duration samples
    pub connection_duration_count: AtomicU64,
    /// Maximum connection duration in ms (stored as f64 bits)
    pub connection_duration_max_ms_bits: AtomicU64,
    /// Minimum connection duration in ms (stored as f64 bits)
    pub connection_duration_min_ms_bits: AtomicU64,
    /// Current idle time gauge in ms (stored as f64 bits)
    pub idle_time_gauge_ms_bits: AtomicU64,
    /// Total reconnection attempts
    pub reconnection_attempts_counter: AtomicU64,
    /// Connections successfully established
    pub connection_established_counter: AtomicU64,
    /// Connections closed
    pub connection_closed_counter: AtomicU64,

    // === PROTOCOL-SPECIFIC DETECTION METRICS ===
    /// Total HTTP requests received
    pub http_requests_total: AtomicU64,
    /// Total Stratum requests received
    pub stratum_requests_total: AtomicU64,
    /// Protocol detection failures
    pub protocol_detection_failures: AtomicU64,
    /// Protocol detection successes
    pub protocol_detection_successes: AtomicU64,
    /// Protocol conversion success rate sum (stored as f64 bits)
    pub protocol_conversion_success_rate_sum_bits: AtomicU64,
    /// Protocol conversion success count
    pub protocol_conversion_success_count: AtomicU64,
    /// Protocol conversion errors
    pub protocol_conversion_errors: AtomicU64,
    /// HTTP CONNECT requests
    pub http_connect_requests: AtomicU64,
    /// Direct Stratum connections
    pub direct_stratum_connections: AtomicU64,

    // === MINING OPERATION PERFORMANCE METRICS ===
    /// Total share submissions
    pub share_submissions_total: AtomicU64,
    /// Share acceptance rate sum (stored as f64 bits)
    pub share_acceptance_rate_sum_bits: AtomicU64,
    /// Share acceptance rate sample count
    pub share_acceptance_rate_count: AtomicU64,
    /// Difficulty adjustments counter
    pub difficulty_adjustments_counter: AtomicU64,
    /// Job distribution latency sum in ms (stored as f64 bits)
    pub job_distribution_latency_sum_ms_bits: AtomicU64,
    /// Job distribution latency sample count
    pub job_distribution_latency_count: AtomicU64,
    /// Maximum job distribution latency in ms (stored as f64 bits)
    pub job_distribution_latency_max_ms_bits: AtomicU64,
    /// Minimum job distribution latency in ms (stored as f64 bits)
    pub job_distribution_latency_min_ms_bits: AtomicU64,
    /// Current mining difficulty (stored as f64 bits)
    pub current_difficulty_bits: AtomicU64,
    /// Shares submitted per minute gauge
    pub shares_per_minute_gauge: AtomicU64,
    /// Stale shares counter
    pub stale_shares_counter: AtomicU64,
    /// Duplicate shares counter
    pub duplicate_shares_counter: AtomicU64,

    // === ERROR CATEGORIZATION METRICS ===
    /// Network-related errors
    pub network_errors_total: AtomicU64,
    /// Authentication failures
    pub authentication_failures_total: AtomicU64,
    /// Timeout errors
    pub timeout_errors_total: AtomicU64,
    /// Protocol parsing errors
    pub protocol_parse_errors_total: AtomicU64,
    /// Protocol version mismatch errors
    pub protocol_version_errors_total: AtomicU64,
    /// Protocol message errors
    pub protocol_message_errors_total: AtomicU64,
    /// Validation errors
    pub validation_errors_total: AtomicU64,
    /// Security violation errors
    pub security_violation_errors_total: AtomicU64,
    /// Resource exhaustion errors
    pub resource_exhaustion_errors_total: AtomicU64,
    /// Internal server errors
    pub internal_errors_total: AtomicU64,

    // === RESOURCE UTILIZATION TRACKING METRICS ===
    /// Memory usage per connection sum (stored as f64 bits)
    pub memory_usage_per_connection_sum_bits: AtomicU64,
    /// Memory usage per connection sample count
    pub memory_usage_per_connection_count: AtomicU64,
    /// Maximum memory usage per connection (stored as f64 bits)
    pub memory_usage_per_connection_max_bits: AtomicU64,
    /// Minimum memory usage per connection (stored as f64 bits)
    pub memory_usage_per_connection_min_bits: AtomicU64,
    /// CPU utilization sample (stored as f64 bits)
    pub cpu_utilization_sample_bits: AtomicU64,
    /// CPU sample count
    pub cpu_sample_count: AtomicU64,
    /// CPU sample timestamp (stored as f64 bits)
    pub cpu_sample_timestamp_bits: AtomicU64,
    /// Network bandwidth receive bytes per second
    pub network_bandwidth_rx_bytes_per_sec: AtomicU64,
    /// Network bandwidth transmit bytes per second
    pub network_bandwidth_tx_bytes_per_sec: AtomicU64,
    /// Total connection memory in bytes
    pub connection_memory_total_bytes: AtomicU64,
    /// Peak connection memory in bytes
    pub connection_memory_peak_bytes: AtomicU64,
    /// Resource pressure events
    pub resource_pressure_events: AtomicU64,
    /// Memory efficiency ratio (stored as f64 bits)
    pub memory_efficiency_ratio_bits: AtomicU64,
}

impl AtomicMetrics {
    /// Creates a new AtomicMetrics instance with all counters initialized to zero.
    ///
    /// All numeric metrics start at 0, float-based metrics use appropriate defaults,
    /// and minimum tracking metrics start at f64::MAX for proper comparison.
    pub fn new() -> Self {
        Self {
            // Connection metrics
            total_connections: AtomicU64::new(0),
            active_connections: AtomicU64::new(0),
            connection_errors: AtomicU64::new(0),
            connection_timeouts: AtomicU64::new(0),

            // Protocol metrics
            messages_received: AtomicU64::new(0),
            messages_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
            protocol_errors: AtomicU64::new(0),

            // Authentication metrics
            auth_attempts: AtomicU64::new(0),
            auth_successes: AtomicU64::new(0),
            auth_failures: AtomicU64::new(0),

            // Job processing metrics
            jobs_received: AtomicU64::new(0),
            jobs_processed: AtomicU64::new(0),
            job_processing_errors: AtomicU64::new(0),

            // Share submission metrics
            submissions_received: AtomicU64::new(0),
            submissions_accepted: AtomicU64::new(0),
            submissions_rejected: AtomicU64::new(0),
            duplicate_submissions: AtomicU64::new(0),

            // Performance timing metrics
            avg_response_time_ms_bits: AtomicU64::new((0.0_f64).to_bits()),
            max_response_time_ms_bits: AtomicU64::new((0.0_f64).to_bits()),
            min_response_time_ms_bits: AtomicU64::new(f64::MAX.to_bits()),

            // Security metrics
            security_violations: AtomicU64::new(0),
            rate_limit_hits: AtomicU64::new(0),
            blocked_ips: AtomicU64::new(0),

            // System resource metrics
            memory_usage_bytes: AtomicU64::new(0),
            cpu_usage_percent_bits: AtomicU64::new((0.0_f64).to_bits()),
            goroutine_count: AtomicU64::new(0),

            // Hashrate calculation metrics
            hashrate_estimate_bits: AtomicU64::new((0.0_f64).to_bits()),
            hashrate_window_start_timestamp_bits: AtomicU64::new((0.0_f64).to_bits()),
            hashrate_window_start_shares: AtomicU64::new(0),
            hashrate_current_difficulty_bits: AtomicU64::new((1.0_f64).to_bits()),

            // Business metrics
            difficulty_adjustments: AtomicU64::new(0),
            pool_switches: AtomicU64::new(0),

            // Connection lifecycle metrics
            connection_duration_sum_ms_bits: AtomicU64::new((0.0_f64).to_bits()),
            connection_duration_count: AtomicU64::new(0),
            connection_duration_max_ms_bits: AtomicU64::new((0.0_f64).to_bits()),
            connection_duration_min_ms_bits: AtomicU64::new(f64::MAX.to_bits()),
            idle_time_gauge_ms_bits: AtomicU64::new((0.0_f64).to_bits()),
            reconnection_attempts_counter: AtomicU64::new(0),
            connection_established_counter: AtomicU64::new(0),
            connection_closed_counter: AtomicU64::new(0),

            // Protocol detection metrics
            http_requests_total: AtomicU64::new(0),
            stratum_requests_total: AtomicU64::new(0),
            protocol_detection_failures: AtomicU64::new(0),
            protocol_detection_successes: AtomicU64::new(0),
            protocol_conversion_success_rate_sum_bits: AtomicU64::new((0.0_f64).to_bits()),
            protocol_conversion_success_count: AtomicU64::new(0),
            protocol_conversion_errors: AtomicU64::new(0),
            http_connect_requests: AtomicU64::new(0),
            direct_stratum_connections: AtomicU64::new(0),

            // Mining operation metrics
            share_submissions_total: AtomicU64::new(0),
            share_acceptance_rate_sum_bits: AtomicU64::new((0.0_f64).to_bits()),
            share_acceptance_rate_count: AtomicU64::new(0),
            difficulty_adjustments_counter: AtomicU64::new(0),
            job_distribution_latency_sum_ms_bits: AtomicU64::new((0.0_f64).to_bits()),
            job_distribution_latency_count: AtomicU64::new(0),
            job_distribution_latency_max_ms_bits: AtomicU64::new((0.0_f64).to_bits()),
            job_distribution_latency_min_ms_bits: AtomicU64::new(f64::MAX.to_bits()),
            current_difficulty_bits: AtomicU64::new((0.0_f64).to_bits()),
            shares_per_minute_gauge: AtomicU64::new(0),
            stale_shares_counter: AtomicU64::new(0),
            duplicate_shares_counter: AtomicU64::new(0),

            // Error categorization metrics
            network_errors_total: AtomicU64::new(0),
            authentication_failures_total: AtomicU64::new(0),
            timeout_errors_total: AtomicU64::new(0),
            protocol_parse_errors_total: AtomicU64::new(0),
            protocol_version_errors_total: AtomicU64::new(0),
            protocol_message_errors_total: AtomicU64::new(0),
            validation_errors_total: AtomicU64::new(0),
            security_violation_errors_total: AtomicU64::new(0),
            resource_exhaustion_errors_total: AtomicU64::new(0),
            internal_errors_total: AtomicU64::new(0),

            // Resource utilization metrics
            memory_usage_per_connection_sum_bits: AtomicU64::new((0.0_f64).to_bits()),
            memory_usage_per_connection_count: AtomicU64::new(0),
            memory_usage_per_connection_max_bits: AtomicU64::new((0.0_f64).to_bits()),
            memory_usage_per_connection_min_bits: AtomicU64::new(f64::MAX.to_bits()),
            cpu_utilization_sample_bits: AtomicU64::new((0.0_f64).to_bits()),
            cpu_sample_count: AtomicU64::new(0),
            cpu_sample_timestamp_bits: AtomicU64::new((0.0_f64).to_bits()),
            network_bandwidth_rx_bytes_per_sec: AtomicU64::new(0),
            network_bandwidth_tx_bytes_per_sec: AtomicU64::new(0),
            connection_memory_total_bytes: AtomicU64::new(0),
            connection_memory_peak_bytes: AtomicU64::new(0),
            resource_pressure_events: AtomicU64::new(0),
            memory_efficiency_ratio_bits: AtomicU64::new((1.0_f64).to_bits()),
        }
    }

    // === HELPER METHODS FOR ATOMIC FLOAT OPERATIONS ===

    /// Store a f64 value atomically by converting to bits
    #[inline]
    pub fn store_f64(&self, atomic: &AtomicU64, value: f64) {
        atomic.store(value.to_bits(), Ordering::Relaxed);
    }

    /// Load a f64 value atomically by converting from bits
    #[inline]
    pub fn load_f64(&self, atomic: &AtomicU64) -> f64 {
        f64::from_bits(atomic.load(Ordering::Relaxed))
    }

    /// Update minimum f64 value atomically
    #[inline]
    pub fn update_min_f64(&self, atomic: &AtomicU64, value: f64) {
        let mut current = self.load_f64(atomic);
        while value < current {
            match atomic.compare_exchange_weak(
                current.to_bits(),
                value.to_bits(),
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => current = f64::from_bits(x),
            }
        }
    }

    /// Update maximum f64 value atomically
    #[inline]
    pub fn update_max_f64(&self, atomic: &AtomicU64, value: f64) {
        let mut current = self.load_f64(atomic);
        while value > current {
            match atomic.compare_exchange_weak(
                current.to_bits(),
                value.to_bits(),
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => current = f64::from_bits(x),
            }
        }
    }

    /// Add to a f64 sum atomically
    #[inline]
    pub fn add_f64(&self, atomic: &AtomicU64, value: f64) {
        let mut current = self.load_f64(atomic);
        loop {
            let new = current + value;
            match atomic.compare_exchange_weak(
                current.to_bits(),
                new.to_bits(),
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => current = f64::from_bits(x),
            }
        }
    }

    // === CONVENIENCE INCREMENT METHODS ===

    /// Increment connection counter
    #[inline]
    pub fn increment_connection(&self) {
        self.total_connections.fetch_add(1, Ordering::Relaxed);
        self.active_connections.fetch_add(1, Ordering::Relaxed);
        self.connection_established_counter.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement connection counter
    #[inline]
    pub fn decrement_connection(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
        self.connection_closed_counter.fetch_add(1, Ordering::Relaxed);
    }

    /// Record bytes received
    #[inline]
    pub fn record_bytes_received(&self, bytes: u64) {
        self.bytes_received.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record bytes sent
    #[inline]
    pub fn record_bytes_sent(&self, bytes: u64) {
        self.bytes_sent.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record messages received
    #[inline]
    pub fn record_messages_received(&self, count: u64) {
        self.messages_received.fetch_add(count, Ordering::Relaxed);
    }

    /// Record messages sent
    #[inline]
    pub fn record_messages_sent(&self, count: u64) {
        self.messages_sent.fetch_add(count, Ordering::Relaxed);
    }

    /// Record submission received
    #[inline]
    pub fn record_submission_received(&self) {
        self.submissions_received.fetch_add(1, Ordering::Relaxed);
    }

    /// Record job received
    #[inline]
    pub fn record_job_received(&self) {
        self.jobs_received.fetch_add(1, Ordering::Relaxed);
    }

    /// Record job processed
    #[inline]
    pub fn record_job_processed(&self) {
        self.jobs_processed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record submission accepted
    #[inline]
    pub fn record_submission_accepted(&self) {
        self.submissions_accepted.fetch_add(1, Ordering::Relaxed);
        self.share_submissions_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record submission rejected
    #[inline]
    pub fn record_submission_rejected(&self) {
        self.submissions_rejected.fetch_add(1, Ordering::Relaxed);
        self.share_submissions_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record authentication attempt
    #[inline]
    pub fn record_auth_attempt(&self, success: bool) {
        self.auth_attempts.fetch_add(1, Ordering::Relaxed);
        if success {
            self.auth_successes.fetch_add(1, Ordering::Relaxed);
        } else {
            self.auth_failures.fetch_add(1, Ordering::Relaxed);
            self.authentication_failures_total.fetch_add(1, Ordering::Relaxed);
        }
    }


    /// Record successful authentication
    #[inline]
    pub fn record_auth_success(&self) {
        self.auth_successes.fetch_add(1, Ordering::Relaxed);
    }

    /// Record failed authentication
    #[inline]
    pub fn record_auth_failure(&self) {
        self.auth_failures.fetch_add(1, Ordering::Relaxed);
        self.authentication_failures_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record protocol error by type
    #[inline]
    pub fn record_protocol_error(&self, error_type: &str) {
        self.protocol_errors.fetch_add(1, Ordering::Relaxed);
        match error_type {
            "parse" => self.protocol_parse_errors_total.fetch_add(1, Ordering::Relaxed),
            "version" => self.protocol_version_errors_total.fetch_add(1, Ordering::Relaxed),
            "message" => self.protocol_message_errors_total.fetch_add(1, Ordering::Relaxed),
            _ => self.protocol_errors.fetch_add(1, Ordering::Relaxed),
        };
    }

    /// Record protocol error (simple version)
    #[inline]
    pub fn record_protocol_error_simple(&self) {
        self.protocol_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Record connection error
    #[inline]
    pub fn record_connection_error(&self) {
        self.connection_errors.fetch_add(1, Ordering::Relaxed);
        self.network_errors_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record connection timeout
    #[inline]
    pub fn record_connection_timeout(&self) {
        self.connection_timeouts.fetch_add(1, Ordering::Relaxed);
        self.timeout_errors_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record security violation
    #[inline]
    pub fn record_security_violation(&self) {
        self.security_violations.fetch_add(1, Ordering::Relaxed);
        self.security_violation_errors_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record rate limit hit
    #[inline]
    pub fn record_rate_limit_hit(&self) {
        self.rate_limit_hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Record network error
    #[inline]
    pub fn record_network_error(&self) {
        self.connection_errors.fetch_add(1, Ordering::Relaxed);
        self.network_errors_total.fetch_add(1, Ordering::Relaxed);
    }

    // Job methods are defined earlier in the file

    /// Update active connections count
    #[inline]
    pub fn update_active_connections(&self, delta: i64) {
        if delta > 0 {
            self.active_connections.fetch_add(delta as u64, Ordering::Relaxed);
        } else {
            self.active_connections.fetch_sub((-delta) as u64, Ordering::Relaxed);
        }
    }

    /// Record connection duration
    #[inline]
    pub fn record_connection_duration(&self, duration_ms: f64) {
        self.add_f64(&self.connection_duration_sum_ms_bits, duration_ms);
        self.connection_duration_count.fetch_add(1, Ordering::Relaxed);
        self.update_max_f64(&self.connection_duration_max_ms_bits, duration_ms);
        self.update_min_f64(&self.connection_duration_min_ms_bits, duration_ms);
    }

    /// Record job distribution latency
    #[inline]
    pub fn record_job_distribution_latency(&self, latency_ms: f64) {
        self.add_f64(&self.job_distribution_latency_sum_ms_bits, latency_ms);
        self.job_distribution_latency_count.fetch_add(1, Ordering::Relaxed);
        self.update_max_f64(&self.job_distribution_latency_max_ms_bits, latency_ms);
        self.update_min_f64(&self.job_distribution_latency_min_ms_bits, latency_ms);
    }

    /// Update current difficulty
    #[inline]
    pub fn update_difficulty(&self, difficulty: f64) {
        self.store_f64(&self.current_difficulty_bits, difficulty);
        self.update_hashrate_difficulty(difficulty); // Also update hashrate calculation difficulty
        self.difficulty_adjustments.fetch_add(1, Ordering::Relaxed);
        self.difficulty_adjustments_counter.fetch_add(1, Ordering::Relaxed);
    }

    /// Record share acceptance rate sample
    #[inline]
    pub fn record_share_acceptance_rate(&self, rate: f64) {
        self.add_f64(&self.share_acceptance_rate_sum_bits, rate);
        self.share_acceptance_rate_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Update resource utilization metrics
    #[inline]
    pub fn update_resource_utilization(&self, memory_mb: f64, cpu_percent: f64) {
        let memory_bytes = (memory_mb * 1024.0 * 1024.0) as u64;
        self.memory_usage_bytes.store(memory_bytes, Ordering::Relaxed);
        self.store_f64(&self.cpu_usage_percent_bits, cpu_percent);
        self.store_f64(&self.cpu_utilization_sample_bits, cpu_percent);
        self.cpu_sample_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Record memory usage per connection
    #[inline]
    pub fn record_memory_per_connection(&self, memory_mb: f64) {
        self.add_f64(&self.memory_usage_per_connection_sum_bits, memory_mb);
        self.memory_usage_per_connection_count.fetch_add(1, Ordering::Relaxed);
        self.update_max_f64(&self.memory_usage_per_connection_max_bits, memory_mb);
        self.update_min_f64(&self.memory_usage_per_connection_min_bits, memory_mb);
    }

    /// Update bandwidth metrics
    #[inline]
    pub fn update_bandwidth(&self, rx_bytes_per_sec: u64, tx_bytes_per_sec: u64) {
        self.network_bandwidth_rx_bytes_per_sec.store(rx_bytes_per_sec, Ordering::Relaxed);
        self.network_bandwidth_tx_bytes_per_sec.store(tx_bytes_per_sec, Ordering::Relaxed);
    }

    /// Update response time metrics with a new measurement.
    #[inline]
    pub fn update_response_time(&self, time_ms: f64) {
        self.update_min_f64(&self.min_response_time_ms_bits, time_ms);
        self.update_max_f64(&self.max_response_time_ms_bits, time_ms);
        // For average, we could track sum and count, but for now just store the latest
        self.store_f64(&self.avg_response_time_ms_bits, time_ms);
    }

    /// Record protocol conversion success with rate.
    #[inline]
    pub fn record_protocol_conversion_success(&self, success_rate: f64) {
        self.protocol_detection_successes.fetch_add(1, Ordering::Relaxed);
        self.store_f64(&self.protocol_conversion_success_rate_sum_bits, success_rate);
    }

    /// Record stale share.
    #[inline]
    pub fn record_stale_share(&self) {
        self.stale_shares_counter.fetch_add(1, Ordering::Relaxed);
    }

    /// Record duplicate share.
    #[inline]
    pub fn record_duplicate_share(&self) {
        self.duplicate_shares_counter.fetch_add(1, Ordering::Relaxed);
    }

    /// Update shares per minute.
    #[inline]
    pub fn update_shares_per_minute(&self, count: u64) {
        self.shares_per_minute_gauge.store(count, Ordering::Relaxed);
    }

    /// Record share submission.
    #[inline]
    pub fn record_share_submission(&self) {
        self.share_submissions_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record share acceptance (for legacy compatibility).
    #[inline]
    pub fn record_share_acceptance(&self, accepted: bool) {
        if accepted {
            self.record_submission_accepted();
        } else {
            self.record_submission_rejected();
        }
        let rate = if accepted { 1.0 } else { 0.0 };
        self.record_share_acceptance_rate(rate);
    }

    /// Record difficulty adjustment (legacy name).
    #[inline]
    pub fn record_difficulty_adjustment(&self, difficulty: f64) {
        self.update_difficulty(difficulty);
    }

    /// Get average share acceptance rate.
    #[inline]
    pub fn get_avg_share_acceptance_rate(&self) -> f64 {
        let sum = self.load_f64(&self.share_acceptance_rate_sum_bits);
        let count = self.share_acceptance_rate_count.load(Ordering::Relaxed);
        if count > 0 { sum / count as f64 } else { 0.0 }
    }

    /// Get current difficulty.
    #[inline]
    pub fn get_current_difficulty(&self) -> f64 {
        self.load_f64(&self.current_difficulty_bits)
    }

    /// Get average job distribution latency in milliseconds.
    #[inline]
    pub fn get_avg_job_distribution_latency_ms(&self) -> f64 {
        let sum = self.load_f64(&self.job_distribution_latency_sum_ms_bits);
        let count = self.job_distribution_latency_count.load(Ordering::Relaxed);
        if count > 0 { sum / count as f64 } else { 0.0 }
    }

    /// Calculate and update hashrate estimate using Bitcoin standard formula.
    /// Should be called periodically (e.g., every 60 seconds) to update hashrate.
    #[inline]
    pub fn calculate_and_update_hashrate(&self, time_window_seconds: f64) {
        use crate::utils::hash_utils::HashUtils;
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let current_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
            
        let window_start = self.load_f64(&self.hashrate_window_start_timestamp_bits);
        let current_shares = self.share_submissions_total.load(Ordering::Relaxed);
        let window_start_shares = self.hashrate_window_start_shares.load(Ordering::Relaxed);
        let difficulty = self.load_f64(&self.hashrate_current_difficulty_bits);
        
        // Initialize window if not set or if enough time has passed
        if window_start == 0.0 || (current_timestamp - window_start) >= time_window_seconds {
            // Calculate hashrate for the completed window
            if window_start > 0.0 && current_shares > window_start_shares {
                let shares_in_window = current_shares - window_start_shares;
                let actual_time_window = current_timestamp - window_start;
                
                if actual_time_window > 0.0 {
                    let hashrate = HashUtils::calculate_hashrate_bitcoin_standard(
                        shares_in_window,
                        actual_time_window as u64,
                        difficulty
                    );
                    
                    self.store_f64(&self.hashrate_estimate_bits, hashrate);
                }
            }
            
            // Reset window
            self.store_f64(&self.hashrate_window_start_timestamp_bits, current_timestamp);
            self.hashrate_window_start_shares.store(current_shares, Ordering::Relaxed);
        }
    }
    
    /// Update difficulty for hashrate calculations
    #[inline]
    pub fn update_hashrate_difficulty(&self, difficulty: f64) {
        self.store_f64(&self.hashrate_current_difficulty_bits, difficulty);
    }
    
    /// Get current hashrate estimate
    #[inline]
    pub fn get_hashrate_estimate(&self) -> f64 {
        self.load_f64(&self.hashrate_estimate_bits)
    }

    /// Create a snapshot of all current metrics.
    ///
    /// This captures the current state of all atomic metrics and returns
    /// a `MetricsSnapshot` that can be used for analysis and reporting.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use loka_stratum::services::metrics::AtomicMetrics;
    /// use std::sync::Arc;
    ///
    /// let metrics = Arc::new(AtomicMetrics::new());
    /// let snapshot = metrics.snapshot();
    /// println!("Active connections: {}", snapshot.active_connections);
    /// ```
    pub fn snapshot(&self) -> super::MetricsSnapshot {
        super::MetricsSnapshot::from_atomic(self)
    }
}

impl Default for AtomicMetrics {
    fn default() -> Self {
        Self::new()
    }
}