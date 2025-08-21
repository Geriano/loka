#[cfg(all(feature = "mock-miner", feature = "mock-pool"))]
mod integration_tests {
    use loka_stratum::miner::{MinerConfig, MinerSimulator};
    use loka_stratum::mock::{MockConfig, MockPool};
    use std::time::Duration;
    use tokio::time::timeout;
    #[tokio::test]
    async fn test_miner_basic_connection() {
        // Start mock pool
        let pool_config = MockConfig {
            accept_rate: 0.95,
            job_interval_secs: 5,
            initial_difficulty: 1024,
            vardiff_enabled: false,
            latency_ms: 10,
            error_rate: 0.0,
            ..Default::default()
        };

        let pool = MockPool::new(pool_config);
        let pool_handle = pool.start("127.0.0.1:0").await.unwrap();
        let pool_addr = pool_handle.local_addr;

        // Start mock miner
        let miner_config = MinerConfig {
            pool_address: format!("127.0.0.1:{}", pool_addr.port()),
            workers: 1,
            hashrate_mhs: 50.0,
            username: "testuser".to_string(),
            password: "testpass".to_string(),
            worker_prefix: "worker".to_string(),
            share_interval_secs: 1.0,
            stale_rate: 0.0,
            invalid_rate: 0.0,
            duration_secs: 2, // Short test duration
            connection_timeout_secs: 5,
            reconnect_attempts: 3,
            reconnect_delay_secs: 1,
            keepalive_interval_secs: 30,
            difficulty_variance: 0.1,
            log_mining: false,
        };

        let simulator = MinerSimulator::new(miner_config);

        // Run simulation with timeout
        let result = timeout(Duration::from_secs(5), simulator.run()).await;
        assert!(result.is_ok(), "Miner simulation should complete successfully");

        pool_handle.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_miner_multiple_workers() {
        // Start mock pool
        let pool_config = MockConfig {
            accept_rate: 0.9,
            job_interval_secs: 3,
            initial_difficulty: 512,
            vardiff_enabled: false,
            latency_ms: 5,
            error_rate: 0.0,
            ..Default::default()
        };

        let pool = MockPool::new(pool_config);
        let pool_handle = pool.start("127.0.0.1:0").await.unwrap();
        let pool_addr = pool_handle.local_addr;

        // Start mock miner with multiple workers
        let miner_config = MinerConfig {
            pool_address: format!("127.0.0.1:{}", pool_addr.port()),
            workers: 3,
            hashrate_mhs: 100.0,
            username: "multiuser".to_string(),
            password: "multipass".to_string(),
            worker_prefix: "mworker".to_string(),
            share_interval_secs: 0.5,
            stale_rate: 0.0,
            invalid_rate: 0.0,
            duration_secs: 3,
            connection_timeout_secs: 5,
            reconnect_attempts: 2,
            reconnect_delay_secs: 1,
            keepalive_interval_secs: 30,
            difficulty_variance: 0.15,
            log_mining: false,
        };

        let simulator = MinerSimulator::new(miner_config);

        let result = timeout(Duration::from_secs(8), simulator.run()).await;
        assert!(result.is_ok(), "Multi-worker miner simulation should complete successfully");

        pool_handle.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_miner_with_stale_shares() {
        // Start mock pool
        let pool_config = MockConfig {
            accept_rate: 0.8,
            job_interval_secs: 2,
            initial_difficulty: 256,
            vardiff_enabled: false,
            latency_ms: 10,
            error_rate: 0.1,
            ..Default::default()
        };

        let pool = MockPool::new(pool_config);
        let pool_handle = pool.start("127.0.0.1:0").await.unwrap();
        let pool_addr = pool_handle.local_addr;

        // Start mock miner with stale shares
        let miner_config = MinerConfig {
            pool_address: format!("127.0.0.1:{}", pool_addr.port()),
            workers: 2,
            hashrate_mhs: 75.0,
            username: "staleuser".to_string(),
            password: "stalepass".to_string(),
            worker_prefix: "stale".to_string(),
            share_interval_secs: 0.8,
            stale_rate: 0.2, // 20% stale rate
            invalid_rate: 0.05, // 5% invalid rate
            duration_secs: 3,
            connection_timeout_secs: 5,
            reconnect_attempts: 2,
            reconnect_delay_secs: 1,
            keepalive_interval_secs: 30,
            difficulty_variance: 0.2,
            log_mining: false,
        };

        let simulator = MinerSimulator::new(miner_config);

        let result = timeout(Duration::from_secs(8), simulator.run()).await;
        assert!(result.is_ok(), "Stale shares miner simulation should complete successfully");

        pool_handle.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_miner_with_vardiff() {
        // Start mock pool with vardiff enabled
        let pool_config = MockConfig {
            accept_rate: 0.95,
            job_interval_secs: 4,
            initial_difficulty: 1024,
            vardiff_enabled: true,
            latency_ms: 15,
            error_rate: 0.02,
            ..Default::default()
        };

        let pool = MockPool::new(pool_config);
        let pool_handle = pool.start("127.0.0.1:0").await.unwrap();
        let pool_addr = pool_handle.local_addr;

        // Start mock miner
        let miner_config = MinerConfig {
            pool_address: format!("127.0.0.1:{}", pool_addr.port()),
            workers: 2,
            hashrate_mhs: 150.0,
            username: "vardiffuser".to_string(),
            password: "vardiffpass".to_string(),
            worker_prefix: "vd".to_string(),
            share_interval_secs: 0.6,
            stale_rate: 0.01,
            invalid_rate: 0.01,
            duration_secs: 4,
            connection_timeout_secs: 5,
            reconnect_attempts: 3,
            reconnect_delay_secs: 1,
            keepalive_interval_secs: 30,
            difficulty_variance: 0.1,
            log_mining: false,
        };

        let simulator = MinerSimulator::new(miner_config);

        let result = timeout(Duration::from_secs(10), simulator.run()).await;
        assert!(result.is_ok(), "Vardiff miner simulation should complete successfully");

        pool_handle.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_miner_high_hashrate() {
        // Start mock pool
        let pool_config = MockConfig {
            accept_rate: 0.98,
            job_interval_secs: 6,
            initial_difficulty: 2048,
            vardiff_enabled: true,
            latency_ms: 5,
            error_rate: 0.01,
            ..Default::default()
        };

        let pool = MockPool::new(pool_config);
        let pool_handle = pool.start("127.0.0.1:0").await.unwrap();
        let pool_addr = pool_handle.local_addr;

        // Start high-hashrate mock miner
        let miner_config = MinerConfig {
            pool_address: format!("127.0.0.1:{}", pool_addr.port()),
            workers: 5,
            hashrate_mhs: 500.0, // High hashrate
            username: "highuser".to_string(),
            password: "highpass".to_string(),
            worker_prefix: "high".to_string(),
            share_interval_secs: 0.3,
            stale_rate: 0.005,
            invalid_rate: 0.005,
            duration_secs: 3,
            connection_timeout_secs: 5,
            reconnect_attempts: 3,
            reconnect_delay_secs: 1,
            keepalive_interval_secs: 30,
            difficulty_variance: 0.05,
            log_mining: false,
        };

        let simulator = MinerSimulator::new(miner_config);

        let result = timeout(Duration::from_secs(8), simulator.run()).await;
        assert!(result.is_ok(), "High hashrate miner simulation should complete successfully");

        pool_handle.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_miner_reconnection() {
        // Start mock pool with higher error rate to trigger reconnections
        let pool_config = MockConfig {
            accept_rate: 0.7,
            job_interval_secs: 3,
            initial_difficulty: 512,
            vardiff_enabled: false,
            latency_ms: 20,
            error_rate: 0.3, // High error rate to trigger reconnections
            ..Default::default()
        };

        let pool = MockPool::new(pool_config);
        let pool_handle = pool.start("127.0.0.1:0").await.unwrap();
        let pool_addr = pool_handle.local_addr;

        // Start mock miner with reconnection settings
        let miner_config = MinerConfig {
            pool_address: format!("127.0.0.1:{}", pool_addr.port()),
            workers: 2,
            hashrate_mhs: 100.0,
            username: "reconuser".to_string(),
            password: "reconpass".to_string(),
            worker_prefix: "recon".to_string(),
            share_interval_secs: 1.0,
            stale_rate: 0.05,
            invalid_rate: 0.05,
            duration_secs: 4,
            connection_timeout_secs: 3,
            reconnect_attempts: 5, // Allow multiple reconnection attempts
            reconnect_delay_secs: 1,
            keepalive_interval_secs: 15,
            difficulty_variance: 0.2,
            log_mining: false,
        };

        let simulator = MinerSimulator::new(miner_config);

        let result = timeout(Duration::from_secs(12), simulator.run()).await;
        assert!(result.is_ok(), "Reconnection miner simulation should complete successfully");

        pool_handle.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_miner_config_validation() {
        // Test various config validations
        let valid_config = MinerConfig {
            pool_address: "127.0.0.1:3333".to_string(),
            workers: 1,
            hashrate_mhs: 100.0,
            username: "user".to_string(),
            password: "pass".to_string(),
            worker_prefix: "w".to_string(),
            share_interval_secs: 1.0,
            stale_rate: 0.0,
            invalid_rate: 0.0,
            duration_secs: 0, // Infinite duration
            connection_timeout_secs: 5,
            reconnect_attempts: 3,
            reconnect_delay_secs: 1,
            keepalive_interval_secs: 30,
            difficulty_variance: 0.1,
            log_mining: false,
        };

        // Test that valid config creates simulator successfully
        let simulator = MinerSimulator::new(valid_config.clone());
        assert_eq!(simulator.config().workers, 1);
        assert_eq!(simulator.config().hashrate_mhs, 100.0);
        assert_eq!(simulator.config().username, "user");

        // Test simulation duration calculation
        assert!(valid_config.simulation_duration().is_none()); // Infinite duration

        let finite_config = MinerConfig {
            duration_secs: 10,
            ..valid_config
        };
        assert_eq!(
            finite_config.simulation_duration().unwrap(),
            Duration::from_secs(10)
        );

        // Test total hashrate calculation
        assert_eq!(finite_config.total_hashrate(), 100.0); // 1 worker * 100 MH/s

        let multi_worker_config = MinerConfig {
            workers: 4,
            hashrate_mhs: 50.0,
            ..finite_config
        };
        assert_eq!(multi_worker_config.total_hashrate(), 200.0); // 4 workers * 50 MH/s
    }

    #[tokio::test]
    async fn test_miner_statistics_collection() {
        // Start mock pool
        let pool_config = MockConfig {
            accept_rate: 0.9,
            job_interval_secs: 2,
            initial_difficulty: 1024,
            vardiff_enabled: false,
            latency_ms: 10,
            error_rate: 0.05,
            ..Default::default()
        };

        let pool = MockPool::new(pool_config);
        let pool_handle = pool.start("127.0.0.1:0").await.unwrap();
        let pool_addr = pool_handle.local_addr;

        // Start mock miner for statistics collection
        let miner_config = MinerConfig {
            pool_address: format!("127.0.0.1:{}", pool_addr.port()),
            workers: 2,
            hashrate_mhs: 100.0,
            username: "statsuser".to_string(),
            password: "statspass".to_string(),
            worker_prefix: "stats".to_string(),
            share_interval_secs: 0.5,
            stale_rate: 0.1,
            invalid_rate: 0.05,
            duration_secs: 3,
            connection_timeout_secs: 5,
            reconnect_attempts: 2,
            reconnect_delay_secs: 1,
            keepalive_interval_secs: 30,
            difficulty_variance: 0.1,
            log_mining: false,
        };

        let simulator = MinerSimulator::new(miner_config);

        // Test initial statistics
        let initial_stats = simulator.get_stats().await;
        assert_eq!(initial_stats.total_workers, 2);
        assert_eq!(initial_stats.connected_workers, 0);
        assert_eq!(initial_stats.authorized_workers, 0);
        assert_eq!(initial_stats.total_shares_submitted, 0);

        // Run simulation to collect statistics
        let result = timeout(Duration::from_secs(8), simulator.run()).await;
        assert!(result.is_ok(), "Statistics collection miner simulation should complete successfully");

        pool_handle.shutdown().await.unwrap();
    }
}

#[cfg(not(all(feature = "mock-miner", feature = "mock-pool")))]
mod disabled_tests {
    #[test]
    fn test_miner_integration_requires_features() {
        // This test ensures the integration tests are only run when both features are enabled
        println!("Mock miner integration tests require both 'mock-miner' and 'mock-pool' features to be enabled");
    }
}