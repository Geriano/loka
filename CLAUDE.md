# Loka Stratum Bitcoin Mining Proxy

High-performance Bitcoin Stratum V1 proxy server written in Rust, featuring lock-free optimizations, comprehensive metrics, SeaORM database integration, and advanced monitoring.

## Project Structure

This is a Rust workspace with multiple crates:

- **`stratum/`** - Core Stratum V1 mining proxy server
- **`model/`** - SeaORM database models and entities  
- **`migration/`** - Database migration scripts
- **`utils/metrics/`** - Custom high-performance metrics library
- **`monitoring/`** - Prometheus/Grafana monitoring stack

## Build & Development Commands

### Core Development

```bash
# Build entire workspace
cargo build

# Build release version
cargo build --release

# Run tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run clippy linter
cargo clippy

# Format code
cargo fmt

# Check code without building
cargo check
```

### Running the Stratum Proxy

```bash
# Basic run with default config
cargo run -p loka-stratum -- start

# Run with custom config
cargo run -p loka-stratum -- -c loka-stratum.toml start

# Run with specific binding and pool
cargo run -p loka-stratum -- start -b 0.0.0.0:3333 -p 130.211.20.161:9200

# Run with verbose logging
cargo run -p loka-stratum -- -vv start

# Show status/metrics
cargo run -p loka-stratum -- status

# Validate configuration
cargo run -p loka-stratum -- config loka-stratum.toml --show

# Generate example config
cargo run -p loka-stratum -- init -o example-config.toml
```

### Development Features

```bash
# Run mock pool for testing (requires mock-pool feature)
cargo run -p loka-stratum --features mock-pool -- mock-pool -b 127.0.0.1:13333

# Run mock miners for load testing (requires mock-miner feature)  
cargo run -p loka-stratum --features mock-miner -- mock-miner -p 127.0.0.1:3333 -w 10

# Run benchmarks
./stratum/run_benchmarks.sh all

# Run specific benchmark
./stratum/run_benchmarks.sh run bench_atomic_metrics

# Create benchmark baseline
./stratum/run_benchmarks.sh baseline v1.0

# Compare with baseline
./stratum/run_benchmarks.sh compare v1.0
```

### Database Operations

```bash
# Run migrations
cd migration && cargo run

# Generate new migration
cd migration && sea-orm-cli migrate generate <migration_name>

# Generate models from database
sea-orm-cli generate entity -o model/src/entities
```

### Docker Development

```bash
# Build Docker image
docker build -f stratum/Dockerfile -t loka-stratum .

# Run with Docker Compose
cd stratum && docker-compose up -d

# View logs
cd stratum && docker-compose logs -f stratum

# Stop services
cd stratum && docker-compose down
```

### Monitoring Stack

```bash
# Start full monitoring stack (Prometheus + Grafana + AlertManager)
cd monitoring && docker-compose up -d

# Access Grafana (admin/admin123)
open http://localhost:3000

# Access Prometheus
open http://localhost:9090

# Access AlertManager  
open http://localhost:9093
```

## Architecture Overview

### Core Components

**Stratum Protocol Handler**
- Bitcoin Stratum V1 protocol implementation
- Message parsing and validation (`protocol/`)
- Connection management with async Tokio runtime
- Lock-free atomic metrics for performance

**Database Layer**
- SeaORM for type-safe database operations
- PostgreSQL support with connection pooling
- Automated migrations with versioning
- Entities: Pools, Miners, Workers, Submissions, Earnings, Distributions

**Network Architecture**
- High-performance TCP proxy with HTTP CONNECT tunneling
- Concurrent connection handling (up to 1000 default)
- Connection pooling and idle timeout management
- Rate limiting and connection bounds

**Metrics & Monitoring**
- Custom `loka-metrics` crate for sub-microsecond operations
- Atomic counters, gauges, and histograms
- Prometheus export endpoint (`/metrics/prometheus`)
- Grafana dashboards for system and application metrics

### Key Architecture Patterns

**Manager Pattern**
- Central `Manager` coordinates auth, jobs, and submissions
- Each component has its own manager (AuthManager, JobManager, etc.)
- Thread-safe operations with Arc<> wrapping

**Lock-Free Performance**
- Atomic operations for metrics collection (~4.6ns per operation)
- DashMap for concurrent hash maps
- Zero-allocation string pooling for frequent operations

**Error Handling**
- `anyhow` for application errors
- `thiserror` for typed errors
- Comprehensive error recovery and logging

**Configuration Management**
- TOML-based configuration with serde
- Environment variable overrides
- CLI argument precedence over config files

## Database Schema

```sql
-- Core entities managed by SeaORM
pools          -- Mining pool configurations
miners         -- Miner authentication and tracking  
workers        -- Individual worker connections
submissions    -- Share submissions and validation
earnings       -- Pool earnings and payouts
distributions  -- Earning distribution records
```

**Key Relationships:**
- Miners have many Workers
- Pools have many Workers and Submissions
- Earnings link to Pools for distribution tracking

## Configuration

**Main config file:** `loka-stratum.toml`

```toml
[server]
bind_address = "0.0.0.0:3333"
max_connections = 1000

[pool]
address = "pool.example.com:4444"
name = "main_pool"
username = "your_btc_address"
separator = [".", "_"]

[limiter]
connections = 1000
jobs = "600s"          # Job expiration
submissions = "172800s" # Auth cleanup threshold
```

## Monitoring & Metrics

**Prometheus Metrics:**
- Connection metrics (active, total, errors, timeouts)
- Protocol metrics (messages sent/received, bytes transferred)
- Authentication metrics (attempts, successes, failures)  
- Mining metrics (jobs, submissions, acceptance rates)
- Performance metrics (response times, throughput)

**Grafana Dashboards:**
- `/monitoring/grafana/dashboards/loka-stratum-overview.json`
- System overview with container metrics
- Real-time performance monitoring

**Health Checks:**
- Application: `http://localhost:9090/health`
- Metrics endpoint: `http://localhost:9090/metrics/prometheus`

## Performance Characteristics

**Benchmark Results:**
- Counter operations: ~4.6ns
- Gauge operations: ~3.7ns
- Histogram operations: ~3.6ns
- Batch operations: 107x speedup over individual records
- Message parsing: Optimized JSON deserialization
- Memory operations: Lock-free string pooling

**Scalability:**
- Up to 1000 concurrent connections default
- Sub-microsecond metrics collection
- Async I/O with Tokio runtime
- Configurable connection limits and timeouts

## Testing & Mock Components

**Mock Pool** (`--features mock-pool`)
- Simulates mining pool behavior
- Configurable acceptance rates and difficulty
- Latency simulation and error injection

**Mock Miner** (`--features mock-miner`)
- Load testing with multiple workers
- Configurable hashrate simulation
- Share submission patterns

**Integration Tests:**
- `tests/mock_pool_integration.rs`
- `tests/mock_miner_integration.rs`

## Deployment

**Production Docker Deployment:**
```bash
# Multi-stage Alpine build
docker build -f stratum/Dockerfile -t loka-stratum .

# With monitoring stack
cd monitoring && docker-compose up -d
cd stratum && docker-compose up -d
```

**Security Features:**
- Non-root container execution
- Minimal Alpine Linux base image
- Health check endpoints
- Proper signal handling with tini

**Ports:**
- `3333` - Stratum mining protocol
- `9090` - Metrics and health endpoints
- `3000` - Grafana dashboard (monitoring stack)
- `9090` - Prometheus (monitoring stack)

## Development Workflow

1. **Setup:** `cargo build && cargo test`
2. **Database:** Run migrations, generate models if schema changes
3. **Development:** Use mock components for testing
4. **Benchmarking:** Use `./stratum/run_benchmarks.sh` for performance testing
5. **Monitoring:** Deploy monitoring stack for observability
6. **Production:** Deploy with Docker Compose for full stack

**Key Files:**
- `/Users/gerianoadikaputra/Programs/loka/loka/stratum/src/lib.rs` - Main library structure
- `/Users/gerianoadikaputra/Programs/loka/loka/stratum/src/manager.rs` - Core manager coordination
- `/Users/gerianoadikaputra/Programs/loka/loka/stratum/src/protocol/` - Stratum protocol implementation
- `/Users/gerianoadikaputra/Programs/loka/loka/model/src/entities/` - Database models
- `/Users/gerianoadikaputra/Programs/loka/loka/migration/src/` - Database migrations
- `/Users/gerianoadikaputra/Programs/loka/loka/utils/metrics/` - Custom metrics library

## Task Master AI Instructions
**Import Task Master's development workflow commands and guidelines, treat as if import is in the main CLAUDE.md file.**
@./.taskmaster/CLAUDE.md
