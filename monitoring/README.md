# Loka Stratum Monitoring Stack

Complete monitoring solution for Loka Stratum Bitcoin Mining Proxy with Prometheus, Grafana, AlertManager, and self-hosted Sentry for comprehensive observability.

## Services Overview

### Core Monitoring
- **Prometheus** (`:9090`) - Metrics collection and storage
- **Grafana** (`:3000`) - Visualization and dashboards
- **AlertManager** (`:9093`) - Alert routing and management

### System Monitoring
- **Node Exporter** (`:9100`) - Host system metrics
- **cAdvisor** (`:8080`) - Container resource monitoring

### Error Tracking & Performance
- **Sentry** (`:9000`) - Error tracking, performance monitoring, and tracing
- **PostgreSQL** - Sentry database backend
- **Redis** - Sentry caching and queuing

## Quick Start

### 1. Environment Setup

```bash
# Copy and configure environment variables
cp .env.example .env
# Edit .env with your preferred passwords and configuration
```

### 2. Start the Stack

```bash
# Start all monitoring services
docker-compose up -d

# View logs
docker-compose logs -f

# Start specific services only
docker-compose up -d prometheus grafana
```

### 3. Initialize Sentry

After the services are running, initialize Sentry:

```bash
# Run Sentry initialization (creates admin user and project)
docker-compose exec sentry /etc/sentry/init-sentry.sh

# Or run it as a one-time container
docker-compose run --rm sentry bash /etc/sentry/init-sentry.sh
```

### 4. Access Services

- **Grafana**: http://localhost:3000 (admin/admin123)
- **Prometheus**: http://localhost:9090
- **AlertManager**: http://localhost:9093  
- **Sentry**: http://localhost:9000 (admin@loka-stratum.local/admin123)
- **Node Exporter**: http://localhost:9100
- **cAdvisor**: http://localhost:8080

## Sentry Integration

### Getting Your DSN

After initializing Sentry, get your project DSN:

```bash
# Get the DSN for Loka Stratum integration
docker exec -it monitoring-sentry-1 cat /var/lib/sentry/files/loka-stratum-dsn.txt
```

### Configure Loka Stratum

Add the DSN to your Loka Stratum configuration:

```toml
# In loka-stratum.toml
[sentry]
dsn = "https://your-sentry-dsn@localhost:9000/1"
environment = "production"
sample_rate = 1.0
traces_sample_rate = 1.0
```

Or use environment variables:
```bash
export SENTRY_DSN="https://your-sentry-dsn@localhost:9000/1"
export SENTRY_ENVIRONMENT="production"
```

## Service Details

### Sentry Configuration

The Sentry setup includes:

- **Multi-service architecture**: Web server, worker, and cron services
- **PostgreSQL backend**: Persistent data storage
- **Redis caching**: Performance optimization and task queuing
- **Health checks**: Automatic service recovery
- **Mining-optimized settings**: Extended retention, higher throughput limits

#### Sentry Features Enabled

- Error tracking with full stack traces
- Performance monitoring and tracing
- Custom inbound filters for mining data
- Session replay for debugging
- Crash rate alerts
- Data forwarding capabilities

### Prometheus Targets

Configured to scrape:
- Loka Stratum metrics (`:9090/metrics/prometheus`)
- Node Exporter (`:9100/metrics`)
- cAdvisor (`:8080/metrics`)  
- Prometheus itself (`:9090/metrics`)

### Grafana Dashboards

Pre-configured dashboards:
- **Loka Stratum Overview** - Key mining proxy metrics
- **Loka Stratum Comprehensive** - Detailed performance monitoring
- **System Overview** - Host and container resource monitoring

### AlertManager Rules

Pre-configured alerts for:
- High error rates
- Connection failures  
- Resource exhaustion
- Service downtime

## Storage & Persistence

Data is persisted using Docker volumes:

- `prometheus-data` - Metrics storage (30-day retention)
- `grafana-data` - Dashboards and configuration
- `alertmanager-data` - Alert state and configuration
- `sentry-postgres-data` - Sentry database
- `sentry-redis-data` - Redis cache data
- `sentry-data` - Sentry file storage

## Configuration Files

### Prometheus
- `prometheus/prometheus.yml` - Scrape configuration
- `prometheus/rules/loka-stratum.yml` - Alerting rules

### Grafana  
- `grafana/provisioning/` - Auto-provisioning configuration
- `grafana/dashboards/` - Dashboard definitions

### AlertManager
- `alertmanager/alertmanager.yml` - Alert routing configuration

### Sentry
- `sentry/config.yml` - Sentry server configuration
- `sentry/init-sentry.sh` - Initialization script
- `sentry/requirements.txt` - Additional Python packages

## Maintenance

### Backup Data

```bash
# Backup Prometheus data
docker run --rm -v monitoring_prometheus-data:/data -v $(pwd):/backup alpine tar czf /backup/prometheus-backup.tar.gz /data

# Backup Grafana data  
docker run --rm -v monitoring_grafana-data:/data -v $(pwd):/backup alpine tar czf /backup/grafana-backup.tar.gz /data

# Backup Sentry data
docker run --rm -v monitoring_sentry-postgres-data:/data -v $(pwd):/backup alpine tar czf /backup/sentry-db-backup.tar.gz /data
```

### Update Services

```bash
# Update all services
docker-compose pull
docker-compose up -d

# Update specific service
docker-compose pull sentry
docker-compose up -d sentry sentry-worker sentry-cron
```

### Clean Up

```bash
# Stop all services
docker-compose down

# Remove volumes (WARNING: destroys all data)
docker-compose down -v

# Clean up unused images
docker image prune -f
```

## Troubleshooting

### Sentry Issues

**Database connection problems:**
```bash
# Check PostgreSQL logs
docker-compose logs sentry-postgres

# Manually run database migrations
docker-compose exec sentry sentry upgrade --noinput
```

**Permission issues:**
```bash
# Fix file permissions
docker-compose exec sentry chown -R sentry:sentry /var/lib/sentry/files
```

**Reset Sentry:**
```bash
# Stop Sentry services
docker-compose stop sentry sentry-worker sentry-cron

# Remove Sentry data (WARNING: destroys Sentry data)
docker volume rm monitoring_sentry-postgres-data monitoring_sentry-data

# Restart and reinitialize
docker-compose up -d sentry-postgres sentry-redis
sleep 10
docker-compose up -d sentry
sleep 30
docker-compose run --rm sentry bash /etc/sentry/init-sentry.sh
```

### Service Health

```bash
# Check service health
docker-compose ps

# Check specific service logs
docker-compose logs sentry
docker-compose logs prometheus
docker-compose logs grafana

# Monitor resource usage
docker stats
```

### Network Issues

```bash
# Test network connectivity between services
docker-compose exec sentry ping sentry-postgres
docker-compose exec sentry ping sentry-redis
docker-compose exec prometheus ping host.docker.internal
```

## Security Considerations

- Change default passwords in `.env`
- Use strong secret keys for Sentry
- Consider enabling TLS/SSL for production
- Restrict network access as needed
- Regular security updates for all services

## Performance Tuning

### Sentry Performance

For high-volume mining operations:

```yaml
# In sentry/config.yml
system:
  worker-pool-size: 20
  celery-worker-concurrency: 8
  event-max-size: 2097152  # 2MB
  transaction-trace-limit: 2000
```

### Prometheus Retention

Adjust retention based on storage capacity:

```yaml
# In docker-compose.yml prometheus command
- '--storage.tsdb.retention.time=90d'
- '--storage.tsdb.retention.size=50GB'
```

## Integration Examples

### Rust Application (Loka Stratum)

```rust
use sentry::{ClientOptions, configure_scope};

fn main() {
    let _guard = sentry::init(ClientOptions {
        dsn: std::env::var("SENTRY_DSN").ok().as_deref().map(|dsn| dsn.parse().unwrap()),
        environment: Some("production".into()),
        sample_rate: 1.0,
        traces_sample_rate: 1.0,
        ..Default::default()
    });

    // Your application code
}
```

### Custom Metrics

```rust
use sentry::{add_breadcrumb, Breadcrumb, Level};

// Add mining context
configure_scope(|scope| {
    scope.set_tag("mining_pool", "bitcoin_pool_1");
    scope.set_user(Some(sentry::User {
        id: Some(miner_id.to_string()),
        ..Default::default()
    }));
});

// Add breadcrumbs for mining operations
add_breadcrumb(Breadcrumb {
    message: Some("Share submitted".into()),
    level: Level::Info,
    data: {
        let mut map = std::collections::BTreeMap::new();
        map.insert("difficulty".to_string(), difficulty.into());
        map.insert("hash_rate".to_string(), hash_rate.into());
        map
    },
    ..Default::default()
});
```