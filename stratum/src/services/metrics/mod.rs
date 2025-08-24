//! High-performance metrics collection system for the Stratum mining proxy.
//!
//! This module provides a comprehensive metrics collection framework with:
//! - Lock-free atomic operations for sub-microsecond performance
//! - Modular organization for different metric categories
//! - Time series data collection and analysis
//! - User-specific and database metrics tracking
//!
//! # Architecture
//!
//! The metrics system is organized into several submodules:
//! - [`atomic`] - Core atomic metrics structure with lock-free counters
//! - [`snapshot`] - Point-in-time snapshots and derived calculations
//! - [`timeseries`] - Time series data collection and windowing
//! - [`user`] - Per-user metrics tracking and aggregation
//! - [`database`] - Database operation metrics
//! - [`service`] - Main metrics service coordinator
//! - [`calculated`] - Derived and calculated metrics
//! - [`resource`] - Resource utilization tracking
//!
//! # Performance Characteristics
//!
//! - Counter operations: ~4.6ns per operation
//! - Gauge operations: ~3.7ns per operation  
//! - Histogram operations: ~3.6ns per operation
//! - Zero-allocation string pooling for frequent operations
//!
//! # Example
//!
//! ```rust
//! use loka_stratum::services::metrics::{AtomicMetrics, MetricsService, MetricsConfig};
//! use std::sync::Arc;
//!
//! // Create metrics instance
//! let metrics = Arc::new(AtomicMetrics::new());
//!
//! // Record events
//! metrics.increment_connection();
//! metrics.record_bytes_received(1024);
//! metrics.record_submission_accepted();
//!
//! // Create service for advanced features
//! let config = MetricsConfig::default();
//! let service = MetricsService::new(config);
//! // Note: snapshot capturing requires async context
//! ```

pub mod atomic;
pub mod calculated;
pub mod constant;
pub mod database;
pub mod resource;
pub mod service;
pub mod snapshot;
pub mod timeseries;
pub mod user;

#[cfg(test)]
pub mod hashrate_test;

#[cfg(test)]
pub mod prometheus_verification_test;

// Re-export main types for convenience
pub use atomic::AtomicMetrics;
pub use calculated::CalculatedMetrics;
pub use database::DatabaseMetrics;
pub use resource::ResourceUtilizationSummary;
pub use service::{MetricsConfig, MetricsService};
pub use snapshot::{DeltaRates, MetricsDelta, MetricsSnapshot};
pub use timeseries::TimeSeriesMetrics;
pub use user::{UserMetrics, UserMetricsSnapshot};
