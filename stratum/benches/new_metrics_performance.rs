use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use loka_stratum::services::metrics::AtomicMetrics;
use std::hint::black_box;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

/// Benchmark connection lifecycle metrics (Task 8.1)
/// Tests all new connection-related atomic operations for sub-microsecond performance
fn bench_connection_lifecycle_metrics(c: &mut Criterion) {
    let metrics = Arc::new(AtomicMetrics::new());

    // Individual metric operations
    c.bench_function("connection_duration_recording", |b| {
        b.iter(|| {
            let duration_ms = Duration::from_millis(black_box(150)).as_millis() as f64;
            let duration_bits = duration_ms.to_bits();

            metrics
                .connection_duration_sum_ms_bits
                .fetch_add(duration_bits, Ordering::Relaxed);
            metrics
                .connection_duration_count
                .fetch_add(1, Ordering::Relaxed);

            // Update min/max with atomic compare-and-swap operations
            let current_max = metrics
                .connection_duration_max_ms_bits
                .load(Ordering::Relaxed);
            if duration_bits > current_max {
                let _ = metrics
                    .connection_duration_max_ms_bits
                    .compare_exchange_weak(
                        current_max,
                        duration_bits,
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                    );
            }
        });
    });

    c.bench_function("connection_lifecycle_events", |b| {
        b.iter(|| {
            metrics
                .connection_established_counter
                .fetch_add(1, Ordering::Relaxed);
            metrics
                .idle_time_gauge_ms_bits
                .store(black_box(1000_f64).to_bits(), Ordering::Relaxed);
            metrics
                .reconnection_attempts_counter
                .fetch_add(1, Ordering::Relaxed);
            metrics
                .connection_closed_counter
                .fetch_add(1, Ordering::Relaxed);
        });
    });

    // Batch operations benchmark
    let mut group = c.benchmark_group("connection_lifecycle_batch");
    for batch_size in [10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*batch_size as u64));
        group.bench_with_input(
            BenchmarkId::new("events_per_batch", batch_size),
            batch_size,
            |b, &batch_size| {
                b.iter(|| {
                    for _ in 0..batch_size {
                        let duration_ms = black_box(150_f64);
                        metrics
                            .connection_duration_sum_ms_bits
                            .fetch_add(duration_ms.to_bits(), Ordering::Relaxed);
                        metrics
                            .connection_duration_count
                            .fetch_add(1, Ordering::Relaxed);
                        metrics
                            .connection_established_counter
                            .fetch_add(1, Ordering::Relaxed);
                    }
                });
            },
        );
    }
    group.finish();

    // Concurrent access test
    let mut group = c.benchmark_group("connection_lifecycle_concurrent");
    for threads in [1, 2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::new("threads", threads),
            threads,
            |b, &threads| {
                let metrics = Arc::clone(&metrics);
                b.iter(|| {
                    let handles: Vec<_> = (0..threads)
                        .map(|_| {
                            let metrics = Arc::clone(&metrics);
                            std::thread::spawn(move || {
                                for i in 0..100 {
                                    let duration_ms = black_box(100 + i) as f64;
                                    metrics
                                        .connection_duration_sum_ms_bits
                                        .fetch_add(duration_ms.to_bits(), Ordering::Relaxed);
                                    metrics
                                        .connection_established_counter
                                        .fetch_add(1, Ordering::Relaxed);
                                }
                            })
                        })
                        .collect();

                    for handle in handles {
                        handle.join().unwrap();
                    }
                });
            },
        );
    }
    group.finish();
}

/// Benchmark protocol detection metrics (Task 8.2)
/// Tests HTTP vs Stratum protocol detection performance
fn bench_protocol_detection_metrics(c: &mut Criterion) {
    let metrics = Arc::new(AtomicMetrics::new());

    c.bench_function("protocol_detection_operations", |b| {
        b.iter(|| {
            // Simulate protocol detection logic
            let is_http = black_box(true);
            let is_successful = black_box(true);

            if is_http {
                metrics.http_requests_total.fetch_add(1, Ordering::Relaxed);
                if black_box(false) {
                    // HTTP CONNECT request
                    metrics
                        .http_connect_requests
                        .fetch_add(1, Ordering::Relaxed);
                }
            } else {
                metrics
                    .stratum_requests_total
                    .fetch_add(1, Ordering::Relaxed);
                metrics
                    .direct_stratum_connections
                    .fetch_add(1, Ordering::Relaxed);
            }

            if is_successful {
                metrics
                    .protocol_detection_successes
                    .fetch_add(1, Ordering::Relaxed);
                // Update success rate using atomic operations
                metrics
                    .protocol_conversion_success_rate_sum_bits
                    .fetch_add(1_f64.to_bits(), Ordering::Relaxed);
                metrics
                    .protocol_conversion_success_count
                    .fetch_add(1, Ordering::Relaxed);
            } else {
                metrics
                    .protocol_detection_failures
                    .fetch_add(1, Ordering::Relaxed);
                metrics
                    .protocol_conversion_errors
                    .fetch_add(1, Ordering::Relaxed);
            }
        });
    });

    // Protocol conversion rate calculation benchmark
    c.bench_function("protocol_conversion_rate_calculation", |b| {
        b.iter(|| {
            let success_sum_bits = metrics
                .protocol_conversion_success_rate_sum_bits
                .load(Ordering::Relaxed);
            let success_count = metrics
                .protocol_conversion_success_count
                .load(Ordering::Relaxed);

            if success_count > 0 {
                let success_sum = f64::from_bits(success_sum_bits);
                let rate = black_box(success_sum / success_count as f64);
                black_box(rate);
            }
        });
    });

    // Throughput test for protocol detection
    let mut group = c.benchmark_group("protocol_detection_throughput");
    for req_count in [100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*req_count as u64));
        group.bench_with_input(
            BenchmarkId::new("requests_per_batch", req_count),
            req_count,
            |b, &req_count| {
                b.iter(|| {
                    for i in 0..req_count {
                        let is_http = i % 3 == 0; // Mix of HTTP and Stratum
                        if is_http {
                            metrics.http_requests_total.fetch_add(1, Ordering::Relaxed);
                        } else {
                            metrics
                                .stratum_requests_total
                                .fetch_add(1, Ordering::Relaxed);
                        }
                        metrics
                            .protocol_detection_successes
                            .fetch_add(1, Ordering::Relaxed);
                    }
                });
            },
        );
    }
    group.finish();
}

/// Benchmark mining operation metrics (Task 8.3)
/// Tests share submission and job distribution performance
fn bench_mining_operation_metrics(c: &mut Criterion) {
    let metrics = Arc::new(AtomicMetrics::new());

    c.bench_function("share_submission_recording", |b| {
        b.iter(|| {
            metrics
                .share_submissions_total
                .fetch_add(1, Ordering::Relaxed);

            let is_accepted = black_box(true);
            if is_accepted {
                // Update acceptance rate using atomic operations
                let rate = 0.95_f64; // 95% acceptance rate
                metrics
                    .share_acceptance_rate_sum_bits
                    .fetch_add(rate.to_bits(), Ordering::Relaxed);
                metrics
                    .share_acceptance_rate_count
                    .fetch_add(1, Ordering::Relaxed);
            } else if black_box(false) {
                // is stale
                metrics.stale_shares_counter.fetch_add(1, Ordering::Relaxed);
            } else {
                metrics
                    .duplicate_shares_counter
                    .fetch_add(1, Ordering::Relaxed);
            }

            metrics
                .shares_per_minute_gauge
                .store(black_box(120), Ordering::Relaxed);
        });
    });

    c.bench_function("job_distribution_latency_recording", |b| {
        b.iter(|| {
            let latency_ms = black_box(5_u64); // 5ms latency
            let latency_bits = (latency_ms as f64).to_bits();

            metrics
                .job_distribution_latency_sum_ms_bits
                .fetch_add(latency_bits, Ordering::Relaxed);
            metrics
                .job_distribution_latency_count
                .fetch_add(1, Ordering::Relaxed);

            // Update min/max latency
            let current_max = metrics
                .job_distribution_latency_max_ms_bits
                .load(Ordering::Relaxed);
            if latency_bits > current_max {
                let _ = metrics
                    .job_distribution_latency_max_ms_bits
                    .compare_exchange_weak(
                        current_max,
                        latency_bits,
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                    );
            }
        });
    });

    c.bench_function("difficulty_operations", |b| {
        b.iter(|| {
            let new_difficulty = black_box(1000000_f64);
            metrics
                .current_difficulty_bits
                .store(new_difficulty.to_bits(), Ordering::Relaxed);
            metrics
                .difficulty_adjustments_counter
                .fetch_add(1, Ordering::Relaxed);
        });
    });

    // Mining metrics batch operations
    let mut group = c.benchmark_group("mining_metrics_batch");
    for shares in [50, 500, 5000].iter() {
        group.throughput(Throughput::Elements(*shares as u64));
        group.bench_with_input(
            BenchmarkId::new("shares_per_batch", shares),
            shares,
            |b, &shares| {
                b.iter(|| {
                    for i in 0..shares {
                        metrics
                            .share_submissions_total
                            .fetch_add(1, Ordering::Relaxed);
                        let acceptance_rate = if i % 20 == 0 { 0.0_f64 } else { 1.0_f64 }; // 95% acceptance
                        metrics
                            .share_acceptance_rate_sum_bits
                            .fetch_add(acceptance_rate.to_bits(), Ordering::Relaxed);
                        metrics
                            .share_acceptance_rate_count
                            .fetch_add(1, Ordering::Relaxed);
                    }
                });
            },
        );
    }
    group.finish();
}

/// Benchmark error categorization metrics (Task 8.4)
/// Tests performance of error classification and counting
fn bench_error_categorization_metrics(c: &mut Criterion) {
    let metrics = Arc::new(AtomicMetrics::new());

    c.bench_function("error_categorization_dispatch", |b| {
        let error_types = [
            "network",
            "auth",
            "timeout",
            "parse",
            "version",
            "message",
            "validation",
            "security",
            "resource",
            "internal",
        ];
        let mut counter = 0;

        b.iter(|| {
            let error_type = error_types[counter % error_types.len()];
            counter += 1;

            // Simulate error categorization logic
            match black_box(error_type) {
                "network" => metrics.network_errors_total.fetch_add(1, Ordering::Relaxed),
                "auth" => metrics
                    .authentication_failures_total
                    .fetch_add(1, Ordering::Relaxed),
                "timeout" => metrics.timeout_errors_total.fetch_add(1, Ordering::Relaxed),
                "parse" => metrics
                    .protocol_parse_errors_total
                    .fetch_add(1, Ordering::Relaxed),
                "version" => metrics
                    .protocol_version_errors_total
                    .fetch_add(1, Ordering::Relaxed),
                "message" => metrics
                    .protocol_message_errors_total
                    .fetch_add(1, Ordering::Relaxed),
                "validation" => metrics
                    .validation_errors_total
                    .fetch_add(1, Ordering::Relaxed),
                "security" => metrics
                    .security_violation_errors_total
                    .fetch_add(1, Ordering::Relaxed),
                "resource" => metrics
                    .resource_exhaustion_errors_total
                    .fetch_add(1, Ordering::Relaxed),
                "internal" => metrics
                    .internal_errors_total
                    .fetch_add(1, Ordering::Relaxed),
                _ => 0,
            };
        });
    });

    // Error burst simulation
    c.bench_function("error_burst_handling", |b| {
        b.iter(|| {
            // Simulate handling multiple errors at once (burst scenario)
            metrics
                .network_errors_total
                .fetch_add(black_box(3), Ordering::Relaxed);
            metrics
                .timeout_errors_total
                .fetch_add(black_box(2), Ordering::Relaxed);
            metrics
                .protocol_parse_errors_total
                .fetch_add(black_box(1), Ordering::Relaxed);
            metrics
                .authentication_failures_total
                .fetch_add(black_box(1), Ordering::Relaxed);
        });
    });

    // Error pattern analysis benchmark
    let mut group = c.benchmark_group("error_pattern_analysis");
    for error_count in [10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*error_count as u64));
        group.bench_with_input(
            BenchmarkId::new("errors_per_batch", error_count),
            error_count,
            |b, &error_count| {
                b.iter(|| {
                    for i in 0..error_count {
                        match i % 10 {
                            0..=3 => metrics.network_errors_total.fetch_add(1, Ordering::Relaxed),
                            4..=5 => metrics.timeout_errors_total.fetch_add(1, Ordering::Relaxed),
                            6 => metrics
                                .authentication_failures_total
                                .fetch_add(1, Ordering::Relaxed),
                            7 => metrics
                                .protocol_parse_errors_total
                                .fetch_add(1, Ordering::Relaxed),
                            8 => metrics
                                .validation_errors_total
                                .fetch_add(1, Ordering::Relaxed),
                            9 => metrics
                                .internal_errors_total
                                .fetch_add(1, Ordering::Relaxed),
                            _ => unreachable!(),
                        };
                    }
                });
            },
        );
    }
    group.finish();
}

/// Benchmark resource utilization metrics (Task 8.5)
/// Tests memory and CPU tracking performance
fn bench_resource_utilization_metrics(c: &mut Criterion) {
    let metrics = Arc::new(AtomicMetrics::new());

    c.bench_function("memory_utilization_recording", |b| {
        b.iter(|| {
            let memory_per_conn = black_box(1024_f64); // 1KB per connection
            let memory_bits = memory_per_conn.to_bits();

            metrics
                .memory_usage_per_connection_sum_bits
                .fetch_add(memory_bits, Ordering::Relaxed);
            metrics
                .memory_usage_per_connection_count
                .fetch_add(1, Ordering::Relaxed);

            // Update min/max memory usage
            let current_max = metrics
                .memory_usage_per_connection_max_bits
                .load(Ordering::Relaxed);
            if memory_bits > current_max {
                let _ = metrics
                    .memory_usage_per_connection_max_bits
                    .compare_exchange_weak(
                        current_max,
                        memory_bits,
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                    );
            }

            let total_memory = black_box(1024 * 1000_u64); // 1MB total
            metrics
                .connection_memory_total_bytes
                .store(total_memory, Ordering::Relaxed);

            let peak_memory = metrics.connection_memory_peak_bytes.load(Ordering::Relaxed);
            if total_memory > peak_memory {
                let _ = metrics.connection_memory_peak_bytes.compare_exchange_weak(
                    peak_memory,
                    total_memory,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                );
            }
        });
    });

    c.bench_function("cpu_utilization_sampling", |b| {
        b.iter(|| {
            let cpu_usage = black_box(45.5_f64); // 45.5% CPU usage
            let timestamp = black_box(1699891200_f64); // Unix timestamp

            metrics
                .cpu_utilization_sample_bits
                .store(cpu_usage.to_bits(), Ordering::Relaxed);
            metrics.cpu_sample_count.fetch_add(1, Ordering::Relaxed);
            metrics
                .cpu_sample_timestamp_bits
                .store(timestamp.to_bits(), Ordering::Relaxed);

            // Calculate memory efficiency ratio
            let efficiency_ratio = black_box(0.85_f64); // 85% efficiency
            metrics
                .memory_efficiency_ratio_bits
                .store(efficiency_ratio.to_bits(), Ordering::Relaxed);
        });
    });

    c.bench_function("network_bandwidth_tracking", |b| {
        b.iter(|| {
            let rx_bytes_per_sec = black_box(1024 * 1024_u64); // 1MB/s rx
            let tx_bytes_per_sec = black_box(512 * 1024_u64); // 512KB/s tx

            metrics
                .network_bandwidth_rx_bytes_per_sec
                .store(rx_bytes_per_sec, Ordering::Relaxed);
            metrics
                .network_bandwidth_tx_bytes_per_sec
                .store(tx_bytes_per_sec, Ordering::Relaxed);

            // Resource pressure events
            if black_box(false) {
                // Simulate pressure condition
                metrics
                    .resource_pressure_events
                    .fetch_add(1, Ordering::Relaxed);
            }
        });
    });

    // Resource monitoring batch operations
    let mut group = c.benchmark_group("resource_metrics_batch");
    for samples in [10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*samples as u64));
        group.bench_with_input(
            BenchmarkId::new("samples_per_batch", samples),
            samples,
            |b, &samples| {
                b.iter(|| {
                    for i in 0..samples {
                        let memory_usage = (1000 + i) as f64;
                        let cpu_usage = (30.0 + (i as f64 * 0.1)) % 100.0;

                        metrics
                            .memory_usage_per_connection_sum_bits
                            .fetch_add(memory_usage.to_bits(), Ordering::Relaxed);
                        metrics
                            .cpu_utilization_sample_bits
                            .store(cpu_usage.to_bits(), Ordering::Relaxed);
                        metrics.cpu_sample_count.fetch_add(1, Ordering::Relaxed);
                    }
                });
            },
        );
    }
    group.finish();
}

/// Comprehensive benchmark comparing new metrics against baseline performance
fn bench_new_metrics_vs_baseline(c: &mut Criterion) {
    let metrics = Arc::new(AtomicMetrics::new());

    // Baseline: original atomic operations
    c.bench_function("baseline_original_metrics", |b| {
        b.iter(|| {
            metrics.total_connections.fetch_add(1, Ordering::Relaxed);
            metrics.messages_received.fetch_add(1, Ordering::Relaxed);
            metrics
                .bytes_received
                .fetch_add(black_box(512), Ordering::Relaxed);
            metrics.auth_attempts.fetch_add(1, Ordering::Relaxed);
        });
    });

    // New metrics: all categories combined
    c.bench_function("new_metrics_comprehensive", |b| {
        b.iter(|| {
            // Connection lifecycle (Task 8.1)
            let duration_bits = black_box(150_f64).to_bits();
            metrics
                .connection_duration_sum_ms_bits
                .fetch_add(duration_bits, Ordering::Relaxed);
            metrics
                .connection_established_counter
                .fetch_add(1, Ordering::Relaxed);

            // Protocol detection (Task 8.2)
            metrics
                .stratum_requests_total
                .fetch_add(1, Ordering::Relaxed);
            metrics
                .protocol_detection_successes
                .fetch_add(1, Ordering::Relaxed);

            // Mining operations (Task 8.3)
            metrics
                .share_submissions_total
                .fetch_add(1, Ordering::Relaxed);
            metrics
                .share_acceptance_rate_sum_bits
                .fetch_add(0.95_f64.to_bits(), Ordering::Relaxed);

            // Error categorization (Task 8.4)
            metrics
                .network_errors_total
                .fetch_add(black_box(0), Ordering::Relaxed);

            // Resource utilization (Task 8.5)
            metrics
                .memory_usage_per_connection_sum_bits
                .fetch_add(black_box(1024_f64).to_bits(), Ordering::Relaxed);
            metrics
                .cpu_utilization_sample_bits
                .store(black_box(45.0_f64).to_bits(), Ordering::Relaxed);
        });
    });

    // Performance ratio analysis
    c.bench_function("performance_overhead_analysis", |b| {
        let start = Instant::now();
        b.iter(|| {
            // Measure baseline operation
            let baseline_start = Instant::now();
            metrics.total_connections.fetch_add(1, Ordering::Relaxed);
            let baseline_duration = baseline_start.elapsed();

            // Measure new metric operation
            let new_metric_start = Instant::now();
            metrics
                .connection_established_counter
                .fetch_add(1, Ordering::Relaxed);
            let new_metric_duration = new_metric_start.elapsed();

            black_box((baseline_duration, new_metric_duration));
        });
        let total_duration = start.elapsed();
        black_box(total_duration);
    });
}

/// Benchmark atomic operations bit manipulation performance
fn bench_atomic_bit_operations(c: &mut Criterion) {
    let metrics = Arc::new(AtomicMetrics::new());

    c.bench_function("float_to_bits_conversion", |b| {
        b.iter(|| {
            let value = black_box(std::f64::consts::PI);
            let bits = value.to_bits();
            metrics
                .avg_response_time_ms_bits
                .store(bits, Ordering::Relaxed);

            // Reverse conversion
            let loaded_bits = metrics.avg_response_time_ms_bits.load(Ordering::Relaxed);
            let reconstructed = f64::from_bits(loaded_bits);
            black_box(reconstructed);
        });
    });

    c.bench_function("atomic_compare_and_swap", |b| {
        b.iter(|| {
            let new_value = black_box(100_u64);
            let current = metrics
                .connection_duration_max_ms_bits
                .load(Ordering::Relaxed);

            if new_value > current {
                let _ = metrics
                    .connection_duration_max_ms_bits
                    .compare_exchange_weak(
                        current,
                        new_value,
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                    );
            }
        });
    });

    c.bench_function("atomic_fetch_operations", |b| {
        b.iter(|| {
            metrics
                .connection_duration_count
                .fetch_add(1, Ordering::Relaxed);
            metrics
                .share_submissions_total
                .fetch_add(1, Ordering::Relaxed);
            metrics
                .protocol_detection_successes
                .fetch_add(1, Ordering::Relaxed);
            metrics.cpu_sample_count.fetch_add(1, Ordering::Relaxed);
        });
    });
}

/// Stress test for concurrent access to all new metrics
fn bench_concurrent_new_metrics_stress(c: &mut Criterion) {
    let metrics = Arc::new(AtomicMetrics::new());

    let mut group = c.benchmark_group("concurrent_new_metrics_stress");

    for threads in [1, 2, 4, 8, 16].iter() {
        group.bench_with_input(
            BenchmarkId::new("threads", threads),
            threads,
            |b, &threads| {
                let metrics = Arc::clone(&metrics);
                b.iter(|| {
                    let handles: Vec<_> = (0..threads)
                        .map(|thread_id| {
                            let metrics = Arc::clone(&metrics);
                            std::thread::spawn(move || {
                                for i in 0..1000 {
                                    let value = (thread_id * 1000 + i) as u64;

                                    // Connection lifecycle metrics
                                    metrics
                                        .connection_established_counter
                                        .fetch_add(1, Ordering::Relaxed);
                                    metrics
                                        .connection_duration_sum_ms_bits
                                        .fetch_add((value as f64).to_bits(), Ordering::Relaxed);

                                    // Protocol detection metrics
                                    if i % 2 == 0 {
                                        metrics.http_requests_total.fetch_add(1, Ordering::Relaxed);
                                    } else {
                                        metrics
                                            .stratum_requests_total
                                            .fetch_add(1, Ordering::Relaxed);
                                    }

                                    // Mining operation metrics
                                    metrics
                                        .share_submissions_total
                                        .fetch_add(1, Ordering::Relaxed);
                                    if i % 20 != 0 {
                                        metrics
                                            .share_acceptance_rate_sum_bits
                                            .fetch_add(1.0_f64.to_bits(), Ordering::Relaxed);
                                    }

                                    // Error categorization
                                    if i % 50 == 0 {
                                        match thread_id % 4 {
                                            0 => metrics
                                                .network_errors_total
                                                .fetch_add(1, Ordering::Relaxed),
                                            1 => metrics
                                                .timeout_errors_total
                                                .fetch_add(1, Ordering::Relaxed),
                                            2 => metrics
                                                .authentication_failures_total
                                                .fetch_add(1, Ordering::Relaxed),
                                            3 => metrics
                                                .protocol_parse_errors_total
                                                .fetch_add(1, Ordering::Relaxed),
                                            _ => unreachable!(),
                                        };
                                    }

                                    // Resource utilization
                                    metrics
                                        .memory_usage_per_connection_sum_bits
                                        .fetch_add((value as f64).to_bits(), Ordering::Relaxed);
                                    metrics
                                        .cpu_utilization_sample_bits
                                        .store((value as f64 % 100.0).to_bits(), Ordering::Relaxed);
                                }
                            })
                        })
                        .collect();

                    for handle in handles {
                        handle.join().unwrap();
                    }
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    new_metrics_benches,
    bench_connection_lifecycle_metrics,
    bench_protocol_detection_metrics,
    bench_mining_operation_metrics,
    bench_error_categorization_metrics,
    bench_resource_utilization_metrics,
    bench_new_metrics_vs_baseline,
    bench_atomic_bit_operations,
    bench_concurrent_new_metrics_stress
);

criterion_main!(new_metrics_benches);
