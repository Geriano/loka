use loka_metrics::Histogram;
use metrics::HistogramFn;

fn main() {
    println!("=== Histogram Bounds and Clamping Demo ===\n");

    // Create histogram with bounds 0.0 to 100.0
    let histogram = Histogram::new();
    histogram.set_bounds(0.0, 100.0);

    println!(
        "Histogram bounds: [{}, {}]",
        histogram.lower(),
        histogram.upper()
    );
    println!("Range: {}", histogram.upper() - histogram.lower());
    println!(
        "Slot width: {:.2} units per slot\n",
        (histogram.upper() - histogram.lower()) / 12.0
    );

    // Test values within bounds
    println!("--- Values Within Bounds ---");
    test_value(&histogram, 25.0, "25.0 (within bounds)");
    test_value(&histogram, 50.0, "50.0 (within bounds)");
    test_value(&histogram, 75.0, "75.0 (within bounds)");

    // Test values exceeding upper bound
    println!("\n--- Values Exceeding Upper Bound ---");
    test_value(&histogram, 150.0, "150.0 (exceeds upper bound)");
    test_value(&histogram, 200.0, "200.0 (exceeds upper bound)");
    test_value(&histogram, 1000.0, "1000.0 (exceeds upper bound)");

    // Test values below lower bound
    println!("\n--- Values Below Lower Bound ---");
    test_value(&histogram, -50.0, "-50.0 (below lower bound)");
    test_value(&histogram, -100.0, "-100.0 (below lower bound)");
    test_value(&histogram, -1000.0, "-1000.0 (below lower bound)");

    // Test edge cases
    println!("\n--- Edge Cases ---");
    test_value(&histogram, 0.0, "0.0 (exactly at lower bound)");
    test_value(&histogram, 100.0, "100.0 (exactly at upper bound)");

    // Show final summary
    println!("\n=== Final Histogram Summary ===");
    println!("Total count: {}", histogram.count());
    println!("Sum: {:.1}", histogram.sum());
    println!("Mean: {:.2}", histogram.mean());
    println!("Min: {:.1}", histogram.min());
    println!("Max: {:.1}", histogram.max());

    // Show slot distribution explanation
    println!("\n=== Slot Distribution Explanation ===");
    println!("Values are clamped to bounds before slot calculation:");
    println!("  - Values < 0.0 are clamped to 0.0 (go to slot 0)");
    println!("  - Values > 100.0 are clamped to 100.0 (go to slot 11)");
    println!("  - Values 0.0-100.0 are distributed across slots 0-11");
    println!("  - Slot 0: [0.0, 8.33)");
    println!("  - Slot 6: [50.0, 58.33)");
    println!("  - Slot 11: [91.67, 100.0]");

    // Demonstrate with record_many
    println!("\n--- Batch Recording with Exceeding Values ---");
    let histogram2 = Histogram::new();
    histogram2.set_bounds(0.0, 100.0);

    println!("Recording 5 values of 150.0 (will be clamped to 100.0)...");
    histogram2.record_many(150.0, 5);

    println!("Recording 3 values of -25.0 (will be clamped to 0.0)...");
    histogram2.record_many(-25.0, 3);

    println!("Final count: {}", histogram2.count());
    println!("Final sum: {:.1}", histogram2.sum());
    println!("Final mean: {:.2}", histogram2.mean());
    println!("Final min: {:.1}", histogram2.min());
    println!("Final max: {:.1}", histogram2.max());
}

fn test_value(histogram: &Histogram, value: f64, description: &str) {
    let original_value = value;
    let (lower, upper) = histogram.bounds();
    let clamped = value.clamp(lower, upper);

    // Calculate which slot the value would go to
    let slot = if upper - lower == 0.0 {
        0
    } else {
        let normalized = (clamped - lower) / (upper - lower);
        let scaled = normalized * 12.0;
        let slot = scaled as usize;
        slot.min(11)
    };

    let slot_range_start = slot as f64 * (upper - lower) / 12.0 + lower;
    let slot_range_end = (slot + 1) as f64 * (upper - lower) / 12.0 + lower;

    println!("  {description}: ");
    println!("    Original value: {original_value:.1}");
    println!("    Clamped value: {clamped:.1}");
    println!("    Goes to slot: {slot} [{slot_range_start:.1}, {slot_range_end:.1})");

    if original_value != clamped {
        println!("    ⚠️  Value was clamped!");
    }
    println!();

    // Record the value
    histogram.record(value);
}
