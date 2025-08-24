# Docker Usage Guide

This guide covers Docker deployment, configuration, and usage for the Loka Stratum Bitcoin Mining Proxy.

## Image Variants

### Production Images

#### Release (Recommended)
```bash
# Pull latest release
docker pull ghcr.io/geriano/loka/loka-stratum:latest

# Pull specific version
docker pull ghcr.io/geriano/loka/loka-stratum:v0.1.0
```

**Characteristics:**
- Optimized for production use
- Full feature set
- Multi-platform support (amd64, arm64)
- Size: ~50MB
- Base: Alpine Linux 3.19

#### Slim
```bash
# Pull slim variant
docker pull ghcr.io/geriano/loka/loka-stratum:latest-slim
```

**Characteristics:**
- Minimal attack surface
- Essential features only
- Smallest possible size
- Size: ~25MB
- Use case: Resource-constrained environments

#### Debug
```bash
# Pull debug variant
docker pull ghcr.io/geriano/loka/loka-stratum:latest-debug
```

**Characteristics:**
- Debug symbols included
- Development tools available
- Verbose logging enabled
- Size: ~80MB
- Use case: Development and troubleshooting

## Quick Start

### Basic Usage

```bash
# Run with minimal configuration
docker run -d \
  --name loka-stratum \
  -p 3333:3333 \
  -p 9090:9090 \
  -e DATABASE_URL=postgres://user:pass@host:5432/db \
  -e POOL_ADDRESS=stratum.pool.com \
  -e POOL_USERNAME=your_btc_address \
  ghcr.io/geriano/loka/loka-stratum:latest
```

### With Configuration File

```bash
# Create configuration directory
mkdir -p ./config

# Create configuration file (see Configuration section below)
cat > ./config/loka-stratum.toml << EOF
[server]
bind_address = "0.0.0.0:3333"
max_connections = 1000

[pool]
address = "stratum.pool.com:4444"
username = "your_btc_address"
EOF

# Run with configuration
docker run -d \
  --name loka-stratum \
  -p 3333:3333 \
  -p 9090:9090 \
  -v ./config:/app/config:ro \
  -e DATABASE_URL=postgres://user:pass@host:5432/db \
  ghcr.io/geriano/loka/loka-stratum:latest
```

## Docker Compose Deployments

### Development Environment

Use the development compose configuration for local development:

```bash
# Start development environment
docker-compose -f docker-compose.development.yml up -d

# View logs
docker-compose -f docker-compose.development.yml logs -f stratum-dev

# Stop environment
docker-compose -f docker-compose.development.yml down
```

**Includes:**
- PostgreSQL database
- Redis for caching
- Prometheus for metrics
- Grafana for dashboards
- Development tools container

**Access Points:**
- Stratum proxy: `localhost:3333`
- Metrics: `http://localhost:9090/metrics/prometheus`
- Grafana: `http://localhost:3001` (admin/admin123)
- Prometheus: `http://localhost:9091`

### Production Environment

Use the production compose configuration for production deployments:

```bash
# Set required environment variables
export POSTGRES_PASSWORD=secure_password
export GRAFANA_PASSWORD=secure_grafana_password
export POOL_ADDRESS=your.pool.com
export POOL_USERNAME=your_btc_address

# Start production environment
docker-compose -f docker-compose.production.yml up -d

# Monitor startup
docker-compose -f docker-compose.production.yml logs -f

# Check health
docker-compose -f docker-compose.production.yml ps
```

**Includes:**
- PostgreSQL with production configuration
- Loka Stratum proxy
- Nginx reverse proxy
- Redis for caching
- Prometheus with 30-day retention
- Grafana with security hardening
- AlertManager for notifications

## Configuration

### Environment Variables

#### Required
```bash
DATABASE_URL=postgres://user:password@host:port/database
POOL_ADDRESS=pool.example.com        # Mining pool address
POOL_USERNAME=your_btc_address       # Your Bitcoin address
```

#### Optional
```bash
# Server Configuration
STRATUM_PORT=3333                    # Stratum protocol port
METRICS_PORT=9090                    # Metrics endpoint port
MAX_CONNECTIONS=1000                 # Maximum concurrent connections

# Pool Configuration
POOL_PORT=4444                       # Mining pool port
POOL_PASSWORD=                       # Pool password (if required)

# Logging
RUST_LOG=info                        # Log level (error, warn, info, debug, trace)
RUST_BACKTRACE=0                     # Backtrace on errors (0, 1, full)

# Performance
WORKER_THREADS=                      # Tokio worker threads (auto-detect if unset)

# Security
SENTRY_DSN=                          # Sentry error tracking DSN

# Database Pool
DB_MAX_CONNECTIONS=10                # Database connection pool size
DB_IDLE_TIMEOUT=600                  # Connection idle timeout (seconds)

# Redis (if using caching)
REDIS_URL=redis://localhost:6379    # Redis connection URL
REDIS_PASSWORD=                      # Redis password
```

### Configuration File

Create `/app/config/loka-stratum.toml`:

```toml
[server]
bind_address = "0.0.0.0:3333"
max_connections = 1000
worker_threads = 4

[pool]
address = "stratum.pool.com:4444"
name = "main_pool"
username = "your_btc_address"
password = ""
separator = [".", "_"]

[database]
url = "postgres://user:pass@postgres:5432/loka"
max_connections = 10
idle_timeout = 600

[limiter]
connections = 1000
jobs = "600s"
submissions = "172800s"

[monitoring]
metrics_port = 9090
enable_prometheus = true
enable_health_check = true

[logging]
level = "info"
format = "json"
```

## Health Checks and Monitoring

### Built-in Health Checks

```bash
# Application health
curl http://localhost:9090/health

# Metrics endpoint
curl http://localhost:9090/metrics/prometheus

# Status information
curl http://localhost:9090/status
```

### Docker Health Checks

The containers include built-in health checks:

```bash
# Check container health
docker ps
# HEALTHY status indicates successful health checks

# View health check logs
docker inspect loka-stratum | jq '.[0].State.Health'
```

### Monitoring Integration

#### Prometheus Metrics

The application exposes comprehensive metrics:

```prometheus
# Connection metrics
loka_connections_active
loka_connections_total
loka_connection_errors_total

# Protocol metrics  
loka_messages_sent_total
loka_messages_received_total
loka_bytes_transferred_total

# Mining metrics
loka_jobs_sent_total
loka_submissions_received_total
loka_shares_accepted_total
loka_shares_rejected_total

# Performance metrics
loka_response_duration_seconds
loka_memory_usage_bytes
loka_cpu_usage_percent
```

#### Grafana Dashboards

Import the provided dashboards:

```bash
# Copy dashboards to Grafana
docker cp monitoring/grafana/dashboards/loka-stratum-comprehensive.json \
  grafana:/var/lib/grafana/dashboards/

# Or use provisioning (recommended)
# See docker-compose files for automatic provisioning
```

## Deployment Scenarios

### Single Container Deployment

For simple deployments without monitoring:

```bash
docker run -d \
  --name loka-stratum \
  --restart unless-stopped \
  -p 3333:3333 \
  -p 9090:9090 \
  -e DATABASE_URL=postgres://user:pass@host:5432/db \
  -e POOL_ADDRESS=pool.example.com \
  -e POOL_USERNAME=your_btc_address \
  -e RUST_LOG=info \
  -v ./data:/app/data \
  -v ./logs:/app/logs \
  -v ./config:/app/config:ro \
  ghcr.io/geriano/loka/loka-stratum:latest
```

### Docker Swarm Deployment

```yaml
# docker-stack.yml
version: '3.8'
services:
  stratum:
    image: ghcr.io/geriano/loka/loka-stratum:latest
    ports:
      - "3333:3333"
      - "9090:9090"
    environment:
      - DATABASE_URL=postgres://user:pass@db:5432/loka
    deploy:
      replicas: 3
      update_config:
        parallelism: 1
        delay: 10s
        order: start-first
      restart_policy:
        condition: on-failure
        delay: 5s
        max_attempts: 3
      resources:
        limits:
          memory: 1G
          cpus: '1.0'
        reservations:
          memory: 512M
          cpus: '0.5'
    networks:
      - loka-swarm

networks:
  loka-swarm:
    external: true
```

Deploy with:
```bash
docker stack deploy -c docker-stack.yml loka
```

### Kubernetes Deployment

```yaml
# k8s/deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: loka-stratum
  labels:
    app: loka-stratum
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
        image: ghcr.io/geriano/loka/loka-stratum:latest
        ports:
        - containerPort: 3333
          name: stratum
        - containerPort: 9090
          name: metrics
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: loka-secrets
              key: database-url
        - name: POOL_ADDRESS
          value: "pool.example.com"
        - name: POOL_USERNAME
          valueFrom:
            secretKeyRef:
              name: loka-secrets
              key: pool-username
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "1Gi"
            cpu: "1000m"
        livenessProbe:
          httpGet:
            path: /health
            port: 9090
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health
            port: 9090
          initialDelaySeconds: 5
          periodSeconds: 5
---
apiVersion: v1
kind: Service
metadata:
  name: loka-stratum-service
spec:
  selector:
    app: loka-stratum
  ports:
  - name: stratum
    port: 3333
    targetPort: 3333
  - name: metrics
    port: 9090
    targetPort: 9090
  type: LoadBalancer
```

## Performance Optimization

### Build Optimization

#### Multi-stage Build Benefits
- **Dependency Caching**: Dependencies built separately from application code
- **Size Reduction**: Only runtime artifacts included in final image
- **Security**: No build tools in production image

#### Build Arguments
```bash
# Optimize for specific architecture
docker build --platform linux/amd64 \
  --build-arg BUILDKIT_INLINE_CACHE=1 \
  -t loka-stratum:optimized .

# Build with custom Rust features
docker build \
  --build-arg CARGO_FEATURES="metrics,monitoring" \
  -t loka-stratum:custom-features .
```

### Runtime Optimization

#### Resource Limits
```bash
# Memory-constrained deployment
docker run -d \
  --memory=512m \
  --cpus=1.0 \
  --oom-kill-disable=false \
  ghcr.io/geriano/loka/loka-stratum:slim

# High-performance deployment
docker run -d \
  --memory=2g \
  --cpus=4.0 \
  --ulimit nofile=65536:65536 \
  ghcr.io/geriano/loka/loka-stratum:latest
```

#### Volume Optimization
```bash
# Use tmpfs for temporary data
docker run -d \
  --tmpfs /tmp:rw,noexec,nosuid,size=100m \
  --tmpfs /app/logs:rw,noexec,nosuid,size=200m \
  -v ./data:/app/data \
  ghcr.io/geriano/loka/loka-stratum:latest
```

## Security Considerations

### Container Security

#### Non-root Execution
All images run as user `loka` (UID 1001) for security:

```bash
# Verify non-root execution
docker run --rm ghcr.io/geriano/loka/loka-stratum:latest whoami
# Output: loka
```

#### Minimal Attack Surface
```bash
# Check image contents
docker run --rm ghcr.io/geriano/loka/loka-stratum:latest \
  sh -c "ls -la /usr/local/bin/ && apk list --installed"
```

#### Security Scanning
```bash
# Scan with Trivy
docker run --rm -v /var/run/docker.sock:/var/run/docker.sock \
  aquasec/trivy:latest image ghcr.io/geriano/loka/loka-stratum:latest

# Scan with Snyk (requires token)
docker scan ghcr.io/geriano/loka/loka-stratum:latest
```

### Network Security

#### Port Configuration
```bash
# Stratum protocol: 3333 (required)
# Metrics endpoint: 9090 (optional, for monitoring)

# Secure deployment - internal metrics only
docker run -d \
  -p 3333:3333 \
  --expose 9090 \
  ghcr.io/geriano/loka/loka-stratum:latest
```

#### TLS/SSL Configuration
```bash
# Use nginx reverse proxy for TLS termination
# See docker-compose.production.yml for full setup

# Or configure TLS in application (if supported)
docker run -d \
  -p 3334:3333 \
  -v ./certs:/app/certs:ro \
  -e TLS_CERT_PATH=/app/certs/cert.pem \
  -e TLS_KEY_PATH=/app/certs/key.pem \
  ghcr.io/geriano/loka/loka-stratum:latest
```

## Troubleshooting

### Common Issues

#### 1. Container Won't Start

**Check logs:**
```bash
docker logs loka-stratum
```

**Common causes:**
- Missing required environment variables
- Database connection issues
- Port conflicts
- Insufficient resources

**Resolution:**
```bash
# Check environment variables
docker inspect loka-stratum | jq '.[0].Config.Env'

# Test database connectivity
docker run --rm \
  -e DATABASE_URL=your_database_url \
  postgres:15-alpine \
  psql $DATABASE_URL -c "SELECT 1;"

# Check port availability
netstat -tlnp | grep :3333
```

#### 2. Performance Issues

**Monitor resource usage:**
```bash
# Check container stats
docker stats loka-stratum

# Check system resources
docker system df
docker system events
```

**Optimization steps:**
```bash
# Increase memory limit
docker update --memory=2g loka-stratum

# Check for memory leaks
docker exec loka-stratum ps aux
docker exec loka-stratum free -h
```

#### 3. Connection Issues

**Debug networking:**
```bash
# Check port bindings
docker port loka-stratum

# Test internal connectivity
docker exec loka-stratum netstat -tlnp
docker exec loka-stratum curl http://localhost:9090/health

# Test external connectivity
curl http://localhost:3333
telnet localhost 3333
```

#### 4. Database Issues

**Test database connectivity:**
```bash
# Test from container
docker exec loka-stratum \
  wget --spider http://localhost:9090/health

# Check database logs
docker logs postgres_container

# Test manual connection
docker run --rm -it \
  -e PGPASSWORD=your_password \
  postgres:15-alpine \
  psql -h your_host -U your_user -d your_db -c "SELECT version();"
```

### Debug Mode

#### Enable Debug Logging
```bash
docker run -d \
  --name loka-stratum-debug \
  -p 3333:3333 \
  -p 9090:9090 \
  -e RUST_LOG=debug \
  -e RUST_BACKTRACE=full \
  ghcr.io/geriano/loka/loka-stratum:debug
```

#### Interactive Debugging
```bash
# Start container with shell access
docker run -it \
  --entrypoint /bin/sh \
  ghcr.io/geriano/loka/loka-stratum:debug

# Or attach to running container
docker exec -it loka-stratum /bin/sh
```

#### Performance Profiling
```bash
# Enable performance tracing
docker run -d \
  -e RUST_LOG=trace \
  -e TOKIO_CONSOLE_BIND=0.0.0.0:6669 \
  -p 6669:6669 \
  ghcr.io/geriano/loka/loka-stratum:debug
```

## Maintenance

### Image Updates

#### Automated Updates
```bash
# Use Watchtower for automatic updates
docker run -d \
  --name watchtower \
  -v /var/run/docker.sock:/var/run/docker.sock \
  containrrr/watchtower \
  --interval 3600 \
  loka-stratum
```

#### Manual Updates
```bash
# Pull latest image
docker pull ghcr.io/geriano/loka/loka-stratum:latest

# Stop current container
docker stop loka-stratum

# Remove old container
docker rm loka-stratum

# Start with new image
docker run -d \
  --name loka-stratum \
  # ... same configuration as before ...
  ghcr.io/geriano/loka/loka-stratum:latest
```

### Data Backup

#### Database Backup
```bash
# Backup PostgreSQL data
docker exec postgres pg_dump -U user database > backup.sql

# Restore from backup
docker exec -i postgres psql -U user database < backup.sql
```

#### Application Data Backup
```bash
# Backup application data
docker cp loka-stratum:/app/data ./backup/data/
docker cp loka-stratum:/app/logs ./backup/logs/

# Create volume backup
docker run --rm \
  -v loka_stratum-data:/data \
  -v $(pwd)/backup:/backup \
  alpine tar czf /backup/stratum-data.tar.gz -C /data .
```

### Log Management

#### Log Collection
```bash
# View real-time logs
docker logs -f loka-stratum

# Export logs
docker logs loka-stratum > loka-stratum.log

# Rotate logs (if using file logging)
docker exec loka-stratum logrotate /etc/logrotate.conf
```

#### Log Analysis
```bash
# Search for errors
docker logs loka-stratum 2>&1 | grep -i error

# Monitor connection metrics
docker logs loka-stratum 2>&1 | grep -i "connection"

# Extract JSON logs for analysis
docker logs loka-stratum --since=1h | jq 'select(.level == "ERROR")'
```

## Advanced Configuration

### Custom Entrypoints

#### Development Entrypoint
```bash
#!/bin/sh
# custom-dev-entrypoint.sh
echo "Starting Loka Stratum in development mode..."
export RUST_LOG=debug
export RUST_BACKTRACE=full
exec loka-stratum start --bind 0.0.0.0:3333 --dev-mode
```

```bash
# Use custom entrypoint
docker run -d \
  -v ./custom-dev-entrypoint.sh:/custom-entrypoint.sh:ro \
  --entrypoint /custom-entrypoint.sh \
  ghcr.io/geriano/loka/loka-stratum:latest
```

#### Production Entrypoint with Initialization
```bash
#!/bin/sh
# production-entrypoint.sh

# Wait for database
while ! pg_isready -h $DATABASE_HOST -p $DATABASE_PORT -U $DATABASE_USER; do
  echo "Waiting for database..."
  sleep 2
done

# Run migrations if needed
if [ "$AUTO_MIGRATE" = "true" ]; then
  echo "Running database migrations..."
  migration
fi

# Start application
echo "Starting Loka Stratum..."
exec loka-stratum start --bind 0.0.0.0:3333
```

### Multi-environment Setup

#### Environment-specific Configurations
```bash
# Development
docker run -d \
  --env-file .env.development \
  -v ./config/development.toml:/app/config/loka-stratum.toml:ro \
  ghcr.io/geriano/loka/loka-stratum:debug

# Staging  
docker run -d \
  --env-file .env.staging \
  -v ./config/staging.toml:/app/config/loka-stratum.toml:ro \
  ghcr.io/geriano/loka/loka-stratum:latest

# Production
docker run -d \
  --env-file .env.production \
  -v ./config/production.toml:/app/config/loka-stratum.toml:ro \
  ghcr.io/geriano/loka/loka-stratum:latest
```

### Integration Examples

#### With External Monitoring
```bash
# With external Prometheus
docker run -d \
  --name loka-stratum \
  -p 3333:3333 \
  --expose 9090 \
  --label prometheus.io/scrape=true \
  --label prometheus.io/port=9090 \
  --label prometheus.io/path=/metrics/prometheus \
  ghcr.io/geriano/loka/loka-stratum:latest
```

#### With Service Discovery
```bash
# With Consul for service discovery
docker run -d \
  --name loka-stratum \
  -p 3333:3333 \
  -e CONSUL_AGENT=consul:8500 \
  -e SERVICE_NAME=loka-stratum \
  -e SERVICE_TAGS=mining,proxy,stratum \
  ghcr.io/geriano/loka/loka-stratum:latest
```

## Best Practices

### Production Deployment

1. **Always use specific version tags**, never `:latest` in production
2. **Set resource limits** to prevent resource exhaustion
3. **Use secrets management** for sensitive configuration
4. **Enable health checks** for proper orchestration
5. **Monitor resource usage** and application metrics
6. **Implement proper backup strategies** for data persistence
7. **Use read-only filesystems** where possible for security

### Development Workflow

1. **Use development compose configuration** for local development
2. **Mount source code volumes** for hot-reload development
3. **Enable debug logging** for troubleshooting
4. **Use debug image variants** for development and testing
5. **Test Docker builds locally** before pushing changes

### Monitoring and Alerting

1. **Configure Prometheus scraping** for all instances
2. **Set up Grafana dashboards** for visualization
3. **Implement alerting rules** for critical metrics
4. **Monitor container health** and resource usage
5. **Set up log aggregation** for centralized logging

## Registry Information

### GitHub Container Registry
- **Registry**: `ghcr.io/geriano/loka/loka-stratum`
- **Authentication**: GitHub token required for private repositories
- **Features**: Multi-arch support, vulnerability scanning, package management

### Docker Hub
- **Registry**: `docker.io/geriano/loka-stratum`
- **Authentication**: Docker Hub credentials required
- **Features**: Public distribution, usage analytics, automated builds

### Image Tags

**Version Tags:**
- `v0.1.0` - Specific release version
- `0.1` - Minor version latest
- `0` - Major version latest

**Branch Tags:**
- `master` - Latest master branch build
- `develop` - Latest develop branch build

**Special Tags:**
- `latest` - Latest stable release
- `latest-slim` - Latest slim variant
- `latest-debug` - Latest debug variant

**SHA Tags:**
- `master-abc1234` - Specific commit from master branch
- `develop-def5678` - Specific commit from develop branch