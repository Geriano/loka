#!/bin/bash

# Test Error Tracking System
echo "🧪 Testing Error Tracking System..."

# Test Pushgateway (error-tracker)
echo "📊 Testing Pushgateway..."
if curl -s http://localhost:9091/-/healthy > /dev/null; then
    echo "✅ Pushgateway is healthy"
else
    echo "❌ Pushgateway not responding"
    exit 1
fi

# Push a test error metric
echo "📤 Pushing test error metrics..."
cat <<EOF | curl --data-binary @- http://localhost:9091/metrics/job/loka-stratum-test/instance/test-instance
# TYPE loka_stratum_test_errors_total counter
# HELP loka_stratum_test_errors_total Total number of test errors
loka_stratum_test_errors_total{error_type="connection_failure",service="loka-stratum"} 5

# TYPE loka_stratum_test_response_time histogram
# HELP loka_stratum_test_response_time Response time in seconds
loka_stratum_test_response_time_bucket{le="0.1"} 10
loka_stratum_test_response_time_bucket{le="0.5"} 15
loka_stratum_test_response_time_bucket{le="1.0"} 20
loka_stratum_test_response_time_bucket{le="2.0"} 25
loka_stratum_test_response_time_bucket{le="+Inf"} 30
loka_stratum_test_response_time_sum 25.5
loka_stratum_test_response_time_count 30

# TYPE loka_stratum_test_active_connections gauge
# HELP loka_stratum_test_active_connections Number of active connections
loka_stratum_test_active_connections{service="loka-stratum"} 42
EOF

if [ $? -eq 0 ]; then
    echo "✅ Test metrics pushed successfully"
else
    echo "❌ Failed to push test metrics"
    exit 1
fi

# Check if Prometheus can scrape the metrics
echo "🎯 Testing Prometheus scraping..."
sleep 5
if curl -s http://localhost:$(docker-compose port prometheus 9090 | cut -d: -f2)/api/v1/query?query=loka_stratum_test_errors_total | grep -q "loka_stratum_test_errors_total"; then
    echo "✅ Prometheus successfully scraped test metrics"
else
    echo "⚠️  Prometheus might not be scraping pushgateway yet (check configuration)"
fi

# Display pushgateway metrics
echo "📋 Current metrics in pushgateway:"
curl -s http://localhost:9091/metrics | grep "loka_stratum_test" | head -10

echo ""
echo "🎉 Error tracking system test completed!"
echo ""
echo "📊 Access points:"
echo "   - Pushgateway: http://localhost:9091"
echo "   - Prometheus: http://localhost:$(docker-compose port prometheus 9090 | cut -d: -f2)"
echo "   - Grafana: http://localhost:3000 (admin/admin123)"
echo ""
echo "🔧 To push metrics from Loka Stratum, use:"
echo "   curl --data-binary @metrics.txt http://localhost:9091/metrics/job/loka-stratum/instance/\$HOSTNAME"