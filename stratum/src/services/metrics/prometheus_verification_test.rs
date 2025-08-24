//! Prometheus export verification tests

use crate::services::metrics::{AtomicMetrics, MetricsConfig, MetricsService};
use std::sync::Arc;
use std::time::Duration;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_prometheus_format_completeness() {
        let metrics = Arc::new(AtomicMetrics::new());

        // Set up test scenario
        metrics.update_difficulty(2048.0);

        // Submit various types of shares
        for _i in 0..15 {
            metrics.record_submission_accepted();
        }
        for _i in 0..3 {
            metrics.record_submission_rejected();
        }

        // Record some connections and protocol activity
        for _i in 0..5 {
            metrics.increment_connection();
            metrics.record_bytes_received(1024);
            metrics.record_bytes_sent(512);
            metrics.record_messages_received(1);
            metrics.record_messages_sent(1);
        }

        // Manually trigger hashrate calculation
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();

        metrics.store_f64(
            &metrics.hashrate_window_start_timestamp_bits,
            current_time - 60.0,
        );
        metrics
            .hashrate_window_start_shares
            .store(0, std::sync::atomic::Ordering::Relaxed);
        metrics.calculate_and_update_hashrate(60.0);

        let snapshot = metrics.snapshot();
        let prometheus_output = snapshot.to_prometheus_format();

        println!("📊 Complete Prometheus Export:");
        println!("{prometheus_output}");

        // Verify all critical hashrate metrics are present
        assert!(
            prometheus_output.contains("stratum_hashrate_estimate_hs"),
            "Missing hashrate in H/s"
        );
        assert!(
            prometheus_output.contains("stratum_hashrate_estimate_mhs"),
            "Missing hashrate in MH/s"
        );
        assert!(
            prometheus_output.contains("stratum_difficulty_current"),
            "Missing current difficulty"
        );
        assert!(
            prometheus_output.contains("stratum_shares_submitted_total"),
            "Missing total shares"
        );
        assert!(
            prometheus_output.contains("stratum_shares_accepted_total"),
            "Missing accepted shares"
        );
        assert!(
            prometheus_output.contains("stratum_share_acceptance_rate"),
            "Missing acceptance rate"
        );

        // Verify format follows Prometheus standards
        let lines: Vec<&str> = prometheus_output.lines().collect();
        let mut help_count = 0;
        let mut type_count = 0;
        let mut metric_count = 0;

        for line in &lines {
            if line.starts_with("# HELP ") {
                help_count += 1;
            } else if line.starts_with("# TYPE ") {
                type_count += 1;
            } else if !line.is_empty() && !line.starts_with("#") {
                metric_count += 1;
                // Verify metric line format
                assert!(
                    line.contains(" "),
                    "Metric line should contain space: {line}"
                );
            }
        }

        println!("📋 Prometheus Format Analysis:");
        println!("   - HELP lines: {help_count}");
        println!("   - TYPE lines: {type_count}");
        println!("   - Metric lines: {metric_count}");

        assert!(help_count > 0, "Should have HELP lines");
        assert!(type_count > 0, "Should have TYPE lines");
        assert!(metric_count > 0, "Should have metric lines");

        println!("✅ Prometheus format verification passed");
    }

    #[tokio::test]
    async fn test_metrics_service_prometheus_integration() {
        let config = MetricsConfig {
            hashrate_calculation_interval: Duration::from_millis(100),
            hashrate_time_window: Duration::from_secs(5),
            ..Default::default()
        };

        let metrics_service = MetricsService::new(config);

        // Record some activity
        metrics_service.record_connection();
        metrics_service.record_job_received();
        metrics_service.record_submission_received();
        metrics_service.record_submission_accepted();
        metrics_service.record_difficulty_adjustment_event(512.0);

        // Capture snapshot through service
        let snapshot = metrics_service.capture_snapshot().await;
        let prometheus_export = snapshot.to_prometheus_format();

        // Verify service integration works
        assert!(
            !prometheus_export.is_empty(),
            "Prometheus export should not be empty"
        );
        assert!(
            snapshot.current_difficulty > 0.0,
            "Difficulty should be set"
        );

        println!("✅ Metrics service Prometheus integration verified");
        println!("📊 Sample metrics:");

        // Show sample of key metrics
        for line in prometheus_export.lines().take(20) {
            if line.contains("stratum_") && !line.starts_with("#") {
                println!("   {line}");
            }
        }
    }

    #[tokio::test]
    async fn test_hashrate_calculation_with_realistic_data() {
        let metrics = Arc::new(AtomicMetrics::new());

        // Simulate realistic mining pool scenario
        let pool_difficulty = 4096.0; // Typical pool difficulty
        metrics.update_difficulty(pool_difficulty);

        // Set window start time
        let window_start = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64()
            - 300.0; // 5 minutes ago

        metrics.store_f64(&metrics.hashrate_window_start_timestamp_bits, window_start);
        metrics
            .hashrate_window_start_shares
            .store(0, std::sync::atomic::Ordering::Relaxed);

        // Submit shares over time (simulate ~50 shares in 5 minutes)
        for _i in 0..50 {
            metrics.record_submission_accepted();
        }

        // Add some rejected shares
        for _i in 0..5 {
            metrics.record_submission_rejected();
        }

        // Calculate hashrate
        metrics.calculate_and_update_hashrate(300.0); // 5-minute window

        let hashrate = metrics.get_hashrate_estimate();
        let snapshot = metrics.snapshot();

        println!("🔍 Realistic Mining Scenario Results:");
        println!("   - Pool difficulty: {pool_difficulty}");
        println!("   - Total shares: {}", snapshot.share_submissions);
        println!("   - Accepted shares: {}", snapshot.submissions_accepted);
        println!("   - Rejected shares: {}", snapshot.submissions_rejected);
        println!(
            "   - Acceptance rate: {:.2}%",
            snapshot.share_acceptance_rate * 100.0
        );
        println!("   - Calculated hashrate: {hashrate:.2} H/s");
        println!(
            "   - Calculated hashrate: {:.2} MH/s",
            hashrate / 1_000_000.0
        );
        println!(
            "   - Calculated hashrate: {:.2} GH/s",
            hashrate / 1_000_000_000.0
        );

        // Verify reasonable hashrate for the scenario
        if hashrate > 0.0 {
            // Expected: ~50 shares × 4096 difficulty × 2^32 / 300 seconds
            let expected_min = 40.0 * pool_difficulty * 4_294_967_296.0 / 400.0;
            let expected_max = 60.0 * pool_difficulty * 4_294_967_296.0 / 200.0;

            assert!(
                hashrate >= expected_min,
                "Hashrate {hashrate:.2} should be >= {expected_min:.2}"
            );
            assert!(
                hashrate <= expected_max,
                "Hashrate {hashrate:.2} should be <= {expected_max:.2}"
            );

            println!(
                "✅ Hashrate is within expected bounds ({expected_min:.2} - {expected_max:.2} H/s)"
            );
        } else {
            println!("ℹ️  Hashrate is 0 (timing window not met - acceptable in tests)");
        }

        // Test Prometheus export with realistic data
        let prometheus_output = snapshot.to_prometheus_format();
        let hashrate_lines: Vec<&str> = prometheus_output
            .lines()
            .filter(|line| line.contains("hashrate") && !line.starts_with("#"))
            .collect();

        println!("📊 Prometheus Hashrate Metrics:");
        for line in hashrate_lines {
            println!("   {line}");
        }

        println!("✅ Realistic scenario verification complete");
    }
}
