# Loka Stratum Grafana Dashboards

This directory contains comprehensive Grafana dashboards for monitoring the Loka Stratum Bitcoin mining proxy.

## Available Dashboards

### 1. Loka Stratum Comprehensive Metrics Dashboard (`loka-stratum-comprehensive.json`)

A comprehensive dashboard that visualizes all Loka Stratum metrics across 6 main categories:

#### Dashboard Sections:

**Connection Monitoring - Overview & Health**
- **Active Connections Gauge**: Real-time active connection count with capacity thresholds
- **Connection Pool Utilization Gauge**: Percentage utilization of maximum connection capacity  
- **Connection Duration & Idle Time**: Trends with alert threshold indicators
- **Connection Lifecycle Events**: Connection establishments, closures, and reconnection attempts

**Protocol Operations - Detection & Conversion**
- **Protocol Request Distribution**: Pie chart showing HTTP vs Stratum vs HTTP CONNECT vs Direct Stratum ratios
- **Protocol Operation Success Rates**: Gauge showing protocol detection and conversion success percentages
- **Protocol Response Time Histogram**: P50, P95, P99 response time percentiles

**Mining Performance - Shares, Jobs & Hashrate**
- **Mining Performance Overview**: Shares per minute, difficulty, and adjustment rates
- **Share Acceptance Rate Gauge**: Real-time acceptance percentage with quality thresholds
- **Estimated Hashrate**: Calculated hashrate based on share submissions and difficulty
- **Job Distribution Latency**: Min, average, and maximum job distribution times
- **Share Quality Metrics**: Stale shares, duplicate shares, accepted/rejected breakdowns

**Error Monitoring & Alerting**  
- **Error Rates by Category**: Stacked view of network, timeout, auth, parse, and validation errors
- **Critical Error Categories**: Security violations, resource exhaustion, internal errors, protocol version errors

**Resource Utilization & Performance**
- **Memory Utilization**: Per-connection memory usage trends and totals
- **CPU & Memory Efficiency**: CPU utilization and memory efficiency percentages
- **Network Bandwidth**: RX/TX bandwidth utilization
- **Resource Pressure Events**: System stress indicators

#### Features:
- **Refresh Rate**: 5-second auto-refresh for real-time monitoring
- **Time Range Variables**: Configurable rate calculation intervals (1m, 5m, 15m, 30m)
- **Data Source Variables**: Easy switching between Prometheus instances
- **Comprehensive Legends**: Min/max/mean/last value calculations in table format
- **Multi-tooltip Support**: Detailed hover information across all metrics
- **Threshold Alerts**: Visual threshold indicators on critical metrics

### 2. Legacy Dashboard (`loka-stratum-overview.json`)

The original basic dashboard with essential metrics - kept for backward compatibility.

## Dashboard Import Instructions

### Using Docker Compose (Recommended)

1. **Start the monitoring stack**:
   ```bash
   cd monitoring
   docker-compose up -d
   ```

2. **Access Grafana**:
   - URL: `http://localhost:3000`
   - Username: `admin`
   - Password: `admin123`

3. **Dashboards are automatically provisioned** from this directory.

### Manual Import

1. **Copy dashboard JSON**:
   ```bash
   cat loka-stratum-comprehensive.json
   ```

2. **In Grafana UI**:
   - Go to "+" → Import
   - Paste JSON content
   - Set data source to your Prometheus instance
   - Click Import

## Data Sources Required

### Prometheus
- **URL**: `http://prometheus:9090` (Docker) or `http://localhost:9090` (local)
- **Required for**: All metric queries and alerting rules

### AlertManager (Optional)
- **URL**: `http://alertmanager:9093` (Docker) or `http://localhost:9093` (local)  
- **Required for**: Alert notifications and management

### Loki (Optional)
- **URL**: `http://loki:3100` (Docker) or `http://localhost:3100` (local)
- **Required for**: Log correlation and trace analysis

## Alert Rules Integration

The dashboard integrates with Prometheus alerting rules defined in:
`../prometheus/rules/loka-stratum.yml`

### Alert Categories:
- **Service Health**: Service availability and uptime
- **Connection Health**: Pool utilization, reconnection rates
- **Protocol Performance**: Detection and conversion success rates
- **Mining Performance**: Share acceptance, job latency, hashrate
- **Resource Utilization**: CPU, memory, network usage
- **Error Monitoring**: All error categories with appropriate thresholds
- **Security**: Authentication failures, security violations

### Alert Severities:
- **Critical**: Immediate attention required (pager/SMS)
- **Warning**: Investigation needed (email/Slack)
- **Info**: Informational only (dashboard/logs)

## Customization

### Adding New Panels

1. **Edit the JSON file**:
   - Add new panel object to `panels` array
   - Set appropriate `gridPos` for layout
   - Configure data source and queries

2. **Panel Configuration**:
   ```json
   {
     "id": <unique_id>,
     "title": "Panel Title",
     "type": "timeseries|gauge|stat|piechart",
     "gridPos": {"h": 8, "w": 12, "x": 0, "y": 0},
     "targets": [
       {
         "expr": "prometheus_query",
         "legendFormat": "Legend Name"
       }
     ]
   }
   ```

### Modifying Thresholds

Edit threshold values in panel `fieldConfig.defaults.thresholds.steps`:

```json
"thresholds": {
  "steps": [
    {"color": "green", "value": null},
    {"color": "yellow", "value": 75},
    {"color": "red", "value": 90}
  ]
}
```

### Variables

The dashboard supports these variables:
- `$datasource`: Prometheus data source selection
- `$rate_interval`: Rate calculation time window (1m, 5m, 15m, 30m)

Add variables in the `templating.list` section for dynamic filtering.

## Performance Considerations

### Dashboard Optimization:
- **Query Efficiency**: Uses recording rules from Prometheus for complex calculations
- **Refresh Rates**: 5s refresh rate - adjust in dashboard settings if needed
- **Time Range**: Default 1-hour window - suitable for real-time monitoring
- **Panel Limits**: Reasonable panel count to prevent browser performance issues

### Metric Efficiency:
- Utilizes atomic operations for sub-microsecond metric collection
- Leverages loka-metrics crate's lock-free design
- Recording rules pre-calculate complex expressions

## Troubleshooting

### Dashboard Not Loading:
1. Verify Prometheus data source connectivity
2. Check metric names match actual Stratum exports
3. Validate JSON syntax if manually imported

### Missing Data:
1. Confirm Loka Stratum is running and exposing metrics on `/metrics/prometheus`
2. Check Prometheus is successfully scraping the target
3. Verify metric names in queries match implementation

### Alert Issues:
1. Check Prometheus rules are loaded: `http://prometheus:9090/rules`  
2. Verify AlertManager is receiving notifications
3. Review alert rule expressions for accuracy

## Related Documentation

- **Stratum Metrics Implementation**: `../../stratum/src/services/metrics.rs`
- **Prometheus Configuration**: `../prometheus/prometheus.yml`
- **Alert Rules**: `../prometheus/rules/loka-stratum.yml`
- **Docker Compose Setup**: `../docker-compose.yml`
- **Main Project README**: `../../README.md`

## Support

For dashboard issues or feature requests, see the main project documentation and issue tracker.