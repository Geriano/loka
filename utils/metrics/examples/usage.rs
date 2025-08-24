use loka_metrics::Recorder;

fn main() {
    tracing_subscriber::fmt().init();

    let recorder = Recorder::init().expect("Failed to initialize metrics recorder");

    println!("=== Real-World Metrics Example ===\n");

    // Simulate CPU usage monitoring
    println!("--- CPU Usage Monitoring ---");
    simulate_cpu_monitoring();

    // Simulate memory usage monitoring
    println!("--- Memory Usage Monitoring ---");
    simulate_memory_monitoring();

    // Simulate response time monitoring
    println!("--- Response Time Monitoring ---");
    simulate_response_time_monitoring();

    // Simulate batch operations
    println!("--- Batch Operations ---");
    simulate_batch_operations();

    println!("\n=== Final Metrics Summary ===");
    println!("Counters: {:#?}", recorder.counters());
    println!("Gauges: {:#?}", recorder.gauges());
    println!("Histograms: {:#?}", recorder.histograms());
}

fn simulate_cpu_monitoring() {
    // Simulate CPU usage over time
    let cpu_readings = [15.2, 23.7, 45.1, 67.8, 89.2, 95.5, 78.3, 56.7, 34.2, 12.8];

    for (i, usage) in cpu_readings.iter().enumerate() {
        // Record individual CPU readings
        metrics::histogram!("cpu_usage_percent").record(*usage);

        // Update CPU gauge with current value
        metrics::gauge!("cpu_current_percent").set(*usage);

        // Increment CPU measurement counter
        metrics::counter!("cpu_measurements_total").increment(1);

        if *usage > 80.0 {
            metrics::counter!("cpu_high_usage_alerts").increment(1);
        }

        println!("  CPU Reading {}: {:.1}%", i + 1, usage);
    }
}

fn simulate_memory_monitoring() {
    // Simulate memory usage in MB
    let memory_readings = [128.5, 256.0, 512.3, 1024.7, 2048.1, 4096.5, 8192.0];

    for (i, usage) in memory_readings.iter().enumerate() {
        // Record memory usage histogram
        metrics::histogram!("memory_usage_mb").record(*usage);

        // Update memory gauge
        metrics::gauge!("memory_current_mb").set(*usage);

        // Increment memory measurement counter
        metrics::counter!("memory_measurements_total").increment(1);

        // Categorize memory usage
        if *usage > 4096.0 {
            metrics::counter!("memory_high_usage_alerts").increment(1);
        } else if *usage < 512.0 {
            metrics::counter!("memory_low_usage_alerts").increment(1);
        }

        println!("  Memory Reading {}: {:.1} MB", i + 1, usage);
    }
}

fn simulate_response_time_monitoring() {
    // Simulate API response times in milliseconds
    let response_times = [12.5, 45.2, 78.9, 156.3, 234.7, 456.1, 789.5, 1234.7];

    for (i, time) in response_times.iter().enumerate() {
        // Record response time histogram
        metrics::histogram!("api_response_time_ms").record(*time);

        // Update current response time gauge
        metrics::gauge!("api_current_response_time_ms").set(*time);

        // Increment request counter
        metrics::counter!("api_requests_total").increment(1);

        // Categorize response times
        if *time < 100.0 {
            metrics::counter!("api_response_time_fast").increment(1);
        } else if *time < 500.0 {
            metrics::counter!("api_response_time_normal").increment(1);
        } else {
            metrics::counter!("api_response_time_slow").increment(1);
        }

        println!("  Response Time {}: {:.1} ms", i + 1, time);
    }
}

fn simulate_batch_operations() {
    // Simulate batch processing with record_many
    let batch_sizes = [100, 500, 1000, 5000, 10000];

    for (i, batch_size) in batch_sizes.iter().enumerate() {
        // Simulate processing time based on batch size
        let processing_time = (*batch_size) as f64 * 0.001; // 1ms per item

        // Record batch processing time histogram
        metrics::histogram!("batch_processing_time_ms").record(processing_time);

        // Record batch size histogram
        metrics::histogram!("batch_size_items").record((*batch_size) as f64);

        // Update current batch size gauge
        metrics::gauge!("batch_current_size_items").set((*batch_size) as f64);

        // Increment batch counter
        metrics::counter!("batch_operations_total").increment(1);

        // Categorize batch sizes
        if *batch_size < 1000 {
            metrics::counter!("batch_size_small").increment(1);
        } else if *batch_size < 5000 {
            metrics::counter!("batch_size_medium").increment(1);
        } else {
            metrics::counter!("batch_size_large").increment(1);
        }

        println!(
            "  Batch {}: {} items, {:.1} ms processing time",
            i + 1,
            batch_size,
            processing_time
        );
    }
}
