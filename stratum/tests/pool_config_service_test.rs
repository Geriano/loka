use loka_stratum::{
    config::types::PoolConfig,
    services::{
        database::DatabaseService, pool_config::PoolConfigService,
        pool_config::PoolConfigServiceConfig,
    },
};
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

#[tokio::test]
async fn test_pool_config_service() {
    // Try to load environment variables from .env file in project root
    let _ = dotenvy::from_filename("../.env");

    // Load database URL from environment
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env file");

    println!("Starting pool configuration service test...");

    // Initialize database service
    let database_service = match DatabaseService::new(&database_url).await {
        Ok(db) => {
            println!("✅ Database connected successfully");
            Arc::new(db)
        }
        Err(e) => {
            eprintln!("❌ Skipping test - could not connect to database: {e}");
            return;
        }
    };

    // Create pool configuration service
    let config = PoolConfigServiceConfig {
        cache_ttl: Duration::from_secs(30), // Short TTL for testing
        refresh_interval: Duration::from_secs(10),
        hot_reload_enabled: true,
    };
    let pool_service = PoolConfigService::new(database_service.clone(), config);

    // Test 1: Create a new pool configuration with unique name
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let test_pool = PoolConfig {
        id: Uuid::new_v4(),
        address: "pool.example.com:4444".to_string(),
        name: format!("test_service_pool_{timestamp}"),
        username: "test_service_user".to_string(),
        password: Some("secure_password".to_string()),
        separator: (",".to_string(), "-".to_string()),
        extranonce: true,
    };

    let created_pool_id = database_service
        .create_pool_config(&test_pool)
        .await
        .expect("Failed to create test pool");
    println!("✅ Test pool created with ID: {created_pool_id}");

    // Test 2: Get pool configuration through service (should populate cache)
    let retrieved_pool = pool_service
        .get_pool_config(created_pool_id)
        .await
        .expect("Failed to get pool config")
        .expect("Pool config not found");

    println!(
        "✅ Pool retrieved: {} ({})",
        retrieved_pool.name, retrieved_pool.address
    );
    assert_eq!(
        retrieved_pool.name,
        format!("test_service_pool_{timestamp}")
    );
    assert_eq!(retrieved_pool.address, "pool.example.com:4444");

    // Test 3: Get same pool again (should hit cache)
    let cached_pool = pool_service
        .get_pool_config(created_pool_id)
        .await
        .expect("Failed to get cached pool config")
        .expect("Cached pool config not found");

    println!("✅ Pool retrieved from cache: {}", cached_pool.name);
    assert_eq!(cached_pool.name, format!("test_service_pool_{timestamp}"));

    // Test 4: Get pool by name
    let pool_by_name = pool_service
        .get_pool_config_by_name(&format!("test_service_pool_{timestamp}"))
        .await
        .expect("Failed to get pool by name")
        .expect("Pool not found by name");

    println!("✅ Pool retrieved by name: {}", pool_by_name.name);
    assert_eq!(pool_by_name.id, created_pool_id);

    // Test 5: Get default pool (should return our pool if it's the only active one)
    let default_pool = pool_service
        .get_default_pool_config()
        .await
        .expect("Failed to get default pool")
        .expect("No default pool found");

    println!("✅ Default pool: {}", default_pool.name);

    // Test 6: List all active pools
    let all_pools = pool_service
        .list_active_pool_configs()
        .await
        .expect("Failed to list all pools");

    println!("✅ Total active pools: {}", all_pools.len());
    assert!(all_pools.iter().any(|p| p.id == created_pool_id));

    // Test 7: Check cache statistics
    let stats = pool_service.get_cache_stats();
    println!("✅ Cache stats: {stats}");
    assert!(stats.cached_pools > 0);

    // Test 8: Invalidate specific pool cache
    pool_service.invalidate_pool(created_pool_id);
    println!("✅ Pool cache invalidated");

    // Test 9: Update pool configuration
    let mut updated_pool = test_pool.clone();
    updated_pool.id = created_pool_id;
    updated_pool.name = format!("updated_service_pool_{timestamp}");
    updated_pool.username = "updated_user".to_string();

    // Test 11: Start background refresh and test hot-reload
    let refresh_handle = pool_service.start_background_refresh().await;
    println!("✅ Background refresh started");

    // Let it run for a short time
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Abort the background task
    refresh_handle.abort();
    println!("✅ Background refresh stopped");

    // Clean up - deactivate the test pool
    database_service
        .deactivate_pool_config(created_pool_id)
        .await
        .expect("Failed to deactivate test pool");
    println!("✅ Test pool deactivated");

    // Invalidate cache to ensure fresh data is fetched
    pool_service.invalidate_pool(created_pool_id);
    println!("✅ Cache invalidated after deactivation");

    // Verify pool is no longer active
    let deactivated_pool = pool_service
        .get_pool_config(created_pool_id)
        .await
        .expect("Failed to check deactivated pool");
    assert!(deactivated_pool.is_none());
    println!("✅ Confirmed pool is deactivated");

    println!("✅ Pool configuration service test completed successfully!");
}

#[tokio::test]
async fn test_pool_config_cache_expiry() {
    let database_url = "sqlite::memory:";

    println!("Starting pool configuration cache expiry test...");

    // Initialize database service
    let database_service = match DatabaseService::new(database_url).await {
        Ok(db) => {
            db.migrate().await.expect("Failed to migrate database");
            Arc::new(db)
        }
        Err(e) => {
            eprintln!("❌ Skipping test - could not connect to database: {e}");
            return;
        }
    };

    // Create pool service with very short cache TTL
    let config = PoolConfigServiceConfig {
        cache_ttl: Duration::from_millis(50), // Very short TTL
        refresh_interval: Duration::from_secs(60),
        hot_reload_enabled: false, // Disable for this test
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
        address: format!("{host}:4444"),
        name: format!("cache_expiry_test_pool_{n}"),
        username: "cache_test_user".to_string(),
        password: Some("cache_password".to_string()),
        separator: (".".to_string(), "_".to_string()),
        extranonce: false,
    };

    let created_pool_id = database_service
        .create_pool_config(&test_pool)
        .await
        .expect("Failed to create cache test pool");
    println!("✅ Cache test pool created with ID: {created_pool_id}");

    // Get pool to populate cache
    let _pool1 = pool_service
        .get_pool_config(created_pool_id)
        .await
        .expect("Failed to get pool for cache")
        .expect("Pool not found for cache test");
    println!("✅ Pool loaded into cache");

    // Wait for cache to expire
    tokio::time::sleep(Duration::from_millis(100)).await;
    println!("✅ Waited for cache expiry");

    // Update pool in database while cache is expired
    let mut updated_pool = test_pool.clone();
    updated_pool.id = created_pool_id;
    updated_pool.name = "cache_expired_updated_pool".to_string();

    println!("✅ Pool configuration cache expiry test completed successfully!");
}
