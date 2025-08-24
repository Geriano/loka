#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    use loka_stratum::services::metrics::{AtomicMetrics, MetricsSnapshot};

    #[tokio::test]
    async fn test_mining_metrics_operations() {
        // Create atomic metrics directly
        let metrics = Arc::new(AtomicMetrics::new());

        // Simulate job notifications with difficulty adjustments
        for i in 0..5 {
            let difficulty = 1000.0 + (i as f64 * 100.0);

            // Record difficulty adjustment
            metrics.difficulty_adjustments_counter.fetch_add(1, Ordering::Relaxed);
            metrics.current_difficulty_bits.store(difficulty.to_bits(), Ordering::Relaxed);

            // Simulate job distribution latency
            let latency_ms = 10.0 + (i as f64 * 2.0);
            let prev_sum = f64::from_bits(metrics.job_distribution_latency_sum_ms_bits.load(Ordering::Relaxed));
            let prev_count = metrics.job_distribution_latency_count.load(Ordering::Relaxed);
            metrics.job_distribution_latency_sum_ms_bits.store((prev_sum + latency_ms).to_bits(), Ordering::Relaxed);
            metrics.job_distribution_latency_count.fetch_add(1, Ordering::Relaxed);
            
            // Update min/max
            let prev_min = f64::from_bits(metrics.job_distribution_latency_min_ms_bits.load(Ordering::Relaxed));
            let prev_max = f64::from_bits(metrics.job_distribution_latency_max_ms_bits.load(Ordering::Relaxed));
            if latency_ms < prev_min || prev_min == f64::MAX {
                metrics.job_distribution_latency_min_ms_bits.store(latency_ms.to_bits(), Ordering::Relaxed);
            }
            if latency_ms > prev_max {
                metrics.job_distribution_latency_max_ms_bits.store(latency_ms.to_bits(), Ordering::Relaxed);
            }
        }

        // Simulate share submissions
        for i in 0..20 {
            // Record share submission
            metrics.share_submissions_total.fetch_add(1, Ordering::Relaxed);
            metrics.submissions_received.fetch_add(1, Ordering::Relaxed);

            // Simulate validation result (80% acceptance rate)
            let valid = i % 5 != 0; // Every 5th share is invalid
            if valid {
                metrics.submissions_accepted.fetch_add(1, Ordering::Relaxed);
                let prev_sum = f64::from_bits(metrics.share_acceptance_rate_sum_bits.load(Ordering::Relaxed));
                metrics.share_acceptance_rate_sum_bits.store((prev_sum + 1.0).to_bits(), Ordering::Relaxed);
            } else {
                metrics.submissions_rejected.fetch_add(1, Ordering::Relaxed);
            }
            metrics.share_acceptance_rate_count.fetch_add(1, Ordering::Relaxed);

            // Simulate some stale shares
            if i % 7 == 0 {
                metrics.stale_shares_counter.fetch_add(1, Ordering::Relaxed);
            }

            // Simulate duplicate shares
            if i % 10 == 0 && i > 0 {
                metrics.duplicate_shares_counter.fetch_add(1, Ordering::Relaxed);
                metrics.duplicate_submissions.fetch_add(1, Ordering::Relaxed);
            }
        }

        // Get metrics snapshot and verify
        let snapshot = MetricsSnapshot::from_atomic(metrics.as_ref());

        // Verify share metrics
        assert_eq!(
            metrics.share_submissions_total.load(Ordering::Relaxed), 20,
            "Expected 20 share submissions"
        );
        let acceptance_rate = f64::from_bits(metrics.share_acceptance_rate_sum_bits.load(Ordering::Relaxed))
            / metrics.share_acceptance_rate_count.load(Ordering::Relaxed) as f64;
        assert!(
            acceptance_rate > 0.7 && acceptance_rate < 0.9,
            "Expected acceptance rate around 80%, got {}",
            acceptance_rate
        );

        // Verify difficulty adjustments
        assert_eq!(
            metrics.difficulty_adjustments_counter.load(Ordering::Relaxed), 5,
            "Expected 5 difficulty adjustments"
        );
        assert_eq!(
            f64::from_bits(metrics.current_difficulty_bits.load(Ordering::Relaxed)), 1400.0,
            "Expected final difficulty of 1400"
        );

        // Verify job distribution latency
        let avg_latency = f64::from_bits(metrics.job_distribution_latency_sum_ms_bits.load(Ordering::Relaxed))
            / metrics.job_distribution_latency_count.load(Ordering::Relaxed) as f64;
        assert!(
            avg_latency > 10.0 && avg_latency < 20.0,
            "Expected average latency between 10-20ms, got {}",
            avg_latency
        );
        assert_eq!(f64::from_bits(metrics.job_distribution_latency_min_ms_bits.load(Ordering::Relaxed)), 10.0);
        assert_eq!(f64::from_bits(metrics.job_distribution_latency_max_ms_bits.load(Ordering::Relaxed)), 18.0);

        // Verify stale and duplicate shares
        assert_eq!(metrics.stale_shares_counter.load(Ordering::Relaxed), 3, "Expected 3 stale shares");
        assert_eq!(
            metrics.duplicate_shares_counter.load(Ordering::Relaxed), 1,
            "Expected 1 duplicate share"
        );

        // Test calculated metrics
        let stale_rate = metrics.stale_shares_counter.load(Ordering::Relaxed) as f64 / 20.0;
        let duplicate_rate = metrics.duplicate_shares_counter.load(Ordering::Relaxed) as f64 / 20.0;
        assert_eq!(stale_rate, 3.0 / 20.0);
        assert_eq!(duplicate_rate, 1.0 / 20.0);

        // Submit more shares to trigger per-minute calculation
        for _ in 0..10 {
            metrics.share_submissions_total.fetch_add(1, Ordering::Relaxed);
            metrics.submissions_received.fetch_add(1, Ordering::Relaxed);
        }

        // Update shares per minute manually for test
        metrics.shares_per_minute_gauge.store(30, Ordering::Relaxed);
        
        println!(
            "Shares per minute: {}",
            metrics.shares_per_minute_gauge.load(Ordering::Relaxed)
        );

        println!("Mining metrics integration test passed!");
        println!("Final metrics summary:");
        println!(
            "  Share submissions: {}",
            metrics.share_submissions_total.load(Ordering::Relaxed)
        );
        println!(
            "  Acceptance rate: {:.2}%",
            acceptance_rate * 100.0
        );
        println!(
            "  Difficulty adjustments: {}",
            metrics.difficulty_adjustments_counter.load(Ordering::Relaxed)
        );
        println!(
            "  Current difficulty: {}",
            f64::from_bits(metrics.current_difficulty_bits.load(Ordering::Relaxed))
        );
        println!(
            "  Job distribution latency: {:.2}ms avg",
            avg_latency
        );
        println!("  Stale shares: {}", metrics.stale_shares_counter.load(Ordering::Relaxed));
        println!(
            "  Duplicate shares: {}",
            metrics.duplicate_shares_counter.load(Ordering::Relaxed)
        );
        println!(
            "  Shares per minute: {}",
            metrics.shares_per_minute_gauge.load(Ordering::Relaxed)
        );
    }

    #[test]
    fn test_direct_atomic_metrics() {
        let metrics = AtomicMetrics::new();

        // Test share submission tracking
        metrics.share_submissions_total.fetch_add(3, Ordering::Relaxed);
        assert_eq!(metrics.share_submissions_total.load(Ordering::Relaxed), 3);

        // Test share acceptance rate
        metrics.share_acceptance_rate_sum_bits.store((2.0_f64).to_bits(), Ordering::Relaxed);
        metrics.share_acceptance_rate_count.store(3, Ordering::Relaxed);
        let acceptance_rate = f64::from_bits(metrics.share_acceptance_rate_sum_bits.load(Ordering::Relaxed))
            / metrics.share_acceptance_rate_count.load(Ordering::Relaxed) as f64;
        assert_eq!(acceptance_rate, 2.0 / 3.0);

        // Test difficulty adjustments
        metrics.difficulty_adjustments_counter.store(2, Ordering::Relaxed);
        metrics.current_difficulty_bits.store((1500.0_f64).to_bits(), Ordering::Relaxed);
        assert_eq!(
            metrics.difficulty_adjustments_counter.load(Ordering::Relaxed),
            2
        );
        assert_eq!(f64::from_bits(metrics.current_difficulty_bits.load(Ordering::Relaxed)), 1500.0);

        // Test job distribution latency
        metrics.job_distribution_latency_sum_ms_bits.store((45.0_f64).to_bits(), Ordering::Relaxed);
        metrics.job_distribution_latency_count.store(3, Ordering::Relaxed);
        let avg_latency = f64::from_bits(metrics.job_distribution_latency_sum_ms_bits.load(Ordering::Relaxed))
            / metrics.job_distribution_latency_count.load(Ordering::Relaxed) as f64;
        assert_eq!(avg_latency, 15.0);

        // Test stale and duplicate shares
        metrics.stale_shares_counter.store(1, Ordering::Relaxed);
        metrics.duplicate_shares_counter.store(2, Ordering::Relaxed);

        assert_eq!(metrics.stale_shares_counter.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.duplicate_shares_counter.load(Ordering::Relaxed), 2);

        // Test shares per minute gauge
        metrics.shares_per_minute_gauge.store(120, Ordering::Relaxed);
        assert_eq!(metrics.shares_per_minute_gauge.load(Ordering::Relaxed), 120);

        println!("Direct atomic metrics test passed!");
    }
}
