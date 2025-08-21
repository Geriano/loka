# Loka Stratum - Docker Deployment Guide

This guide provides comprehensive instructions for deploying Loka Stratum with Docker and Docker Compose, including full monitoring stack integration.

## 🚀 Quick Start

1. **Copy environment configuration:**
   ```bash
   cp .env.example .env
   # Edit .env with your specific configuration
   ```

2. **Start the complete stack:**
   ```bash
   docker-compose up -d
   ```

3. **Access services:**
   - **Loka Stratum Metrics**: http://localhost:9090/metrics
   - **Prometheus**: http://localhost:9091
   - **Grafana**: http://localhost:3000 (admin/admin123)
   - **AlertManager**: http://localhost:9093

## 📦 Services Overview

### Core Services
- **loka-stratum** (port 3333): Bitcoin Stratum V1 mining proxy
- **loka-stratum-metrics** (port 9090): HTTP metrics endpoint

### Monitoring Stack
- **prometheus** (port 9091): Metrics collection and alerting
- **grafana** (port 3000): Visualization and dashboards
- **alertmanager** (port 9093): Alert routing and notification
- **grafana-renderer** (port 8081): PDF report generation

### System Monitoring
- **node-exporter** (port 9100): Host system metrics
- **cadvisor** (port 8080): Container metrics and resource usage

## 🔧 Configuration

### Environment Variables

Create `.env` file from `.env.example`:

```bash
# Mining Configuration
POOL_ADDRESS=stratum.example.com:4444
RUST_LOG=info

# Grafana Access
GRAFANA_PASSWORD=your-secure-password

# Alert Notifications (optional)
SLACK_WEBHOOK_URL=https://hooks.slack.com/services/YOUR/WEBHOOK
SMTP_HOST=smtp.your-domain.com
SMTP_PORT=587
SMTP_USERNAME=alerts@your-domain.com
SMTP_PASSWORD=your-email-password
```

### Custom Configuration Files

#### Prometheus Configuration
- **Location**: `monitoring/prometheus/prometheus.yml`
- **Alert Rules**: `monitoring/prometheus/rules/loka-stratum.yml`
- **Scrape Interval**: 15s (configurable)
- **Retention**: 30 days / 10GB

#### Grafana Dashboards
- **Main Dashboard**: Loka Stratum - Mining Overview
- **Performance**: Performance Analysis (detailed metrics)
- **Infrastructure**: System Infrastructure Overview
- **Data Sources**: Auto-provisioned Prometheus + AlertManager

#### AlertManager Configuration
- **Location**: `monitoring/alertmanager/alertmanager.yml`
- **Routing**: Severity-based (critical/warning/info)
- **Channels**: Email, Slack, Webhook
- **Grouping**: By alertname, cluster, service

## 📊 Monitoring Features

### Loka Stratum Metrics
- **Connection Metrics**: Active connections, connection rate, success rate
- **Mining Performance**: Shares/second, acceptance rate, hashrate estimation
- **Message Processing**: Throughput, latency, error rates
- **Resource Usage**: Memory, CPU, cache efficiency
- **Protocol Performance**: JSON parsing, string operations, hash calculations

### System Metrics (Node Exporter)
- **CPU**: Usage percentages, load averages
- **Memory**: Total, used, available, swap
- **Disk**: Usage, I/O rates, filesystem stats
- **Network**: Interface statistics, bandwidth usage

### Container Metrics (cAdvisor)
- **Resource Usage**: Per-container CPU and memory
- **Performance**: I/O statistics, network metrics
- **Health**: Container status and restart counts

## 🚨 Alerting

### Pre-configured Alerts

**Critical Alerts** (immediate notification):
- `LokaStratumDown`: Service unavailable
- `HighErrorRate`: Error rate > 10/second
- `LowShareAcceptanceRate`: Acceptance rate < 90%

**Warning Alerts** (30-minute intervals):
- `HighMemoryUsage`: Memory usage > 80%
- `HighCPUUsage`: CPU usage > 80%
- `SlowMessageProcessing`: Processing time > 1ms
- `NoActiveConnections`: No miners connected

**Info Alerts** (daily digest):
- `LowHashrate`: Network hashrate < 1 GH/s

### Notification Channels

1. **Email**: Detailed alert information with context
2. **Slack**: Real-time notifications with severity indicators
3. **Webhook**: Custom integrations for external systems

## 🛠️ Operations

### Start Services
```bash
# Start all services
docker-compose up -d

# Start specific services
docker-compose up -d loka-stratum prometheus grafana

# View logs
docker-compose logs -f loka-stratum
```

### Health Checks
```bash
# Check service health
docker-compose ps

# Test Loka Stratum health
curl http://localhost:9090/health

# Test Prometheus
curl http://localhost:9091/-/healthy

# Test Grafana
curl http://localhost:3000/api/health
```

### Scaling and Updates
```bash
# Update services
docker-compose pull
docker-compose up -d

# Scale Loka Stratum (if using swarm)
docker service scale loka-stratum_loka-stratum=3

# Restart specific service
docker-compose restart loka-stratum
```

### Backup and Recovery
```bash
# Backup Grafana dashboards and data
docker cp loka-grafana:/var/lib/grafana ./backup/grafana-$(date +%Y%m%d)

# Backup Prometheus data
docker cp loka-prometheus:/prometheus ./backup/prometheus-$(date +%Y%m%d)

# Backup configuration
tar -czf config-backup-$(date +%Y%m%d).tar.gz monitoring/
```

## 🔍 Troubleshooting

### Common Issues

**1. Loka Stratum fails to start**
```bash
# Check logs
docker-compose logs loka-stratum

# Verify configuration
docker-compose config

# Test without dependencies
docker run --rm loka-stratum:latest loka-stratum --help
```

**2. Prometheus can't scrape metrics**
```bash
# Test metrics endpoint
curl http://localhost:9090/metrics/prometheus

# Check Prometheus config
docker exec loka-prometheus promtool check config /etc/prometheus/prometheus.yml

# Verify network connectivity
docker exec loka-prometheus wget -qO- http://loka-stratum:9090/health
```

**3. Grafana dashboards not loading**
```bash
# Check Grafana logs
docker-compose logs grafana

# Verify data source
curl -u admin:admin123 http://localhost:3000/api/datasources

# Test Prometheus connectivity
docker exec loka-grafana wget -qO- http://prometheus:9090/api/v1/label/__name__/values
```

**4. Alerts not firing**
```bash
# Check AlertManager status
curl http://localhost:9093/api/v1/status

# Verify alert rules
docker exec loka-prometheus promtool check rules /etc/prometheus/rules/loka-stratum.yml

# Test alert routing
curl -XPOST http://localhost:9093/api/v1/alerts -d '[{"labels":{"alertname":"test"}}]'
```

### Performance Optimization

**1. Reduce resource usage**
```yaml
# In docker-compose.yml, add resource limits
services:
  loka-stratum:
    deploy:
      resources:
        limits:
          memory: 512M
          cpus: '1.0'
        reservations:
          memory: 256M
          cpus: '0.5'
```

**2. Optimize Prometheus retention**
```yaml
# Reduce retention for development
command:
  - '--storage.tsdb.retention.time=7d'
  - '--storage.tsdb.retention.size=1GB'
```

**3. Tune scrape intervals**
```yaml
# In prometheus.yml
global:
  scrape_interval: 30s  # Increase for lower load
  evaluation_interval: 30s
```

## 🔐 Security Considerations

### Production Deployment

1. **Change default passwords**:
   ```bash
   # Generate secure passwords
   openssl rand -base64 32
   ```

2. **Use secrets management**:
   ```yaml
   # docker-compose.yml
   secrets:
     grafana_password:
       file: ./secrets/grafana_password.txt
   ```

3. **Enable TLS**:
   ```bash
   # Generate certificates
   openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
     -keyout tls.key -out tls.crt
   ```

4. **Network isolation**:
   ```yaml
   # Restrict external access
   networks:
     internal:
       internal: true
     external:
       driver: bridge
   ```

5. **Regular updates**:
   ```bash
   # Update images regularly
   docker-compose pull && docker-compose up -d
   ```

## 📈 Performance Benchmarks

After deployment, run performance benchmarks:

```bash
# Build with benchmarks
docker exec loka-stratum cargo bench

# Run critical path benchmarks
docker exec loka-stratum ./run_benchmarks.sh

# Monitor performance in Grafana
# Navigate to "Performance Analysis" dashboard
```

Expected Performance (optimized deployment):
- **Message Processing**: < 1μs average latency
- **Connection Handling**: 1000+ concurrent connections
- **Memory Usage**: < 512MB under normal load
- **CPU Usage**: < 50% under full load
- **Network Throughput**: 10K+ messages/second

## 📞 Support

For issues and questions:
1. Check logs: `docker-compose logs -f`
2. Verify configuration: `docker-compose config`
3. Test connectivity: Use provided curl commands
4. Monitor dashboards: Check Grafana for system health
5. Review alerts: Check AlertManager for active issues

## 🚀 Next Steps

After successful deployment:

1. **Customize Dashboards**: Modify Grafana dashboards for your needs
2. **Configure Alerts**: Set up notification channels (Slack, email)
3. **Optimize Performance**: Use benchmark results to tune configuration
4. **Scale Horizontally**: Deploy multiple instances with load balancing
5. **Monitoring**: Set up long-term metrics storage and analysis

---

**Production Ready**: This Docker deployment provides enterprise-grade monitoring, alerting, and observability for Bitcoin mining operations.