# Mock Pool Feature Documentation

## Overview

The `mock` feature provides a fully functional mock Bitcoin mining pool server for testing and development of the Loka Stratum proxy. This allows you to test the proxy without connecting to a real mining pool, enabling isolated testing, CI/CD integration, and rapid development.

## Features

### Core Functionality
- **Full Stratum V1 Protocol**: Implements all essential Stratum V1 methods
- **Realistic Job Generation**: Creates valid mock Bitcoin block headers
- **Dynamic Difficulty Adjustment**: Vardiff algorithm simulation
- **Share Validation**: Configurable acceptance rates and validation logic
- **Connection Management**: Handles multiple concurrent miner connections
- **Error Simulation**: Inject errors for robustness testing

### Supported Stratum Methods
- `mining.subscribe` - Connection initialization
- `mining.authorize` - Worker authentication  
- `mining.notify` - Job broadcasts
- `mining.set_difficulty` - Difficulty updates
- `mining.submit` - Share submissions
- `mining.extranonce.subscribe` - Extra nonce support
- `mining.ping` - Connection keepalive
- `mining.get_version` - Version information

## Installation

Enable the mock feature in your `Cargo.toml`:

```toml
[dependencies]
loka-stratum = { version = "0.1.0", features = ["mock-pool"] }
```

Or build with the feature flag:

```bash
cargo build --features mock-pool
cargo test --features mock-pool
```

## Usage

### Command Line Interface

Start a standalone mock pool server:

```bash
# Basic usage with defaults
loka-stratum mock-pool

# Custom configuration
loka-stratum mock-pool \
  --bind 127.0.0.1:13333 \
  --accept-rate 0.95 \
  --job-interval 30 \
  --difficulty 1024 \
  --vardiff true \
  --latency 50 \
  --error-rate 0.01

# Load configuration from file
loka-stratum mock-pool --config mock-pool.toml
```

### Configuration File

Create a `mock-pool.toml` configuration file:

```toml
# Mock pool configuration
port = 13333
accept_rate = 0.95              # 95% share acceptance
job_interval_secs = 30          # New job every 30 seconds
initial_difficulty = 1024       # Starting difficulty
vardiff_enabled = true          # Enable vardiff
vardiff_target_time_secs = 10   # Target time between shares
latency_ms = 50                 # Simulated network latency
error_rate = 0.01               # 1% random errors
max_connections = 100           # Maximum concurrent connections
extranonce_size = 4             # Extra nonce size in bytes
```

### Programmatic Usage

Use the mock pool in your Rust tests:

```rust
use loka_stratum::mock::{MockConfig, MockPool};

#[tokio::test]
async fn test_with_mock_pool() {
    // Create configuration
    let config = MockConfig {
        accept_rate: 0.95,
        job_interval_secs: 30,
        initial_difficulty: 1024,
        vardiff_enabled: true,
        ..Default::default()
    };
    
    // Start mock pool
    let pool = MockPool::new(config);
    let handle = pool.start("127.0.0.1:13333").await?;
    
    // Connect your proxy/miner to 127.0.0.1:13333
    // Run your tests...
    
    // Shutdown
    handle.shutdown().await?;
}
```

## Testing Scenarios

### 1. Basic Connectivity Test

```rust
#[cfg(feature = "mock-pool")]
#[tokio::test]
async fn test_basic_connection() {
    let pool = MockPool::new(MockConfig::default());
    let handle = pool.start("127.0.0.1:13333").await?;
    
    // Connect and subscribe
    let mut miner = connect_miner("127.0.0.1:13333").await?;
    miner.subscribe().await?;
    miner.authorize("worker1", "password").await?;
    
    // Submit a share
    let result = miner.submit_share(...).await?;
    assert!(result.accepted);
    
    handle.shutdown().await?;
}
```

### 2. High Load Testing

```rust
#[cfg(feature = "mock-pool")]
#[tokio::test]
async fn test_high_load() {
    let config = MockConfig {
        max_connections: 1000,
        ..Default::default()
    };
    
    let pool = MockPool::new(config);
    let handle = pool.start("127.0.0.1:13333").await?;
    
    // Spawn multiple concurrent miners
    let mut handles = vec![];
    for i in 0..100 {
        handles.push(tokio::spawn(async move {
            let miner = connect_miner("127.0.0.1:13333").await?;
            // Simulate mining activity...
        }));
    }
    
    // Wait for all miners
    for h in handles {
        h.await??;
    }
    
    handle.shutdown().await?;
}
```

### 3. Error Handling Test

```rust
#[cfg(feature = "mock-pool")]
#[tokio::test]
async fn test_error_handling() {
    let config = MockConfig {
        error_rate: 0.5,  // 50% error rate
        ..Default::default()
    };
    
    let pool = MockPool::new(config);
    let handle = pool.start("127.0.0.1:13333").await?;
    
    // Test error handling
    let mut error_count = 0;
    for _ in 0..10 {
        match miner.submit_share(...).await {
            Ok(_) => {},
            Err(_) => error_count += 1,
        }
    }
    
    assert!(error_count > 3);  // Expect ~50% errors
    
    handle.shutdown().await?;
}
```

### 4. Vardiff Testing

```rust
#[cfg(feature = "mock-pool")]
#[tokio::test]
async fn test_vardiff() {
    let config = MockConfig {
        vardiff_enabled: true,
        vardiff_target_time_secs: 1,
        initial_difficulty: 128,
        ..Default::default()
    };
    
    let pool = MockPool::new(config);
    let handle = pool.start("127.0.0.1:13333").await?;
    
    // Submit shares rapidly to trigger difficulty adjustment
    let miner = connect_miner("127.0.0.1:13333").await?;
    
    for _ in 0..20 {
        miner.submit_share(...).await?;
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    // Difficulty should have increased due to rapid submissions
    
    handle.shutdown().await?;
}
```

## Integration with CI/CD

### GitHub Actions Example

```yaml
name: Mining Proxy Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          
      - name: Run tests with mock pool
        run: |
          cargo test --features mock-pool
          
      - name: Integration test
        run: |
          # Start mock pool in background
          cargo run --features mock-pool -- mock-pool &
          POOL_PID=$!
          
          # Wait for pool to start
          sleep 2
          
          # Run proxy tests against mock pool
          cargo test --features integration_tests
          
          # Cleanup
          kill $POOL_PID
```

### Docker Compose Example

```yaml
version: '3.8'

services:
  mock-pool:
    build:
      context: .
      dockerfile: Dockerfile
    command: ["loka-stratum", "mock-pool", "--bind", "0.0.0.0:13333"]
    ports:
      - "13333:13333"
    environment:
      - RUST_LOG=debug
      
  proxy:
    build:
      context: .
      dockerfile: Dockerfile
    command: ["loka-stratum", "start", "--pool", "mock-pool:13333"]
    depends_on:
      - mock-pool
    ports:
      - "3333:3333"
```

## Performance Considerations

### Resource Usage
- **Memory**: ~10-50MB for typical usage
- **CPU**: Minimal, mostly I/O bound
- **Connections**: Default max 100, configurable up to system limits

### Optimization Tips
1. Disable vardiff for simpler testing scenarios
2. Reduce job rotation frequency for stability testing
3. Increase latency for realistic network simulation
4. Use error injection sparingly in performance tests

## Troubleshooting

### Common Issues

**Port Already in Use**
```bash
Error: Address already in use (os error 48)
```
Solution: Change the port or stop the conflicting process

**Too Many Open Files**
```bash
Error: Too many open files (os error 24)
```
Solution: Increase system file descriptor limits or reduce max_connections

**Connection Refused**
```bash
Error: Connection refused (os error 61)
```
Solution: Ensure the mock pool is running and the address is correct

### Debug Logging

Enable detailed logging:
```bash
RUST_LOG=debug loka-stratum mock-pool
```

Trace level for maximum detail:
```bash
RUST_LOG=trace loka-stratum mock-pool
```

## Advanced Features

### Custom Behaviors

Extend the mock pool with custom behaviors:

```rust
use loka_stratum::mock::{MockPool, MockConfig};

impl MockPool {
    // Add custom job generation logic
    pub fn with_custom_jobs(mut self, generator: JobGenerator) -> Self {
        self.job_generator = generator;
        self
    }
    
    // Add custom validation logic
    pub fn with_custom_validator(mut self, validator: Validator) -> Self {
        self.validator = validator;
        self
    }
}
```

### Metrics Collection

The mock pool exposes metrics for monitoring:

- `mock_pool_connections_total` - Total connections handled
- `mock_pool_shares_accepted` - Accepted shares count
- `mock_pool_shares_rejected` - Rejected shares count
- `mock_pool_jobs_generated` - Jobs generated count
- `mock_pool_difficulty_adjustments` - Vardiff adjustments count

## Security Considerations

The mock pool is designed for **testing only** and should not be exposed to public networks:

- No authentication beyond basic Stratum
- No real cryptocurrency validation
- Simplified security model
- Predictable random values

Always use `127.0.0.1` or `localhost` for binding in production environments.

## Contributing

To contribute to the mock pool feature:

1. Add new Stratum methods in `responses.rs`
2. Enhance job generation in `job_manager.rs`
3. Improve difficulty algorithms in `difficulty.rs`
4. Add validation logic in `validator.rs`
5. Write tests in `tests/mock_pool_integration.rs`

## License

The mock pool feature is part of the Loka Stratum project and follows the same MIT license.