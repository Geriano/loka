use anyhow::Result;
use axum::{Router, http::StatusCode, response::Json, routing::get};
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tracing::{error, info, warn};

use crate::cli::Args;
use crate::config::Config;
use crate::services::{
    database::DatabaseService,
    metrics::{MetricsService, MetricsSnapshot, UserMetricsSnapshot},
};

// Global metrics service for HTTP endpoints
static STRATUM_METRICS: std::sync::OnceLock<Arc<MetricsService>> = std::sync::OnceLock::new();

pub async fn execute(args: Args) -> Result<()> {
    setup_logging(&args)?;

    // Initialize Sentry early for error capture during startup
    let _sentry_guard = crate::sentry::init_sentry(None)?;

    match args.command {
        crate::cli::Commands::Start {
            bind,
            database_url: database,
            max_connections,
            idle_timeout,
            daemon,
        } => {
            start_server(
                bind,
                database,
                max_connections,
                idle_timeout,
                daemon,
                args.metrics,
                args.metrics_addr,
            )
            .await
        }
        crate::cli::Commands::Status {
            endpoint,
            format,
            watch,
        } => show_status(endpoint, format, watch).await,
        crate::cli::Commands::Config { file, show } => validate_config(file, show).await,
        crate::cli::Commands::Bench {
            target,
            connections,
            duration,
            format,
        } => run_benchmark(target, connections, duration, format).await,
        crate::cli::Commands::Database { command, url } => {
            handle_database_command(command, url).await
        }
    }
}

async fn start_server(
    bind: u16,
    database: String,
    max_connections: usize,
    idle_timeout: u64,
    daemon: bool,
    enable_metrics: bool,
    metrics_addr: String,
) -> Result<()> {
    info!("Starting Loka Stratum server");

    // Initialize database service first (needed for pool configuration)
    let database_service = DatabaseService::new(&database)
        .await
        .expect("Failed to initialize database service");

    // Load configuration with database-driven pool config if available
    let mut config = Config::load(&database_service).await?;

    config.server.bind_address = SocketAddr::from((Ipv4Addr::UNSPECIFIED, bind));
    config.server.max_connections = max_connections;
    config.server.idle_timeout = Duration::from_secs(idle_timeout);

    // Validate configuration using the validation module method
    config.validate()?;
    info!("Configuration validated successfully");

    // Initialize Sentry with configuration if available
    if let Some(ref sentry_config) = config.sentry {
        let _sentry_guard = crate::sentry::init_sentry(Some(sentry_config))?;
        info!("✅ Sentry integration configured from config file");
    }

    // Initialize metrics if enabled
    if enable_metrics {
        // Initialize the global metrics recorder
        match loka_metrics::Recorder::init() {
            Ok(_) => {
                info!("✅ Metrics recorder initialized successfully");
            }
            Err(e) => {
                warn!("⚠️ Metrics recorder already initialized: {}", e);
            }
        }

        // Initialize Stratum metrics service
        let stratum_metrics = Arc::new(MetricsService::default());
        STRATUM_METRICS.set(stratum_metrics.clone()).ok();
        info!("✅ Stratum metrics service initialized");

        info!("Starting metrics server on {}", metrics_addr);
        let metrics_server = start_metrics_server(metrics_addr).await?;
        tokio::spawn(metrics_server);
    }

    // Create and start the actual server
    let listener = crate::Listener::new(config).await?;

    // Start server in background task
    let server_handle = tokio::spawn(async move {
        if let Err(e) = listener.accept().await {
            error!("Server error: {}", e);
        }
    });

    info!("Loka Stratum server started successfully");

    if daemon {
        info!("Running in daemon mode");
        server_handle.await?;
    } else {
        // Wait for shutdown signal
        tokio::select! {
            _ = signal::ctrl_c() => {
                info!("Received shutdown signal");
            }
            result = server_handle => {
                if let Err(e) = result {
                    error!("Server task failed: {}", e);
                }
            }
        }
    }

    info!("Shutting down server...");
    // TODO: Implement actual shutdown logic
    info!("Server shutdown completed");

    Ok(())
}

async fn show_status(endpoint: String, format: String, watch: Option<u64>) -> Result<()> {
    if let Some(interval) = watch {
        info!(
            "Watching status every {} seconds (press Ctrl+C to stop)",
            interval
        );
        let mut interval = tokio::time::interval(Duration::from_secs(interval));

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = fetch_and_display_status(&endpoint, &format).await {
                        error!("Failed to fetch status: {}", e);
                    }
                    println!("\n---\n");
                }
                _ = signal::ctrl_c() => {
                    break;
                }
            }
        }
    } else {
        fetch_and_display_status(&endpoint, &format).await?;
    }

    Ok(())
}

async fn fetch_and_display_status(endpoint: &str, format: &str) -> Result<()> {
    // TODO: Implement metrics fetching from HTTP endpoint
    let status = format!(
        "Loka Stratum Server Status\n\
         Endpoint: {}\n\
         Format: {}\n\
         Status: Running\n\
         Active Connections: N/A\n\
         Messages Processed: N/A\n\
         Uptime: N/A",
        endpoint, format
    );

    match format {
        "json" => {
            println!(
                "{{\
                \"endpoint\": \"{}\",\
                \"status\": \"running\",\
                \"active_connections\": null,\
                \"messages_processed\": null,\
                \"uptime\": null\
            }}",
                endpoint
            );
        }
        "yaml" => {
            println!(
                "endpoint: {}\nstatus: running\nactive_connections: null\nmessages_processed: null\nuptime: null",
                endpoint
            );
        }
        _ => {
            println!("{}", status);
        }
    }

    Ok(())
}

async fn validate_config(file: std::path::PathBuf, show: bool) -> Result<()> {
    info!("Validating configuration file: {}", file.display());

    let config = Config::load_from_file(file)?;
    config.validate()?;

    info!("✅ Configuration is valid");

    if show {
        println!("Effective configuration:");
        println!("{:#?}", config);
    }

    Ok(())
}

async fn run_benchmark(
    target: String,
    connections: usize,
    duration: u64,
    format: String,
) -> Result<()> {
    info!(
        "Starting benchmark against {} with {} connections for {} seconds",
        target, connections, duration
    );

    // TODO: Implement actual benchmarking logic
    warn!("Benchmark functionality is not yet implemented");

    println!("Benchmark Results:");
    println!("Target: {}", target);
    println!("Connections: {}", connections);
    println!("Duration: {}s", duration);
    println!("Format: {}", format);
    println!("Status: Not implemented");

    Ok(())
}

/// Serializable histogram summary for HTTP response
#[derive(Debug, Serialize, Deserialize)]
struct SerializableHistogramSummary {
    count: u64,
    sum: f64,
    mean: f64,
    min: f64,
    max: f64,
    p50: f64,
    p90: f64,
    p95: f64,
    p99: f64,
}

impl From<loka_metrics::HistogramSummary> for SerializableHistogramSummary {
    fn from(summary: loka_metrics::HistogramSummary) -> Self {
        Self {
            count: summary.count,
            sum: summary.sum,
            mean: summary.mean,
            min: summary.min,
            max: summary.max,
            // For now, we'll use simple placeholders for percentiles
            // since the basic HistogramSummary doesn't include percentile calculation
            p50: summary.mean,                            // Approximate using mean
            p90: summary.max * 0.9 + summary.min * 0.1,   // Approximate
            p95: summary.max * 0.95 + summary.min * 0.05, // Approximate
            p99: summary.max * 0.99 + summary.min * 0.01, // Approximate
        }
    }
}

/// Metrics response structure
#[derive(Debug, Serialize, Deserialize)]
struct MetricsResponse {
    timestamp: u64,
    // Loka-metrics data
    counters: std::collections::HashMap<String, u64>,
    gauges: std::collections::HashMap<String, f64>,
    histograms: std::collections::HashMap<String, SerializableHistogramSummary>,
    // Stratum-specific metrics
    stratum_global: Option<MetricsSnapshot>,
    stratum_users: Vec<UserMetricsSnapshot>,
}

/// Health check response
#[derive(Debug, Serialize, Deserialize)]
struct HealthResponse {
    status: String,
    timestamp: u64,
}

async fn get_metrics() -> Result<Json<MetricsResponse>, StatusCode> {
    // Get the current metrics recorder
    let recorder = loka_metrics::Recorder::current();

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Convert histograms to serializable format
    let histograms = recorder
        .histograms()
        .into_iter()
        .map(|(key, summary)| (key, SerializableHistogramSummary::from(summary)))
        .collect();

    // Get Stratum metrics if available
    let (stratum_global, stratum_users) = if let Some(stratum_metrics) = STRATUM_METRICS.get() {
        let global = stratum_metrics.get_global_snapshot();
        let users = stratum_metrics.get_all_user_metrics();
        (Some(global), users)
    } else {
        (None, Vec::new())
    };

    let response = MetricsResponse {
        timestamp,
        counters: recorder.counters(),
        gauges: recorder.gauges(),
        histograms,
        stratum_global,
        stratum_users,
    };

    Ok(Json(response))
}

async fn health_check() -> Json<HealthResponse> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    Json(HealthResponse {
        status: "healthy".to_string(),
        timestamp,
    })
}

async fn get_prometheus_metrics() -> Result<String, StatusCode> {
    // Get the current metrics recorder
    let recorder = loka_metrics::Recorder::current();

    let mut prometheus_output = String::new();

    // Add counters
    for (name, value) in recorder.counters() {
        prometheus_output.push_str(&format!(
            "# TYPE {} counter\n{} {}\n",
            sanitize_metric_name(&name),
            sanitize_metric_name(&name),
            value
        ));
    }

    // Add gauges
    for (name, value) in recorder.gauges() {
        prometheus_output.push_str(&format!(
            "# TYPE {} gauge\n{} {}\n",
            sanitize_metric_name(&name),
            sanitize_metric_name(&name),
            value
        ));
    }

    // Add histograms
    for (name, summary) in recorder.histograms() {
        let metric_name = sanitize_metric_name(&name);
        prometheus_output.push_str(&format!("# TYPE {} histogram\n", metric_name));
        prometheus_output.push_str(&format!("{}_count {}\n", metric_name, summary.count));
        prometheus_output.push_str(&format!("{}_sum {}\n", metric_name, summary.sum));
    }

    // Add Stratum metrics if available
    if let Some(stratum_metrics) = STRATUM_METRICS.get() {
        let global = stratum_metrics.get_global_snapshot();

        // Basic connection metrics
        prometheus_output
            .push_str("# HELP stratum_total_connections Total number of connections established\n");
        prometheus_output.push_str("# TYPE stratum_total_connections counter\n");
        prometheus_output.push_str(&format!(
            "stratum_total_connections {}\n",
            global.total_connections
        ));

        prometheus_output
            .push_str("# HELP stratum_active_connections Current number of active connections\n");
        prometheus_output.push_str("# TYPE stratum_active_connections gauge\n");
        prometheus_output.push_str(&format!(
            "stratum_active_connections {}\n",
            global.active_connections
        ));

        prometheus_output.push_str("# HELP stratum_connection_errors Total connection errors\n");
        prometheus_output.push_str("# TYPE stratum_connection_errors counter\n");
        prometheus_output.push_str(&format!(
            "stratum_connection_errors {}\n",
            global.connection_errors
        ));

        // Connection lifecycle metrics (Task 8.1)
        prometheus_output.push_str(
            "# HELP stratum_connection_duration_avg_seconds Average connection duration\n",
        );
        prometheus_output.push_str("# TYPE stratum_connection_duration_avg_seconds gauge\n");
        prometheus_output.push_str(&format!(
            "stratum_connection_duration_avg_seconds {:.6}\n",
            global.avg_connection_duration_ms / 1000.0
        ));

        prometheus_output.push_str(
            "# HELP stratum_connection_duration_max_seconds Maximum connection duration\n",
        );
        prometheus_output.push_str("# TYPE stratum_connection_duration_max_seconds gauge\n");
        prometheus_output.push_str(&format!(
            "stratum_connection_duration_max_seconds {:.6}\n",
            global.max_connection_duration_ms / 1000.0
        ));

        prometheus_output.push_str(
            "# HELP stratum_connection_duration_min_seconds Minimum connection duration\n",
        );
        prometheus_output.push_str("# TYPE stratum_connection_duration_min_seconds gauge\n");
        prometheus_output.push_str(&format!(
            "stratum_connection_duration_min_seconds {:.6}\n",
            global.min_connection_duration_ms / 1000.0
        ));

        prometheus_output.push_str("# HELP stratum_idle_time_seconds Current idle time gauge\n");
        prometheus_output.push_str("# TYPE stratum_idle_time_seconds gauge\n");
        prometheus_output.push_str(&format!(
            "stratum_idle_time_seconds {:.6}\n",
            global.idle_time_ms / 1000.0
        ));

        prometheus_output
            .push_str("# HELP stratum_reconnection_attempts_total Total reconnection attempts\n");
        prometheus_output.push_str("# TYPE stratum_reconnection_attempts_total counter\n");
        prometheus_output.push_str(&format!(
            "stratum_reconnection_attempts_total {}\n",
            global.reconnection_attempts
        ));

        prometheus_output.push_str(
            "# HELP stratum_connection_established_total Total connections established\n",
        );
        prometheus_output.push_str("# TYPE stratum_connection_established_total counter\n");
        prometheus_output.push_str(&format!(
            "stratum_connection_established_total {}\n",
            global.connections_established
        ));

        prometheus_output
            .push_str("# HELP stratum_connection_closed_total Total connections closed\n");
        prometheus_output.push_str("# TYPE stratum_connection_closed_total counter\n");
        prometheus_output.push_str(&format!(
            "stratum_connection_closed_total {}\n",
            global.connections_closed
        ));

        // Protocol detection metrics (Task 8.2)
        prometheus_output
            .push_str("# HELP stratum_http_requests_total Total HTTP requests detected\n");
        prometheus_output.push_str("# TYPE stratum_http_requests_total counter\n");
        prometheus_output.push_str(&format!(
            "stratum_http_requests_total {}\n",
            global.http_requests
        ));

        prometheus_output
            .push_str("# HELP stratum_stratum_requests_total Total Stratum requests detected\n");
        prometheus_output.push_str("# TYPE stratum_stratum_requests_total counter\n");
        prometheus_output.push_str(&format!(
            "stratum_stratum_requests_total {}\n",
            global.stratum_requests
        ));

        prometheus_output.push_str(
            "# HELP stratum_protocol_detection_failures_total Protocol detection failures\n",
        );
        prometheus_output.push_str("# TYPE stratum_protocol_detection_failures_total counter\n");
        prometheus_output.push_str(&format!(
            "stratum_protocol_detection_failures_total {}\n",
            global.protocol_detection_failures
        ));

        prometheus_output.push_str(
            "# HELP stratum_protocol_detection_successes_total Protocol detection successes\n",
        );
        prometheus_output.push_str("# TYPE stratum_protocol_detection_successes_total counter\n");
        prometheus_output.push_str(&format!(
            "stratum_protocol_detection_successes_total {}\n",
            global.protocol_detection_successes
        ));

        prometheus_output.push_str(
            "# HELP stratum_protocol_conversion_success_rate Protocol conversion success rate\n",
        );
        prometheus_output.push_str("# TYPE stratum_protocol_conversion_success_rate gauge\n");
        prometheus_output.push_str(&format!(
            "stratum_protocol_conversion_success_rate {:.6}\n",
            global.protocol_conversion_success_rate
        ));

        prometheus_output.push_str(
            "# HELP stratum_protocol_conversion_errors_total Protocol conversion errors\n",
        );
        prometheus_output.push_str("# TYPE stratum_protocol_conversion_errors_total counter\n");
        prometheus_output.push_str(&format!(
            "stratum_protocol_conversion_errors_total {}\n",
            global.protocol_conversion_errors
        ));

        prometheus_output
            .push_str("# HELP stratum_http_connect_requests_total HTTP CONNECT requests\n");
        prometheus_output.push_str("# TYPE stratum_http_connect_requests_total counter\n");
        prometheus_output.push_str(&format!(
            "stratum_http_connect_requests_total {}\n",
            global.http_connect_requests
        ));

        prometheus_output.push_str(
            "# HELP stratum_direct_stratum_connections_total Direct Stratum connections\n",
        );
        prometheus_output.push_str("# TYPE stratum_direct_stratum_connections_total counter\n");
        prometheus_output.push_str(&format!(
            "stratum_direct_stratum_connections_total {}\n",
            global.direct_stratum_connections
        ));

        // Mining operation metrics (Task 8.3)
        prometheus_output
            .push_str("# HELP stratum_share_submissions_total Total share submissions\n");
        prometheus_output.push_str("# TYPE stratum_share_submissions_total counter\n");
        prometheus_output.push_str(&format!(
            "stratum_share_submissions_total {}\n",
            global.submissions_received
        ));

        prometheus_output.push_str("# HELP stratum_share_acceptance_rate Share acceptance rate\n");
        prometheus_output.push_str("# TYPE stratum_share_acceptance_rate gauge\n");
        let acceptance_rate = if global.submissions_received > 0 {
            global.submissions_accepted as f64 / global.submissions_received as f64
        } else {
            0.0
        };
        prometheus_output.push_str(&format!(
            "stratum_share_acceptance_rate {:.6}\n",
            acceptance_rate
        ));

        prometheus_output
            .push_str("# HELP stratum_difficulty_adjustments_total Total difficulty adjustments\n");
        prometheus_output.push_str("# TYPE stratum_difficulty_adjustments_total counter\n");
        prometheus_output.push_str(&format!(
            "stratum_difficulty_adjustments_total {}\n",
            global.difficulty_adjustments
        ));

        prometheus_output.push_str("# HELP stratum_job_distribution_latency_avg_seconds Average job distribution latency\n");
        prometheus_output.push_str("# TYPE stratum_job_distribution_latency_avg_seconds gauge\n");
        prometheus_output.push_str(&format!(
            "stratum_job_distribution_latency_avg_seconds {:.6}\n",
            global.avg_job_distribution_latency_ms / 1000.0
        ));

        prometheus_output.push_str("# HELP stratum_job_distribution_latency_max_seconds Maximum job distribution latency\n");
        prometheus_output.push_str("# TYPE stratum_job_distribution_latency_max_seconds gauge\n");
        prometheus_output.push_str(&format!(
            "stratum_job_distribution_latency_max_seconds {:.6}\n",
            global.max_job_distribution_latency_ms / 1000.0
        ));

        prometheus_output.push_str("# HELP stratum_job_distribution_latency_min_seconds Minimum job distribution latency\n");
        prometheus_output.push_str("# TYPE stratum_job_distribution_latency_min_seconds gauge\n");
        prometheus_output.push_str(&format!(
            "stratum_job_distribution_latency_min_seconds {:.6}\n",
            global.min_job_distribution_latency_ms / 1000.0
        ));

        prometheus_output.push_str("# HELP stratum_current_difficulty Current mining difficulty\n");
        prometheus_output.push_str("# TYPE stratum_current_difficulty gauge\n");
        prometheus_output.push_str(&format!(
            "stratum_current_difficulty {:.2}\n",
            global.current_difficulty
        ));

        prometheus_output.push_str("# HELP stratum_shares_per_minute Current shares per minute\n");
        prometheus_output.push_str("# TYPE stratum_shares_per_minute gauge\n");
        prometheus_output.push_str(&format!(
            "stratum_shares_per_minute {}\n",
            global.shares_per_minute
        ));

        prometheus_output.push_str("# HELP stratum_stale_shares_total Total stale shares\n");
        prometheus_output.push_str("# TYPE stratum_stale_shares_total counter\n");
        prometheus_output.push_str(&format!(
            "stratum_stale_shares_total {}\n",
            global.stale_shares
        ));

        prometheus_output
            .push_str("# HELP stratum_duplicate_shares_total Total duplicate shares\n");
        prometheus_output.push_str("# TYPE stratum_duplicate_shares_total counter\n");
        prometheus_output.push_str(&format!(
            "stratum_duplicate_shares_total {}\n",
            global.duplicate_submissions
        ));

        // Error categorization metrics (Task 8.4)
        prometheus_output.push_str("# HELP stratum_network_errors_total Network-related errors\n");
        prometheus_output.push_str("# TYPE stratum_network_errors_total counter\n");
        prometheus_output.push_str(&format!(
            "stratum_network_errors_total {}\n",
            global.network_errors
        ));

        prometheus_output
            .push_str("# HELP stratum_authentication_failures_total Authentication failures\n");
        prometheus_output.push_str("# TYPE stratum_authentication_failures_total counter\n");
        prometheus_output.push_str(&format!(
            "stratum_authentication_failures_total {}\n",
            global.authentication_failures
        ));

        prometheus_output.push_str("# HELP stratum_timeout_errors Timeout errors\n");
        prometheus_output.push_str("# TYPE stratum_timeout_errors counter\n");
        prometheus_output.push_str(&format!(
            "stratum_timeout_errors {}\n",
            global.timeout_errors
        ));

        prometheus_output
            .push_str("# HELP stratum_protocol_parse_errors Protocol parsing errors\n");
        prometheus_output.push_str("# TYPE stratum_protocol_parse_errors counter\n");
        prometheus_output.push_str(&format!(
            "stratum_protocol_parse_errors {}\n",
            global.protocol_parse_errors
        ));

        prometheus_output
            .push_str("# HELP stratum_protocol_version_errors Protocol version errors\n");
        prometheus_output.push_str("# TYPE stratum_protocol_version_errors counter\n");
        prometheus_output.push_str(&format!(
            "stratum_protocol_version_errors {}\n",
            global.protocol_version_errors
        ));

        prometheus_output
            .push_str("# HELP stratum_protocol_message_errors Protocol message errors\n");
        prometheus_output.push_str("# TYPE stratum_protocol_message_errors counter\n");
        prometheus_output.push_str(&format!(
            "stratum_protocol_message_errors {}\n",
            global.protocol_message_errors
        ));

        prometheus_output.push_str("# HELP stratum_validation_errors Validation errors\n");
        prometheus_output.push_str("# TYPE stratum_validation_errors counter\n");
        prometheus_output.push_str(&format!(
            "stratum_validation_errors {}\n",
            global.validation_errors
        ));

        prometheus_output
            .push_str("# HELP stratum_security_violation_errors Security violation errors\n");
        prometheus_output.push_str("# TYPE stratum_security_violation_errors counter\n");
        prometheus_output.push_str(&format!(
            "stratum_security_violation_errors {}\n",
            global.security_violation_errors
        ));

        prometheus_output.push_str(
            "# HELP stratum_resource_exhaustion_errors Resource exhaustion errors\n",
        );
        prometheus_output.push_str("# TYPE stratum_resource_exhaustion_errors counter\n");
        prometheus_output.push_str(&format!(
            "stratum_resource_exhaustion_errors {}\n",
            global.resource_exhaustion_errors
        ));

        prometheus_output.push_str("# HELP stratum_internal_errors Internal errors\n");
        prometheus_output.push_str("# TYPE stratum_internal_errors counter\n");
        prometheus_output.push_str(&format!(
            "stratum_internal_errors {}\n",
            global.internal_errors
        ));

        // Resource utilization metrics (Task 8.5)
        prometheus_output.push_str("# HELP stratum_memory_usage_per_connection_avg_bytes Average memory usage per connection\n");
        prometheus_output.push_str("# TYPE stratum_memory_usage_per_connection_avg_bytes gauge\n");
        prometheus_output.push_str(&format!(
            "stratum_memory_usage_per_connection_avg_bytes {:.0}\n",
            global.avg_memory_per_connection_mb * 1024.0 * 1024.0
        ));

        prometheus_output.push_str("# HELP stratum_memory_usage_per_connection_max_bytes Maximum memory usage per connection\n");
        prometheus_output.push_str("# TYPE stratum_memory_usage_per_connection_max_bytes gauge\n");
        prometheus_output.push_str(&format!(
            "stratum_memory_usage_per_connection_max_bytes {:.0}\n",
            global.max_memory_per_connection_mb * 1024.0 * 1024.0
        ));

        prometheus_output.push_str("# HELP stratum_memory_usage_per_connection_min_bytes Minimum memory usage per connection\n");
        prometheus_output.push_str("# TYPE stratum_memory_usage_per_connection_min_bytes gauge\n");
        prometheus_output.push_str(&format!(
            "stratum_memory_usage_per_connection_min_bytes {:.0}\n",
            global.min_memory_per_connection_mb * 1024.0 * 1024.0
        ));

        prometheus_output.push_str(
            "# HELP stratum_cpu_utilization_percent Current CPU utilization percentage\n",
        );
        prometheus_output.push_str("# TYPE stratum_cpu_utilization_percent gauge\n");
        prometheus_output.push_str(&format!(
            "stratum_cpu_utilization_percent {:.2}\n",
            global.cpu_utilization
        ));

        prometheus_output.push_str(
            "# HELP stratum_network_bandwidth_rx_bpsond Network bandwidth receive rate\n",
        );
        prometheus_output.push_str("# TYPE stratum_network_bandwidth_rx_bpsond gauge\n");
        prometheus_output.push_str(&format!(
            "stratum_network_bandwidth_rx_bpsond {}\n",
            global.network_bandwidth_rx_bps
        ));

        prometheus_output.push_str("# HELP stratum_network_bandwidth_tx_bpsond Network bandwidth transmit rate\n");
        prometheus_output.push_str("# TYPE stratum_network_bandwidth_tx_bpsond gauge\n");
        prometheus_output.push_str(&format!(
            "stratum_network_bandwidth_tx_bpsond {}\n",
            global.network_bandwidth_tx_bps
        ));

        prometheus_output.push_str(
            "# HELP stratum_memory_usage_bytes Total memory used by connections\n",
        );
        prometheus_output.push_str("# TYPE stratum_memory_usage_bytes gauge\n");
        prometheus_output.push_str(&format!(
            "stratum_memory_usage_bytes {}\n",
            global.memory_usage_bytes
        ));

        prometheus_output.push_str(
            "# HELP stratum_connection_memory_peak Peak memory used by connections\n",
        );
        prometheus_output.push_str("# TYPE stratum_connection_memory_peak gauge\n");
        prometheus_output.push_str(&format!(
            "stratum_connection_memory_peak {}\n",
            global.connection_memory_peak
        ));

        prometheus_output
            .push_str("# HELP stratum_resource_pressure_events_total Resource pressure events\n");
        prometheus_output.push_str("# TYPE stratum_resource_pressure_events_total counter\n");
        prometheus_output.push_str(&format!(
            "stratum_resource_pressure_events_total {}\n",
            global.resource_pressure_events
        ));

        prometheus_output
            .push_str("# HELP stratum_memory_efficiency_ratio Memory efficiency ratio\n");
        prometheus_output.push_str("# TYPE stratum_memory_efficiency_ratio gauge\n");
        prometheus_output.push_str(&format!(
            "stratum_memory_efficiency_ratio {:.6}\n",
            global.memory_efficiency_ratio
        ));

        // Legacy protocol metrics (existing)
        prometheus_output.push_str("# HELP stratum_messages_received Total messages received\n");
        prometheus_output.push_str("# TYPE stratum_messages_received counter\n");
        prometheus_output.push_str(&format!(
            "stratum_messages_received {}\n",
            global.messages_received
        ));

        prometheus_output.push_str("# HELP stratum_messages_sent Total messages sent\n");
        prometheus_output.push_str("# TYPE stratum_messages_sent counter\n");
        prometheus_output.push_str(&format!("stratum_messages_sent {}\n", global.messages_sent));

        prometheus_output.push_str("# HELP stratum_bytes_received Total bytes received\n");
        prometheus_output.push_str("# TYPE stratum_bytes_received counter\n");
        prometheus_output.push_str(&format!(
            "stratum_bytes_received {}\n",
            global.bytes_received
        ));

        prometheus_output.push_str("# HELP stratum_bytes_sent Total bytes sent\n");
        prometheus_output.push_str("# TYPE stratum_bytes_sent counter\n");
        prometheus_output.push_str(&format!("stratum_bytes_sent {}\n", global.bytes_sent));

        // Legacy auth metrics (existing)
        prometheus_output.push_str("# HELP stratum_auth_attempts Total authentication attempts\n");
        prometheus_output.push_str("# TYPE stratum_auth_attempts counter\n");
        prometheus_output.push_str(&format!("stratum_auth_attempts {}\n", global.auth_attempts));

        prometheus_output
            .push_str("# HELP stratum_auth_successes Total authentication successes\n");
        prometheus_output.push_str("# TYPE stratum_auth_successes counter\n");
        prometheus_output.push_str(&format!(
            "stratum_auth_successes {}\n",
            global.auth_successes
        ));

        prometheus_output.push_str("# HELP stratum_auth_failures Total authentication failures\n");
        prometheus_output.push_str("# TYPE stratum_auth_failures counter\n");
        prometheus_output.push_str(&format!("stratum_auth_failures {}\n", global.auth_failures));

        // Legacy submission metrics (existing)
        prometheus_output
            .push_str("# HELP stratum_total_submissions Total submissions received\n");
        prometheus_output.push_str("# TYPE stratum_total_submissions counter\n");
        prometheus_output.push_str(&format!(
            "stratum_total_submissions {}\n",
            global.submissions_received
        ));

        prometheus_output
            .push_str("# HELP stratum_accepted_submissions Total submissions accepted\n");
        prometheus_output.push_str("# TYPE stratum_accepted_submissions counter\n");
        prometheus_output.push_str(&format!(
            "stratum_accepted_submissions {}\n",
            global.submissions_accepted
        ));

        prometheus_output
            .push_str("# HELP stratum_rejected_submissions Total submissions rejected\n");
        prometheus_output.push_str("# TYPE stratum_rejected_submissions counter\n");
        prometheus_output.push_str(&format!(
            "stratum_rejected_submissions {}\n",
            global.submissions_rejected
        ));

        // Legacy security metrics (existing)
        prometheus_output
            .push_str("# HELP stratum_security_violations Total security violations\n");
        prometheus_output.push_str("# TYPE stratum_security_violations counter\n");
        prometheus_output.push_str(&format!(
            "stratum_security_violations {}\n",
            global.security_violations
        ));

        prometheus_output.push_str("# HELP stratum_rate_limit_hits Total rate limit hits\n");
        prometheus_output.push_str("# TYPE stratum_rate_limit_hits counter\n");
        prometheus_output.push_str(&format!(
            "stratum_rate_limit_hits {}\n",
            global.rate_limit_hits
        ));

        // User metrics with proper TYPE declarations
        let users = stratum_metrics.get_all_user_metrics();
        if !users.is_empty() {
            prometheus_output
                .push_str("# HELP stratum_user_total_submissions User submissions received\n");
            prometheus_output.push_str("# TYPE stratum_user_total_submissions gauge\n");
            prometheus_output
                .push_str("# HELP stratum_user_accepted_submissions User submissions accepted\n");
            prometheus_output.push_str("# TYPE stratum_user_accepted_submissions gauge\n");
            prometheus_output
                .push_str("# HELP stratum_user_rejected_submissions User submissions rejected\n");
            prometheus_output.push_str("# TYPE stratum_user_rejected_submissions gauge\n");
            prometheus_output.push_str("# HELP stratum_user_hashrate User hashrate estimate\n");
            prometheus_output.push_str("# TYPE stratum_user_hashrate gauge\n");
            prometheus_output.push_str("# HELP stratum_user_connections User connection count\n");
            prometheus_output.push_str("# TYPE stratum_user_connections gauge\n");
            prometheus_output
                .push_str("# HELP stratum_user_messages_received User messages received\n");
            prometheus_output.push_str("# TYPE stratum_user_messages_received gauge\n");
            prometheus_output.push_str("# HELP stratum_user_messages_sent User messages sent\n");
            prometheus_output.push_str("# TYPE stratum_user_messages_sent gauge\n");

            for user in users.iter().take(100) {
                // Limit to 100 users to avoid huge responses
                let user_id = sanitize_label_value(&user.user_id);
                prometheus_output.push_str(&format!(
                    "stratum_user_total_submissions{{user=\"{}\"}} {}\n",
                    user_id, user.total_submissions
                ));
                prometheus_output.push_str(&format!(
                    "stratum_user_accepted_submissions{{user=\"{}\"}} {}\n",
                    user_id, user.accepted_submissions
                ));
                prometheus_output.push_str(&format!(
                    "stratum_user_rejected_submissions{{user=\"{}\"}} {}\n",
                    user_id, user.rejected_submissions
                ));
                prometheus_output.push_str(&format!(
                    "stratum_user_hashrate{{user=\"{}\"}} {}\n",
                    user_id, 0.0 // TODO: calculate from submission rate
                ));
                prometheus_output.push_str(&format!(
                    "stratum_user_connections{{user=\"{}\"}} {}\n",
                    user_id, user.total_connections
                ));
                prometheus_output.push_str(&format!(
                    "stratum_user_messages_received{{user=\"{}\"}} {}\n",
                    user_id, user.bytes_received
                ));
                prometheus_output.push_str(&format!(
                    "stratum_user_messages_sent{{user=\"{}\"}} {}\n",
                    user_id, user.bytes_sent
                ));
            }
        }
    }

    Ok(prometheus_output)
}

fn sanitize_metric_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn sanitize_label_value(value: &str) -> String {
    value
        .chars()
        .map(|c| match c {
            '"' => '\'',
            '\\' => '/',
            '\n' => ' ',
            '\r' => ' ',
            _ => c,
        })
        .collect()
}

async fn start_metrics_server(bind_addr: String) -> Result<impl std::future::Future<Output = ()>> {
    let app = Router::new()
        .route("/metrics", get(get_metrics))
        .route("/metrics/prometheus", get(get_prometheus_metrics))
        .route("/health", get(health_check))
        .route("/", get(health_check)); // Root path also serves health check

    // Note: Sentry tower layer integration will be implemented in subtask 10.4

    info!("Starting metrics HTTP server on {}", bind_addr);

    Ok(async move {
        let listener = match tokio::net::TcpListener::bind(&bind_addr).await {
            Ok(listener) => {
                info!("✅ Metrics server listening on {}", bind_addr);
                listener
            }
            Err(e) => {
                error!("❌ Failed to bind metrics server to {}: {}", bind_addr, e);
                return;
            }
        };

        if let Err(e) = axum::serve(listener, app).await {
            error!("Metrics server error: {}", e);
        }
    })
}

fn setup_logging(args: &Args) -> Result<()> {
    use tracing_subscriber::{EnvFilter, fmt, prelude::*};

    let log_level = match args.verbose {
        0 => &args.log_level,
        1 => "debug",
        _ => "trace",
    };

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));

    // Check if JSON formatting is requested via environment or deployment
    let use_json_format = std::env::var("LOG_FORMAT")
        .map(|v| v.to_lowercase() == "json")
        .unwrap_or_else(|_| {
            // Auto-detect production environment for JSON formatting
            std::env::var("ENVIRONMENT")
                .or_else(|_| std::env::var("ENV"))
                .map(|v| matches!(v.to_lowercase().as_str(), "production" | "prod"))
                .unwrap_or(false)
        });

    if use_json_format {
        // Production/JSON logging with structured fields
        let subscriber = tracing_subscriber::registry().with(env_filter).with(
            fmt::layer()
                .json()
                .with_current_span(true)
                .with_span_list(true)
                .with_target(true)
                .with_line_number(true)
                .with_file(true)
                .with_thread_ids(true)
                .with_thread_names(true)
                .flatten_event(false),
        );

        // Note: Sentry tracing layer integration will be implemented in subtask 10.4
        subscriber.init();

        info!("Structured JSON logging initialized");
    } else {
        // Development/human-readable logging
        let subscriber = tracing_subscriber::registry().with(env_filter).with(
            fmt::layer()
                .with_target(true)
                .with_line_number(true)
                .with_file(false)
                .with_thread_ids(false)
                .with_thread_names(false)
                .compact(),
        );

        // Note: Sentry tracing layer integration will be implemented in subtask 10.4
        subscriber.init();

        info!("Human-readable logging initialized");
    }

    Ok(())
}


/// Handle database management commands
async fn handle_database_command(command: crate::cli::DatabaseCommands, url: String) -> Result<()> {
    use crate::cli::DatabaseCommands;
    use crate::services::database::DatabaseService;
    use migration::{Migrator, MigratorTrait};

    // Load configuration to get database URL
    info!("Connecting to database...");
    let database_service = DatabaseService::new(&url).await?;

    match command {
        DatabaseCommands::Migrate { up, down } => match (up, down) {
            (true, None) => {
                info!("Running up migrations...");
                Migrator::up(&database_service.connection, None).await?;
                info!("All migrations completed successfully");
            }
            (false, Some(steps)) => {
                info!("Running down migrations ({} steps)...", steps);
                Migrator::down(&database_service.connection, Some(steps)).await?;
                info!("Down migrations completed successfully");
            }
            (false, None) => {
                info!("Running all pending up migrations...");
                Migrator::up(&database_service.connection, None).await?;
                info!("All migrations completed successfully");
            }
            (true, Some(_)) => {
                error!("Cannot specify both --up and --down");
                return Ok(());
            }
        },
        DatabaseCommands::Status => {
            info!("Checking database connection: {}", url);

            match database_service.health_check().await {
                Ok(_) => {
                    info!("✅ Database connection successful");
                    info!("Database URL: {}", url);
                }
                Err(e) => {
                    error!("❌ Database connection failed: {}", e);
                }
            }
        }
        DatabaseCommands::Reset { confirm } => {
            if !confirm {
                error!("Database reset requires --confirm flag");
                warn!("This will drop ALL tables and data!");
                return Ok(());
            }

            info!("🚨 RESETTING DATABASE: {}", url);

            info!("Dropping all tables...");
            Migrator::down(&database_service.connection, None).await?;

            info!("Running fresh migrations...");
            Migrator::up(&database_service.connection, None).await?;

            info!("✅ Database reset completed successfully");
        }
    }

    Ok(())
}
