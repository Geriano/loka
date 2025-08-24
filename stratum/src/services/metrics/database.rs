//! Database-specific metrics tracking and analysis.
//!
//! This module provides metrics related to database operations, connection
//! pooling, query performance, and data persistence operations.

use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

/// Database operation metrics for performance monitoring and optimization.
///
/// Tracks database-related performance metrics including connection pool health,
/// query execution times, and transaction success rates.
///
/// # Examples
///
/// ```rust
/// use loka_stratum::services::metrics::DatabaseMetrics;
///
/// let mut db_metrics = DatabaseMetrics::default();
/// 
/// // Record database operations
/// db_metrics.record_query_execution(Duration::from_millis(15), true);
/// db_metrics.record_connection_acquired(Duration::from_micros(500));
/// 
/// println!("Query success rate: {:.2}%", db_metrics.query_success_rate() * 100.0);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseMetrics {
    // Connection Pool Metrics
    /// Total number of connections created
    pub connections_created: u64,
    /// Total number of connections destroyed
    pub connections_destroyed: u64,
    /// Current number of active connections
    pub connections_active: u64,
    /// Current number of idle connections in pool
    pub connections_idle: u64,
    /// Maximum connections reached simultaneously
    pub connections_max_reached: u64,
    /// Total connection acquisition timeouts
    pub connection_timeouts: u64,

    // Query Performance Metrics
    /// Total number of queries executed
    pub queries_total: u64,
    /// Number of successful queries
    pub queries_successful: u64,
    /// Number of failed queries
    pub queries_failed: u64,
    /// Total query execution time in milliseconds
    pub query_duration_total_ms: u64,
    /// Shortest query execution time in milliseconds
    pub query_duration_min_ms: u64,
    /// Longest query execution time in milliseconds
    pub query_duration_max_ms: u64,

    // Transaction Metrics
    /// Total number of transactions started
    pub transactions_started: u64,
    /// Number of committed transactions
    pub transactions_committed: u64,
    /// Number of rolled back transactions
    pub transactions_rolled_back: u64,

    // Table-Specific Metrics
    /// Number of miner records inserted
    pub miners_inserted: u64,
    /// Number of miner records updated
    pub miners_updated: u64,
    /// Number of worker records inserted
    pub workers_inserted: u64,
    /// Number of submission records inserted
    pub submissions_inserted: u64,
    /// Number of earnings records inserted
    pub earnings_inserted: u64,

    // Pool Connection Timing
    /// Total time spent waiting for connections (microseconds)
    pub connection_wait_time_total_us: u64,
    /// Number of successful connection acquisitions
    pub connection_acquisitions: u64,

    /// Timestamp of last metrics update
    pub last_updated: SystemTime,
}

impl DatabaseMetrics {
    /// Create a new database metrics instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a query execution.
    ///
    /// # Arguments
    ///
    /// * `duration` - Time taken to execute the query
    /// * `success` - Whether the query executed successfully
    pub fn record_query_execution(&mut self, duration: Duration, success: bool) {
        self.queries_total += 1;
        
        if success {
            self.queries_successful += 1;
        } else {
            self.queries_failed += 1;
        }
        
        let duration_ms = duration.as_millis() as u64;
        self.query_duration_total_ms += duration_ms;
        
        if self.query_duration_min_ms == 0 || duration_ms < self.query_duration_min_ms {
            self.query_duration_min_ms = duration_ms;
        }
        
        if duration_ms > self.query_duration_max_ms {
            self.query_duration_max_ms = duration_ms;
        }
        
        self.last_updated = SystemTime::now();
    }

    /// Record a successful connection acquisition.
    ///
    /// # Arguments
    ///
    /// * `wait_time` - Time spent waiting for the connection
    pub fn record_connection_acquired(&mut self, wait_time: Duration) {
        self.connection_acquisitions += 1;
        self.connection_wait_time_total_us += wait_time.as_micros() as u64;
        self.last_updated = SystemTime::now();
    }

    /// Record a connection timeout.
    pub fn record_connection_timeout(&mut self) {
        self.connection_timeouts += 1;
        self.last_updated = SystemTime::now();
    }

    /// Record a transaction start.
    pub fn record_transaction_started(&mut self) {
        self.transactions_started += 1;
        self.last_updated = SystemTime::now();
    }

    /// Record a transaction commit.
    pub fn record_transaction_committed(&mut self) {
        self.transactions_committed += 1;
        self.last_updated = SystemTime::now();
    }

    /// Record a transaction rollback.
    pub fn record_transaction_rolled_back(&mut self) {
        self.transactions_rolled_back += 1;
        self.last_updated = SystemTime::now();
    }

    /// Record insertion of database records by table.
    ///
    /// # Arguments
    ///
    /// * `table` - Table name where record was inserted
    /// * `count` - Number of records inserted
    pub fn record_insert(&mut self, table: &str, count: u64) {
        match table {
            "miners" => self.miners_inserted += count,
            "workers" => self.workers_inserted += count,
            "submissions" => self.submissions_inserted += count,
            "earnings" => self.earnings_inserted += count,
            _ => {}, // Unknown table
        }
        self.last_updated = SystemTime::now();
    }

    /// Record update of database records.
    ///
    /// # Arguments
    ///
    /// * `table` - Table name where records were updated
    /// * `count` - Number of records updated
    pub fn record_update(&mut self, table: &str, count: u64) {
        match table {
            "miners" => self.miners_updated += count,
            _ => {}, // Other tables updates can be added as needed
        }
        self.last_updated = SystemTime::now();
    }

    /// Update connection pool statistics.
    ///
    /// # Arguments
    ///
    /// * `active` - Current active connections
    /// * `idle` - Current idle connections  
    /// * `max_reached` - Maximum connections reached
    pub fn update_pool_stats(&mut self, active: u64, idle: u64, max_reached: u64) {
        self.connections_active = active;
        self.connections_idle = idle;
        
        if max_reached > self.connections_max_reached {
            self.connections_max_reached = max_reached;
        }
        
        self.last_updated = SystemTime::now();
    }

    /// Calculate query success rate (0.0 to 1.0).
    pub fn query_success_rate(&self) -> f64 {
        if self.queries_total == 0 {
            0.0
        } else {
            self.queries_successful as f64 / self.queries_total as f64
        }
    }

    /// Calculate average query execution time in milliseconds.
    pub fn avg_query_duration_ms(&self) -> f64 {
        if self.queries_total == 0 {
            0.0
        } else {
            self.query_duration_total_ms as f64 / self.queries_total as f64
        }
    }

    /// Calculate transaction success rate (0.0 to 1.0).
    pub fn transaction_success_rate(&self) -> f64 {
        if self.transactions_started == 0 {
            0.0
        } else {
            self.transactions_committed as f64 / self.transactions_started as f64
        }
    }

    /// Calculate average connection wait time in microseconds.
    pub fn avg_connection_wait_time_us(&self) -> f64 {
        if self.connection_acquisitions == 0 {
            0.0
        } else {
            self.connection_wait_time_total_us as f64 / self.connection_acquisitions as f64
        }
    }

    /// Get total number of connections in use (active + idle).
    pub fn total_pool_connections(&self) -> u64 {
        self.connections_active + self.connections_idle
    }

    /// Calculate pool utilization rate (0.0 to 1.0).
    pub fn pool_utilization_rate(&self) -> f64 {
        let total = self.total_pool_connections();
        if total == 0 {
            0.0
        } else {
            self.connections_active as f64 / total as f64
        }
    }
}

impl Default for DatabaseMetrics {
    fn default() -> Self {
        Self {
            connections_created: 0,
            connections_destroyed: 0,
            connections_active: 0,
            connections_idle: 0,
            connections_max_reached: 0,
            connection_timeouts: 0,
            queries_total: 0,
            queries_successful: 0,
            queries_failed: 0,
            query_duration_total_ms: 0,
            query_duration_min_ms: 0,
            query_duration_max_ms: 0,
            transactions_started: 0,
            transactions_committed: 0,
            transactions_rolled_back: 0,
            miners_inserted: 0,
            miners_updated: 0,
            workers_inserted: 0,
            submissions_inserted: 0,
            earnings_inserted: 0,
            connection_wait_time_total_us: 0,
            connection_acquisitions: 0,
            last_updated: SystemTime::now(),
        }
    }
}