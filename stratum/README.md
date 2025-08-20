# Loka Stratum - Enterprise Bitcoin Mining Proxy

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)](https://github.com/loka/stratum)

**Loka Stratum** is a high-performance, production-ready Bitcoin mining proxy built in Rust. It provides enterprise-grade features including advanced monitoring, security validation, performance optimization, and comprehensive error handling.

## 🚀 Features

### Core Mining Proxy
- **Stratum V1 Protocol Support** - Full Bitcoin mining protocol compliance
- **1:1 Connection Model** - Dedicated pool connection per miner for optimal isolation
- **Multi-Pool Support** - Ready for multiple mining pool configurations
- **Real-time Forwarding** - Low-latency message proxying with async I/O

### Enterprise Features
- **🔒 Advanced Security** - Multi-layer validation, rate limiting, DDoS protection
- **📊 Real-time Monitoring** - Atomic metrics collection with Prometheus export
- **⚡ Performance Optimization** - String interning, connection pooling, caching
- **🛡️ Error Recovery** - Circuit breakers, retry strategies, graceful degradation
- **📈 Resource Management** - Memory pools, CPU optimization, background cleanup

### Architecture
- **Modular Design** - Clean separation of concerns with middleware pipeline
- **Protocol Handler** - Advanced Stratum V1 implementation with middleware
- **Configuration Driven** - Flexible configuration with environment variable support
- **Production Ready** - Comprehensive logging, alerting, and health checks

## 📋 Table of Contents

- [Quick Start](#quick-start)
- [Architecture Overview](#architecture-overview)
- [Configuration](#configuration)
- [Protocol Flow](#protocol-flow)
- [System Components](#system-components)
- [Performance Features](#performance-features)
- [Monitoring & Metrics](#monitoring--metrics)
- [Security Features](#security-features)
- [API Reference](#api-reference)
- [Development](#development)
- [Deployment](#deployment)
- [Troubleshooting](#troubleshooting)

## 🚀 Quick Start

### Prerequisites
- Rust 1.88 
- Tokio async runtime
- Access to Bitcoin mining pool

### Installation
```bash
# Clone the repository
git clone https://github.com/loka/stratum.git
cd stratum

# Build the project
cargo build --release

# Run with default configuration
cargo run --bin loka-stratum
```

### Basic Configuration
```toml
# config.toml
[server]
port = 3333

[pool]
name = "mining_pool"
host = "130.211.20.161"
port = 9200
username = "your_username"
password = "your_password"

[limiter]
connections = 1000
jobs = "10m"
submissions = "2d"
```

## 🏗️ Architecture Overview

### System Architecture

```mermaid
graph TB
    subgraph "Loka Stratum Architecture"
        subgraph "Client Layer"
            M1[Miner 1]
            M2[Miner 2]
            M3[Miner N...]
        end
        
        subgraph "Proxy Layer"
            L[TCP Listener :3333]
            HF[Handler Factory]
        end
        
        subgraph "Processing Pipeline"
            PH[Protocol Handler]
            PP[Message Pipeline]
            MW[Middleware Stack]
        end
        
        subgraph "Services Layer"
            CM[Connection Manager]
            MM[Metrics Service]
            MS[Monitoring Service]
            CS[Caching Service]
            PS[Performance Service]
        end
        
        subgraph "Infrastructure"
            EM[Error Management]
            SM[Security Manager]
            RM[Resource Monitor]
        end
        
        subgraph "Mining Pool"
            P1[Pool Connection 1]
            P2[Pool Connection 2]
            P3[Pool Connection N...]
        end
    end
    
    M1 --> L
    M2 --> L
    M3 --> L
    
    L --> HF
    HF --> PH
    
    PH --> PP --> MW
    
    PH --> CM
    PH --> MM
    PH --> MS
    PH --> CS
    PH --> PS
    
    CM --> EM
    MM --> SM
    MS --> RM
    
    PH --> P1
    PH --> P2
    PH --> P3
```

### Component Overview

| Component | Purpose | Status |
|-----------|---------|---------|
| **Protocol Handler** | Modular architecture with middleware pipeline | ✅ Active |
| **Connection Manager** | 1:1 miner-pool connection management | ✅ Active |
| **Metrics Service** | Real-time performance monitoring | ✅ Active |
| **Security Manager** | Multi-layer validation and protection | ✅ Active |
| **Performance Service** | Memory and CPU optimization | ✅ Active |

## ⚙️ Configuration

### Configuration File Structure

```toml
[server]
# Server listening port
port = 3333

[pool]
# Mining pool configuration
name = "default_pool"
host = "pool.example.com"
port = 4444
username = "mining_user"
password = "optional_password"
separator = [".", "_"]  # Worker name separators
extranonce = false

[limiter]
# Connection and resource limits
connections = 1000        # Max concurrent connections
jobs = "10m"             # Job cache duration
submissions = "2d"       # Submission cache duration
```

### Environment Variables

```bash
# Override configuration via environment
export LOKA_SERVER_PORT=3333
export LOKA_POOL_HOST=pool.example.com
export LOKA_POOL_PORT=4444
export LOKA_LOG_LEVEL=info
```

## 🔄 Protocol Flow

### Miner Connection Flow

```mermaid
sequenceDiagram
    participant M as Miner
    participant L as Loka Stratum
    participant P as Mining Pool
    
    Note over M,P: Connection Establishment
    M->>L: TCP Connect :3333
    L->>P: TCP Connect (dedicated)
    P-->>L: Connection Established
    L-->>M: Connection Accepted
    
    Note over M,P: Stratum Protocol Handshake
    M->>L: mining.subscribe ["cpuminer/2.5.1"]
    L->>L: Parse & Validate (no auth required)
    L->>P: Forward mining.subscribe
    P-->>L: {"id":1,"result":[[["mining.set_difficulty","..."]]],"error":null}
    L-->>M: Forward subscription response
    
    Note over M,P: Authentication Flow
    M->>L: mining.authorize ["user.worker", "password"]
    L->>L: Authentication Middleware
    L->>L: Update Auth State
    L->>P: Forward mining.authorize
    P-->>L: {"id":2,"result":true,"error":null}
    L-->>M: Forward auth response
    
    Note over M,P: Mining Operations
    P->>L: mining.set_difficulty [4]
    L->>M: Forward difficulty
    
    P->>L: mining.notify [job_params...]
    L->>M: Forward work notification
    
    M->>L: mining.submit [job_data...]
    L->>L: Validate submission (auth required)
    L->>P: Forward submission
    P-->>L: {"id":3,"result":true,"error":null}
    L-->>M: Forward result
    
    Note over M,P: Continuous Operation
    loop Work Cycle
        P->>L: New work notifications
        L->>M: Forward work
        M->>L: Submit shares
        L->>P: Forward shares
        P-->>L: Accept/reject shares
        L-->>M: Forward results
    end
    
    Note over M,P: Connection Cleanup
    M->>L: Disconnect
    L->>P: Close pool connection
    L->>L: Cleanup resources
```

### Message Processing Pipeline

```mermaid
flowchart TD
    Start([Incoming Message]) --> Parse{Parse JSON}
    Parse -->|Success| Auth[Authentication Check]
    Parse -->|Fail| Error1[Return Parse Error]
    
    Auth --> Subscribe{mining.subscribe?}
    Subscribe -->|Yes| Allow1[Allow without auth]
    Subscribe -->|No| AuthCheck{Authenticated?}
    
    AuthCheck -->|Yes| Validate[Security Validation]
    AuthCheck -->|No| Error2[Auth Required Error]
    
    Allow1 --> Validate
    Validate --> RateLimit[Rate Limiting]
    RateLimit --> Process[Process Message]
    
    Process --> Forward{Forward to Pool?}
    Forward -->|Yes| Pool[Send to Pool]
    Forward -->|No| Respond[Send Response]
    
    Pool --> PoolResp[Await Pool Response]
    PoolResp --> Metrics[Update Metrics]
    Respond --> Metrics
    
    Metrics --> Success([Message Complete])
    
    Error1 --> ErrorHandle[Error Handler]
    Error2 --> ErrorHandle
    ErrorHandle --> Recovery{Recoverable?}
    Recovery -->|Yes| Retry[Retry Logic]
    Recovery -->|No| Fail([Connection Failed])
    Retry --> Circuit{Circuit Breaker}
    Circuit --> Start
```

## 🧩 System Components

### 1. Connection Management

```mermaid
graph LR
    subgraph "Connection Lifecycle"
        A[Accept Connection] --> B[Create Handler]
        B --> C[Establish Pool Connection]
        C --> D[Start Message Loop]
        D --> E[Handle Messages]
        E --> F{Connection Active?}
        F -->|Yes| E
        F -->|No| G[Cleanup Resources]
        G --> H[Close Connections]
    end
```

**Features:**
- 1:1 miner-to-pool connection mapping
- Automatic resource cleanup on disconnect
- Connection state tracking and metrics
- Graceful shutdown handling

### 2. Middleware Pipeline

```mermaid
graph TD
    Input[Raw Message] --> M1[Logging Middleware]
    M1 --> M2[Authentication Middleware]
    M2 --> M3[Security Validation]
    M3 --> M4[Rate Limiting]
    M4 --> M5[Metrics Collection]
    M5 --> Handler[Message Handler]
    
    Handler --> Response[Response/Forward]
```

**Middleware Components:**
- **Logging**: Comprehensive request/response logging
- **Authentication**: Stratum protocol authentication
- **Security**: Input validation and threat detection
- **Rate Limiting**: Per-client request throttling
- **Metrics**: Real-time performance data collection

### 3. Error Management

```mermaid
stateDiagram-v2
    [*] --> Healthy
    Healthy --> Degraded: Errors detected
    Degraded --> Failed: Threshold exceeded
    Failed --> Recovering: Circuit breaker open
    Recovering --> Healthy: Recovery success
    Recovering --> Failed: Recovery failed
    Degraded --> Healthy: Errors resolved
    
    note right of Failed: Automatic error recovery
    note left of Recovering: Exponential backoff retry
```

**Error Recovery Features:**
- Circuit breaker patterns for fault tolerance
- Exponential backoff retry strategies
- Graceful error handling and recovery
- Comprehensive error context and logging

## 📊 Performance Features

### Memory Optimization

```mermaid
graph TB
    subgraph "Memory Management"
        SP[String Pool] --> SI[String Interning]
        OP[Object Pool] --> OR[Object Reuse]
        CP[Connection Pool] --> CR[Connection Reuse]
        MP[Memory Pool] --> BA[Buffer Allocation]
    end
    
    subgraph "Benefits"
        SI --> MR1[30-50% Memory Reduction]
        OR --> MR2[Reduced GC Pressure]
        CR --> MR3[Lower Connection Overhead]
        BA --> MR4[Efficient Buffer Management]
    end
    
    style SP fill:#e1f5fe
    style OP fill:#f3e5f5
    style CP fill:#e8f5e8
    style MP fill:#fff3e0
```

### CPU Optimization

- **Lock-free Operations**: Atomic metrics with zero contention
- **Fast Hashing**: Optimized hash calculations for mining workloads
- **Efficient Parsing**: Zero-copy string processing where possible
- **Batch Processing**: Bulk operations for reduced per-item overhead

### I/O Optimization

- **Async Architecture**: Non-blocking I/O with Tokio runtime
- **Connection Pooling**: Reuse TCP connections to reduce overhead
- **Background Tasks**: Non-blocking maintenance operations
- **Buffer Management**: Optimized serialization with buffer reuse

## 📈 Monitoring & Metrics

### Metrics Collection

```mermaid
graph TD
    subgraph "Metrics Types"
        CM[Connection Metrics]
        MM[Message Metrics] 
        PM[Performance Metrics]
        SM[Security Metrics]
    end
    
    subgraph "Storage"
        AM[Atomic Metrics]
        TS[Time Series]
        UM[User Metrics]
    end
    
    subgraph "Export"
        PR[Prometheus]
        HS[HTTP/SSE]
        WS[WebSocket]
        API[REST API]
    end
    
    CM --> AM
    MM --> AM
    PM --> TS
    SM --> UM
    
    AM --> PR
    TS --> HS
    UM --> WS
    AM --> API
```

### Available Metrics

#### Connection Metrics
- `total_connections` - Total connections established
- `active_connections` - Currently active connections
- `connection_errors` - Connection establishment failures
- `avg_connection_time` - Average connection establishment time

#### Message Metrics
- `messages_received` - Total messages received from miners
- `messages_sent` - Total messages sent to miners
- `protocol_errors` - Protocol parsing/validation errors
- `auth_success_rate` - Authentication success percentage

#### Performance Metrics
- `cpu_usage_percent` - Current CPU utilization
- `memory_usage_bytes` - Current memory usage
- `response_time_ms` - Average response time
- `pool_latency_ms` - Average pool connection latency

#### Security Metrics
- `security_violations` - Security rule violations
- `rate_limit_hits` - Rate limiting activations
- `blocked_ips` - Number of blocked IP addresses

### Real-time Monitoring

```bash
# HTTP metrics endpoint
curl http://localhost:3333/metrics

# WebSocket real-time feed
wscat -c ws://localhost:3333/metrics/stream

# Server-sent events
curl -N http://localhost:3333/metrics/events
```

## 🔒 Security Features

### Multi-layer Security Architecture

```mermaid
graph TB
    subgraph "Security Layers"
        L1[Input Validation]
        L2[Rate Limiting]
        L3[Authentication]
        L4[Authorization]
        L5[Threat Detection]
        L6[Response Filtering]
    end
    
    Request[Incoming Request] --> L1
    L1 --> L2
    L2 --> L3
    L3 --> L4
    L4 --> L5
    L5 --> L6
    L6 --> Allow[Process Request]
    
    L1 --> Block1[Block Invalid]
    L2 --> Block2[Rate Limited]
    L3 --> Block3[Auth Failed]
    L4 --> Block4[Unauthorized]
    L5 --> Block5[Threat Detected]
```

### Security Features

#### Input Validation
- JSON schema validation
- Parameter type checking
- Size and format limits
- Injection attack prevention

#### Rate Limiting
- Per-client message rate limits
- Configurable time windows
- Automatic IP blocking for violations
- Whitelist/blacklist support

#### DDoS Protection
- Connection limits per IP
- Traffic pattern analysis
- Automatic mitigation responses
- Circuit breaker activation

#### Threat Detection
- Malformed request detection
- Suspicious pattern identification
- Real-time violation logging
- Automated response triggers

## 🔧 API Reference

### Configuration API

```rust
// Server configuration
pub struct ServerConfig {
    pub port: u16,
    pub use_protocol_handler: bool,
}

// Pool configuration  
pub struct PoolConfig {
    pub name: String,
    pub host: Ipv4Addr,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
}

// Limiter configuration
pub struct LimiterConfig {
    pub connections: usize,
    pub jobs: Duration,
    pub submissions: Duration,
}
```

### Metrics API

```rust
// Get current metrics snapshot
GET /metrics
Content-Type: application/json

{
  "connections": {
    "total": 1500,
    "active": 234,
    "errors": 12
  },
  "messages": {
    "received": 45672,
    "sent": 45651,
    "protocol_errors": 5
  },
  "performance": {
    "avg_response_time": 12.5,
    "cpu_usage": 15.2,
    "memory_usage": 128456
  }
}
```

### Health Check API

```rust
// Health check endpoint
GET /health
Content-Type: application/json

{
  "status": "healthy",
  "uptime": "2d 4h 23m",
  "version": "2.0.0",
  "connections": {
    "active": 234,
    "max": 1000
  },
  "services": {
    "pool_connection": "healthy",
    "metrics": "healthy",
    "monitoring": "healthy"
  }
}
```

## 🛠️ Development

### Building from Source

```bash
# Clone repository
git clone https://github.com/loka/stratum.git
cd stratum

# Install dependencies
cargo fetch

# Run tests
cargo test

# Build debug version
cargo build

# Build release version
cargo build --release

# Run with logging
RUST_LOG=debug cargo run --bin loka-stratum
```

### Development Environment

```bash
# Install development tools
cargo install cargo-watch
cargo install cargo-audit
cargo install cargo-tarpaulin

# Auto-rebuild on changes
cargo watch -x "run --bin loka-stratum"

# Security audit
cargo audit

# Code coverage
cargo tarpaulin --out html
```

### Project Structure

```
src/
├── config/           # Configuration management
├── error/           # Error handling and recovery
├── network/         # Network layer components
├── protocol/        # Stratum protocol implementation
├── services/        # Core services (metrics, monitoring)
├── storage/         # Data persistence and caching
├── utils/           # Utility functions and helpers
├── handler.rs       # Legacy connection handler
├── handler_factory.rs # Handler creation and management
├── listener.rs      # TCP listener and connection acceptance
├── lib.rs          # Library exports
└── main.rs         # Application entry point

tests/
├── integration/     # Integration tests
└── unit/           # Unit tests
```

### Adding Custom Middleware

```rust
use crate::protocol::traits::Middleware;
use crate::protocol::pipeline::MessageContext;
use crate::error::Result;

#[derive(Debug)]
pub struct CustomMiddleware {
    // Your middleware state
}

#[async_trait::async_trait]
impl Middleware for CustomMiddleware {
    async fn process(&self, context: MessageContext) -> Result<MessageContext> {
        // Your middleware logic here
        Ok(context)
    }
}

// Register in pipeline
let pipeline = MessagePipelineBuilder::new()
    .with_authentication()
    .with_security()
    .add_middleware(CustomMiddleware::new())
    .build();
```

## 🚀 Deployment

### Docker Deployment

```dockerfile
FROM rust:1.70-slim as builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/loka-stratum /usr/local/bin/

EXPOSE 3333
CMD ["loka-stratum"]
```

```bash
# Build and run
docker build -t loka-stratum .
docker run -p 3333:3333 -v ./config.toml:/etc/loka-stratum/config.toml loka-stratum
```

### Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: loka-stratum
spec:
  replicas: 3
  selector:
    matchLabels:
      app: loka-stratum
  template:
    metadata:
      labels:
        app: loka-stratum
    spec:
      containers:
      - name: loka-stratum
        image: loka-stratum:latest
        ports:
        - containerPort: 3333
        env:
        - name: LOKA_POOL_HOST
          value: "mining-pool.example.com"
        - name: LOKA_POOL_PORT
          value: "4444"
        resources:
          requests:
            cpu: 100m
            memory: 128Mi
          limits:
            cpu: 500m
            memory: 512Mi
---
apiVersion: v1
kind: Service
metadata:
  name: loka-stratum-service
spec:
  selector:
    app: loka-stratum
  ports:
  - port: 3333
    targetPort: 3333
  type: LoadBalancer
```

### Production Configuration

```toml
[server]
port = 3333

[pool]
name = "production_pool"
host = "pool.example.com"
port = 4444
username = "production_user"
password = "secure_password"

[limiter]
connections = 5000
jobs = "15m"
submissions = "7d"

[security]
enable_rate_limiting = true
max_requests_per_minute = 300
enable_ddos_protection = true
blocked_ip_ttl = "1h"

[metrics]
enable_prometheus = true
prometheus_port = 9090
collection_interval = "30s"

[logging]
level = "info"
format = "json"
enable_file_logging = true
log_file = "/var/log/loka-stratum/app.log"
```

### Monitoring Setup

```yaml
# Prometheus configuration
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'loka-stratum'
    static_configs:
      - targets: ['loka-stratum:9090']
    scrape_interval: 5s
    metrics_path: /metrics

# Grafana dashboard
{
  "dashboard": {
    "title": "Loka Stratum Monitoring",
    "panels": [
      {
        "title": "Active Connections",
        "type": "graph",
        "targets": [
          {
            "expr": "loka_stratum_active_connections",
            "legendFormat": "Active Connections"
          }
        ]
      }
    ]
  }
}
```

## 🔧 Troubleshooting

### Common Issues

#### 1. Connection Refused
```bash
# Check if port is available
netstat -ln | grep 3333

# Check firewall settings
sudo ufw status

# Verify configuration
loka-stratum --config config.toml --validate
```

#### 2. High Memory Usage
```bash
# Monitor memory usage
RUST_LOG=debug loka-stratum | grep memory

# Enable memory profiling
export LOKA_ENABLE_MEMORY_PROFILING=true
```

#### 3. Pool Connection Issues
```bash
# Test pool connectivity
telnet pool.example.com 4444

# Check pool credentials
loka-stratum --test-pool-connection
```

### Debug Logging

```bash
# Enable detailed logging
export RUST_LOG=loka_stratum=debug

# Log specific components
export RUST_LOG=loka_stratum::protocol=trace,loka_stratum::network=debug

# Save logs to file
loka-stratum 2>&1 | tee loka-stratum.log
```

### Performance Tuning

```toml
[performance]
# Connection pool settings
connection_pool_size = 100
connection_timeout = "30s"

# Memory optimization
enable_string_interning = true
buffer_pool_size = 1000

# CPU optimization  
worker_threads = 4
blocking_threads = 4
```

### Health Monitoring

```bash
# Check system health
curl http://localhost:3333/health

# Monitor metrics
curl http://localhost:3333/metrics | jq

# Real-time monitoring
watch -n 1 'curl -s http://localhost:3333/health | jq .connections'
```

## 📜 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🤝 Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📞 Support

- **Documentation**: [https://docs.loka-stratum.org](https://docs.loka-stratum.org)
- **Issues**: [GitHub Issues](https://github.com/loka/stratum/issues)
- **Discord**: [Community Chat](https://discord.gg/loka-stratum)
- **Email**: support@loka-stratum.org

## 🙏 Acknowledgments

- Bitcoin Core developers for the Stratum protocol specification
- Tokio team for the excellent async runtime
- Rust community for the amazing ecosystem
- Mining community for feedback and testing

---

**Built with ❤️ in Rust** | **Enterprise-Grade Bitcoin Mining Infrastructure**