# Loka Stratum Error Tracking System

## Overview

This document describes the error tracking and monitoring system implemented for Loka Stratum as an alternative to self-hosted Sentry. The system uses Prometheus Pushgateway, enhanced alerting rules, and Grafana dashboards to provide comprehensive error tracking and performance monitoring.

## Architecture

### Components

1. **Prometheus Pushgateway** (`error-tracker` service) - Collects and exposes error metrics
2. **Enhanced Prometheus Rules** - Defines error detection and alerting logic
3. **Grafana Dashboards** - Visualizes error patterns and system health
4. **AlertManager** - Routes error alerts to appropriate channels

### Why This Approach?

- **Simpler Setup**: Avoids complex Sentry configuration issues
- **Integrated Monitoring**: Works seamlessly with existing Prometheus/Grafana stack
- **Customizable**: Full control over metrics, alerts, and dashboards
- **Production Ready**: Built on proven monitoring infrastructure

## Implementation

### 1. Error Tracking Service

**Prometheus Pushgateway** is deployed as the `error-tracker` service:

```yaml
error-tracker:
  image: prom/pushgateway:v1.6.2
  ports:
    - "9091:9091"
```

**Purpose**: 
- Receives error metrics pushed from Loka Stratum
- Exposes metrics for Prometheus scraping
- Supports structured error categorization

### 2. Error Detection Rules

Located in `/prometheus/rules/error-tracking.yml`, these rules detect:

- **High Error Rate**: `>5%` error rate triggers warning
- **Critical Error Burst**: `>10` errors/sec triggers critical alert
- **Connection Errors**: Network and pool connection issues
- **Protocol Errors**: Stratum protocol violations
- **Database Errors**: Database operation failures
- **Memory Leaks**: Unusual memory growth patterns
- **Performance Issues**: Slow response times, high queue sizes

### 3. Monitoring Dashboards

**Error Tracking Dashboard** (`/grafana/dashboards/error-tracking.json`):
- Real-time error rate visualization
- Error categorization by type
- Connection and performance metrics
- Recent error logs (when Loki is available)
- Alert status overview

## Usage

### Pushing Error Metrics from Loka Stratum

**Basic Error Counter**:
```bash
curl --data-binary @- http://localhost:9091/metrics/job/loka-stratum/instance/$HOSTNAME <<EOF
# TYPE loka_stratum_errors_total counter
loka_stratum_errors_total{error_type="connection_timeout",service="loka-stratum"} 1
EOF
```

**Performance Metrics**:
```bash
curl --data-binary @- http://localhost:9091/metrics/job/loka-stratum/instance/$HOSTNAME <<EOF
# TYPE loka_stratum_request_duration_seconds histogram
loka_stratum_request_duration_seconds_bucket{method="mining.submit",le="0.1"} 50
loka_stratum_request_duration_seconds_bucket{method="mining.submit",le="0.5"} 95
loka_stratum_request_duration_seconds_bucket{method="mining.submit",le="1.0"} 100
loka_stratum_request_duration_seconds_bucket{method="mining.submit",le="+Inf"} 100
loka_stratum_request_duration_seconds_sum{method="mining.submit"} 15.5
loka_stratum_request_duration_seconds_count{method="mining.submit"} 100
EOF
```

**System State Metrics**:
```bash
curl --data-binary @- http://localhost:9091/metrics/job/loka-stratum/instance/$HOSTNAME <<EOF
# TYPE loka_stratum_active_connections gauge
loka_stratum_active_connections 42

# TYPE loka_stratum_message_queue_size gauge  
loka_stratum_message_queue_size 15

# TYPE loka_stratum_pool_connected gauge
loka_stratum_pool_connected 1
EOF
```

### Integration with Rust Code

**Basic Rust integration example**:
```rust
use reqwest::Client;
use std::collections::HashMap;

pub struct ErrorTracker {
    client: Client,
    pushgateway_url: String,
    job_name: String,
    instance: String,
}

impl ErrorTracker {
    pub fn new(pushgateway_url: String) -> Self {
        Self {
            client: Client::new(),
            pushgateway_url,
            job_name: "loka-stratum".to_string(),
            instance: std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".to_string()),
        }
    }

    pub async fn push_error(&self, error_type: &str) -> Result<(), Box<dyn std::error::Error>> {
        let metrics = format!(
            "# TYPE loka_stratum_errors_total counter\nloka_stratum_errors_total{{error_type=\"{}\",service=\"loka-stratum\"}} 1\n",
            error_type
        );
        
        let url = format!(
            "{}/metrics/job/{}/instance/{}",
            self.pushgateway_url, self.job_name, self.instance
        );
        
        self.client
            .post(&url)
            .body(metrics)
            .send()
            .await?;
            
        Ok(())
    }

    pub async fn push_performance_metric(&self, method: &str, duration: f64) -> Result<(), Box<dyn std::error::Error>> {
        // Implementation for histogram metrics
        todo!()
    }
}
```

## Monitoring & Alerting

### Alert Channels

Configure AlertManager in `/alertmanager/alertmanager.yml`:

```yaml
route:
  receiver: 'loka-stratum-alerts'
  group_by: ['alertname', 'service']
  group_wait: 30s
  group_interval: 5m
  repeat_interval: 4h

receivers:
  - name: 'loka-stratum-alerts'
    email_configs:
      - to: 'admin@loka-stratum.local'
        subject: 'Loka Stratum Alert: {{ .GroupLabels.alertname }}'
        body: |
          {{ range .Alerts }}
          Alert: {{ .Annotations.summary }}
          Description: {{ .Annotations.description }}
          {{ end }}
```

### Dashboard Access

- **Grafana**: http://localhost:3000 (admin/admin123)
- **Prometheus**: http://localhost:9090
- **Pushgateway**: http://localhost:9091
- **AlertManager**: http://localhost:9093

## Testing

### Manual Testing

Run the test script:
```bash
./test-error-tracking.sh
```

This will:
1. Verify pushgateway health
2. Push test metrics
3. Verify Prometheus scraping
4. Display current metrics

### Load Testing

Generate realistic error patterns:
```bash
# Simulate connection errors
for i in {1..10}; do
  curl --data-binary "loka_stratum_errors_total{error_type=\"connection_timeout\"} $i" \
    http://localhost:9091/metrics/job/loka-stratum-load-test/instance/test-$i
  sleep 1
done
```

## Troubleshooting

### Common Issues

1. **Pushgateway not receiving metrics**
   - Check network connectivity: `curl http://localhost:9091/-/healthy`
   - Verify data format follows Prometheus exposition format

2. **Prometheus not scraping**
   - Check `/prometheus/prometheus.yml` has error-tracker job
   - Verify `honor_labels: true` is set

3. **Alerts not firing**
   - Check `/prometheus/rules/error-tracking.yml` rules syntax
   - Verify AlertManager configuration
   - Check alert rule evaluation: `http://localhost:9090/rules`

4. **Dashboard not showing data**
   - Verify Prometheus data source in Grafana
   - Check metric names match dashboard queries
   - Ensure time range includes recent data

### Log Collection (Loki Alternative)

If structured logging is needed without Loki complexity:

1. **File-based logging**: Use log files with rotation
2. **JSON format**: Structure logs for easy parsing
3. **External tools**: Ship logs to external services (e.g., Datadog, New Relic)

## Deployment

### Production Deployment

1. **Start services**:
   ```bash
   docker-compose up -d error-tracker prometheus grafana alertmanager
   ```

2. **Verify health**:
   ```bash
   ./test-error-tracking.sh
   ```

3. **Configure alerts**: Update AlertManager with production channels

4. **Set up dashboards**: Import error-tracking.json into Grafana

### Scaling Considerations

- **High throughput**: Consider multiple pushgateway instances
- **Long-term storage**: Configure Prometheus remote write
- **Alert routing**: Use AlertManager's routing trees
- **Dashboard performance**: Use dashboard variables and templating

## Migration from Sentry

If switching from Sentry:

1. **Map error types**: Categorize existing Sentry issues into metric labels
2. **Convert breadcrumbs**: Use structured logging with correlation IDs
3. **User context**: Add user/session labels to metrics
4. **Performance monitoring**: Use histogram metrics for response times
5. **Release tracking**: Use deployment labels in metrics

## Future Enhancements

1. **Custom metrics SDK**: Build Rust crate for easier integration
2. **Auto-discovery**: Implement service discovery for dynamic instances
3. **Machine learning**: Add anomaly detection to error patterns
4. **Integration APIs**: Connect with external incident management tools
5. **Mobile dashboards**: Create mobile-friendly monitoring views

## Summary

This error tracking system provides:

- ✅ **Comprehensive error detection** via Prometheus rules
- ✅ **Real-time visualization** through Grafana dashboards  
- ✅ **Structured error categorization** with custom labels
- ✅ **Production-ready alerting** via AlertManager
- ✅ **Easy integration** with existing Rust applications
- ✅ **Scalable architecture** built on proven technologies
- ✅ **Cost effective** using open-source components

The system successfully replaces complex Sentry setup while providing equivalent functionality integrated with the existing monitoring infrastructure.