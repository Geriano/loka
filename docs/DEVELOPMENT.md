# Development Guide

This guide covers local development setup, testing workflows, and contribution guidelines for the Loka Stratum Bitcoin Mining Proxy project.

## Quick Start

### Prerequisites

- **Rust**: 1.88.0 or later
- **Docker**: 20.10 or later
- **Docker Compose**: 2.0 or later
- **PostgreSQL**: 15 or later (or use Docker)
- **Git**: 2.30 or later

### One-Command Setup

```bash
# Clone and setup everything
git clone https://github.com/geriano/loka.git
cd loka
make setup
make dev
```

This will:
1. Install Rust toolchain and components
2. Install required CLI tools
3. Start development environment with monitoring
4. Run initial health checks

## Development Environment

### Local Setup (Without Docker)

#### 1. Install Dependencies
```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update stable

# Install development tools
make setup

# Or manually:
rustup component add rustfmt clippy
cargo install sea-orm-cli --locked
cargo install cargo-audit --locked
cargo install cargo-outdated --locked
```

#### 2. Database Setup
```bash
# Start PostgreSQL (using Docker)
docker run -d \
  --name loka-dev-db \
  -p 5432:5432 \
  -e POSTGRES_USER=root \
  -e POSTGRES_PASSWORD=root \
  -e POSTGRES_DB=loka_dev \
  postgres:15-alpine

# Set database URL
export DATABASE_URL=postgres://root:root@localhost:5432/loka_dev

# Run migrations
make migrate
```

#### 3. Build and Test
```bash
# Build the project
make build

# Run tests
make test

# Run with development configuration
cargo run -p loka-stratum -- start -c stratum/config/loka-stratum.toml
```

### Docker Development Environment

#### 1. Start Full Development Stack
```bash
# Start monitoring and database
make up-monitoring

# Start application
make up

# Or start everything at once
make dev
```

#### 2. Access Development Services
- **Loka Stratum**: http://localhost:9091 (metrics)
- **Grafana**: http://localhost:3001 (admin/admin123)
- **Prometheus**: http://localhost:9091
- **Database**: localhost:5432 (root/root)

#### 3. Development Workflow
```bash
# Make changes to code
# ...

# Quick test cycle
make quick-test

# Full CI simulation
make ci

# Performance testing
make perf-test
```

## Development Workflows

### Daily Development

#### 1. Morning Routine
```bash
# Update dependencies
git pull origin master
make update

# Check system status
make status
make health

# Start development environment
make dev
```

#### 2. Code Development Cycle
```bash
# Make changes
# ...

# Format and lint
make fmt
make lint

# Test changes
make test

# Test integration with database
make test-integration

# Performance check (if relevant)
make bench-critical
```

#### 3. Pre-commit Checklist
```bash
# Full quality check
make quick-test

# Docker validation
make docker-build
make docker-test

# Integration testing
make up-full
# ... test functionality ...
make down-full
```

### Feature Development

#### 1. Branch Strategy
```bash
# Create feature branch
git checkout -b feature/awesome-feature

# Work on feature
# ...

# Test thoroughly
make ci-performance

# Create PR
gh pr create --title "Add awesome feature" --body "Description..."
```

#### 2. Database Schema Changes
```bash
# Generate new migration
cd migration
sea-orm-cli migrate generate add_awesome_table

# Edit migration file
# ...

# Test migration
make migrate
make test-integration

# Generate updated entities
make generate-entities

# Commit changes
git add migration/ model/
git commit -m "feat: add awesome table schema"
```

#### 3. Performance-Critical Changes
```bash
# Establish baseline
cd stratum
./run_benchmarks.sh baseline before-change

# Make changes
# ...

# Test new performance
./run_benchmarks.sh baseline after-change

# Compare results
./run_benchmarks.sh compare before-change after-change
```

### Testing Strategies

#### Unit Testing
```bash
# Test specific module
cargo test --lib manager

# Test with output
cargo test --lib -- --nocapture

# Test single function
cargo test --lib test_connection_manager
```

#### Integration Testing
```bash
# Ensure database is running
make up-monitoring

# Run integration tests
make test-integration

# Test specific integration
cargo test --test mining_metrics_integration_test
```

#### Benchmark Testing
```bash
# Run all benchmarks
make bench

# Run specific benchmark category
make bench-critical
make bench-metrics

# Performance validation
make validate-performance
```

#### Load Testing
```bash
# Using Docker Compose profile
docker-compose -f docker-compose.development.yml --profile load-test up

# Manual load testing
cd stratum
python test_stratum_client.py --connections 100 --duration 60
```

## Performance Development

### Profiling Tools

#### CPU Profiling
```bash
# Install profiling tools
cargo install cargo-profiler --locked

# Profile critical functions
cargo profiler callgrind --bin loka-stratum
```

#### Memory Profiling
```bash
# Install heaptrack
cargo install cargo-heaptrack --locked

# Profile memory usage
cargo heaptrack --bin loka-stratum
```

#### Async Profiling
```bash
# Install tokio-console
cargo install tokio-console --locked

# Enable console in development
export TOKIO_CONSOLE_BIND=127.0.0.1:6669
cargo run -p loka-stratum

# In another terminal
tokio-console http://127.0.0.1:6669
```

### Performance Optimization Guidelines

#### Lock-free Programming
```rust
// Use atomic operations for shared state
use std::sync::atomic::{AtomicU64, Ordering};

let counter = AtomicU64::new(0);
counter.fetch_add(1, Ordering::Relaxed);
```

#### Memory Pool Optimization
```rust
// Use string pools for frequent allocations
use crate::utils::string_pool::StringPool;

let pool = StringPool::new();
let reused_string = pool.get_or_insert("frequent_string");
```

#### Async Optimization
```rust
// Use buffered channels for high throughput
use tokio::sync::mpsc;

let (tx, rx) = mpsc::channel(1000);  // Buffered channel
```

## Database Development

### Schema Management

#### Creating Migrations
```bash
# Generate new migration
cd migration
sea-orm-cli migrate generate create_new_table

# Edit the generated file
# migration/src/m{timestamp}_create_new_table.rs

# Test migration
make migrate
```

#### Entity Generation
```bash
# Generate entities from database
make generate-entities

# Verify entities match expectations
cargo test -p model
```

#### Schema Validation
```bash
# Validate schema consistency
cd migration
cargo run -- status

# Check foreign key constraints
psql $DATABASE_URL -c "\d+ tablename"
```

### Testing with Database

#### Test Database Setup
```bash
# Create test database
createdb loka_test

# Run migrations
DATABASE_URL=postgres://root:root@localhost:5432/loka_test make migrate

# Run database tests
DATABASE_URL=postgres://root:root@localhost:5432/loka_test make test-integration
```

#### Database Test Patterns
```rust
// Example integration test
#[tokio::test]
async fn test_pool_operations() {
    let db = setup_test_db().await;
    
    // Test pool creation
    let pool = create_test_pool(&db).await.unwrap();
    assert!(pool.id.is_some());
    
    // Test pool retrieval
    let found_pool = find_pool_by_name(&db, &pool.name).await.unwrap();
    assert_eq!(found_pool.name, pool.name);
    
    cleanup_test_db(&db).await;
}
```

## Monitoring Development

### Metrics Development

#### Custom Metrics
```rust
// Adding new metrics
use loka_metrics::{Counter, Gauge, Histogram};

// Create metrics
let request_count = Counter::new("requests_total", "Total requests");
let active_connections = Gauge::new("connections_active", "Active connections");
let response_time = Histogram::new("response_duration_seconds", "Response time");

// Use metrics
request_count.increment(1);
active_connections.set(42.0);
response_time.observe(0.123);
```

#### Prometheus Integration
```rust
// Export to Prometheus format
use loka_metrics::prometheus::PrometheusExporter;

let exporter = PrometheusExporter::new();
let metrics_output = exporter.export_metrics();
```

### Tracing and Logging

#### Structured Logging
```rust
use tracing::{info, warn, error, instrument};

#[instrument(skip(self))]
async fn process_message(&self, message: &str) -> Result<()> {
    info!(message_len = message.len(), "Processing message");
    
    // Processing logic
    
    info!("Message processed successfully");
    Ok(())
}
```

#### Error Context
```rust
use anyhow::{Context, Result};

fn connect_to_pool() -> Result<Connection> {
    let conn = TcpStream::connect(&self.pool_address)
        .context("Failed to connect to mining pool")?;
    
    Ok(Connection::new(conn))
}
```

## Testing Development

### Test Organization

```
tests/
├── unit/                 # Unit tests (in src/ files)
├── integration/          # Integration tests
│   ├── database/        # Database integration tests
│   ├── protocol/        # Protocol integration tests
│   └── performance/     # Performance integration tests
└── e2e/                 # End-to-end tests
```

### Test Data Management

#### Test Fixtures
```rust
// Create reusable test fixtures
pub async fn create_test_pool(db: &DatabaseConnection) -> Pool {
    let pool = pools::ActiveModel {
        name: Set("test_pool".to_string()),
        address: Set("test.pool.com".to_string()),
        port: Set(4444),
        username: Set("test_user".to_string()),
        ..Default::default()
    };
    
    pool.insert(db).await.unwrap()
}
```

#### Test Database Management
```rust
// Setup and teardown test database
pub async fn setup_test_db() -> DatabaseConnection {
    let db_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://root:root@localhost:5432/loka_test".to_string());
    
    let db = Database::connect(db_url).await.unwrap();
    
    // Run migrations
    Migrator::up(&db, None).await.unwrap();
    
    db
}

pub async fn cleanup_test_db(db: &DatabaseConnection) {
    // Clean up test data
    // Or drop and recreate database
}
```

### Benchmark Development

#### Creating Benchmarks
```rust
// benches/new_feature_benchmark.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use loka_stratum::new_feature::NewFeature;

fn benchmark_new_feature(c: &mut Criterion) {
    let feature = NewFeature::new();
    
    c.bench_function("new_feature_operation", |b| {
        b.iter(|| {
            feature.operation(black_box(42))
        })
    });
}

criterion_group!(benches, benchmark_new_feature);
criterion_main!(benches);
```

#### Running Benchmarks
```bash
# Run specific benchmark
cargo bench --bench new_feature_benchmark

# Run with custom script
cd stratum
./run_benchmarks.sh run new_feature_benchmark

# Create baseline for comparison
./run_benchmarks.sh baseline v1.0
./run_benchmarks.sh compare v1.0
```

## Debugging

### Local Debugging

#### Application Debugging
```bash
# Debug mode with full backtraces
RUST_LOG=debug RUST_BACKTRACE=full cargo run -p loka-stratum -- start

# With debugging tools
cargo run -p loka-stratum --features debug-tools -- start --debug
```

#### Protocol Debugging
```bash
# Enable protocol-level logging
RUST_LOG=loka_stratum::protocol=trace cargo run -p loka-stratum

# Test with mock client
python stratum/test_stratum_client.py --debug
```

#### Database Debugging
```bash
# Enable SQL query logging
RUST_LOG=sea_orm::query=debug cargo run -p loka-stratum

# Check database connections
psql $DATABASE_URL -c "SELECT * FROM pg_stat_activity WHERE datname = 'loka_dev';"
```

### Container Debugging

#### Debug Running Container
```bash
# Access running container
docker exec -it loka-stratum /bin/sh

# Check process status
docker exec loka-stratum ps aux

# Check network connectivity
docker exec loka-stratum netstat -tlnp
docker exec loka-stratum nslookup pool.example.com
```

#### Debug Build Issues
```bash
# Build with debug output
docker build --progress=plain -f stratum/Dockerfile .

# Debug multi-stage build
docker build --target builder -t loka-debug-builder .
docker run -it loka-debug-builder /bin/sh
```

### Remote Debugging

#### Production Debugging
```bash
# Enable debug mode temporarily
docker exec loka-stratum kill -USR1 1  # Increase log level
docker logs -f loka-stratum

# Collect diagnostic information
docker exec loka-stratum loka-stratum status
curl http://localhost:9090/debug/pprof/heap > heap.prof
```

## Release Process

### Version Management

#### Semantic Versioning
- **Major** (1.0.0): Breaking changes
- **Minor** (0.1.0): New features, backward compatible
- **Patch** (0.0.1): Bug fixes, backward compatible

#### Creating Releases

1. **Prepare Release**
   ```bash
   # Ensure all tests pass
   make release-check
   
   # Update version in Cargo.toml files
   # Update CHANGELOG.md
   
   # Commit version bump
   git add .
   git commit -m "chore: bump version to v0.2.0"
   ```

2. **Tag and Push**
   ```bash
   # Create and push tag
   git tag -a v0.2.0 -m "Release v0.2.0"
   git push origin v0.2.0
   ```

3. **Verify Release**
   ```bash
   # Check CI/CD pipeline
   gh workflow list
   gh run list --workflow=ci-cd.yml
   
   # Verify Docker images
   docker pull ghcr.io/geriano/loka/loka-stratum:v0.2.0
   ```

### Hotfix Process

#### Emergency Fixes
```bash
# Create hotfix branch from master
git checkout master
git checkout -b hotfix/critical-security-fix

# Make minimal fix
# ...

# Test fix
make ci

# Create PR for review
gh pr create --title "hotfix: critical security fix" --label "hotfix"

# After approval, merge and tag
git checkout master
git merge hotfix/critical-security-fix
git tag -a v0.1.1 -m "Hotfix v0.1.1 - critical security fix"
git push origin master v0.1.1
```

## Contribution Guidelines

### Code Standards

#### Rust Code Style
```rust
// Use consistent naming
struct ConnectionManager {
    active_connections: AtomicU64,
    pool_address: String,
}

// Implement proper error handling
use anyhow::{Context, Result};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("Invalid message format: {0}")]
    InvalidFormat(String),
    #[error("Connection timeout")]
    Timeout,
}

// Use tracing for observability
use tracing::{info, warn, instrument};

#[instrument(skip(self, data))]
async fn process_data(&self, data: &[u8]) -> Result<()> {
    info!(data_len = data.len(), "Processing incoming data");
    // ...
}
```

#### Documentation Standards
```rust
/// Manages connections to mining pools
/// 
/// The `ConnectionManager` handles concurrent connections to multiple
/// mining pools, providing load balancing and failover capabilities.
/// 
/// # Examples
/// 
/// ```rust
/// use loka_stratum::manager::ConnectionManager;
/// 
/// let manager = ConnectionManager::new().await?;
/// let connection = manager.get_connection("pool.example.com").await?;
/// ```
pub struct ConnectionManager {
    // ...
}
```

### Testing Requirements

#### Test Coverage
- **Unit tests**: All public functions must have tests
- **Integration tests**: All major features must have integration tests
- **Performance tests**: Performance-critical code must have benchmarks

#### Test Structure
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;
    
    #[tokio::test]
    async fn test_connection_manager_creation() {
        let manager = ConnectionManager::new().await;
        assert!(manager.is_ok());
    }
    
    #[tokio::test]
    async fn test_invalid_pool_address() {
        let manager = ConnectionManager::new().await.unwrap();
        let result = manager.connect("invalid://address").await;
        assert!(result.is_err());
    }
}
```

### Performance Requirements

#### Benchmarking
- **Critical path**: <10ms for mining operations
- **Memory**: <1GB for 1000 concurrent connections
- **CPU**: <50% usage under normal load
- **Metrics**: <10ns per operation

#### Performance Testing
```bash
# Before submitting performance changes
make bench-critical
make validate-performance

# Load testing
make load-test

# Memory profiling
cargo run --bin memory-profiler
```

## IDE Configuration

### VS Code Setup

#### Extensions
- rust-analyzer
- CodeLLDB (for debugging)
- Error Lens
- GitLens
- Docker
- TOML Language Support

#### Settings (`.vscode/settings.json`)
```json
{
  "rust-analyzer.check.command": "clippy",
  "rust-analyzer.cargo.buildScripts.enable": true,
  "rust-analyzer.procMacro.enable": true,
  "editor.formatOnSave": true,
  "editor.codeActionsOnSave": {
    "source.fixAll": true
  }
}
```

#### Launch Configuration (`.vscode/launch.json`)
```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "name": "Debug Loka Stratum",
      "type": "lldb",
      "request": "launch",
      "program": "${workspaceFolder}/target/debug/loka-stratum",
      "args": ["start", "-c", "stratum/config/loka-stratum.toml"],
      "cwd": "${workspaceFolder}",
      "env": {
        "RUST_LOG": "debug",
        "DATABASE_URL": "postgres://root:root@localhost:5432/loka_dev"
      }
    }
  ]
}
```

### IntelliJ/CLion Setup

#### Rust Plugin Configuration
- Enable Rust plugin
- Configure toolchain path
- Enable Clippy integration
- Configure formatter to run on save

#### Run Configurations
```xml
<!-- Loka Stratum Debug -->
<component name="ProjectRunConfigurationManager">
  <configuration default="false" name="Loka Stratum Debug" type="CargoCommandRunConfiguration">
    <option name="command" value="run -p loka-stratum -- start -c stratum/config/loka-stratum.toml" />
    <option name="workingDirectory" value="$PROJECT_DIR$" />
    <envs>
      <env name="RUST_LOG" value="debug" />
      <env name="DATABASE_URL" value="postgres://root:root@localhost:5432/loka_dev" />
    </envs>
  </configuration>
</component>
```

## Advanced Development

### Protocol Development

#### Adding New Stratum Methods
```rust
// 1. Add method to enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StratumMethod {
    Authorize,
    Submit,
    NewMethod,  // Add here
}

// 2. Implement handler
#[instrument(skip(self))]
async fn handle_new_method(&self, params: &[Value]) -> Result<Value> {
    // Implementation
}

// 3. Add to dispatcher
match method {
    StratumMethod::Authorize => self.handle_authorize(params).await,
    StratumMethod::Submit => self.handle_submit(params).await,
    StratumMethod::NewMethod => self.handle_new_method(params).await,
}

// 4. Add tests
#[tokio::test]
async fn test_new_method() {
    // Test implementation
}
```

#### Protocol Testing
```python
# Test with Python client
import socket
import json

def test_new_method():
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.connect(('localhost', 3333))
    
    # Send new method request
    request = {
        "id": 1,
        "method": "mining.new_method",
        "params": ["param1", "param2"]
    }
    
    sock.send((json.dumps(request) + '\n').encode())
    response = sock.recv(1024).decode()
    
    print("Response:", response)
    sock.close()
```

### Metrics Development

#### Custom Metric Types
```rust
// Create custom histogram with specific buckets
use loka_metrics::Histogram;

let latency_histogram = Histogram::with_buckets(
    "request_latency_seconds",
    "Request latency distribution",
    vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]
);

// Use in request handlers
let start = Instant::now();
// ... process request ...
latency_histogram.observe(start.elapsed().as_secs_f64());
```

#### Metric Testing
```rust
#[cfg(test)]
mod metrics_tests {
    use super::*;
    
    #[test]
    fn test_counter_increment() {
        let counter = Counter::new("test_counter", "Test counter");
        counter.increment(5);
        assert_eq!(counter.value(), 5);
    }
}
```

## Security Development

### Secure Coding Practices

#### Input Validation
```rust
use regex::Regex;

fn validate_bitcoin_address(address: &str) -> Result<()> {
    let btc_regex = Regex::new(r"^[13][a-km-zA-HJ-NP-Z1-9]{25,34}$")?;
    
    if !btc_regex.is_match(address) {
        return Err(anyhow::anyhow!("Invalid Bitcoin address format"));
    }
    
    Ok(())
}
```

#### Rate Limiting
```rust
use tokio::time::{Duration, Instant};
use std::collections::HashMap;

struct RateLimiter {
    requests: HashMap<String, Vec<Instant>>,
    max_requests: usize,
    window: Duration,
}

impl RateLimiter {
    fn check_rate_limit(&mut self, client_id: &str) -> bool {
        let now = Instant::now();
        let requests = self.requests.entry(client_id.to_string()).or_default();
        
        // Remove expired requests
        requests.retain(|&req_time| now.duration_since(req_time) < self.window);
        
        if requests.len() >= self.max_requests {
            return false;  // Rate limit exceeded
        }
        
        requests.push(now);
        true
    }
}
```

### Security Testing

#### Vulnerability Testing
```bash
# Test for common vulnerabilities
cargo audit

# Test Docker image for vulnerabilities
docker run --rm -v /var/run/docker.sock:/var/run/docker.sock \
  aquasec/trivy image ghcr.io/geriano/loka/loka-stratum:latest
```

#### Penetration Testing
```bash
# Network security testing
nmap -sV -sC localhost -p 3333,9090

# Application security testing
python security_tests/test_stratum_security.py
```

## Troubleshooting Development Issues

### Common Development Problems

#### 1. Compilation Errors
```bash
# Clear cache and rebuild
cargo clean
cargo build

# Check for conflicting features
cargo tree --duplicates

# Update dependencies
cargo update
```

#### 2. Test Failures
```bash
# Run specific failing test
cargo test test_name -- --nocapture

# Run tests with database debugging
RUST_LOG=sea_orm::query=debug cargo test

# Check test database state
psql $DATABASE_URL -c "\dt"
```

#### 3. Performance Issues
```bash
# Profile the application
cargo build --release
perf record --call-graph=dwarf target/release/loka-stratum start
perf report

# Memory profiling
valgrind --tool=massif target/release/loka-stratum start
```

#### 4. Docker Issues
```bash
# Debug Dockerfile
docker build --no-cache --progress=plain .

# Check image layers
docker history loka-stratum:latest

# Test container networking
docker run --rm -it --network container:loka-stratum alpine:latest netstat -tlnp
```

### Getting Help

#### Internal Resources
1. **Code Documentation**: `cargo doc --open`
2. **Architecture Diagrams**: See `ARCHITECTURE.md`
3. **Performance Guides**: See `stratum/BENCHMARK_GUIDE.md`

#### External Resources
1. **Rust Documentation**: https://doc.rust-lang.org/
2. **Tokio Documentation**: https://docs.rs/tokio/
3. **SeaORM Documentation**: https://www.sea-ql.org/SeaORM/
4. **Bitcoin Stratum Protocol**: https://braiins.com/stratum-v1/docs

#### Community Support
1. **GitHub Issues**: Create detailed issue reports
2. **Discord/Slack**: Join development channels
3. **Stack Overflow**: Use tags `rust`, `bitcoin`, `stratum`

## Development Best Practices

### Code Organization

1. **Module Structure**: Follow Rust module conventions
2. **Error Handling**: Use `anyhow` and `thiserror` appropriately
3. **Async Patterns**: Prefer async/await over direct futures
4. **Documentation**: Document all public APIs
5. **Testing**: Write tests before implementation (TDD)

### Performance Considerations

1. **Lock-free Design**: Use atomics instead of mutexes where possible
2. **Memory Efficiency**: Use string pools and object reuse
3. **Async Efficiency**: Avoid blocking operations in async context
4. **Metric Collection**: Keep metrics collection sub-microsecond

### Security Mindset

1. **Input Validation**: Validate all external inputs
2. **Error Information**: Don't leak sensitive data in errors
3. **Resource Limits**: Implement proper rate limiting
4. **Dependency Security**: Regular security audits

### Monitoring Integration

1. **Structured Logging**: Use consistent log formats
2. **Metric Naming**: Follow Prometheus naming conventions
3. **Health Checks**: Implement comprehensive health endpoints
4. **Error Tracking**: Integrate with error tracking systems

This development guide provides the foundation for contributing to the Loka Stratum project while maintaining its high standards for performance, security, and reliability.