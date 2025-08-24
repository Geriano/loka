//! Calculated metrics derived from raw atomic metrics.
//!
//! This module provides derived and calculated metrics that are computed from
//! the raw atomic metrics to provide higher-level insights into system performance.

use serde::{Deserialize, Serialize};

/// Calculated metrics derived from raw atomic metrics.
///
/// These metrics are computed on-demand from the raw atomic counters
/// to provide higher-level insights into system performance, health,
/// and efficiency metrics.
///
/// # Examples
///
/// ```rust
/// use loka_stratum::services::metrics::{CalculatedMetrics, MetricsSnapshot};
///
/// let snapshot = MetricsSnapshot::default();
/// let calculated = snapshot.calculated_metrics();
///
/// println!("Connection success rate: {:.2}%", calculated.connection_success_rate * 100.0);
/// println!("Auth success rate: {:.2}%", calculated.auth_success_rate * 100.0);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalculatedMetrics {
    /// Connection success rate (0.0 to 1.0)
    pub connection_success_rate: f64,
    /// Authentication success rate (0.0 to 1.0)
    pub auth_success_rate: f64,
    /// Share acceptance rate (0.0 to 1.0)
    pub share_acceptance_rate: f64,
    /// Protocol error rate (0.0 to 1.0)
    pub protocol_error_rate: f64,
    /// Messages per second throughput
    pub messages_per_second: f64,
    /// Bytes per second throughput
    pub bytes_per_second: f64,
    /// Average bytes per message
    pub avg_bytes_per_message: f64,
    /// Job processing efficiency (0.0 to 1.0)
    pub job_processing_efficiency: f64,
    /// Security violation rate per connection
    pub security_violations_per_connection: f64,
    /// Memory usage per connection in MB
    pub memory_usage_per_connection: f64,
    /// Response time health score (0.0 to 1.0, higher is better)
    pub response_time_health_score: f64,
    /// Overall system efficiency score (0.0 to 1.0)
    pub system_efficiency_score: f64,

    // Connection lifecycle efficiency metrics
    /// Average connection uptime efficiency (0.0 to 1.0)
    pub connection_uptime_efficiency: f64,
    /// Reconnection rate per active connection
    pub reconnection_rate: f64,

    // Protocol detection efficiency
    /// Protocol detection accuracy (0.0 to 1.0)
    pub protocol_detection_accuracy: f64,
    /// HTTP vs Stratum traffic ratio
    pub http_stratum_traffic_ratio: f64,

    // Mining operation efficiency
    /// Shares per minute per connection
    pub shares_per_minute_per_connection: f64,
    /// Stale share percentage (0.0 to 1.0)
    pub stale_share_percentage: f64,
    /// Job distribution efficiency (0.0 to 1.0)
    pub job_distribution_efficiency: f64,

    // Resource utilization efficiency
    /// Memory utilization efficiency (current/peak ratio)
    pub memory_utilization_efficiency: f64,
    /// Network bandwidth total in Mbps
    pub network_bandwidth_total_mbps: f64,
    /// CPU health score (inverse of utilization)
    pub cpu_utilization_health_score: f64,
    /// Resource pressure rate (events per connection)
    pub resource_pressure_rate: f64,
    /// Memory usage per connection variance
    pub memory_usage_per_connection_variance: f64,
}

impl CalculatedMetrics {
    /// Calculate derived metrics from a snapshot.
    ///
    /// This computes rates, ratios, and efficiency scores from raw metric values.
    ///
    /// # Arguments
    ///
    /// * `snapshot` - The metrics snapshot to calculate from
    ///
    /// # Examples
    ///
    /// ```rust
    /// use loka_stratum::services::metrics::{CalculatedMetrics, MetricsSnapshot};
    ///
    /// let snapshot = MetricsSnapshot::default();
    /// let calculated = CalculatedMetrics::from_snapshot(&snapshot);
    /// ```
    pub fn from_snapshot(snapshot: &super::MetricsSnapshot) -> Self {
        // Helper function for safe division
        let safe_divide = |numerator: f64, denominator: f64| -> f64 {
            if denominator == 0.0 {
                0.0
            } else {
                numerator / denominator
            }
        };

        // Connection success rate
        let connection_success_rate = safe_divide(
            (snapshot.total_connections - snapshot.connection_errors) as f64,
            snapshot.total_connections as f64,
        );

        // Authentication success rate
        let auth_success_rate = safe_divide(
            snapshot.auth_successes as f64,
            snapshot.auth_attempts as f64,
        );

        // Share acceptance rate
        let share_acceptance_rate = safe_divide(
            snapshot.submissions_accepted as f64,
            snapshot.submissions_received as f64,
        );

        // Protocol error rate
        let protocol_error_rate = safe_divide(
            snapshot.protocol_errors as f64,
            snapshot.messages_received as f64,
        );

        // Throughput calculations (would need time duration for accurate per-second rates)
        let total_messages = snapshot.messages_received + snapshot.messages_sent;
        let total_bytes = snapshot.bytes_received + snapshot.bytes_sent;
        let avg_bytes_per_message = safe_divide(total_bytes as f64, total_messages as f64);

        // Job processing efficiency
        let job_processing_efficiency = safe_divide(
            snapshot.jobs_processed as f64,
            snapshot.jobs_received as f64,
        );

        // Security violations per connection
        let security_violations_per_connection = safe_divide(
            snapshot.security_violations as f64,
            snapshot.total_connections as f64,
        );

        // Memory usage per connection (convert bytes to MB)
        let memory_usage_per_connection = if snapshot.active_connections > 0 {
            (snapshot.memory_usage_bytes as f64)
                / (1024.0 * 1024.0)
                / (snapshot.active_connections as f64)
        } else {
            0.0
        };

        // Response time health score (inverse of response time, normalized)
        let response_time_health_score = if snapshot.avg_response_time_ms > 0.0 {
            (1000.0 / (snapshot.avg_response_time_ms + 1.0)).min(1.0)
        } else {
            1.0
        };

        // Connection uptime efficiency
        let connection_uptime_efficiency = safe_divide(
            snapshot.connections_established as f64,
            (snapshot.connections_established + snapshot.reconnection_attempts) as f64,
        );

        // Reconnection rate
        let reconnection_rate = safe_divide(
            snapshot.reconnection_attempts as f64,
            snapshot.active_connections as f64,
        );

        // Protocol detection accuracy
        let protocol_detection_accuracy = safe_divide(
            snapshot.protocol_detection_successes as f64,
            (snapshot.protocol_detection_successes + snapshot.protocol_detection_failures) as f64,
        );

        // HTTP vs Stratum traffic ratio
        let http_stratum_traffic_ratio = safe_divide(
            snapshot.http_requests as f64,
            (snapshot.http_requests + snapshot.stratum_requests) as f64,
        );

        // Shares per minute per connection
        let shares_per_minute_per_connection = safe_divide(
            snapshot.shares_per_minute as f64,
            snapshot.active_connections as f64,
        );

        // Stale share percentage
        let stale_share_percentage = safe_divide(
            snapshot.stale_shares as f64,
            snapshot.share_submissions as f64,
        );

        // Job distribution efficiency (based on latency)
        let job_distribution_efficiency = if snapshot.avg_job_distribution_latency_ms > 0.0 {
            (1000.0 / (snapshot.avg_job_distribution_latency_ms + 1.0)).min(1.0)
        } else {
            1.0
        };

        // Memory utilization efficiency
        let memory_utilization_efficiency = if snapshot.connection_memory_peak > 0 {
            snapshot.connection_memory_total as f64 / snapshot.connection_memory_peak as f64
        } else {
            1.0
        };

        // Network bandwidth total in Mbps
        let network_bandwidth_total_mbps =
            ((snapshot.network_bandwidth_rx_bps + snapshot.network_bandwidth_tx_bps) as f64 * 8.0)
                / (1024.0 * 1024.0);

        // CPU health score (inverse of utilization)
        let cpu_utilization_health_score = (100.0 - snapshot.cpu_utilization) / 100.0;

        // Resource pressure rate
        let resource_pressure_rate = safe_divide(
            snapshot.resource_pressure_events as f64,
            snapshot.total_connections as f64,
        );

        // Memory usage variance per connection
        let memory_usage_per_connection_variance =
            snapshot.max_memory_per_connection_mb - snapshot.min_memory_per_connection_mb;

        // Overall system efficiency score (weighted average of key metrics)
        let system_efficiency_score = (connection_success_rate * 0.2
            + auth_success_rate * 0.15
            + share_acceptance_rate * 0.25
            + job_processing_efficiency * 0.2
            + response_time_health_score * 0.1
            + cpu_utilization_health_score * 0.1)
            .min(1.0);

        Self {
            connection_success_rate,
            auth_success_rate,
            share_acceptance_rate,
            protocol_error_rate,
            messages_per_second: 0.0, // Would need time duration
            bytes_per_second: 0.0,    // Would need time duration
            avg_bytes_per_message,
            job_processing_efficiency,
            security_violations_per_connection,
            memory_usage_per_connection,
            response_time_health_score,
            system_efficiency_score,
            connection_uptime_efficiency,
            reconnection_rate,
            protocol_detection_accuracy,
            http_stratum_traffic_ratio,
            shares_per_minute_per_connection,
            stale_share_percentage,
            job_distribution_efficiency,
            memory_utilization_efficiency,
            network_bandwidth_total_mbps,
            cpu_utilization_health_score,
            resource_pressure_rate,
            memory_usage_per_connection_variance,
        }
    }
}

impl Default for CalculatedMetrics {
    fn default() -> Self {
        Self {
            connection_success_rate: 0.0,
            auth_success_rate: 0.0,
            share_acceptance_rate: 0.0,
            protocol_error_rate: 0.0,
            messages_per_second: 0.0,
            bytes_per_second: 0.0,
            avg_bytes_per_message: 0.0,
            job_processing_efficiency: 0.0,
            security_violations_per_connection: 0.0,
            memory_usage_per_connection: 0.0,
            response_time_health_score: 1.0,
            system_efficiency_score: 0.0,
            connection_uptime_efficiency: 0.0,
            reconnection_rate: 0.0,
            protocol_detection_accuracy: 0.0,
            http_stratum_traffic_ratio: 0.0,
            shares_per_minute_per_connection: 0.0,
            stale_share_percentage: 0.0,
            job_distribution_efficiency: 0.0,
            memory_utilization_efficiency: 0.0,
            network_bandwidth_total_mbps: 0.0,
            cpu_utilization_health_score: 1.0,
            resource_pressure_rate: 0.0,
            memory_usage_per_connection_variance: 0.0,
        }
    }
}
