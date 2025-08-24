use loka_metrics::Histogram;
use metrics::HistogramFn;
use std::time::Instant;

fn main() {
    println!("=== Histogram Slot Distribution Demo ===\n");

    // Create histogram with bounds 0.0 to 120.0
    let histogram = Histogram::new();
    histogram.set_bounds(0.0, 120.0);

    println!("Bounds: ({}, {})", histogram.lower(), histogram.upper());
    println!("Range: {}", histogram.upper() - histogram.lower());
    println!(
        "Slot width: {:.2} units per slot\n",
        (histogram.upper() - histogram.lower()) / 12.0
    );

    // Record some values and show which slots they go to
    let test_values = vec![10.0, 30.0, 60.0, 90.0, 110.0];

    for value in &test_values {
        let slot = calculate_slot(*value, histogram.lower(), histogram.upper());
        println!(
            "Value: {:>6.1} → Slot: {:>2} (range: {:>6.1}-{:<6.1})",
            value,
            slot,
            slot as f64 * 10.0,
            (slot + 1) as f64 * 10.0
        );

        histogram.record(*value);
    }

    println!("\n=== After Recording Values ===");
    println!("Total count: {}", histogram.count());
    println!("Sum: {:.1}", histogram.sum());
    println!("Mean: {:.2}", histogram.mean());
    println!("Min: {:.1}", histogram.min());
    println!("Max: {:.1}", histogram.max());

    // Show slot distribution (we can't access private slots, so we'll show the math)
    println!("\n=== Slot Distribution Math ===");
    for value in &test_values {
        let slot = calculate_slot(*value, histogram.lower(), histogram.upper());
        let range_start = slot as f64 * 10.0;
        let range_end = (slot + 1) as f64 * 10.0;
        println!(
            "Value {value:>6.1} → Slot {slot:>2} [{range_start:>6.1}, {range_end:<6.1})"
        );
    }

    // Run benchmarks
    println!("\n=== Performance Benchmarks ===");
    benchmark_histogram_operations();
    benchmark_record_many_vs_single();
    benchmark_concurrent_operations();
}

fn calculate_slot(value: f64, lower: f64, upper: f64) -> usize {
    let range = upper - lower;
    if range == 0.0 {
        0
    } else {
        let normalized = (value - lower) / range;
        let scaled = normalized * 12.0;
        let slot = scaled as usize;
        slot.min(11)
    }
}

fn benchmark_histogram_operations() {
    println!("\n--- Histogram Operations Benchmark ---");

    let histogram = Histogram::new();
    histogram.set_bounds(0.0, 1000.0);

    // Benchmark single record operations
    let iterations = 100_000;
    let start = Instant::now();

    for i in 0..iterations {
        histogram.record(i as f64);
    }

    let duration = start.elapsed();
    let ops_per_sec = iterations as f64 / duration.as_secs_f64();

    println!("Single record operations:");
    println!("  Iterations: {iterations}");
    println!("  Duration: {duration:.2?}");
    println!("  Operations/sec: {ops_per_sec:.0}");
    println!(
        "  Average time per operation: {:.3} ns",
        duration.as_nanos() as f64 / iterations as f64
    );
}

fn benchmark_record_many_vs_single() {
    println!("\n--- Record Many vs Single Benchmark ---");

    let histogram1 = Histogram::new();
    let histogram2 = Histogram::new();
    histogram1.set_bounds(0.0, 1000.0);
    histogram2.set_bounds(0.0, 1000.0);

    let iterations = 100_000;
    let batch_size = 100;

    // Benchmark single record operations
    let start = Instant::now();
    for _ in 0..iterations {
        histogram1.record(42.0);
    }
    let single_duration = start.elapsed();

    // Benchmark batch record operations
    let start = Instant::now();
    for _ in 0..(iterations / batch_size) {
        histogram2.record_many(42.0, batch_size);
    }
    let batch_duration = start.elapsed();

    println!("Single record ({iterations} operations):");
    println!("  Duration: {single_duration:.2?}");
    println!(
        "  Operations/sec: {:.0}",
        iterations as f64 / single_duration.as_secs_f64()
    );

    println!(
        "Batch record ({} batches of {}):",
        iterations / batch_size,
        batch_size
    );
    println!("  Duration: {batch_duration:.2?}");
    println!(
        "  Operations/sec: {:.0}",
        iterations as f64 / batch_duration.as_secs_f64()
    );

    let speedup = single_duration.as_nanos() as f64 / batch_duration.as_nanos() as f64;
    println!("Speedup: {speedup:.2}x");
}

fn benchmark_concurrent_operations() {
    println!("\n--- Concurrent Operations Benchmark ---");

    use std::sync::Arc;
    use std::thread;

    let histogram = Arc::new(Histogram::new());
    histogram.set_bounds(0.0, 1000.0);

    let num_threads = 8;
    let operations_per_thread = 10_000;
    let total_operations = num_threads * operations_per_thread;

    let start = Instant::now();
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

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    let duration = start.elapsed();
    let ops_per_sec = total_operations as f64 / duration.as_secs_f64();

    println!("Concurrent operations:");
    println!("  Threads: {num_threads}");
    println!("  Operations per thread: {operations_per_thread}");
    println!("  Total operations: {total_operations}");
    println!("  Duration: {duration:.2?}");
    println!("  Operations/sec: {ops_per_sec:.0}");
    println!("  Final count: {}", histogram.count());
    println!("  Final sum: {:.1}", histogram.sum());
}
