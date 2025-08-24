# Loka Stratum Benchmark Suite Guide

This guide covers the comprehensive benchmark suite for Loka Stratum, including both the original critical path benchmarks and the new metrics performance validation benchmarks.

## Overview

The benchmark suite consists of two main components:
1. **Critical Path Benchmarks** (`critical_path.rs`) - Original performance benchmarks
2. **New Metrics Performance Benchmarks** (`new_metrics_performance.rs`) - Validation for new metrics added in Tasks 8.1-8.5

## Performance Targets

| Operation Type | Target Performance | Benchmark Coverage |
|---|---|---|
| Counter operations | ~4.6ns | ✅ Validated |
| Gauge operations | <15ns | ✅ Validated |
| Float-to-bits conversions | <20ns | ✅ Validated |
| Batch operations | 50x+ speedup | ✅ Validated |
| Concurrent access | Linear scaling | ✅ Validated |

## Running Benchmarks

### Quick Commands

```bash
# Quick performance check (30 seconds)
./run_benchmarks.sh quick-all

# Run all benchmarks
./run_benchmarks.sh all

# List available benchmarks
./run_benchmarks.sh list

# Run specific benchmark
./run_benchmarks.sh run bench_connection_lifecycle_metrics
```

### Performance Validation

```bash
# Validate all new metrics meet performance targets
./validate_performance.sh

# Expected output:
# ✅ All performance tests PASSED!
# ✅ All new metrics meet sub-microsecond performance targets
```

### Baseline Management

```bash
# Create performance baseline
./run_benchmarks.sh baseline v1.0

# Compare against baseline
./run_benchmarks.sh compare v1.0
```

## Benchmark Categories

### 1. Connection Lifecycle Metrics (Task 8.1)

**Benchmark:** `bench_connection_lifecycle_metrics`

Tests performance of connection tracking operations:
- Connection duration recording
- Idle time gauge updates  
- Reconnection attempt counters
- Connection establishment/closure events

**Key Operations Tested:**
```rust
metrics.connection_established_counter.fetch_add(1, Ordering::Relaxed);
metrics.connection_duration_sum_ms_bits.fetch_add(duration_bits, Ordering::Relaxed);
metrics.idle_time_gauge_ms_bits.store(idle_time_bits, Ordering::Relaxed);
```

**Performance Expectation:** <10ns per operation

### 2. Protocol Detection Metrics (Task 8.2)

**Benchmark:** `bench_protocol_detection_metrics`

Tests HTTP vs Stratum protocol detection performance:
- HTTP request counting
- Stratum request counting
- Protocol conversion success rate calculation
- Detection failure tracking

**Key Operations Tested:**
```rust
metrics.http_requests_total.fetch_add(1, Ordering::Relaxed);
metrics.stratum_requests_total.fetch_add(1, Ordering::Relaxed);
metrics.protocol_detection_successes.fetch_add(1, Ordering::Relaxed);
```

**Performance Expectation:** <10ns per operation, 100x+ throughput for batches

### 3. Mining Operation Metrics (Task 8.3)

**Benchmark:** `bench_mining_operation_metrics`

Tests share submission and job distribution performance:
- Share submission recording
- Acceptance rate calculation
- Job distribution latency tracking
- Difficulty adjustment recording

**Key Operations Tested:**
```rust
metrics.share_submissions_total.fetch_add(1, Ordering::Relaxed);
metrics.share_acceptance_rate_sum_bits.fetch_add(rate_bits, Ordering::Relaxed);
metrics.job_distribution_latency_sum_ms_bits.fetch_add(latency_bits, Ordering::Relaxed);
```

**Performance Expectation:** <10ns per counter, <15ns for gauge operations

### 4. Error Categorization Metrics (Task 8.4)

**Benchmark:** `bench_error_categorization_metrics`

Tests error classification and counting performance:
- Error type dispatch (10 categories)
- Error burst handling
- Pattern analysis for high-frequency errors

**Key Operations Tested:**
```rust
// Dispatch to appropriate error counter based on error type
match error_type {
    "network" => metrics.network_errors_total.fetch_add(1, Ordering::Relaxed),
    "auth" => metrics.authentication_failures_total.fetch_add(1, Ordering::Relaxed),
    // ... other error types
}
```

**Performance Expectation:** <10ns per error categorization

### 5. Resource Utilization Metrics (Task 8.5)

**Benchmark:** `bench_resource_utilization_metrics`

Tests memory and CPU tracking performance:
- Memory usage per connection tracking
- CPU utilization sampling
- Network bandwidth monitoring
- Resource efficiency calculations

**Key Operations Tested:**
```rust
metrics.memory_usage_per_connection_sum_bits.fetch_add(memory_bits, Ordering::Relaxed);
metrics.cpu_utilization_sample_bits.store(cpu_bits, Ordering::Relaxed);
metrics.network_bandwidth_rx_bytes_per_sec.store(rx_bytes, Ordering::Relaxed);
```

**Performance Expectation:** <15ns per gauge operation

### 6. Atomic Bit Operations

**Benchmark:** `bench_atomic_bit_operations`

Tests float-to-bits conversions and atomic operations:
- Float to bits conversion
- Atomic compare-and-swap operations
- Atomic fetch operations

**Performance Expectation:** <20ns for float conversions, <10ns for fetch operations

### 7. Concurrent Stress Testing

**Benchmark:** `bench_concurrent_new_metrics_stress`

Tests concurrent access across all new metrics:
- 1, 2, 4, 8, 16 thread scalability
- Lock-free performance validation
- Memory contention analysis

**Performance Expectation:** Linear scaling with thread count

### 8. Baseline Comparison

**Benchmark:** `bench_new_metrics_vs_baseline`

Compares new metrics performance against original metrics:
- Direct timing comparison
- Performance overhead analysis
- Comprehensive operation testing

**Performance Expectation:** <2x overhead compared to baseline

## Benchmark Infrastructure

### Files Structure

```
stratum/
├── benches/
│   ├── critical_path.rs           # Original benchmarks
│   └── new_metrics_performance.rs # New metrics benchmarks
├── run_benchmarks.sh              # Benchmark runner script  
├── validate_performance.sh        # Performance validation
└── BENCHMARK_GUIDE.md             # This guide
```

### Criterion Configuration

Both benchmark files use Criterion with:
- Statistical analysis
- Outlier detection
- HTML report generation
- Baseline comparison support

### Performance Analysis

The benchmark suite provides:
- **Timing Analysis:** Mean, median, std deviation
- **Throughput Testing:** Operations per second
- **Scalability Testing:** Multi-thread performance
- **Regression Detection:** Baseline comparisons
- **Memory Analysis:** Allocation patterns

## Integration with CI/CD

### GitHub Actions Integration

```yaml
- name: Run Performance Benchmarks
  run: |
    cd stratum
    ./validate_performance.sh
    
- name: Check Performance Regression
  run: |
    cd stratum
    ./run_benchmarks.sh compare main
```

### Performance Monitoring

- Continuous tracking of key metrics
- Automated alerts for performance regressions
- Historical performance trending

## Troubleshooting

### Common Issues

1. **Benchmark Timeout**
   ```bash
   # Reduce benchmark duration
   timeout 30s ./run_benchmarks.sh quick-all
   ```

2. **High Performance Variance**
   - Ensure system is idle during benchmarking
   - Run multiple iterations: `--sample-size 1000`
   - Check for CPU throttling

3. **Memory Issues**
   - Monitor memory usage during stress tests
   - Reduce concurrent thread count if needed

4. **Missing Dependencies**
   ```bash
   # Ensure all dependencies are built
   cargo build --release
   cargo test
   ```

### Performance Regression Analysis

If performance tests fail:

1. **Identify the failing metric:**
   ```bash
   ./validate_performance.sh | grep "FAIL"
   ```

2. **Run detailed analysis:**
   ```bash
   ./run_benchmarks.sh run <specific_benchmark>
   ```

3. **Compare with baseline:**
   ```bash
   ./run_benchmarks.sh compare <baseline_name>
   ```

4. **Check system resources:**
   - CPU utilization
   - Memory pressure
   - I/O contention

## Best Practices

### Benchmarking Environment

- Run on dedicated hardware when possible
- Minimize background processes
- Use consistent system configuration
- Monitor system temperature and throttling

### Benchmark Maintenance

- Update baselines after verified improvements
- Add new benchmarks for new metrics
- Regular performance regression testing
- Document performance characteristics

### Performance Optimization

- Focus on hot paths identified by benchmarks
- Use lock-free algorithms where possible
- Minimize memory allocations in critical paths
- Optimize batch operations over individual operations

## Future Enhancements

### Planned Additions

1. **Real-world Load Simulation:**
   - Mining pool traffic patterns
   - Connection lifecycle simulation
   - Error injection testing

2. **Memory Profiling:**
   - Allocation tracking
   - Memory leak detection
   - Cache performance analysis

3. **Network Performance:**
   - TCP connection benchmarks
   - Serialization performance
   - Protocol overhead analysis

### Extensibility

The benchmark suite is designed to be easily extended:

1. **Add new benchmark function**
2. **Update `criterion_group!` macro**
3. **Add to `run_benchmarks.sh` script**
4. **Include in validation suite**

This comprehensive benchmark suite ensures that all new metrics maintain the high-performance standards required for production Bitcoin mining operations while providing the observability needed for monitoring and debugging.