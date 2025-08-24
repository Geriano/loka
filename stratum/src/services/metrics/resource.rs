//! Resource utilization tracking and system metrics.
//!
//! This module provides system resource monitoring including CPU, memory,
//! network, and other system-level performance metrics.

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Resource utilization summary with system-level metrics.
///
/// Contains current and peak resource utilization measurements for
/// monitoring system health and performance characteristics.
///
/// # Examples
///
/// ```rust
/// use loka_stratum::services::metrics::ResourceUtilizationSummary;
///
/// let mut summary = ResourceUtilizationSummary::default();
/// summary.update_memory(512.5, 1024.0);
/// summary.update_cpu(25.3);
///
/// println!("Memory efficiency: {:.2}", summary.memory_efficiency_ratio());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUtilizationSummary {
    // Memory Metrics
    /// Current memory usage in MB
    pub current_memory_mb: f64,
    /// Peak memory usage in MB since start
    pub peak_memory_mb: f64,
    /// Available system memory in MB
    pub available_memory_mb: f64,
    /// Memory used specifically for connection handling in bytes
    pub connection_memory_bytes: u64,

    // CPU Metrics
    /// Current CPU utilization percentage (0-100)
    pub current_cpu_percent: f64,
    /// Peak CPU utilization percentage since start
    pub peak_cpu_percent: f64,
    /// Number of CPU samples taken for averaging
    pub cpu_sample_count: u64,

    // Network Metrics
    /// Current network receive rate in bytes per second
    pub network_rx_bps: u64,
    /// Current network transmit rate in bytes per second
    pub network_tx_bps: u64,
    /// Peak network receive rate in bytes per second
    pub peak_network_rx_bps: u64,
    /// Peak network transmit rate in bytes per second
    pub peak_network_tx_bps: u64,

    // System Load Metrics
    /// System load average (1 minute)
    pub load_avg_1min: f64,
    /// System load average (5 minutes)
    pub load_avg_5min: f64,
    /// System load average (15 minutes)
    pub load_avg_15min: f64,

    // Connection Resource Metrics
    /// Memory per connection in MB
    pub memory_per_connection_mb: f64,
    /// Peak memory per connection in MB
    pub peak_memory_per_connection_mb: f64,

    // Resource Pressure Indicators
    /// Number of resource pressure events detected
    pub resource_pressure_events: u64,
    /// Timestamp of last resource pressure event
    pub last_pressure_event: Option<SystemTime>,

    /// Timestamp of last update
    pub last_updated: SystemTime,
}

impl ResourceUtilizationSummary {
    /// Create a new resource utilization summary.
    pub fn new() -> Self {
        Self::default()
    }

    /// Update memory usage statistics.
    ///
    /// # Arguments
    ///
    /// * `current_mb` - Current memory usage in MB
    /// * `available_mb` - Available system memory in MB
    pub fn update_memory(&mut self, current_mb: f64, available_mb: f64) {
        self.current_memory_mb = current_mb;
        self.available_memory_mb = available_mb;

        if current_mb > self.peak_memory_mb {
            self.peak_memory_mb = current_mb;
        }

        self.last_updated = SystemTime::now();
    }

    /// Update CPU utilization statistics.
    ///
    /// # Arguments
    ///
    /// * `cpu_percent` - Current CPU utilization (0-100)
    pub fn update_cpu(&mut self, cpu_percent: f64) {
        self.current_cpu_percent = cpu_percent;
        self.cpu_sample_count += 1;

        if cpu_percent > self.peak_cpu_percent {
            self.peak_cpu_percent = cpu_percent;
        }

        // Check for resource pressure (high CPU usage)
        if cpu_percent > 90.0 {
            self.record_pressure_event();
        }

        self.last_updated = SystemTime::now();
    }

    /// Update network utilization statistics.
    ///
    /// # Arguments
    ///
    /// * `rx_bps` - Current receive rate in bytes per second
    /// * `tx_bps` - Current transmit rate in bytes per second
    pub fn update_network(&mut self, rx_bps: u64, tx_bps: u64) {
        self.network_rx_bps = rx_bps;
        self.network_tx_bps = tx_bps;

        if rx_bps > self.peak_network_rx_bps {
            self.peak_network_rx_bps = rx_bps;
        }

        if tx_bps > self.peak_network_tx_bps {
            self.peak_network_tx_bps = tx_bps;
        }

        self.last_updated = SystemTime::now();
    }

    /// Update system load averages.
    ///
    /// # Arguments
    ///
    /// * `load_1min` - 1-minute load average
    /// * `load_5min` - 5-minute load average  
    /// * `load_15min` - 15-minute load average
    pub fn update_load_avg(&mut self, load_1min: f64, load_5min: f64, load_15min: f64) {
        self.load_avg_1min = load_1min;
        self.load_avg_5min = load_5min;
        self.load_avg_15min = load_15min;

        // Check for resource pressure (high load)
        if load_1min > 8.0 {
            // Assuming typical server with multiple cores
            self.record_pressure_event();
        }

        self.last_updated = SystemTime::now();
    }

    /// Update per-connection memory statistics.
    ///
    /// # Arguments
    ///
    /// * `connections` - Number of active connections
    pub fn update_connection_memory(&mut self, connections: u64) {
        if connections > 0 {
            self.memory_per_connection_mb = self.current_memory_mb / connections as f64;

            if self.memory_per_connection_mb > self.peak_memory_per_connection_mb {
                self.peak_memory_per_connection_mb = self.memory_per_connection_mb;
            }
        }

        self.last_updated = SystemTime::now();
    }

    /// Calculate memory efficiency ratio (current/peak).
    ///
    /// # Returns
    ///
    /// Efficiency ratio from 0.0 to 1.0 (higher is better)
    pub fn memory_efficiency_ratio(&self) -> f64 {
        if self.peak_memory_mb == 0.0 {
            1.0
        } else {
            self.current_memory_mb / self.peak_memory_mb
        }
    }

    /// Calculate CPU health score (inverse of utilization).
    ///
    /// # Returns
    ///
    /// Health score from 0.0 to 1.0 (higher is better)
    pub fn cpu_health_score(&self) -> f64 {
        1.0 - (self.current_cpu_percent / 100.0).min(1.0)
    }

    /// Calculate total network bandwidth in Mbps.
    pub fn total_network_bandwidth_mbps(&self) -> f64 {
        let total_bps = (self.network_rx_bps + self.network_tx_bps) as f64;
        (total_bps * 8.0) / (1024.0 * 1024.0) // Convert bytes/sec to Mbps
    }

    /// Calculate network utilization ratio.
    ///
    /// # Arguments
    ///
    /// * `max_bandwidth_mbps` - Maximum available bandwidth in Mbps
    ///
    /// # Returns
    ///
    /// Utilization ratio from 0.0 to 1.0
    pub fn network_utilization_ratio(&self, max_bandwidth_mbps: f64) -> f64 {
        let current_mbps = self.total_network_bandwidth_mbps();
        if max_bandwidth_mbps == 0.0 {
            0.0
        } else {
            (current_mbps / max_bandwidth_mbps).min(1.0)
        }
    }

    /// Check if system is under resource pressure.
    ///
    /// # Returns
    ///
    /// True if any resource pressure indicators are active
    pub fn is_under_pressure(&self) -> bool {
        self.current_cpu_percent > 85.0
            || self.current_memory_mb / self.available_memory_mb.max(1.0) > 0.9
            || self.load_avg_1min > 8.0
    }

    /// Get resource pressure rate (events per hour).
    ///
    /// # Arguments
    ///
    /// * `uptime_hours` - System uptime in hours
    ///
    /// # Returns
    ///
    /// Pressure events per hour
    pub fn pressure_rate_per_hour(&self, uptime_hours: f64) -> f64 {
        if uptime_hours == 0.0 {
            0.0
        } else {
            self.resource_pressure_events as f64 / uptime_hours
        }
    }

    /// Record a resource pressure event.
    fn record_pressure_event(&mut self) {
        self.resource_pressure_events += 1;
        self.last_pressure_event = Some(SystemTime::now());
    }

    /// Calculate overall system health score.
    ///
    /// # Returns
    ///
    /// Health score from 0.0 to 1.0 (higher is better)
    pub fn overall_health_score(&self) -> f64 {
        let cpu_score = self.cpu_health_score();
        let memory_score = self.memory_efficiency_ratio();
        let load_score = if self.load_avg_1min > 4.0 {
            (8.0 - self.load_avg_1min).max(0.0) / 4.0
        } else {
            1.0
        };

        // Weighted average of different health indicators
        (cpu_score * 0.4 + memory_score * 0.4 + load_score * 0.2).min(1.0)
    }
}

impl Default for ResourceUtilizationSummary {
    fn default() -> Self {
        Self {
            current_memory_mb: 0.0,
            peak_memory_mb: 0.0,
            available_memory_mb: 0.0,
            connection_memory_bytes: 0,
            current_cpu_percent: 0.0,
            peak_cpu_percent: 0.0,
            cpu_sample_count: 0,
            network_rx_bps: 0,
            network_tx_bps: 0,
            peak_network_rx_bps: 0,
            peak_network_tx_bps: 0,
            load_avg_1min: 0.0,
            load_avg_5min: 0.0,
            load_avg_15min: 0.0,
            memory_per_connection_mb: 0.0,
            peak_memory_per_connection_mb: 0.0,
            resource_pressure_events: 0,
            last_pressure_event: None,
            last_updated: SystemTime::now(),
        }
    }
}
