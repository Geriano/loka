# Loka Metrics

[![Crates.io](https://img.shields.io/crates/v/loka-metrics)](https://crates.io/crates/loka-metrics)
[![Documentation](https://docs.rs/loka-metrics/badge.svg)](https://docs.rs/loka-metrics)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70+-blue.svg)](https://www.rust-lang.org)

A high-performance, thread-safe metrics collection library for Rust applications.

## 🚀 Features

- **⚡ High Performance**: Sub-microsecond operations with atomic memory operations
- **🔒 Thread Safety**: All operations are thread-safe using atomic primitives
- **📊 Flexible Metrics**: Support for counters, gauges, and histograms
- **🎯 Configurable Bounds**: Customizable histogram ranges with automatic clamping
- **📈 Rich Statistics**: Comprehensive statistical summaries including percentiles
- **🔄 Zero-Copy Retrieval**: Efficient data access without unnecessary allocations
- **⚡ Batch Operations**: Optimized `record_many` for high-throughput scenarios

## 📊 Performance Characteristics

Based on benchmark results (see `cargo bench` for latest):

| Operation | Performance | Notes |
|-----------|-------------|-------|
| **Counter operations** | ~4.6 ns per operation | Atomic increment |
| **Gauge operations** | ~3.7 ns per operation | Atomic store |
| **Histogram operations** | ~3.6 ns per operation | Atomic updates |
| **Batch operations** | ~4.0 ns per operation | 107x speedup for large batches |
| **Data retrieval** | ~1.1 μs for all metric types | Zero-copy access |
| **Mixed operations** | ~378 ns for typical app metrics | Real-world scenarios |

## 🛠️ Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
loka-metrics = "0.1.0"
```

## 🚀 Quick Start

```rust
use loka_metrics::Recorder;

// Initialize the metrics recorder
let recorder = Recorder::init().expect("Failed to initialize metrics recorder");

// Record some metrics
metrics::counter!("app.requests.total").increment(1);
metrics::gauge!("app.memory.usage").set(128.5);
metrics::histogram!("app.response_time").record(45.2);

// Retrieve metrics
let counters = recorder.counters();
let gauges = recorder.gauges();
let histograms = recorder.histograms();
```

## 📊 Histogram Usage

### Basic Histogram

```rust
use loka_metrics::Histogram;
use metrics::HistogramFn;

let histogram = Histogram::new();
histogram.set_bounds(0.0, 100.0);

// Record individual values
histogram.record(25.0);
histogram.record(50.0);
histogram.record(75.0);

// Get statistics
println!("Count: {}", histogram.count());
println!("Sum: {:.1}", histogram.sum());
println!("Mean: {:.2}", histogram.mean());
println!("Min: {:.1}", histogram.min());
println!("Max: {:.1}", histogram.max());
```

### Batch Recording

For high-throughput scenarios, use `record_many` for significant performance gains:

```rust
// Instead of multiple individual records
for _ in 0..1000 {
    histogram.record(42.0);
}

// Use batch recording (107x faster!)
histogram.record_many(42.0, 1000);
```

### Bounds and Clamping

Histograms automatically clamp values to fit within configured bounds:

```rust
histogram.set_bounds(0.0, 100.0);

histogram.record(150.0);  // Clamped to 100.0
histogram.record(-25.0);  // Clamped to 0.0
histogram.record(50.0);   // No change (within bounds)
```

## 🧪 Examples

### Real-World Metrics Example

```bash
cargo run --example usage
```

Demonstrates comprehensive metrics collection including:
- CPU usage monitoring
- Memory usage tracking
- API response time monitoring
- Batch operations

### Histogram Demo

```bash
cargo run --example histogram_demo
```

Shows:
- Histogram slot distribution
- Performance benchmarks
- Concurrent operations
- Real-time analysis

### Bounds Demo

```bash
cargo run --example bounds_demo
```

Demonstrates:
- Bounds clamping behavior
- Slot distribution for out-of-bounds values
- Edge case handling

## 📈 Benchmarks

Run comprehensive benchmarks:

```bash
cargo bench
```

The benchmark suite covers:
- **Metrics recording performance**: Counter, gauge, and histogram operations
- **Data retrieval efficiency**: Metrics collection and access patterns
- **Mixed operations**: Real-world application scenarios
- **Histogram performance**: Different bounds and batch sizes
- **Concurrent operations**: Multi-threaded access patterns

### Sample Benchmark Results

```
metrics_recording/counter_increment
                        time:   [4.5903 ns 4.6004 ns 4.6131 ns]

metrics_recording/histogram_record
                        time:   [3.6148 ns 3.6239 ns 3.6349 ns]

metrics_recording/histogram_record_many
                        time:   [3.9514 ns 3.9676 ns 3.9867 ns]

concurrent_operations/concurrent_histogram_access
                        time:   [162.34 µs 163.66 µs 165.00 µs]
```

## 🔧 Configuration

### Histogram Bounds

Set appropriate bounds for your use case:

```rust
// Response times: 0ms to 5 seconds
histogram.set_bounds(0.0, 5000.0);

// Memory usage: 0MB to 16GB
histogram.set_bounds(0.0, 16384.0);

// CPU usage: 0% to 100%
histogram.set_bounds(0.0, 100.0);

// Custom ranges
histogram.set_bounds(-1000.0, 1000.0);
```

### Default Configuration

- **Bounds**: `[-1,000,000.0, 1,000,000.0]` (wide range for general use)
- **Slots**: 12 slots for value distribution
- **Memory**: ~128 bytes total (cache-friendly)

## 🔒 Thread Safety

All operations are thread-safe using atomic primitives:

```rust
use std::sync::Arc;
use std::thread;

let histogram = Arc::new(Histogram::new());
let mut handles = vec![];

for _ in 0..8 {
    let histogram_clone = Arc::clone(&histogram);
    let handle = thread::spawn(move || {
        for i in 0..1000 {
            histogram_clone.record(i as f64);
        }
    });
    handles.push(handle);
}

for handle in handles {
    handle.join().unwrap();
}
```

## 📊 Slot Distribution

Values are distributed across 12 slots based on their position within bounds:

| Slot | Range Example (bounds [0.0, 100.0]) | Description |
|------|-------------------------------------|-------------|
| 0 | [0.0, 8.33) | Lower bound values |
| 3 | [25.0, 33.3) | Lower-middle values |
| 6 | [50.0, 58.3) | Middle values |
| 9 | [75.0, 83.3) | Upper-middle values |
| 11 | [91.67, 100.0] | Upper bound values |

## 🚨 Error Handling

The library gracefully handles edge cases:

- **NaN values**: Automatically skipped without errors
- **Out-of-bounds values**: Automatically clamped to bounds
- **Invalid bounds**: Gracefully handled with fallbacks
- **Concurrent access**: No race conditions or data corruption

## 💾 Memory Efficiency

- **Fixed-size storage**: Histograms use exactly 12 slots regardless of data volume
- **Atomic operations**: No locks or mutexes required
- **Zero allocations**: Most operations don't allocate memory
- **Cache-friendly**: Data structures designed for CPU cache efficiency

## 🏭 Production Considerations

### Performance
- Sub-microsecond operations suitable for high-frequency metrics
- Scales linearly with thread count
- Minimal memory overhead

### Reliability
- Atomic operations ensure data consistency under load
- Built-in bounds checking prevents metric overflow
- Graceful handling of edge cases

### Monitoring
- Rich statistical summaries for health checks
- Configurable bounds for application-specific ranges
- Efficient data retrieval for external systems

## 📚 API Reference

### Core Types

- **`Histogram`**: High-performance histogram with configurable bounds
- **`HistogramSummary`**: Comprehensive statistical summary
- **`Recorder`**: Metrics collection and retrieval interface

### Key Methods

- **`Histogram::new()`**: Create new histogram with default bounds
- **`Histogram::set_bounds(lower, upper)`**: Configure value clamping bounds
- **`Histogram::record(value)`**: Record single value
- **`Histogram::record_many(value, count)`**: Record multiple identical values
- **`Histogram::summary()`**: Get comprehensive statistical summary
- **`Histogram::reset()`**: Clear data while preserving bounds

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

### Development Setup

```bash
git clone <repository-url>
cd loka-metrics
cargo build
cargo test
cargo bench
```

### Running Examples

```bash
# Real-world metrics example
cargo run --example usage

# Histogram demonstration
cargo run --example histogram_demo

# Bounds and clamping demo
cargo run --example bounds_demo
```

### Running Tests

```bash
# All tests
cargo test

# Specific test
cargo test test_histogram_values_exceeding_bounds

# Tests with output
cargo test -- --nocapture
```

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built with Rust's excellent `std::sync::atomic` primitives
- Inspired by the need for high-performance metrics in production systems
- Designed with efficiency and maintainability in mind

## 📞 Support

- **Issues**: [GitHub Issues](https://github.com/your-org/loka-metrics/issues)
- **Documentation**: [docs.rs](https://docs.rs/loka-metrics)
- **Discussions**: [GitHub Discussions](https://github.com/your-org/loka-metrics/discussions)

---

**Loka Metrics** - High-performance metrics collection for Rust applications 🚀
