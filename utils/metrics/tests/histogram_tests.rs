use loka_metrics::Histogram;
use metrics::HistogramFn;

#[test]
fn test_histogram_new() {
    let histogram = Histogram::new();

    assert_eq!(histogram.count(), 0);
    assert_eq!(histogram.sum(), 0.0);
    assert_eq!(histogram.min(), 0.0);
    assert_eq!(histogram.max(), 0.0);
    assert_eq!(histogram.bounds(), (-1e6, 1e6));
}

#[test]
fn test_histogram_default() {
    let histogram = Histogram::default();

    assert_eq!(histogram.count(), 0);
    assert_eq!(histogram.sum(), 0.0);
    assert_eq!(histogram.min(), 0.0);
    assert_eq!(histogram.max(), 0.0);
    assert_eq!(histogram.bounds(), (-1e6, 1e6));
}

#[test]
fn test_histogram_set_bounds() {
    let histogram = Histogram::new();

    histogram.set_bounds(0.0, 100.0);
    assert_eq!(histogram.bounds(), (0.0, 100.0));
    assert_eq!(histogram.lower(), 0.0);
    assert_eq!(histogram.upper(), 100.0);

    // Test invalid bounds (lower > upper)
    histogram.set_bounds(100.0, 0.0);
    assert_eq!(histogram.bounds(), (0.0, 100.0)); // Should remain unchanged
}

#[test]
fn test_histogram_record_single_value_with_bounds() {
    let histogram = Histogram::new();
    histogram.set_bounds(0.0, 100.0);

    histogram.record(42.5);

    assert_eq!(histogram.count(), 1);
    assert_eq!(histogram.sum(), 42.5);
    assert_eq!(histogram.min(), 42.5);
    assert_eq!(histogram.max(), 42.5);
    assert_eq!(histogram.mean(), 42.5);
}

#[test]
fn test_histogram_record_multiple_values_with_bounds() {
    let histogram = Histogram::new();
    histogram.set_bounds(0.0, 100.0);

    histogram.record(10.0);
    histogram.record(20.0);
    histogram.record(30.0);

    assert_eq!(histogram.count(), 3);
    assert_eq!(histogram.sum(), 60.0);
    assert_eq!(histogram.min(), 10.0);
    assert_eq!(histogram.max(), 30.0);
    assert_eq!(histogram.mean(), 20.0);
}

#[test]
fn test_histogram_record_negative_values_with_bounds() {
    let histogram = Histogram::new();
    histogram.set_bounds(-100.0, 100.0);

    histogram.record(-5.0);
    histogram.record(-10.0);

    assert_eq!(histogram.count(), 2);
    assert_eq!(histogram.sum(), -15.0);
    assert_eq!(histogram.min(), -10.0);
    assert_eq!(histogram.max(), -5.0);
    assert_eq!(histogram.mean(), -7.5);
}

#[test]
fn test_histogram_record_zero_values() {
    let histogram = Histogram::new();

    histogram.record(0.0);
    histogram.record(0.0);

    assert_eq!(histogram.count(), 2);
    assert_eq!(histogram.sum(), 0.0);
    assert_eq!(histogram.min(), 0.0);
    assert_eq!(histogram.max(), 0.0);
    assert_eq!(histogram.mean(), 0.0);
}

#[test]
fn test_histogram_record_large_values_with_bounds() {
    let histogram = Histogram::new();
    histogram.set_bounds(0.0, 1e16);

    let large_value = 1e15;
    histogram.record(large_value);

    assert_eq!(histogram.count(), 1);
    assert_eq!(histogram.sum(), large_value);
    assert_eq!(histogram.min(), large_value);
    assert_eq!(histogram.max(), large_value);
    assert_eq!(histogram.mean(), large_value);
}

#[test]
fn test_histogram_record_small_values_with_bounds() {
    let histogram = Histogram::new();
    histogram.set_bounds(0.0, 1.0);

    let small_value = 1e-15;
    histogram.record(small_value);

    assert_eq!(histogram.count(), 1);
    assert_eq!(histogram.sum(), small_value);
    assert_eq!(histogram.min(), small_value);
    assert_eq!(histogram.max(), small_value);
    assert_eq!(histogram.mean(), small_value);
}

#[test]
fn test_histogram_record_infinity_with_bounds() {
    let histogram = Histogram::new();
    histogram.set_bounds(-1e6, 1e6);

    histogram.record(f64::INFINITY);

    assert_eq!(histogram.count(), 1);
    // Infinity should be clamped to upper bound
    assert_eq!(histogram.sum(), 1e6);
    assert_eq!(histogram.min(), 1e6);
    assert_eq!(histogram.max(), 1e6);
    assert_eq!(histogram.mean(), 1e6);
}

#[test]
fn test_histogram_record_negative_infinity_with_bounds() {
    let histogram = Histogram::new();
    histogram.set_bounds(-1e6, 1e6);

    histogram.record(f64::NEG_INFINITY);

    assert_eq!(histogram.count(), 1);
    // Negative infinity should be clamped to lower bound
    assert_eq!(histogram.sum(), -1e6);
    assert_eq!(histogram.min(), -1e6);
    assert_eq!(histogram.max(), -1e6);
    assert_eq!(histogram.mean(), -1e6);
}

#[test]
fn test_histogram_record_nan() {
    let histogram = Histogram::new();

    histogram.record(f64::NAN);

    // NaN values should not be recorded
    assert_eq!(histogram.count(), 0);
    assert_eq!(histogram.sum(), 0.0);
}

#[test]
fn test_histogram_mean_with_zero_count() {
    let histogram = Histogram::new();

    assert_eq!(histogram.mean(), 0.0);
}

#[test]
fn test_histogram_reset() {
    let histogram = Histogram::new();
    histogram.set_bounds(0.0, 100.0);

    // Record some values
    histogram.record(10.0);
    histogram.record(20.0);

    assert_eq!(histogram.count(), 2);
    assert_eq!(histogram.sum(), 30.0);

    // Reset
    histogram.reset();

    assert_eq!(histogram.count(), 0);
    assert_eq!(histogram.sum(), 0.0);
    assert_eq!(histogram.min(), 0.0);
    assert_eq!(histogram.max(), 0.0);
    assert_eq!(histogram.mean(), 0.0);
    // Bounds should remain unchanged after reset
    assert_eq!(histogram.bounds(), (0.0, 100.0));
}

#[test]
fn test_histogram_concurrent_access() {
    use std::sync::Arc;
    use std::thread;

    let histogram = Arc::new(Histogram::new());
    histogram.set_bounds(0.0, 1000.0);
    let mut handles = vec![];

    // Spawn multiple threads to record values concurrently
    for i in 0..10 {
        let histogram_clone = Arc::clone(&histogram);
        let handle = thread::spawn(move || {
            for j in 0..100 {
                let value = (i * 100 + j) as f64;
                histogram_clone.record(value);
            }
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify the results
    assert_eq!(histogram.count(), 1000);
    assert_eq!(histogram.min(), 0.0);
    assert_eq!(histogram.max(), 999.0);

    // The sum should be the sum of all values from 0 to 999
    let expected_sum: f64 = (0..1000).map(|i| i as f64).sum();
    assert!((histogram.sum() - expected_sum).abs() < f64::EPSILON);
}

#[test]
fn test_histogram_edge_cases_with_bounds() {
    let histogram = Histogram::new();
    histogram.set_bounds(0.0, 10.0);

    // Test with very small differences
    histogram.record(1.0);
    histogram.record(1.0 + f64::EPSILON);

    assert_eq!(histogram.count(), 2);
    assert_eq!(histogram.min(), 1.0);
    assert_eq!(histogram.max(), 1.0 + f64::EPSILON);

    // Test with identical values
    histogram.record(1.0);

    assert_eq!(histogram.count(), 3);
    assert_eq!(histogram.min(), 1.0);
    assert_eq!(histogram.max(), 1.0 + f64::EPSILON);
}

#[test]
fn test_histogram_bounds_consistency() {
    let histogram = Histogram::new();

    // Initially bounds should be (-1e6, 1e6)
    assert_eq!(histogram.bounds(), (-1e6, 1e6));
    assert_eq!(histogram.lower(), -1e6);
    assert_eq!(histogram.upper(), 1e6);

    // After recording values, bounds should remain consistent
    histogram.record(100.0);
    histogram.record(-50.0);

    assert_eq!(histogram.bounds(), (-1e6, 1e6));
    assert_eq!(histogram.lower(), -1e6);
    assert_eq!(histogram.upper(), 1e6);
}

#[test]
fn test_histogram_slot_distribution() {
    let histogram = Histogram::new();

    // Record values that should fall into different slots
    // With new default bounds (-1e6, 1e6), values will be distributed across slots
    histogram.record(10.0);
    histogram.record(50.0);
    histogram.record(100.0);

    assert_eq!(histogram.count(), 3);
    // Values should no longer be clamped to 0.0 due to new bounds
    assert_eq!(histogram.sum(), 160.0);
    assert_eq!(histogram.min(), 10.0);
    assert_eq!(histogram.max(), 100.0);
}

#[test]
fn test_histogram_slot_distribution_with_bounds() {
    let histogram = Histogram::new();
    histogram.set_bounds(0.0, 120.0);

    // Record values that should fall into different slots
    histogram.record(10.0); // Should go to slot 1 (10/120 * 12 = 1)
    histogram.record(50.0); // Should go to slot 5 (50/120 * 12 = 5)
    histogram.record(100.0); // Should go to slot 10 (100/120 * 12 = 10)

    assert_eq!(histogram.count(), 3);
    assert_eq!(histogram.sum(), 160.0);
    assert_eq!(histogram.min(), 10.0);
    assert_eq!(histogram.max(), 100.0);
}

#[test]
fn test_histogram_clamping_behavior() {
    let histogram = Histogram::new();
    histogram.set_bounds(10.0, 90.0);

    // Record values outside bounds
    histogram.record(5.0); // Should be clamped to 10.0
    histogram.record(100.0); // Should be clamped to 90.0
    histogram.record(50.0); // Should remain 50.0

    assert_eq!(histogram.count(), 3);
    assert_eq!(histogram.sum(), 150.0); // 10.0 + 90.0 + 50.0
    assert_eq!(histogram.min(), 10.0);
    assert_eq!(histogram.max(), 90.0);
    assert_eq!(histogram.mean(), 50.0);
}

#[test]
fn test_histogram_record_many_basic() {
    let histogram = Histogram::new();
    histogram.set_bounds(0.0, 100.0);

    // Record 100 values of 42.0
    histogram.record_many(42.0, 100);

    assert_eq!(histogram.count(), 100);
    assert_eq!(histogram.sum(), 4200.0); // 42.0 * 100
    assert_eq!(histogram.min(), 42.0);
    assert_eq!(histogram.max(), 42.0);
    assert_eq!(histogram.mean(), 42.0);
}

#[test]
fn test_histogram_record_many_zero_count() {
    let histogram = Histogram::new();
    histogram.set_bounds(0.0, 100.0);

    // Record 0 values - should do nothing
    histogram.record_many(42.0, 0);

    assert_eq!(histogram.count(), 0);
    assert_eq!(histogram.sum(), 0.0);
    assert_eq!(histogram.min(), 0.0);
    assert_eq!(histogram.max(), 0.0);
}

#[test]
fn test_histogram_record_many_nan_value() {
    let histogram = Histogram::new();
    histogram.set_bounds(0.0, 100.0);

    // Record NaN values - should be skipped
    histogram.record_many(f64::NAN, 100);

    assert_eq!(histogram.count(), 0);
    assert_eq!(histogram.sum(), 0.0);
    assert_eq!(histogram.min(), 0.0);
    assert_eq!(histogram.max(), 0.0);
}

#[test]
fn test_histogram_record_many_negative_values() {
    let histogram = Histogram::new();
    histogram.set_bounds(-100.0, 100.0);

    // Record 50 negative values
    histogram.record_many(-25.0, 50);

    assert_eq!(histogram.count(), 50);
    assert_eq!(histogram.sum(), -1250.0); // -25.0 * 50
    assert_eq!(histogram.min(), -25.0);
    assert_eq!(histogram.max(), -25.0);
    assert_eq!(histogram.mean(), -25.0);
}

#[test]
fn test_histogram_record_many_with_clamping() {
    let histogram = Histogram::new();
    histogram.set_bounds(10.0, 90.0);

    // Record values that will be clamped
    histogram.record_many(5.0, 30); // Will be clamped to 10.0
    histogram.record_many(100.0, 20); // Will be clamped to 90.0

    assert_eq!(histogram.count(), 50);
    assert_eq!(histogram.sum(), 2100.0); // (10.0 * 30) + (90.0 * 20) = 300 + 1800 = 2100
    assert_eq!(histogram.min(), 10.0);
    assert_eq!(histogram.max(), 90.0);
    assert_eq!(histogram.mean(), 42.0); // 2100.0 / 50 = 42.0
}

#[test]
fn test_histogram_record_many_large_batch() {
    let histogram = Histogram::new();
    histogram.set_bounds(0.0, 1000.0);

    // Record a large batch
    histogram.record_many(500.0, 10000);

    assert_eq!(histogram.count(), 10000);
    assert_eq!(histogram.sum(), 5000000.0); // 500.0 * 10000
    assert_eq!(histogram.min(), 500.0);
    assert_eq!(histogram.max(), 500.0);
    assert_eq!(histogram.mean(), 500.0);
}

#[test]
fn test_histogram_record_many_edge_cases() {
    let histogram = Histogram::new();
    histogram.set_bounds(0.0, 100.0);

    // Record edge values
    histogram.record_many(0.0, 100); // Lower bound
    histogram.record_many(100.0, 50); // Upper bound

    assert_eq!(histogram.count(), 150);
    assert_eq!(histogram.sum(), 5000.0); // (0.0 * 100) + (100.0 * 50)
    assert_eq!(histogram.min(), 0.0);
    assert_eq!(histogram.max(), 100.0);
    assert_eq!(histogram.mean(), 33.333333333333336); // 5000.0 / 150
}

#[test]
fn test_histogram_record_many_mixed_with_single() {
    let histogram = Histogram::new();
    histogram.set_bounds(0.0, 100.0);

    // Record some values with record_many
    histogram.record_many(25.0, 3);
    histogram.record_many(75.0, 2);

    // Record some individual values
    histogram.record(50.0);
    histogram.record(10.0);

    // Expected: 3*25 + 2*75 + 50 + 10 = 75 + 150 + 50 + 10 = 285
    let expected_sum = 285.0;
    let expected_count = 7;
    let expected_mean = expected_sum / expected_count as f64;

    assert_eq!(histogram.count(), expected_count);
    assert_eq!(histogram.sum(), expected_sum);
    assert!((histogram.mean() - expected_mean).abs() < 1e-14);
    assert_eq!(histogram.min(), 10.0);
    assert_eq!(histogram.max(), 75.0);
}

#[test]
fn test_histogram_values_exceeding_bounds() {
    let histogram = Histogram::new();
    histogram.set_bounds(0.0, 100.0);

    // Record values within bounds
    histogram.record(50.0);
    histogram.record(25.0);
    histogram.record(75.0);

    // Record values exceeding upper bound
    histogram.record(150.0); // Should be clamped to 100.0
    histogram.record(200.0); // Should be clamped to 100.0
    histogram.record(1000.0); // Should be clamped to 100.0

    // Record values below lower bound
    histogram.record(-50.0); // Should be clamped to 0.0
    histogram.record(-100.0); // Should be clamped to 0.0

    // Check that all values were recorded (clamped)
    assert_eq!(histogram.count(), 8);

    // Check that sum reflects clamped values
    // 50 + 25 + 75 + 100 + 100 + 100 + 0 + 0 = 450
    assert_eq!(histogram.sum(), 450.0);

    // Check that min/max reflect actual recorded values (clamped)
    assert_eq!(histogram.min(), 0.0); // Clamped from -100
    assert_eq!(histogram.max(), 100.0); // Clamped from 1000

    // Check that mean reflects clamped values
    assert_eq!(histogram.mean(), 450.0 / 8.0); // 56.25
}

#[test]
fn test_histogram_slot_distribution_with_exceeding_values() {
    let histogram = Histogram::new();
    histogram.set_bounds(0.0, 100.0);

    // Record values that will be clamped to the edges
    histogram.record(-100.0); // Will be clamped to 0.0, should go to slot 0
    histogram.record(200.0); // Will be clamped to 100.0, should go to slot 11

    // Record some middle values
    histogram.record(50.0); // Should go to slot 6 (middle)

    assert_eq!(histogram.count(), 3);
    assert_eq!(histogram.sum(), 0.0 + 100.0 + 50.0); // 150.0

    // The slots should show the distribution of clamped values
    // Slot 0: 1 (clamped from -100 to 0.0)
    // Slot 6: 1 (50.0)
    // Slot 11: 1 (clamped from 200 to 100.0)
}

#[test]
fn test_histogram_record_many_with_exceeding_bounds() {
    let histogram = Histogram::new();
    histogram.set_bounds(0.0, 100.0);

    // Record many values that exceed bounds
    histogram.record_many(150.0, 5); // 5 values clamped to 100.0
    histogram.record_many(-25.0, 3); // 3 values clamped to 0.0

    // Check that all values were recorded (clamped)
    assert_eq!(histogram.count(), 8);

    // Check that sum reflects clamped values
    // 5 * 100.0 + 3 * 0.0 = 500.0
    assert_eq!(histogram.sum(), 500.0);

    // Check that min/max reflect actual recorded values (clamped)
    assert_eq!(histogram.min(), 0.0); // Clamped from -25
    assert_eq!(histogram.max(), 100.0); // Clamped from 150

    // Check that mean reflects clamped values
    assert_eq!(histogram.mean(), 500.0 / 8.0); // 62.5
}

#[test]
fn test_histogram_record_many_concurrent() {
    use std::sync::Arc;
    use std::thread;

    let histogram = Arc::new(Histogram::new());
    histogram.set_bounds(0.0, 1000.0);
    let mut handles = vec![];

    // Spawn multiple threads to record batches concurrently
    for i in 0..10 {
        let histogram_clone = Arc::clone(&histogram);
        let handle = thread::spawn(move || {
            // Each thread records 100 values in batches
            for j in 0..10 {
                let value = (i * 10 + j) as f64;
                histogram_clone.record_many(value, 10);
            }
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify the results
    assert_eq!(histogram.count(), 1000);
    assert_eq!(histogram.min(), 0.0);
    assert_eq!(histogram.max(), 99.0);

    // The sum should be the sum of all values from 0 to 99, each repeated 10 times
    let expected_sum: f64 = (0..100).map(|i| i as f64 * 10.0).sum();
    assert!((histogram.sum() - expected_sum).abs() < f64::EPSILON);
}

#[test]
fn test_histogram_record_many_performance_comparison() {
    let histogram1 = Histogram::new();
    let histogram2 = Histogram::new();
    histogram1.set_bounds(0.0, 100.0);
    histogram2.set_bounds(0.0, 100.0);

    // Record 1000 values using single record
    for _ in 0..1000 {
        histogram1.record(42.0);
    }

    // Record 1000 values using record_many
    histogram2.record_many(42.0, 1000);

    // Both should have the same results
    assert_eq!(histogram1.count(), histogram2.count());
    assert_eq!(histogram1.sum(), histogram2.sum());
    assert_eq!(histogram1.min(), histogram2.min());
    assert_eq!(histogram1.max(), histogram2.max());
    assert_eq!(histogram1.mean(), histogram2.mean());
}

#[test]
fn test_histogram_record_many_zero_value() {
    let histogram = Histogram::new();
    histogram.set_bounds(0.0, 100.0);

    // Record zero values - should not affect min/max
    histogram.record_many(0.0, 1000);

    assert_eq!(histogram.count(), 1000);
    assert_eq!(histogram.sum(), 0.0);
    // Min and max should remain at initial values since 0.0 doesn't affect them
    assert_eq!(histogram.min(), 0.0);
    assert_eq!(histogram.max(), 0.0);
    assert_eq!(histogram.mean(), 0.0);
}
