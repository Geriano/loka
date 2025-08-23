use loka_stratum::{
    config::types::PoolConfig,
    protocol::handler::ProtocolHandler,
    services::{
        database::DatabaseService,
        pool_config::{PoolConfigService, PoolConfigServiceConfig},
    },
};
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

#[tokio::test]
async fn test_connect_target_parsing() {
    // Test valid CONNECT target parsing
    let valid_messages = vec![
        "CONNECT 192.168.1.100:4444 HTTP/1.1",
        "CONNECT pool.example.com:3333 HTTP/1.1",
        "CONNECT 127.0.0.1:8080 HTTP/1.1",
    ];

    for message in valid_messages {
        let target = ProtocolHandler::parse_connect_target_from_message(message);
        assert!(
            target.is_some(),
            "Should parse valid CONNECT message: {}",
            message
        );

        let target = target.unwrap();
        assert!(target.contains(':'), "Target should contain port separator");

        // Basic validation that we can split host and port
        let parts: Vec<&str> = target.rsplitn(2, ':').collect();
        assert_eq!(parts.len(), 2, "Target should have host and port");

        let port_str = parts[0];
        assert!(
            port_str.parse::<u16>().is_ok(),
            "Port should be numeric: {}",
            port_str
        );
    }
}

#[tokio::test]
async fn test_connect_target_parsing_invalid() {
    // Test invalid CONNECT messages
    let invalid_messages = vec![
        "GET /path HTTP/1.1",              // Not CONNECT
        "CONNECT host HTTP/1.1",           // Missing port
        "CONNECT host:port/path HTTP/1.1", // Contains path (invalid)
        "CONNECT host:abc HTTP/1.1",       // Invalid port
        "CONNECT :1234 HTTP/1.1",          // Missing host
        "",                                // Empty message
    ];

    for message in invalid_messages {
        let target = ProtocolHandler::parse_connect_target_from_message(message);
        assert!(
            target.is_none(),
            "Should reject invalid CONNECT message: {}",
            message
        );
    }
}

#[tokio::test]
async fn test_pool_config_target_lookup() {
    // Load database URL from environment
    let database_url = "sqlite::memory:";

    println!("Starting pool config target lookup test...");

    // Initialize database service
    let database_service = match DatabaseService::new(&database_url).await {
        Ok(db) => {
            println!("✅ Database connected successfully");
            db.migrate().await.expect("Failed to migrate database");
            Arc::new(db)
        }
        Err(e) => {
            eprintln!("❌ Skipping test - could not connect to database: {}", e);
            return;
        }
    };

    // Create pool configuration service
    let config = PoolConfigServiceConfig {
        cache_ttl: Duration::from_secs(30),
        refresh_interval: Duration::from_secs(10),
        hot_reload_enabled: false, // Disabled for testing
    };
    let pool_service = PoolConfigService::new(database_service.clone(), config);

    let n = rand::random::<u16>();
    let host = Ipv4Addr::new(
        rand::random::<u8>(),
        rand::random::<u8>(),
        rand::random::<u8>(),
        rand::random::<u8>(),
    );

    // Create a test pool configuration
    let test_pool = PoolConfig {
        id: Uuid::new_v4(),
        address: format!("{}:4444", host),
        name: format!("connect_validation_test_pool_{}", n),
        username: "test_connect_user".to_string(),
        password: Some("test_password".to_string()),
        separator: (".".to_string(), "_".to_string()),
        extranonce: false,
    };

    let created_pool_id = database_service
        .create_pool_config(&test_pool)
        .await
        .expect("Failed to create test pool");
    println!("✅ Test pool created with ID: {}", created_pool_id);

    // Test target lookup - should find the pool
    let found_pool = pool_service
        .get_pool_config_by_target(&test_pool.address)
        .await
        .expect("Failed to lookup pool by target")
        .expect("Pool should be found by target");

    println!("✅ Pool found by target: {}", found_pool.name);
    assert_eq!(found_pool.name, test_pool.name);
    assert_eq!(found_pool.address, test_pool.address);

    // Test non-existent target lookup
    let not_found_pool = pool_service
        .get_pool_config_by_target(&format!("nf-{}:4444", host))
        .await
        .expect("Lookup should succeed even if pool not found");

    assert!(
        not_found_pool.is_none(),
        "Non-existent target should return None"
    );
    println!("✅ Non-existent target correctly returns None");

    // Test invalid target format
    let invalid_target_result = pool_service
        .get_pool_config_by_target("invalid-target")
        .await
        .expect("Invalid target should handle gracefully");

    assert!(
        invalid_target_result.is_none(),
        "Invalid target format should return None"
    );
    println!("✅ Invalid target format correctly returns None");

    // Clean up test data
    database_service
        .deactivate_pool_config(created_pool_id)
        .await
        .expect("Failed to deactivate test pool");
    println!("✅ Test pool deactivated");

    println!("✅ Pool config target lookup test completed successfully!");
}

#[tokio::test]
async fn test_connect_validation_metrics() {
    use loka_stratum::protocol::implementations::DefaultProtocolMetrics;
    use std::time::Duration;

    let metrics = DefaultProtocolMetrics::new();

    // Test recording various validation outcomes
    metrics.record_connect_validation("success", Duration::from_millis(5));
    metrics.record_connect_validation("success", Duration::from_millis(3));
    metrics.record_connect_validation("not_found", Duration::from_millis(2));
    metrics.record_connect_validation("error", Duration::from_millis(10));
    metrics.record_connect_validation("invalid_format", Duration::from_nanos(100));

    // Check metrics
    let (stats, avg_time) = metrics.get_connect_validation_stats();

    assert_eq!(
        stats.get("success").unwrap_or(&0),
        &2,
        "Should have 2 successful validations"
    );
    assert_eq!(
        stats.get("not_found").unwrap_or(&0),
        &1,
        "Should have 1 not_found validation"
    );
    assert_eq!(
        stats.get("error").unwrap_or(&0),
        &1,
        "Should have 1 error validation"
    );
    assert_eq!(
        stats.get("invalid_format").unwrap_or(&0),
        &1,
        "Should have 1 invalid_format validation"
    );

    let total_validations: u64 = stats.values().sum();
    assert_eq!(total_validations, 5, "Should have 5 total validations");

    // Average time should be reasonable (we added ~20ms total / 5 validations = ~4ms average)
    assert!(
        avg_time.as_millis() > 0,
        "Average time should be greater than 0"
    );
    assert!(
        avg_time.as_millis() < 100,
        "Average time should be less than 100ms"
    );

    println!("✅ CONNECT validation metrics test completed");
    println!("   Total validations: {}", total_validations);
    println!("   Success: {}", stats.get("success").unwrap_or(&0));
    println!("   Not found: {}", stats.get("not_found").unwrap_or(&0));
    println!("   Errors: {}", stats.get("error").unwrap_or(&0));
    println!(
        "   Invalid format: {}",
        stats.get("invalid_format").unwrap_or(&0)
    );
    println!("   Average time: {:?}", avg_time);
}

#[tokio::test]
async fn test_performance_requirements() {
    // Load database URL from environment
    let database_url = "sqlite::memory:";

    println!("Starting performance requirements test...");

    // Initialize database service
    let database_service = match DatabaseService::new(&database_url).await {
        Ok(db) => {
            db.migrate().await.expect("Failed to migrate database");
            Arc::new(db)
        }
        Err(e) => {
            eprintln!("❌ Skipping test - could not connect to database: {}", e);
            return;
        }
    };

    // Create pool configuration service with performance-oriented settings
    let config = PoolConfigServiceConfig {
        cache_ttl: Duration::from_secs(300), // 5 minutes
        refresh_interval: Duration::from_secs(60),
        hot_reload_enabled: true,
    };
    let pool_service = PoolConfigService::new(database_service.clone(), config);

    let n = rand::random::<u16>();
    let host = Ipv4Addr::new(
        rand::random::<u8>(),
        rand::random::<u8>(),
        rand::random::<u8>(),
        rand::random::<u8>(),
    );

    // Create a test pool
    let test_pool = PoolConfig {
        id: Uuid::new_v4(),
        address: format!("{}:4444", host),
        name: format!("performance_test_pool_{}", n),
        username: "perf_test_user".to_string(),
        password: Some("perf_password".to_string()),
        separator: (".".to_string(), "_".to_string()),
        extranonce: false,
    };

    let created_pool_id = database_service
        .create_pool_config(&test_pool)
        .await
        .expect("Failed to create performance test pool");

    // Warm up the cache with one lookup
    let _ = pool_service
        .get_pool_config_by_target(&test_pool.address)
        .await
        .expect("Warmup lookup failed");

    // Performance test: validate should complete within 10ms requirement
    let num_tests = 10;
    let mut total_time = Duration::from_nanos(0);

    for i in 0..num_tests {
        let start = std::time::Instant::now();

        let result = pool_service
            .get_pool_config_by_target(&test_pool.address)
            .await
            .expect("Performance test lookup failed");

        let elapsed = start.elapsed();
        total_time += elapsed;

        assert!(
            result.is_some(),
            "Pool should be found in performance test iteration {}",
            i
        );

        // Individual lookup should be under 10ms (requirement from PRD)
        assert!(
            elapsed.as_millis() < 10,
            "Lookup took {}ms, exceeds 10ms requirement (iteration {})",
            elapsed.as_millis(),
            i
        );
    }

    let avg_time = total_time / num_tests;
    println!("✅ Performance test passed");
    println!("   {} lookups completed", num_tests);
    println!("   Average time: {:?}", avg_time);
    println!("   Max individual time: < 10ms (requirement met)");

    // Clean up
    database_service
        .deactivate_pool_config(created_pool_id)
        .await
        .expect("Failed to deactivate performance test pool");

    println!("✅ Performance requirements test completed successfully!");
}
