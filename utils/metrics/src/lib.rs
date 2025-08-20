//! # Loka Metrics
//!
//! A high-performance, thread-safe metrics collection library for Rust applications.
//!
//! ## Overview
//!
//! Loka Metrics provides a robust foundation for collecting, aggregating, and retrieving
//! application metrics with minimal performance overhead. Built with efficiency and
//! production-readiness in mind, it offers atomic operations, configurable bounds,
//! and optimized batch recording capabilities.
//!
//! ## Features
//!
//! - **🚀 High Performance**: Sub-microsecond operations with atomic memory operations
//! - **🔒 Thread Safety**: All operations are thread-safe using atomic primitives
//! - **📊 Flexible Metrics**: Support for counters, gauges, and histograms
//! - **⚡ Batch Operations**: Optimized `record_many` for high-throughput scenarios
//! - **🎯 Configurable Bounds**: Customizable histogram ranges with automatic clamping
//! - **📈 Rich Statistics**: Comprehensive statistical summaries including percentiles
//! - **🔄 Zero-Copy Retrieval**: Efficient data access without unnecessary allocations
//!
//! ## Performance Characteristics
//!
//! Based on benchmark results (see `cargo bench` for latest):
//!
//! - **Counter operations**: ~4.6 ns per operation
//! - **Gauge operations**: ~3.7 ns per operation  
//! - **Histogram operations**: ~3.6 ns per operation
//! - **Batch operations**: ~4.0 ns per operation
//! - **Data retrieval**: ~1.1 μs for all metric types
//! - **Mixed operations**: ~378 ns for typical app metrics
//! - **Batch recording**: 107x speedup over individual records
//!
//! ## Quick Start
//!
//! ```rust
//! use loka_metrics::Recorder;
//!
//! // Initialize the metrics recorder
//! let recorder = Recorder::init().expect("Failed to initialize metrics recorder");
//!
//! // Record some metrics
//! metrics::counter!("app.requests.total").increment(1);
//! metrics::gauge!("app.memory.usage").set(128.5);
//! metrics::histogram!("app.response_time").record(45.2);
//!
//! // Retrieve metrics
//! let counters = recorder.counters();
//! let gauges = recorder.gauges();
//! let histograms = recorder.histograms();
//! ```
//!
//! ## Histogram Bounds and Clamping
//!
//! Histograms automatically clamp values to fit within configured bounds:
//!
//! ```rust
//! use loka_metrics::Histogram;
//! use metrics::HistogramFn;
//!
//! let histogram = Histogram::new();
//! histogram.set_bounds(0.0, 100.0);
//!
//! // Values are automatically clamped
//! histogram.record(150.0);  // Clamped to 100.0
//! histogram.record(-25.0);  // Clamped to 0.0
//! histogram.record(50.0);   // No change (within bounds)
//! ```
//!
//! ## Batch Operations
//!
//! For high-throughput scenarios, use `record_many` for significant performance gains:
//!
//! ```rust
//! use loka_metrics::Histogram;
//! use metrics::HistogramFn;
//!
//! let histogram = Histogram::new();
//!
//! // Instead of multiple individual records
//! for _ in 0..1000 {
//!     histogram.record(42.0);
//! }
//!
//! // Use batch recording (107x faster!)
//! histogram.record_many(42.0, 1000);
//! ```
//!
//! ## Examples
//!
//! - **`usage`**: Comprehensive real-world metrics example with CPU, memory, and API monitoring
//! - **`histogram_demo`**: Histogram slot distribution visualization and performance benchmarks
//! - **`bounds_demo`**: Demonstration of bounds clamping behavior
//!
//! Run examples with: `cargo run --example <example_name>`
//!
//! ## Benchmarks
//!
//! Run comprehensive benchmarks with: `cargo bench`
//!
//! The benchmark suite covers:
//! - Metrics recording performance
//! - Data retrieval efficiency
//! - Mixed operations scenarios
//! - Histogram performance analysis
//! - Concurrent operations
//!
//! ## Thread Safety
//!
//! All operations are thread-safe using atomic primitives:
//!
//! ```rust
//! use std::sync::Arc;
//! use std::thread;
//! use loka_metrics::Histogram;
//! use metrics::HistogramFn;
//!
//! let histogram = Arc::new(Histogram::new());
//! let mut handles = vec![];
//!
//! for _ in 0..8 {
//!     let histogram_clone = Arc::clone(&histogram);
//!     let handle = thread::spawn(move || {
//!         for i in 0..1000 {
//!             histogram_clone.record(i as f64);
//!         }
//!     });
//!     handles.push(handle);
//! }
//!
//! for handle in handles {
//!     handle.join().unwrap();
//! }
//! ```
//!
//! ## Error Handling
//!
//! The library gracefully handles edge cases:
//! - **NaN values**: Automatically skipped without errors
//! - **Out-of-bounds values**: Automatically clamped to bounds
//! - **Invalid bounds**: Gracefully handled with fallbacks
//! - **Concurrent access**: No race conditions or data corruption
//!
//! ## Memory Efficiency
//!
//! - **Fixed-size storage**: Histograms use exactly 12 slots regardless of data volume
//! - **Atomic operations**: No locks or mutexes required
//! - **Zero allocations**: Most operations don't allocate memory
//! - **Cache-friendly**: Data structures designed for CPU cache efficiency
//!
//! ## Production Considerations
//!
//! - **Performance**: Sub-microsecond operations suitable for high-frequency metrics
//! - **Reliability**: Atomic operations ensure data consistency under load
//! - **Monitoring**: Built-in bounds checking prevents metric overflow
//! - **Scalability**: Thread-safe design supports high-concurrency applications
//!
//! ## License
//!
//! This project is licensed under the MIT License - see the LICENSE file for details.

#[cfg(feature = "collector")]
pub(crate) mod collector;

pub(crate) mod key;
pub(crate) mod recorder;

pub mod types;

pub use recorder::Recorder;
pub use types::{Counter, Gauge, Histogram, HistogramSummary};
