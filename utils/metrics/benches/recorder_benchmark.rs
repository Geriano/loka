use criterion::{Criterion, criterion_group, criterion_main};
use loka_metrics::Recorder;
use metrics::{HistogramFn, counter, gauge, histogram};
use std::hint::black_box;

fn benchmark_metrics_recording(c: &mut Criterion) {
    let mut group = c.benchmark_group("metrics_recording");

    group.bench_function("counter_increment", |b| {
        b.iter(|| {
            counter!("benchmark.counter").increment(black_box(1));
        });
    });

    group.bench_function("gauge_set", |b| {
        b.iter(|| {
            gauge!("benchmark.gauge").set(black_box(42.0));
        });
    });

    group.bench_function("histogram_record", |b| {
        b.iter(|| {
            histogram!("benchmark.histogram").record(black_box(42.0));
        });
    });

    group.bench_function("histogram_record_many", |b| {
        b.iter(|| {
            histogram!("benchmark.histogram_batch").record_many(black_box(42.0), black_box(100));
        });
    });

    group.finish();
}

fn benchmark_data_retrieval(c: &mut Criterion) {
    let mut group = c.benchmark_group("data_retrieval");

    // Setup: populate some data first
    let recorder = Recorder::init().expect("Failed to initialize recorder");

    // Populate data
    for i in 0..1000 {
        counter!("retrieval.counter").increment(i as u64);
        gauge!("retrieval.gauge").set(i as f64);
        histogram!("retrieval.histogram").record(i as f64);
    }

    group.bench_function("counters_retrieval", |b| {
        b.iter(|| {
            black_box(recorder.counters());
        });
    });

    group.bench_function("gauges_retrieval", |b| {
        b.iter(|| {
            black_box(recorder.gauges());
        });
    });

    group.bench_function("histograms_retrieval", |b| {
        b.iter(|| {
            black_box(recorder.histograms());
        });
    });

    group.finish();
}

fn benchmark_mixed_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_operations");

    group.bench_function("typical_app_metrics", |b| {
        b.iter(|| {
            // Simulate typical application metrics
            counter!("app.requests.total").increment(black_box(1));
            gauge!("app.memory.usage").set(black_box(100.0));
            histogram!("app.response_time").record(black_box(50.0));

            // Simulate batch operations
            histogram!("app.batch.size").record_many(black_box(100.0), black_box(100));

            // Simulate alerts
            counter!("app.alerts.high_memory").increment(black_box(1));
        });
    });

    group.bench_function("high_frequency_metrics", |b| {
        b.iter(|| {
            for i in 0..10 {
                counter!("app.requests.total").increment(black_box(1));
                gauge!("app.memory.usage").set(black_box(100.0 + i as f64));
                histogram!("app.response_time").record(black_box(10.0 + i as f64));
            }
        });
    });

    group.finish();
}

fn benchmark_histogram_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("histogram_performance");

    // Test different histogram bounds with static names
    group.bench_function("record_small_range_0_100", |b| {
        b.iter(|| {
            histogram!("histogram_bounds_0_100").record(black_box(50.0));
        });
    });

    group.bench_function("record_medium_range_0_10000", |b| {
        b.iter(|| {
            histogram!("histogram_bounds_0_10000").record(black_box(5000.0));
        });
    });

    group.bench_function("record_large_range_default", |b| {
        b.iter(|| {
            histogram!("histogram_bounds_default").record(black_box(0.0));
        });
    });

    // Test record_many vs single record
    group.bench_function("record_many_vs_single", |b| {
        b.iter(|| {
            // Single record
            histogram!("histogram.single").record(black_box(42.0));

            // Batch record
            histogram!("histogram.batch").record_many(black_box(42.0), black_box(100));
        });
    });

    group.finish();
}

fn benchmark_concurrent_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_operations");

    use std::sync::Arc;
    use std::thread;

    group.bench_function("concurrent_histogram_access", |b| {
        b.iter(|| {
            let histogram = Arc::new(loka_metrics::Histogram::new());
            histogram.set_bounds(0.0, 1000.0);

            let num_threads = 4;
            let operations_per_thread = 1000;
            let mut handles = vec![];

            for thread_id in 0..num_threads {
                let histogram_clone = Arc::clone(&histogram);
                let handle = thread::spawn(move || {
                    for i in 0..operations_per_thread {
                        let value = (thread_id * operations_per_thread + i) as f64;
                        histogram_clone.record(value);
                    }
                });
                handles.push(handle);
            }

            for handle in handles {
                handle.join().unwrap();
            }

            black_box(histogram.count());
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_metrics_recording,
    benchmark_data_retrieval,
    benchmark_mixed_operations,
    benchmark_histogram_performance,
    benchmark_concurrent_operations
);
criterion_main!(benches);
