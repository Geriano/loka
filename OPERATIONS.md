# Loka Stratum Operations Manual

Comprehensive operations manual for managing Loka Stratum Bitcoin Mining Proxy in production environments, covering service management, health monitoring, troubleshooting, and maintenance procedures.

## Table of Contents

- [Service Management](#service-management)
- [Health Monitoring](#health-monitoring)
- [Log Management](#log-management)
- [Performance Monitoring](#performance-monitoring)
- [Troubleshooting](#troubleshooting)
- [Maintenance Procedures](#maintenance-procedures)
- [Backup and Recovery](#backup-and-recovery)
- [Scaling Operations](#scaling-operations)
- [Incident Response](#incident-response)
- [Operational Runbooks](#operational-runbooks)

## Service Management

### Starting and Stopping Services

#### Docker Compose Environment

**Start Services:**
```bash
# Start all services
docker-compose up -d

# Start specific service
docker-compose up -d loka-stratum

# Start with build (if image updated)
docker-compose up -d --build

# Start monitoring stack
cd monitoring && docker-compose up -d

# View startup logs
docker-compose logs -f loka-stratum
```

**Stop Services:**
```bash
# Stop all services
docker-compose down

# Stop specific service
docker-compose stop loka-stratum

# Stop with volume cleanup (WARNING: destroys data)
docker-compose down -v

# Graceful shutdown
docker-compose kill -s SIGTERM loka-stratum
sleep 10
docker-compose kill loka-stratum
```

#### Kubernetes Environment

**Service Management:**
```bash
# Check deployment status
kubectl get deployments -n loka-stratum

# Scale deployment
kubectl scale deployment loka-stratum --replicas=5 -n loka-stratum

# Rolling update
kubectl set image deployment/loka-stratum loka-stratum=loka-stratum:v2.1.0 -n loka-stratum

# Rollback deployment
kubectl rollout undo deployment/loka-stratum -n loka-stratum

# Restart deployment
kubectl rollout restart deployment/loka-stratum -n loka-stratum

# Check rollout status
kubectl rollout status deployment/loka-stratum -n loka-stratum
```

#### Systemd Service (Native Installation)

Create systemd service file at `/etc/systemd/system/loka-stratum.service`:
```ini
[Unit]
Description=Loka Stratum Bitcoin Mining Proxy
After=network.target postgresql.service
Wants=network.target

[Service]
Type=exec
User=loka
Group=loka
WorkingDirectory=/opt/loka-stratum
Environment=RUST_LOG=loka_stratum=info
Environment=DATABASE_URL=postgresql://loka_user:secure_password@localhost:5432/loka_stratum
ExecStart=/opt/loka-stratum/bin/loka-stratum start
ExecReload=/bin/kill -HUP $MAINPID
Restart=always
RestartSec=5
TimeoutStopSec=30
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
```

**Service Operations:**
```bash
# Enable and start service
sudo systemctl enable loka-stratum
sudo systemctl start loka-stratum

# Check service status
sudo systemctl status loka-stratum

# View logs
sudo journalctl -u loka-stratum -f

# Restart service
sudo systemctl restart loka-stratum

# Reload configuration
sudo systemctl reload loka-stratum

# Stop service
sudo systemctl stop loka-stratum
```

### Configuration Management

#### Configuration Validation

```bash
# Validate configuration file
./loka-stratum config validate --config loka-stratum.toml

# Show current configuration
./loka-stratum config show --config loka-stratum.toml

# Test configuration with dry-run
./loka-stratum start --config loka-stratum.toml --dry-run

# Generate example configuration
./loka-stratum config generate --output example.toml
```

#### Configuration Hot Reload

```bash
# Send SIGHUP to reload configuration (Docker)
docker-compose kill -s SIGHUP loka-stratum

# Kubernetes configmap update and restart
kubectl create configmap loka-stratum-config --from-file=loka-stratum.toml --dry-run=client -o yaml | kubectl apply -f -
kubectl rollout restart deployment/loka-stratum -n loka-stratum

# Systemd reload
sudo systemctl reload loka-stratum
```

#### Environment Variable Management

```bash
# View current environment
docker-compose exec loka-stratum env | grep LOKA

# Update environment variables
docker-compose down
# Edit .env file
docker-compose up -d

# Kubernetes secrets update
kubectl create secret generic loka-stratum-secrets \
  --from-literal=database-url="new_database_url" \
  --dry-run=client -o yaml | kubectl apply -f -
kubectl rollout restart deployment/loka-stratum -n loka-stratum
```

## Health Monitoring

### Health Check Endpoints

#### Basic Health Check

```bash
# Check service health
curl -f http://localhost:9090/health

# Detailed health information
curl -s http://localhost:9090/health | jq '.'

# Expected healthy response:
{
  "status": "healthy",
  "uptime": "2d 4h 23m",
  "version": "2.1.0",
  "connections": {
    "active": 234,
    "total": 1523,
    "max": 1000
  },
  "services": {
    "database": "healthy",
    "pool_connection": "healthy",
    "metrics": "healthy"
  },
  "memory_usage": 128456789,
  "cpu_usage": 15.2
}
```

#### Readiness and Liveness Probes

```bash
# Readiness check (can serve traffic)
curl -f http://localhost:9090/ready

# Liveness check (process is alive)
curl -f http://localhost:9090/live

# Metrics endpoint health
curl -f http://localhost:9090/metrics/prometheus | head -n 10
```

### Monitoring Service Status

#### Docker Environment

```bash
# Check container status
docker-compose ps

# View resource usage
docker stats

# Check container health
docker inspect --format='{{.State.Health.Status}}' loka-stratum_loka-stratum_1

# Monitor container logs
docker-compose logs -f --tail=100 loka-stratum
```

#### Kubernetes Environment

```bash
# Check pod status
kubectl get pods -n loka-stratum

# Describe pod details
kubectl describe pod <pod-name> -n loka-stratum

# Check pod resource usage
kubectl top pods -n loka-stratum

# View pod events
kubectl get events --sort-by=.metadata.creationTimestamp -n loka-stratum
```

### Automated Health Monitoring

#### Monitoring Script

Create `/opt/loka-stratum/monitor.sh`:
```bash
#!/bin/bash

# Health monitoring script for Loka Stratum

HEALTH_URL="http://localhost:9090/health"
METRICS_URL="http://localhost:9090/metrics/prometheus"
LOG_FILE="/var/log/loka-stratum/health.log"
ALERT_EMAIL="ops@example.com"

# Check health endpoint
check_health() {
    if ! curl -sf "$HEALTH_URL" > /dev/null; then
        echo "$(date): CRITICAL - Health check failed" >> "$LOG_FILE"
        send_alert "CRITICAL" "Loka Stratum health check failed"
        return 1
    fi
    return 0
}

# Check metrics endpoint
check_metrics() {
    if ! curl -sf "$METRICS_URL" | grep -q "loka_stratum_connections_total"; then
        echo "$(date): WARNING - Metrics endpoint issue" >> "$LOG_FILE"
        send_alert "WARNING" "Loka Stratum metrics endpoint issue"
        return 1
    fi
    return 0
}

# Check active connections
check_connections() {
    local active_connections=$(curl -s "$HEALTH_URL" | jq -r '.connections.active // 0')
    local max_connections=$(curl -s "$HEALTH_URL" | jq -r '.connections.max // 1000')
    local usage_percent=$((active_connections * 100 / max_connections))
    
    if [ "$usage_percent" -gt 90 ]; then
        echo "$(date): WARNING - High connection usage: $usage_percent%" >> "$LOG_FILE"
        send_alert "WARNING" "High connection usage: $usage_percent% ($active_connections/$max_connections)"
    fi
}

# Send alert
send_alert() {
    local level=$1
    local message=$2
    echo "[$level] $message" | mail -s "Loka Stratum Alert: $level" "$ALERT_EMAIL"
}

# Main monitoring loop
main() {
    echo "$(date): Starting health check" >> "$LOG_FILE"
    
    if check_health; then
        check_metrics
        check_connections
        echo "$(date): Health check completed successfully" >> "$LOG_FILE"
    else
        echo "$(date): Health check failed" >> "$LOG_FILE"
        exit 1
    fi
}

main "$@"
```

#### Cron Job Setup

```bash
# Install monitoring script
sudo cp monitor.sh /opt/loka-stratum/monitor.sh
sudo chmod +x /opt/loka-stratum/monitor.sh

# Add cron job (check every 5 minutes)
echo "*/5 * * * * /opt/loka-stratum/monitor.sh" | crontab -

# View cron logs
sudo tail -f /var/log/cron
```

## Log Management

### Log Locations

#### Docker Environment

```bash
# Container logs
docker-compose logs loka-stratum

# Application logs (if volume mounted)
tail -f ./logs/loka-stratum.log

# System logs
sudo journalctl -u docker -f
```

#### Kubernetes Environment

```bash
# Pod logs
kubectl logs -f deployment/loka-stratum -n loka-stratum

# Previous container logs
kubectl logs deployment/loka-stratum --previous -n loka-stratum

# Multiple containers
kubectl logs -f -l app=loka-stratum -n loka-stratum
```

#### Native Installation

```bash
# Application logs
tail -f /var/log/loka-stratum/app.log

# System service logs
sudo journalctl -u loka-stratum -f

# Syslog entries
sudo tail -f /var/log/syslog | grep loka-stratum
```

### Log Analysis

#### Common Log Patterns

```bash
# Connection events
grep "Connection established" /var/log/loka-stratum/app.log

# Authentication events
grep "mining.authorize" /var/log/loka-stratum/app.log

# Error events
grep "ERROR" /var/log/loka-stratum/app.log | tail -n 50

# Performance metrics
grep "Performance" /var/log/loka-stratum/app.log

# Pool communication
grep "Pool" /var/log/loka-stratum/app.log | tail -n 100
```

#### Log Analysis with jq (for JSON logs)

```bash
# Parse JSON logs
cat /var/log/loka-stratum/app.log | grep "^{" | jq '.'

# Filter by log level
cat /var/log/loka-stratum/app.log | grep "^{" | jq 'select(.level=="ERROR")'

# Extract performance metrics
cat /var/log/loka-stratum/app.log | grep "^{" | jq 'select(.target=="performance") | {timestamp, message, metrics}'

# Count errors by type
cat /var/log/loka-stratum/app.log | grep "^{" | jq -r 'select(.level=="ERROR") | .fields.error_type' | sort | uniq -c
```

### Log Rotation and Cleanup

#### Logrotate Configuration

Create `/etc/logrotate.d/loka-stratum`:
```
/var/log/loka-stratum/*.log {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    notifempty
    create 644 loka loka
    postrotate
        /usr/bin/docker-compose -f /opt/loka-stratum/docker-compose.yml kill -s SIGUSR1 loka-stratum || true
    endscript
}
```

#### Manual Log Cleanup

```bash
# Clean logs older than 30 days
find /var/log/loka-stratum -name "*.log" -mtime +30 -delete

# Archive logs
tar -czf /backup/logs-$(date +%Y%m%d).tar.gz /var/log/loka-stratum/*.log

# Docker log cleanup
docker system prune -f
docker volume prune -f
```

### Centralized Logging

#### ELK Stack Integration

```yaml
# docker-compose.yml addition
version: '3.8'
services:
  filebeat:
    image: docker.elastic.co/beats/filebeat:8.0.0
    user: root
    volumes:
      - ./logs:/usr/share/filebeat/logs:ro
      - ./filebeat.yml:/usr/share/filebeat/filebeat.yml:ro
      - /var/lib/docker/containers:/var/lib/docker/containers:ro
      - /var/run/docker.sock:/var/run/docker.sock:ro
    environment:
      - ELASTICSEARCH_HOSTS=http://elasticsearch:9200
```

#### Loki Integration

```yaml
# promtail-config.yaml
server:
  http_listen_port: 9080
  grpc_listen_port: 0

positions:
  filename: /tmp/positions.yaml

clients:
  - url: http://loki:3100/loki/api/v1/push

scrape_configs:
- job_name: loka-stratum
  static_configs:
  - targets:
      - localhost
    labels:
      job: loka-stratum
      __path__: /var/log/loka-stratum/*.log
```

## Performance Monitoring

### Key Performance Metrics

#### Connection Metrics

```bash
# Monitor active connections
watch -n 1 'curl -s http://localhost:9090/health | jq .connections'

# Connection rate
curl -s http://localhost:9090/metrics/prometheus | grep loka_stratum_connections_rate

# Connection errors
curl -s http://localhost:9090/metrics/prometheus | grep loka_stratum_connection_errors
```

#### Mining Metrics

```bash
# Share submission rate
curl -s http://localhost:9090/metrics/prometheus | grep loka_stratum_shares_submitted

# Share acceptance rate
curl -s http://localhost:9090/metrics/prometheus | grep loka_stratum_shares_accepted

# Job distribution rate
curl -s http://localhost:9090/metrics/prometheus | grep loka_stratum_jobs_distributed
```

#### System Resource Metrics

```bash
# Memory usage
docker stats --format "table {{.Container}}\t{{.CPUPerc}}\t{{.MemUsage}}\t{{.NetIO}}\t{{.BlockIO}}"

# CPU usage
top -p $(pgrep loka-stratum)

# Network I/O
nethogs
iftop -i eth0
```

### Performance Analysis Tools

#### Benchmarking

```bash
# Run performance benchmarks
cd stratum
./run_benchmarks.sh all

# Generate performance report
./run_benchmarks.sh report

# Compare with baseline
./run_benchmarks.sh compare baseline_v1.0
```

#### Profiling

```bash
# Memory profiling
RUST_LOG=debug valgrind --tool=memcheck ./target/release/loka-stratum

# CPU profiling with perf
perf record -g ./target/release/loka-stratum
perf report

# Flame graph generation
perf record -g ./target/release/loka-stratum
perf script | stackcollapse-perf.pl | flamegraph.pl > flame.svg
```

### Performance Tuning

#### Configuration Optimization

```toml
# loka-stratum.toml - Performance optimizations

[server]
port = 3333
worker_threads = 8  # Match CPU cores
max_blocking_threads = 16

[pool]
connection_pool_size = 100
connection_timeout = "30s"
keepalive_interval = "60s"

[limiter]
connections = 5000  # Based on system capacity
rate_limit_per_ip = 1000
rate_limit_window = "1m"

[performance]
enable_metrics_caching = true
metrics_cache_ttl = "30s"
enable_string_interning = true
buffer_pool_size = 1000
connection_pool_max_idle = 50

[database]
max_connections = 50
min_connections = 10
acquire_timeout = "30s"
idle_timeout = "10m"
```

#### System-Level Optimizations

```bash
# Increase file descriptor limits
echo "loka soft nofile 65536" >> /etc/security/limits.conf
echo "loka hard nofile 65536" >> /etc/security/limits.conf

# TCP optimizations
echo 'net.core.rmem_max = 134217728' >> /etc/sysctl.conf
echo 'net.core.wmem_max = 134217728' >> /etc/sysctl.conf
echo 'net.ipv4.tcp_rmem = 4096 87380 134217728' >> /etc/sysctl.conf
echo 'net.ipv4.tcp_wmem = 4096 65536 134217728' >> /etc/sysctl.conf
sysctl -p

# CPU governor optimization
echo 'performance' > /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor
```

## Troubleshooting

### Common Issues and Solutions

#### Connection Issues

**Problem:** Miners cannot connect to proxy
```bash
# Check if service is running
systemctl status loka-stratum
docker-compose ps

# Check port binding
netstat -tlnp | grep :3333
ss -tlnp | grep :3333

# Check firewall
sudo ufw status
sudo iptables -L

# Test local connectivity
telnet localhost 3333

# Check logs for connection errors
grep -i "connection" /var/log/loka-stratum/app.log | tail -20
```

**Solution:**
```bash
# Restart service
systemctl restart loka-stratum

# Check and fix firewall
sudo ufw allow 3333/tcp

# Verify configuration
loka-stratum config validate --config loka-stratum.toml
```

#### Pool Connection Issues

**Problem:** Cannot connect to mining pool
```bash
# Test pool connectivity
telnet pool.example.com 4444

# Check DNS resolution
nslookup pool.example.com
dig pool.example.com

# Check logs for pool errors
grep -i "pool" /var/log/loka-stratum/app.log | tail -20

# Test with different pool
curl -v telnet://different-pool.com:4444
```

**Solution:**
```bash
# Update pool configuration
nano loka-stratum.toml  # Update [pool] section

# Test with curl
curl -v --connect-timeout 10 pool.example.com:4444

# Check network policies (Kubernetes)
kubectl get networkpolicies -n loka-stratum
```

#### High Memory Usage

**Problem:** Excessive memory consumption
```bash
# Monitor memory usage
docker stats loka-stratum
top -p $(pgrep loka-stratum)

# Check memory leaks
valgrind --tool=memcheck --leak-check=full ./loka-stratum

# Analyze heap usage
pmap -x $(pgrep loka-stratum)
```

**Solution:**
```bash
# Adjust memory limits in configuration
[limiter]
connections = 2000  # Reduce if needed

# Restart with memory limits
docker run --memory=2g loka-stratum

# Enable garbage collection logging
RUST_LOG=debug RUST_BACKTRACE=1 loka-stratum
```

#### Database Connection Issues

**Problem:** Database connection failures
```bash
# Test database connectivity
psql -h localhost -U loka_user -d loka_stratum -c "SELECT 1;"

# Check database logs
sudo tail -f /var/log/postgresql/postgresql-15-main.log

# Check connection pool status
curl -s http://localhost:9090/metrics/prometheus | grep db_connections
```

**Solution:**
```bash
# Restart database
sudo systemctl restart postgresql

# Check database configuration
sudo nano /etc/postgresql/15/main/postgresql.conf

# Run database migrations
cd migration && DATABASE_URL="postgresql://..." cargo run

# Check database disk space
df -h /var/lib/postgresql
```

### Debug Mode Operations

#### Enable Debug Logging

```bash
# Environment variable
export RUST_LOG=loka_stratum=debug,tower_http=debug

# Docker environment
docker-compose down
echo "RUST_LOG=loka_stratum=debug" >> .env
docker-compose up -d

# Kubernetes
kubectl set env deployment/loka-stratum RUST_LOG=loka_stratum=debug -n loka-stratum
```

#### Debug Specific Components

```bash
# Protocol debugging
export RUST_LOG=loka_stratum::protocol=trace

# Network debugging
export RUST_LOG=loka_stratum::network=debug

# Database debugging
export RUST_LOG=loka_stratum::services::database=debug

# Metrics debugging
export RUST_LOG=loka_stratum::services::metrics=debug
```

#### Trace Analysis

```bash
# Enable tracing
export RUST_LOG=loka_stratum=trace

# Save traces to file
loka-stratum 2>&1 | tee debug.log

# Analyze trace patterns
grep -E "TRACE|DEBUG" debug.log | grep "connection" | tail -50
```

### Network Diagnostics

#### TCP Connection Analysis

```bash
# Monitor TCP connections
netstat -an | grep :3333
ss -an | grep :3333

# Check connection states
netstat -anp | grep loka-stratum

# Monitor network traffic
tcpdump -i any port 3333 -w capture.pcap
wireshark capture.pcap
```

#### Latency Testing

```bash
# Test pool latency
ping pool.example.com
traceroute pool.example.com

# Measure connection time
time telnet pool.example.com 4444

# HTTP latency testing
curl -w "@curl-format.txt" -o /dev/null -s "http://localhost:9090/health"
```

### Performance Diagnostics

#### Resource Monitoring

```bash
# CPU usage analysis
sar -u 1 10
iostat -x 1 10

# Memory analysis
free -h
vmstat 1 10

# Disk I/O analysis
iotop -ao

# Network analysis
iftop -P -i eth0
```

#### Application Profiling

```bash
# Profile with perf
sudo perf top -p $(pgrep loka-stratum)

# Memory profiling
heaptrack ./loka-stratum

# Strace system calls
strace -p $(pgrep loka-stratum) -e trace=network

# GDB debugging
gdb --pid $(pgrep loka-stratum)
```

## Maintenance Procedures

### Regular Maintenance Tasks

#### Daily Tasks

```bash
#!/bin/bash
# daily-maintenance.sh

# Check service health
curl -f http://localhost:9090/health || echo "Health check failed"

# Check disk space
df -h | awk 'NR==1{print $0}; /loka/{if($5+0 > 80) print $0, "WARNING: Disk usage high"}'

# Check log file sizes
find /var/log/loka-stratum -name "*.log" -size +100M -exec ls -lh {} \;

# Verify database connectivity
psql -h localhost -U loka_user -d loka_stratum -c "SELECT COUNT(*) FROM pools;" || echo "Database check failed"

# Check for core dumps
find /tmp -name "core.*" -mtime -1

# Update metrics snapshot
curl -s http://localhost:9090/metrics/prometheus > /tmp/daily-metrics-$(date +%Y%m%d).txt
```

#### Weekly Tasks

```bash
#!/bin/bash
# weekly-maintenance.sh

# Clean old logs
find /var/log/loka-stratum -name "*.log.*" -mtime +7 -delete

# Database maintenance
sudo -u postgres psql loka_stratum -c "VACUUM ANALYZE;"

# Security updates check
apt list --upgradable | grep -i security

# Performance report
cd /opt/loka-stratum && ./run_benchmarks.sh report

# Backup verification
/opt/loka-stratum/scripts/verify-backups.sh

# Certificate expiry check
openssl x509 -in /etc/nginx/ssl/loka-stratum.crt -noout -dates
```

#### Monthly Tasks

```bash
#!/bin/bash
# monthly-maintenance.sh

# Full database backup
pg_dump -U loka_user -h localhost loka_stratum | gzip > /backup/loka-stratum-$(date +%Y%m%d).sql.gz

# Archive old logs
tar -czf /backup/logs-$(date +%Y%m).tar.gz /var/log/loka-stratum/
find /var/log/loka-stratum -name "*.log.*" -mtime +30 -delete

# Security audit
lynis audit system

# Performance baseline update
cd /opt/loka-stratum && ./run_benchmarks.sh baseline monthly-$(date +%Y%m)

# Docker image updates
docker-compose pull
docker image prune -f

# SSL certificate renewal
certbot renew --dry-run
```

### Service Updates

#### Application Updates

```bash
# Docker environment update
docker-compose down
docker-compose pull loka-stratum
docker-compose up -d

# Verify update
curl -s http://localhost:9090/health | jq .version

# Rollback if needed
docker-compose down
docker tag loka-stratum:previous loka-stratum:latest
docker-compose up -d
```

#### Configuration Updates

```bash
# Validate new configuration
loka-stratum config validate --config new-loka-stratum.toml

# Backup current configuration
cp loka-stratum.toml loka-stratum.toml.backup

# Apply new configuration
cp new-loka-stratum.toml loka-stratum.toml

# Graceful reload
kill -SIGHUP $(pgrep loka-stratum)

# Verify configuration applied
curl -s http://localhost:9090/health | jq .
```

#### Database Updates

```bash
# Backup before update
pg_dump -U loka_user loka_stratum > backup_pre_migration.sql

# Run migrations
cd migration && DATABASE_URL="postgresql://..." cargo run

# Verify migration
psql -U loka_user -d loka_stratum -c "\dt"

# Test application
curl -f http://localhost:9090/health
```

## Backup and Recovery

### Backup Procedures

#### Application Backup

```bash
#!/bin/bash
# backup-application.sh

BACKUP_DIR="/backup/loka-stratum"
DATE=$(date +%Y%m%d_%H%M%S)

# Create backup directory
mkdir -p "$BACKUP_DIR"

# Backup configuration
tar -czf "$BACKUP_DIR/config-$DATE.tar.gz" \
    loka-stratum.toml \
    .env \
    docker-compose.yml

# Backup application logs
tar -czf "$BACKUP_DIR/logs-$DATE.tar.gz" \
    /var/log/loka-stratum/

# Backup certificates
tar -czf "$BACKUP_DIR/certs-$DATE.tar.gz" \
    /etc/nginx/ssl/

echo "Application backup completed: $BACKUP_DIR"
```

#### Database Backup

```bash
#!/bin/bash
# backup-database.sh

BACKUP_DIR="/backup/loka-stratum/database"
DATE=$(date +%Y%m%d_%H%M%S)
DB_NAME="loka_stratum"
DB_USER="loka_user"

mkdir -p "$BACKUP_DIR"

# Full database backup
pg_dump -U "$DB_USER" -h localhost "$DB_NAME" | gzip > "$BACKUP_DIR/full-backup-$DATE.sql.gz"

# Schema-only backup
pg_dump -U "$DB_USER" -h localhost -s "$DB_NAME" > "$BACKUP_DIR/schema-backup-$DATE.sql"

# Data-only backup
pg_dump -U "$DB_USER" -h localhost -a "$DB_NAME" | gzip > "$BACKUP_DIR/data-backup-$DATE.sql.gz"

# Verify backup
gunzip -c "$BACKUP_DIR/full-backup-$DATE.sql.gz" | head -20

echo "Database backup completed: $BACKUP_DIR/full-backup-$DATE.sql.gz"
```

#### Container Volume Backup

```bash
#!/bin/bash
# backup-volumes.sh

BACKUP_DIR="/backup/loka-stratum/volumes"
DATE=$(date +%Y%m%d_%H%M%S)

mkdir -p "$BACKUP_DIR"

# Backup Docker volumes
docker run --rm \
    -v loka_stratum_postgres-data:/data \
    -v "$BACKUP_DIR":/backup \
    alpine tar czf "/backup/postgres-data-$DATE.tar.gz" /data

docker run --rm \
    -v loka_stratum_logs:/data \
    -v "$BACKUP_DIR":/backup \
    alpine tar czf "/backup/logs-data-$DATE.tar.gz" /data

echo "Volume backup completed: $BACKUP_DIR"
```

### Recovery Procedures

#### Application Recovery

```bash
#!/bin/bash
# restore-application.sh

BACKUP_FILE="$1"
if [ -z "$BACKUP_FILE" ]; then
    echo "Usage: $0 <backup-tar-gz>"
    exit 1
fi

# Stop services
docker-compose down

# Extract backup
tar -xzf "$BACKUP_FILE" -C /

# Restore configuration
# (files already extracted by tar)

# Start services
docker-compose up -d

# Verify restoration
sleep 10
curl -f http://localhost:9090/health

echo "Application restored from: $BACKUP_FILE"
```

#### Database Recovery

```bash
#!/bin/bash
# restore-database.sh

BACKUP_FILE="$1"
DB_NAME="loka_stratum"
DB_USER="loka_user"

if [ -z "$BACKUP_FILE" ]; then
    echo "Usage: $0 <backup-sql-gz>"
    exit 1
fi

# Stop application
docker-compose stop loka-stratum

# Drop and recreate database
sudo -u postgres psql -c "DROP DATABASE IF EXISTS $DB_NAME;"
sudo -u postgres psql -c "CREATE DATABASE $DB_NAME;"
sudo -u postgres psql -c "GRANT ALL PRIVILEGES ON DATABASE $DB_NAME TO $DB_USER;"

# Restore database
gunzip -c "$BACKUP_FILE" | psql -U "$DB_USER" -h localhost "$DB_NAME"

# Run migrations (if needed)
cd migration && DATABASE_URL="postgresql://$DB_USER:password@localhost:5432/$DB_NAME" cargo run

# Start application
docker-compose start loka-stratum

# Verify restoration
sleep 10
curl -f http://localhost:9090/health

echo "Database restored from: $BACKUP_FILE"
```

### Disaster Recovery

#### Complete System Recovery

```bash
#!/bin/bash
# disaster-recovery.sh

# 1. Restore from backups
echo "Starting disaster recovery..."

# 2. Deploy infrastructure
docker-compose -f docker-compose.production.yml up -d postgres redis

# 3. Wait for database
sleep 30

# 4. Restore database
./restore-database.sh /backup/loka-stratum/database/full-backup-latest.sql.gz

# 5. Restore application configuration
tar -xzf /backup/loka-stratum/config-latest.tar.gz

# 6. Deploy application
docker-compose -f docker-compose.production.yml up -d loka-stratum

# 7. Verify recovery
sleep 30
curl -f http://localhost:9090/health

# 8. Start monitoring
cd monitoring && docker-compose up -d

echo "Disaster recovery completed"
```

#### Recovery Testing

```bash
#!/bin/bash
# test-recovery.sh

# Test environment setup
export TEST_ENV="recovery-test"

# Deploy test environment
docker-compose -f docker-compose.test.yml up -d

# Wait for services
sleep 60

# Test application
curl -f http://localhost:13333/health

# Test mining functionality
python test_stratum_client.py --host localhost --port 13333

# Cleanup test environment
docker-compose -f docker-compose.test.yml down -v

echo "Recovery test completed successfully"
```

## Scaling Operations

### Horizontal Scaling

#### Docker Swarm Scaling

```bash
# Scale service replicas
docker service scale loka-production_loka-stratum=5

# Check scaling status
docker service ps loka-production_loka-stratum

# Monitor resource usage
docker stats
```

#### Kubernetes Scaling

```bash
# Manual scaling
kubectl scale deployment loka-stratum --replicas=10 -n loka-stratum

# Auto-scaling configuration
kubectl autoscale deployment loka-stratum --min=3 --max=20 --cpu-percent=70 -n loka-stratum

# Check auto-scaler status
kubectl get hpa -n loka-stratum
```

### Vertical Scaling

#### Resource Limits Adjustment

```yaml
# Docker Compose
services:
  loka-stratum:
    deploy:
      resources:
        limits:
          cpus: '4.0'
          memory: 4G
        reservations:
          cpus: '2.0'
          memory: 2G
```

```yaml
# Kubernetes
resources:
  requests:
    cpu: 2000m
    memory: 2Gi
  limits:
    cpu: 4000m
    memory: 4Gi
```

### Load Balancer Configuration

#### HAProxy Setup

```bash
# Update HAProxy configuration
sudo nano /etc/haproxy/haproxy.cfg

# Add new backend servers
server stratum4 10.0.1.13:3333 check
server stratum5 10.0.1.14:3333 check

# Reload configuration
sudo systemctl reload haproxy

# Verify configuration
sudo haproxy -c -f /etc/haproxy/haproxy.cfg
```

## Incident Response

### Incident Classifications

#### P0 - Critical (Complete Service Outage)
- All miners disconnected
- Service completely unavailable
- Database corruption
- Security breach

#### P1 - High (Major Functionality Impacted)
- High error rates (>10%)
- Performance degradation (>50% slower)
- Partial service unavailability
- Authentication issues

#### P2 - Medium (Minor Issues)
- Occasional errors (<5%)
- Minor performance issues
- Non-critical feature issues
- Monitoring issues

#### P3 - Low (Cosmetic Issues)
- Documentation issues
- Minor UI problems
- Non-critical logging issues

### Incident Response Procedures

#### P0 Incident Response

```bash
# 1. Immediate Assessment (2 minutes)
curl -f http://localhost:9090/health
docker-compose ps
systemctl status loka-stratum

# 2. Stabilization Actions (5 minutes)
# Check if quick restart resolves issue
docker-compose restart loka-stratum

# Check recent changes
git log --oneline -10
docker images | head -5

# 3. Communication (immediate)
# Notify stakeholders
# Update status page
# Create incident channel

# 4. Investigation
# Collect logs
docker-compose logs --since 1h loka-stratum > incident-logs.txt

# Check metrics
curl -s http://localhost:9090/metrics/prometheus > incident-metrics.txt

# Check system resources
top -bn1 > incident-system.txt
df -h >> incident-system.txt
```

#### Recovery Actions

```bash
# Emergency rollback
docker-compose down
docker tag loka-stratum:previous loka-stratum:latest
docker-compose up -d

# Database rollback (if needed)
./restore-database.sh /backup/emergency-backup.sql.gz

# Traffic routing (if load balanced)
# Remove problematic nodes from load balancer
curl -X POST http://load-balancer/api/nodes/disable/node-3
```

### Post-Incident Procedures

#### Incident Report Template

```markdown
# Incident Report: [Date] - [Brief Description]

## Summary
- **Incident ID**: INC-2024-001
- **Date/Time**: 2024-01-15 14:30 UTC
- **Duration**: 45 minutes
- **Severity**: P1
- **Impact**: 500 miners affected, 30% performance degradation

## Timeline
- 14:30 - Alert triggered: High error rate
- 14:32 - Investigation started
- 14:35 - Root cause identified: Database connection pool exhausted
- 14:40 - Mitigation applied: Increased connection pool size
- 15:15 - Full recovery confirmed

## Root Cause
Database connection pool configuration was insufficient for peak load.

## Resolution
- Increased max_connections from 50 to 200
- Added connection monitoring alerts
- Updated documentation

## Action Items
- [ ] Review connection pool sizing guidelines
- [ ] Implement proactive monitoring
- [ ] Load test connection pool limits
- [ ] Update runbooks

## Lessons Learned
- Need better capacity planning
- Connection pool monitoring is crucial
- Faster escalation procedures needed
```

## Operational Runbooks

### Service Restart Runbook

```bash
#!/bin/bash
# service-restart-runbook.sh

echo "=== Loka Stratum Service Restart Runbook ==="

# 1. Pre-restart checks
echo "1. Checking current status..."
curl -s http://localhost:9090/health | jq .status

# 2. Graceful shutdown
echo "2. Initiating graceful shutdown..."
docker-compose kill -s SIGTERM loka-stratum
sleep 10

# 3. Force stop if needed
if docker-compose ps loka-stratum | grep -q "Up"; then
    echo "3. Force stopping service..."
    docker-compose stop loka-stratum
fi

# 4. Start service
echo "4. Starting service..."
docker-compose up -d loka-stratum

# 5. Verify startup
echo "5. Verifying startup..."
sleep 30
curl -f http://localhost:9090/health || echo "WARNING: Health check failed"

# 6. Monitor for 5 minutes
echo "6. Monitoring for 5 minutes..."
for i in {1..5}; do
    sleep 60
    echo "Minute $i: $(curl -s http://localhost:9090/health | jq .status)"
done

echo "Service restart completed"
```

### Emergency Response Runbook

```bash
#!/bin/bash
# emergency-response-runbook.sh

echo "=== EMERGENCY RESPONSE RUNBOOK ==="

# 1. Immediate triage
echo "1. TRIAGE - Checking service status"
if ! curl -f http://localhost:9090/health &>/dev/null; then
    echo "CRITICAL: Service is DOWN"
    SEVERITY="P0"
else
    echo "Service is UP - checking performance"
    SEVERITY="P1"
fi

# 2. Quick diagnostics
echo "2. DIAGNOSTICS"
docker-compose ps
docker stats --no-stream
df -h | grep -E "(/$|/var|/opt)"

# 3. Immediate actions based on severity
if [ "$SEVERITY" = "P0" ]; then
    echo "3. P0 ACTIONS - Attempting immediate recovery"
    
    # Try quick restart
    docker-compose restart loka-stratum
    sleep 30
    
    if curl -f http://localhost:9090/health &>/dev/null; then
        echo "Quick restart successful"
    else
        echo "Quick restart failed - attempting rollback"
        ./emergency-rollback.sh
    fi
    
elif [ "$SEVERITY" = "P1" ]; then
    echo "3. P1 ACTIONS - Performance investigation"
    
    # Check resource usage
    echo "CPU Usage: $(top -bn1 | grep "Cpu(s)" | awk '{print $2}')"
    echo "Memory Usage: $(free -h | grep Mem | awk '{print $3}')"
    echo "Active Connections: $(curl -s http://localhost:9090/health | jq .connections.active)"
fi

# 4. Log collection
echo "4. COLLECTING LOGS"
mkdir -p /tmp/emergency-$(date +%Y%m%d_%H%M%S)
docker-compose logs --since 1h > /tmp/emergency-$(date +%Y%m%d_%H%M%S)/docker-logs.txt
journalctl -u loka-stratum --since "1 hour ago" > /tmp/emergency-$(date +%Y%m%d_%H%M%S)/systemd-logs.txt

echo "Emergency response completed - logs saved to /tmp/emergency-*"
```

For additional operational support, refer to:
- [Deployment Guide](DEPLOYMENT.md) for infrastructure setup
- [Monitoring Guide](MONITORING.md) for detailed monitoring procedures
- [Security Guide](SECURITY.md) for security incident response
- [API Documentation](API.md) for endpoint specifications