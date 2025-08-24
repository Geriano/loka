use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use dashmap::DashMap;
use loka_stratum::protocol::parser::StratumParser;
use loka_stratum::services::metrics::AtomicMetrics;
use loka_stratum::utils::hash_utils::{HashUtils, RollingHash};
use loka_stratum::utils::string_pool::StringPool;
use serde_json::Value;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::hint::black_box;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Instant;

/// Benchmark atomic metrics operations (critical for lock-free performance)
fn bench_atomic_metrics(c: &mut Criterion) {
    let metrics = Arc::new(AtomicMetrics::new());

    c.bench_function("atomic_metrics_single_thread", |b| {
        b.iter(|| {
            metrics.total_connections.fetch_add(1, Ordering::Relaxed);
            metrics.messages_received.fetch_add(1, Ordering::Relaxed);
            metrics
                .bytes_received
                .fetch_add(black_box(512), Ordering::Relaxed);
            metrics.auth_attempts.fetch_add(1, Ordering::Relaxed);
        })
    });

    // Add benchmark for new metrics operations to ensure consistent performance
    c.bench_function("new_metrics_single_thread", |b| {
        b.iter(|| {
            // Connection lifecycle metrics (Task 8.1)
            metrics
                .connection_established_counter
                .fetch_add(1, Ordering::Relaxed);
            metrics
                .connection_duration_sum_ms_bits
                .fetch_add(black_box(150_f64).to_bits(), Ordering::Relaxed);

            // Protocol detection metrics (Task 8.2)
            metrics
                .stratum_requests_total
                .fetch_add(1, Ordering::Relaxed);
            metrics
                .protocol_detection_successes
                .fetch_add(1, Ordering::Relaxed);

            // Mining operation metrics (Task 8.3)
            metrics
                .share_submissions_total
                .fetch_add(1, Ordering::Relaxed);
            metrics
                .job_distribution_latency_sum_ms_bits
                .fetch_add(black_box(5.0_f64).to_bits(), Ordering::Relaxed);

            // Error categorization metrics (Task 8.4)
            metrics
                .network_errors_total
                .fetch_add(black_box(0), Ordering::Relaxed);

            // Resource utilization metrics (Task 8.5)
            metrics
                .memory_usage_per_connection_sum_bits
                .fetch_add(black_box(1024.0_f64).to_bits(), Ordering::Relaxed);
        })
    });

    // Test concurrent access performance
    let mut group = c.benchmark_group("atomic_metrics_concurrent");
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
                                for _ in 0..1000 {
                                    metrics.total_connections.fetch_add(1, Ordering::Relaxed);
                                    metrics.messages_received.fetch_add(1, Ordering::Relaxed);
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

/// Benchmark Stratum message parsing (critical path for every incoming message)
fn bench_stratum_parsing(c: &mut Criterion) {
    let parser = StratumParser::new();

    // Test messages of varying complexity
    let simple_message = r#"{"id":1,"method":"mining.subscribe","params":[]}"#;
    let complex_message = r#"{"id":42,"method":"mining.submit","params":["username.worker1","job123","extranonce2","ntime","nonce"]}"#;
    let malformed_message = r#"{"id":1,"method":"mining.subscribe","params":[]"#; // Missing closing brace

    c.bench_function("parse_simple_message", |b| {
        b.iter(|| {
            let _result = parser.parse_message(black_box(simple_message));
            let _ = black_box(_result);
        })
    });

    c.bench_function("parse_complex_message", |b| {
        b.iter(|| {
            let _result = parser.parse_message(black_box(complex_message));
            let _ = black_box(_result);
        })
    });

    c.bench_function("parse_malformed_message", |b| {
        b.iter(|| {
            let _result = parser.parse_message(black_box(malformed_message));
            let _ = black_box(_result);
        })
    });

    // Throughput benchmark
    let mut group = c.benchmark_group("stratum_parsing_throughput");
    for size in [100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(
            BenchmarkId::new("messages_per_batch", size),
            size,
            |b, &size| {
                let messages: Vec<&str> = (0..size).map(|_| simple_message).collect();
                b.iter(|| {
                    for msg in &messages {
                        let _result = parser.parse_message(black_box(msg));
                        let _ = black_box(_result);
                    }
                });
            },
        );
    }
    group.finish();
}

/// Benchmark string operations (critical for message processing)
fn bench_string_operations(c: &mut Criterion) {
    c.bench_function("string_pool_intern", |b| {
        let pool = StringPool::new();
        let test_strings = [
            "mining.subscribe",
            "mining.authorize",
            "mining.notify",
            "mining.submit",
            "mining.set_difficulty",
        ];
        let mut counter = 0;

        b.iter(|| {
            let s = test_strings[counter % test_strings.len()];
            let interned = pool.intern(black_box(s));
            black_box(interned);
            counter += 1;
        })
    });

    c.bench_function("string_clone_vs_intern", |b| {
        let pool = StringPool::new();
        let test_string = "mining.subscribe.with.long.method.name";
        b.iter(|| {
            // Test traditional string cloning
            let cloned = black_box(test_string).to_string();
            black_box(cloned);

            // vs string interning
            let interned = pool.intern(black_box(test_string));
            black_box(interned);
        })
    });
}

/// Benchmark hash operations (critical for message routing and caching)
fn bench_hash_operations(c: &mut Criterion) {
    let test_data = b"mining.submit.job123.worker1.extranonce2.ntime.nonce";

    c.bench_function("fast_hash", |b| {
        b.iter(|| {
            let hash =
                HashUtils::fast_string_hash(black_box(std::str::from_utf8(test_data).unwrap()));
            black_box(hash);
        })
    });

    c.bench_function("rolling_hash", |b| {
        let mut hasher = RollingHash::new(32); // 32-byte window
        b.iter(|| {
            let hash = hasher.add_byte(black_box(b'x'));
            black_box(hash);
        })
    });

    // Benchmark hash distribution
    let mut group = c.benchmark_group("hash_distribution");
    for data_size in [32, 128, 512, 1024].iter() {
        group.bench_with_input(
            BenchmarkId::new("bytes", data_size),
            data_size,
            |b, &size| {
                let data = vec![0u8; size];
                b.iter(|| {
                    let mut hasher = DefaultHasher::new();
                    black_box(&data).hash(&mut hasher);
                    let hash = hasher.finish();
                    black_box(hash);
                });
            },
        );
    }
    group.finish();
}

/// Benchmark memory management (critical for high-throughput scenarios)
fn bench_memory_operations(c: &mut Criterion) {
    c.bench_function("vec_allocation", |b| {
        b.iter(|| {
            let vec = Vec::<u8>::with_capacity(black_box(1024));
            black_box(vec);
        })
    });

    c.bench_function("string_allocation", |b| {
        b.iter(|| {
            let string = String::with_capacity(black_box(256));
            black_box(string);
        })
    });

    // Compare allocation strategies
    c.bench_function("reuse_vs_allocate", |b| {
        let mut reusable_vec = Vec::with_capacity(1024);
        b.iter(|| {
            // Test reusing existing vector
            reusable_vec.clear();
            reusable_vec.extend_from_slice(black_box(&[1u8; 512]));
            black_box(&reusable_vec);

            // vs new allocation
            let new_vec = vec![1u8; 512];
            black_box(new_vec);
        })
    });
}

/// Benchmark JSON serialization/deserialization (critical for protocol messages)
fn bench_json_operations(c: &mut Criterion) {
    let subscribe_msg = serde_json::json!({
        "id": 1,
        "method": "mining.subscribe",
        "params": []
    });

    let submit_msg = serde_json::json!({
        "id": 42,
        "method": "mining.submit",
        "params": ["username.worker1", "job123", "00000000", "ntime", "nonce"]
    });

    c.bench_function("serialize_simple", |b| {
        b.iter(|| {
            let json = serde_json::to_string(&black_box(&subscribe_msg)).unwrap();
            black_box(json);
        })
    });

    c.bench_function("serialize_complex", |b| {
        b.iter(|| {
            let json = serde_json::to_string(&black_box(&submit_msg)).unwrap();
            black_box(json);
        })
    });

    let subscribe_str = serde_json::to_string(&subscribe_msg).unwrap();
    let submit_str = serde_json::to_string(&submit_msg).unwrap();

    c.bench_function("deserialize_simple", |b| {
        b.iter(|| {
            let msg: Value = serde_json::from_str(black_box(&subscribe_str)).unwrap();
            black_box(msg);
        })
    });

    c.bench_function("deserialize_complex", |b| {
        b.iter(|| {
            let msg: Value = serde_json::from_str(black_box(&submit_str)).unwrap();
            black_box(msg);
        })
    });

    // Benchmark with Vec<u8> vs String
    c.bench_function("json_from_bytes", |b| {
        let subscribe_bytes = subscribe_str.as_bytes();
        b.iter(|| {
            let msg: Value = serde_json::from_slice(black_box(subscribe_bytes)).unwrap();
            black_box(msg);
        })
    });
}

/// Benchmark message validation (critical for security and correctness)
fn bench_message_validation(c: &mut Criterion) {
    let valid_messages = vec![
        r#"{"id":1,"method":"mining.subscribe","params":[]}"#,
        r#"{"id":2,"method":"mining.authorize","params":["user","pass"]}"#,
        r#"{"id":3,"method":"mining.submit","params":["user","job","nonce2","time","nonce"]}"#,
    ];

    let invalid_messages = vec![
        r#"{"id":"invalid","method":"mining.subscribe","params":[]}"#, // Invalid ID type
        r#"{"method":"mining.authorize","params":["user"]}"#, // Missing ID, insufficient params
        r#"{"id":3,"method":"invalid.method","params":[]}"#,  // Invalid method
    ];

    c.bench_function("validate_valid_messages", |b| {
        b.iter(|| {
            for msg_str in &valid_messages {
                if let Ok(msg) = serde_json::from_str::<Value>(black_box(msg_str)) {
                    // Basic validation checks
                    let has_id = msg.get("id").is_some();
                    let has_method = msg.get("method").and_then(|m| m.as_str()).is_some();
                    let has_params = msg.get("params").is_some();
                    let valid = has_id && has_method && has_params;
                    black_box(valid);
                }
            }
        })
    });

    c.bench_function("validate_invalid_messages", |b| {
        b.iter(|| {
            for msg_str in &invalid_messages {
                if let Ok(msg) = serde_json::from_str::<Value>(black_box(msg_str)) {
                    // Same validation checks
                    let has_id = msg.get("id").is_some();
                    let has_method = msg.get("method").and_then(|m| m.as_str()).is_some();
                    let has_params = msg.get("params").is_some();
                    let valid = has_id && has_method && has_params;
                    black_box(valid);
                }
            }
        })
    });
}

/// Benchmark concurrent data structures (critical for thread-safe operations)
fn bench_concurrent_structures(c: &mut Criterion) {
    use std::sync::RwLock;

    let dashmap = Arc::new(DashMap::<String, u64>::new());
    let hashmap = Arc::new(RwLock::new(HashMap::<String, u64>::new()));

    // Populate with some data
    for i in 0..1000 {
        dashmap.insert(format!("key_{i}"), i);
        hashmap.write().unwrap().insert(format!("key_{i}"), i);
    }

    c.bench_function("dashmap_read", |b| {
        b.iter(|| {
            let key = format!("key_{}", black_box(500));
            let value = dashmap.get(&key);
            black_box(value);
        })
    });

    c.bench_function("hashmap_read", |b| {
        b.iter(|| {
            let key = format!("key_{}", black_box(500));
            let guard = hashmap.read().unwrap();
            let value = guard.get(&key);
            black_box(value);
        })
    });

    c.bench_function("dashmap_write", |b| {
        let mut counter = 0;
        b.iter(|| {
            let key = format!("new_key_{counter}");
            dashmap.insert(key, black_box(counter));
            counter += 1;
        })
    });

    c.bench_function("hashmap_write", |b| {
        let mut counter = 0;
        b.iter(|| {
            let key = format!("new_key_{counter}");
            hashmap.write().unwrap().insert(key, black_box(counter));
            counter += 1;
        })
    });
}

/// Benchmark time-sensitive operations (critical for mining timing)
fn bench_time_operations(c: &mut Criterion) {
    c.bench_function("instant_now", |b| {
        b.iter(|| {
            let now = Instant::now();
            black_box(now);
        })
    });

    c.bench_function("system_time", |b| {
        b.iter(|| {
            let now = std::time::SystemTime::now();
            black_box(now);
        })
    });

    // Benchmark timing calculations
    let start = Instant::now();
    c.bench_function("duration_calculation", |b| {
        b.iter(|| {
            let elapsed = start.elapsed();
            let millis = elapsed.as_millis();
            black_box(millis);
        })
    });

    // Benchmark high-frequency timing
    c.bench_function("duration_as_nanos", |b| {
        let start = Instant::now();
        b.iter(|| {
            let elapsed = start.elapsed();
            let nanos = elapsed.as_nanos();
            black_box(nanos);
        })
    });
}

/// Benchmark protocol method parsing (critical for message routing)
fn bench_protocol_methods(c: &mut Criterion) {
    let methods = [
        "mining.subscribe",
        "mining.authorize",
        "mining.notify",
        "mining.submit",
        "mining.set_difficulty",
    ];

    c.bench_function("method_string_matching", |b| {
        let mut counter = 0;
        b.iter(|| {
            let method = methods[counter % methods.len()];
            let result = match black_box(method) {
                "mining.subscribe" => 1,
                "mining.authorize" => 2,
                "mining.notify" => 3,
                "mining.submit" => 4,
                "mining.set_difficulty" => 5,
                _ => 0,
            };
            black_box(result);
            counter += 1;
        })
    });

    // Compare with hash-based lookup
    let mut method_map = HashMap::new();
    method_map.insert("mining.subscribe", 1);
    method_map.insert("mining.authorize", 2);
    method_map.insert("mining.notify", 3);
    method_map.insert("mining.submit", 4);
    method_map.insert("mining.set_difficulty", 5);

    c.bench_function("method_hash_lookup", |b| {
        let mut counter = 0;
        b.iter(|| {
            let method = methods[counter % methods.len()];
            let result = method_map.get(black_box(method)).copied().unwrap_or(0);
            black_box(result);
            counter += 1;
        })
    });
}

criterion_group!(
    benches,
    bench_atomic_metrics,
    bench_stratum_parsing,
    bench_string_operations,
    bench_hash_operations,
    bench_memory_operations,
    bench_json_operations,
    bench_message_validation,
    bench_concurrent_structures,
    bench_time_operations,
    bench_protocol_methods
);

criterion_main!(benches);
