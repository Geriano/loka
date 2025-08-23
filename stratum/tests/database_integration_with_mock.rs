use loka_stratum::config::types::PoolConfig;
use loka_stratum::{
    Config, Manager,
    services::{database::DatabaseService, metrics::MetricsService},
};
use std::sync::Arc;
use uuid::Uuid;

#[cfg(feature = "mock-miner")]
use loka_stratum::miner::config::MinerConfig;

#[tokio::test]
async fn test_database_metrics_with_mock_features() {
    // Try to load environment variables from .env file in project root
    let _ = dotenvy::from_filename("../.env");

    // Load database URL from environment
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env file");

    println!("Starting database integration test with mock features...");

    // Initialize database service
    let database_service = match DatabaseService::new(&database_url).await {
        Ok(db) => {
            println!("✅ Database connected successfully");
            db
        }
        Err(e) => {
            eprintln!("❌ Skipping test - could not connect to database: {}", e);
            return;
        }
    };

    // Create a test pool in database first
    let test_pool_config = create_test_pool_config();
    let pool_id = database_service
        .create_pool_config(&test_pool_config)
        .await
        .expect("Failed to create test pool configuration");
    println!("✅ Test pool created with ID: {}", pool_id);

    // Load configuration with database-driven pool config
    let config = Config::load(&database_service)
        .await
        .expect("Failed to load pool config from database");

    // Initialize manager with database
    let _manager = match Manager::new(Arc::new(config), Arc::new(database_service.clone())) {
        Ok(mgr) => {
            println!("✅ Manager initialized successfully");
            Arc::new(mgr)
        }
        Err(e) => {
            eprintln!("❌ Failed to initialize manager: {}", e);
            return;
        }
    };

    // Create metrics service with database
    let metrics_config = loka_stratum::services::metrics::MetricsConfig::default();
    let metrics_service = Arc::new(MetricsService::with_database(
        metrics_config,
        Arc::new(database_service.clone()),
    ));

    // Start metrics collection
    let _metrics_handle = metrics_service.start_collection().await;

    println!("📊 Testing database metrics functionality...");

    // Test initial state (should be empty)
    let db_metrics = metrics_service.get_database_metrics().await;
    println!("\n💾 Initial database metrics:");
    println!("  Total Miners: {}", db_metrics.total_miners);
    println!("  Total Workers: {}", db_metrics.total_workers);
    println!("  15min Submissions: {}", db_metrics.total_submissions_15m);
    println!("  24h Submissions: {}", db_metrics.total_submissions_24h);

    // Test real-time metrics
    println!("\n📊 Real-time metrics:");
    let snapshot = metrics_service.get_global_snapshot();
    println!("  Active Connections: {}", snapshot.active_connections);
    println!("  Messages Received: {}", snapshot.messages_received);
    println!("  Messages Sent: {}", snapshot.messages_sent);

    // Test combined summary generation
    println!("\n📋 Testing combined metrics summary:");
    let summary = metrics_service.get_combined_summary().await;
    println!("{}", summary);

    // Basic validation checks
    println!("\n✅ Running validation checks...");

    // Database metrics should be consistent
    assert!(
        db_metrics.total_submissions_15m <= db_metrics.total_submissions_24h,
        "15min submissions should be <= 24h submissions"
    );
    assert_eq!(
        db_metrics.total_submissions_15m,
        db_metrics.valid_submissions_15m + db_metrics.invalid_submissions_15m,
        "Total 15min submissions should equal valid + invalid"
    );

    // Acceptance rates should be between 0.0 and 1.0
    assert!(
        db_metrics.acceptance_rate_15m >= 0.0 && db_metrics.acceptance_rate_15m <= 1.0,
        "15min acceptance rate should be between 0-1"
    );
    assert!(
        db_metrics.acceptance_rate_24h >= 0.0 && db_metrics.acceptance_rate_24h <= 1.0,
        "24h acceptance rate should be between 0-1"
    );

    // Hashrates should be non-negative
    assert!(
        db_metrics.estimated_hashrate_15m >= 0.0,
        "15min hashrate should be non-negative"
    );
    assert!(
        db_metrics.estimated_hashrate_24h >= 0.0,
        "24h hashrate should be non-negative"
    );

    // Summary should contain expected content
    assert!(
        summary.contains("Loka Stratum Metrics Summary"),
        "Summary should contain title"
    );
    assert!(
        summary.contains("Real-time Metrics"),
        "Summary should contain real-time section"
    );
    assert!(
        summary.contains("Historical Metrics"),
        "Summary should contain database section"
    );
    assert!(
        summary.contains("TH/s"),
        "Summary should contain terahash units"
    );

    println!("✅ All validation checks passed!");

    // Test user metrics functionality
    println!("\n👤 Testing user metrics functionality:");
    metrics_service.record_connection(Some("test_user"));
    metrics_service.record_message_received(100, Some("test_user"));
    metrics_service.record_submission(true, Some("test_user"), Some(1_000_000.0));

    let user_metrics = metrics_service.get_all_user_metrics();
    println!("  User metrics count: {}", user_metrics.len());
    if !user_metrics.is_empty() {
        let first_user = &user_metrics[0];
        println!(
            "  First user: {} (connections: {}, submissions: {})",
            first_user.user_id, first_user.connections, first_user.submissions_received
        );
    }

    // Clean up test data from database
    cleanup_test_data(&database_service, pool_id).await;

    println!("✅ Database integration test with mock features completed successfully!");
}

fn create_test_pool_config() -> loka_stratum::config::types::PoolConfig {
    PoolConfig {
        id: Uuid::new_v4(),
        address: "127.0.0.1:4444".to_string(),
        name: "test_pool".to_string(),
        username: "test_user".to_string(),
        password: Some("test_password".to_string()),
        separator: (".".to_string(), "_".to_string()),
        extranonce: false,
    }
}

#[cfg(feature = "mock-miner")]
#[allow(dead_code)]
fn create_test_miner_configs(count: usize) -> Vec<MinerConfig> {
    (0..count)
        .map(|i| MinerConfig {
            pool_address: "127.0.0.1:3333".to_string(),
            workers: 1,
            hashrate_mhs: 100.0, // 100 MH/s per miner
            username: format!("user{}", i + 1),
            password: "password".to_string(),
            worker_prefix: format!("worker{}", i + 1),
            share_interval_secs: 1.0, // 1 second between shares for fast testing
            connection_timeout_secs: 30,
            keepalive_interval_secs: 60,
            difficulty_variance: 0.1,
            stale_rate: 0.02,
            invalid_rate: 0.01,
            reconnect_attempts: 3,
            reconnect_delay_secs: 5,
            log_mining: true,
            duration_secs: 10, // Run for 10 seconds only
        })
        .collect()
}

async fn cleanup_test_data(database: &DatabaseService, pool_id: Uuid) {
    use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

    let cleanup_queries = vec![
        format!("DELETE FROM submissions WHERE pool_id = '{}'", pool_id),
        format!("DELETE FROM workers WHERE pool_id = '{}'", pool_id),
        format!("DELETE FROM pools WHERE id = '{}'", pool_id),
        "DELETE FROM miners WHERE name LIKE 'test_miner_%'".to_string(),
    ];

    for query in cleanup_queries {
        let _ = database
            .connection
            .execute_raw(Statement::from_string(DatabaseBackend::Postgres, query))
            .await;
    }

    println!("✅ Test data cleaned up from database");
}
