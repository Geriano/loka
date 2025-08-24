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
use crate::services::metrics::constant::*;
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
         Endpoint: {endpoint}\n\
         Format: {format}\n\
         Status: Running\n\
         Active Connections: N/A\n\
         Messages Processed: N/A\n\
         Uptime: N/A"
    );

    match format {
        "json" => {
            println!(
                "{{\
                \"endpoint\": \"{endpoint}\",\
                \"status\": \"running\",\
                \"active_connections\": null,\
                \"messages_processed\": null,\
                \"uptime\": null\
            }}"
            );
        }
        "yaml" => {
            println!(
                "endpoint: {endpoint}\nstatus: running\nactive_connections: null\nmessages_processed: null\nuptime: null"
            );
        }
        _ => {
            println!("{status}");
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
        println!("{config:#?}");
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
    println!("Target: {target}");
    println!("Connections: {connections}");
    println!("Duration: {duration}s");
    println!("Format: {format}");
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
        prometheus_output.push_str(&format!("# TYPE {metric_name} histogram\n"));
        prometheus_output.push_str(&format!("{}_count {}\n", metric_name, summary.count));
        prometheus_output.push_str(&format!("{}_sum {}\n", metric_name, summary.sum));
    }

    // Add Stratum metrics if available
    if let Some(stratum_metrics) = STRATUM_METRICS.get() {
        let global = stratum_metrics.get_global_snapshot();

        // Basic connection metrics
        // prometheus_output
        //     .push_str("# HELP stratum_total_connections Total number of connections established\n");
        prometheus_output.push_str(STRATUM_TOTAL_CONNECTIONS_HELP);
        prometheus_output.push_str("\n");
        // prometheus_output.push_str("# TYPE stratum_total_connections counter\n");
        prometheus_output.push_str(STRATUM_TOTAL_CONNECTIONS_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_total_connections {}\n",
            global.total_connections
        ));

        // prometheus_output
        //     .push_str("# HELP stratum_active_connections Current number of active connections\n");
        prometheus_output.push_str(STRATUM_ACTIVE_CONNECTIONS_HELP);
        prometheus_output.push_str("\n");
        // prometheus_output.push_str("# TYPE stratum_active_connections gauge\n");
        prometheus_output.push_str(STRATUM_ACTIVE_CONNECTIONS_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_active_connections {}\n",
            global.active_connections
        ));

        prometheus_output.push_str(STRATUM_CONNECTION_ERRORS_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_CONNECTION_ERRORS_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_connection_errors {}\n",
            global.connection_errors
        ));

        // Connection lifecycle metrics (Task 8.1)
        prometheus_output.push_str(STRATUM_CONNECTION_DURATION_AVG_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_CONNECTION_DURATION_AVG_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_connection_duration_avg_seconds {:.6}\n",
            global.avg_connection_duration_ms / 1000.0
        ));

        prometheus_output.push_str(STRATUM_CONNECTION_DURATION_MAX_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_CONNECTION_DURATION_MAX_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_connection_duration_max_seconds {:.6}\n",
            global.max_connection_duration_ms / 1000.0
        ));

        prometheus_output.push_str(STRATUM_CONNECTION_DURATION_MIN_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_CONNECTION_DURATION_MIN_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_connection_duration_min_seconds {:.6}\n",
            global.min_connection_duration_ms / 1000.0
        ));

        prometheus_output.push_str(STRATUM_IDLE_TIME_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_IDLE_TIME_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_idle_time_seconds {:.6}\n",
            global.idle_time_ms / 1000.0
        ));

        prometheus_output.push_str(STRATUM_RECONNECTION_ATTEMPTS_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_RECONNECTION_ATTEMPTS_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_reconnection_attempts_total {}\n",
            global.reconnection_attempts
        ));

        prometheus_output.push_str(STRATUM_CONNECTION_ESTABLISHED_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_CONNECTION_ESTABLISHED_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_connection_established_total {}\n",
            global.connections_established
        ));

        prometheus_output.push_str(STRATUM_CONNECTION_CLOSED_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_CONNECTION_CLOSED_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_connection_closed_total {}\n",
            global.connections_closed
        ));

        // Protocol detection metrics (Task 8.2)
        prometheus_output.push_str(STRATUM_HTTP_REQUESTS_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_HTTP_REQUESTS_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_http_requests_total {}\n",
            global.http_requests
        ));

        prometheus_output.push_str(STRATUM_STRATUM_REQUESTS_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_STRATUM_REQUESTS_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_stratum_requests_total {}\n",
            global.stratum_requests
        ));

        prometheus_output.push_str(STRATUM_PROTOCOL_DETECTION_FAILURES_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_PROTOCOL_DETECTION_FAILURES_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_protocol_detection_failures_total {}\n",
            global.protocol_detection_failures
        ));

        prometheus_output.push_str(STRATUM_PROTOCOL_DETECTION_SUCCESSES_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_PROTOCOL_DETECTION_SUCCESSES_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_protocol_detection_successes_total {}\n",
            global.protocol_detection_successes
        ));

        prometheus_output.push_str(STRATUM_PROTOCOL_CONVERSION_SUCCESS_RATE_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_PROTOCOL_CONVERSION_SUCCESS_RATE_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_protocol_conversion_success_rate {:.6}\n",
            global.protocol_conversion_success_rate
        ));

        prometheus_output.push_str(STRATUM_PROTOCOL_CONVERSION_ERRORS_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_PROTOCOL_CONVERSION_ERRORS_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_protocol_conversion_errors_total {}\n",
            global.protocol_conversion_errors
        ));

        prometheus_output.push_str(STRATUM_HTTP_CONNECT_REQUESTS_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_HTTP_CONNECT_REQUESTS_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_http_connect_requests_total {}\n",
            global.http_connect_requests
        ));

        prometheus_output.push_str(STRATUM_DIRECT_STRATUM_CONNECTIONS_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_DIRECT_STRATUM_CONNECTIONS_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_direct_stratum_connections_total {}\n",
            global.direct_stratum_connections
        ));

        // Mining operation metrics (Task 8.3)
        prometheus_output.push_str(STRATUM_SHARE_SUBMISSIONS_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_SHARE_SUBMISSIONS_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_share_submissions_total {}\n",
            global.submissions_received
        ));

        prometheus_output.push_str(STRATUM_SHARE_ACCEPTANCE_RATE_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_SHARE_ACCEPTANCE_RATE_TYPE);
        prometheus_output.push_str("\n");
        let acceptance_rate = if global.submissions_received > 0 {
            global.submissions_accepted as f64 / global.submissions_received as f64
        } else {
            0.0
        };
        prometheus_output.push_str(&format!(
            "stratum_share_acceptance_rate {acceptance_rate:.6}\n"
        ));

        prometheus_output.push_str(STRATUM_DIFFICULTY_ADJUSTMENTS_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_DIFFICULTY_ADJUSTMENTS_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_difficulty_adjustments_total {}\n",
            global.difficulty_adjustments
        ));

        prometheus_output.push_str(STRATUM_JOB_DISTRIBUTION_LATENCY_AVG_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_JOB_DISTRIBUTION_LATENCY_AVG_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_job_distribution_latency_avg_seconds {:.6}\n",
            global.avg_job_distribution_latency_ms / 1000.0
        ));

        prometheus_output.push_str(STRATUM_JOB_DISTRIBUTION_LATENCY_MAX_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_JOB_DISTRIBUTION_LATENCY_MAX_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_job_distribution_latency_max_seconds {:.6}\n",
            global.max_job_distribution_latency_ms / 1000.0
        ));

        prometheus_output.push_str(STRATUM_JOB_DISTRIBUTION_LATENCY_MIN_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_JOB_DISTRIBUTION_LATENCY_MIN_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_job_distribution_latency_min_seconds {:.6}\n",
            global.min_job_distribution_latency_ms / 1000.0
        ));

        prometheus_output.push_str(STRATUM_CURRENT_DIFFICULTY_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_CURRENT_DIFFICULTY_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_current_difficulty {:.2}\n",
            global.current_difficulty
        ));

        prometheus_output.push_str(STRATUM_SHARES_PER_MINUTE_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_SHARES_PER_MINUTE_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_shares_per_minute {}\n",
            global.shares_per_minute
        ));

        prometheus_output.push_str(STRATUM_STALE_SHARES_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_STALE_SHARES_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_stale_shares_total {}\n",
            global.stale_shares
        ));

        prometheus_output.push_str(STRATUM_DUPLICATE_SHARES_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_DUPLICATE_SHARES_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_duplicate_shares_total {}\n",
            global.duplicate_submissions
        ));

        // Error categorization metrics (Task 8.4)
        prometheus_output.push_str(STRATUM_NETWORK_ERRORS_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_NETWORK_ERRORS_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_network_errors_total {}\n",
            global.network_errors
        ));

        prometheus_output.push_str(STRATUM_AUTHENTICATION_FAILURES_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_AUTHENTICATION_FAILURES_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_authentication_failures_total {}\n",
            global.authentication_failures
        ));

        prometheus_output.push_str(STRATUM_TIMEOUT_ERRORS_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_TIMEOUT_ERRORS_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_timeout_errors {}\n",
            global.timeout_errors
        ));

        prometheus_output.push_str(STRATUM_PROTOCOL_PARSE_ERRORS_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_PROTOCOL_PARSE_ERRORS_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_protocol_parse_errors {}\n",
            global.protocol_parse_errors
        ));

        prometheus_output.push_str(STRATUM_PROTOCOL_VERSION_ERRORS_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_PROTOCOL_VERSION_ERRORS_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_protocol_version_errors {}\n",
            global.protocol_version_errors
        ));

        prometheus_output.push_str(STRATUM_PROTOCOL_MESSAGE_ERRORS_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_PROTOCOL_MESSAGE_ERRORS_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_protocol_message_errors {}\n",
            global.protocol_message_errors
        ));

        prometheus_output.push_str(STRATUM_VALIDATION_ERRORS_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_VALIDATION_ERRORS_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_validation_errors {}\n",
            global.validation_errors
        ));

        prometheus_output.push_str(STRATUM_SECURITY_VIOLATION_ERRORS_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_SECURITY_VIOLATION_ERRORS_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_security_violation_errors {}\n",
            global.security_violation_errors
        ));

        prometheus_output.push_str(STRATUM_RESOURCE_EXHAUSTION_ERRORS_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_RESOURCE_EXHAUSTION_ERRORS_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_resource_exhaustion_errors {}\n",
            global.resource_exhaustion_errors
        ));

        prometheus_output.push_str(STRATUM_INTERNAL_ERRORS_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_INTERNAL_ERRORS_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_internal_errors {}\n",
            global.internal_errors
        ));

        // Resource utilization metrics (Task 8.5) - Fixed naming to match alert rules
        prometheus_output.push_str(STRATUM_MEMORY_USAGE_AVG_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_MEMORY_USAGE_AVG_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_memory_usage_per_connection_avg_mb {:.2}\n",
            global.avg_memory_per_connection_mb
        ));

        prometheus_output.push_str(STRATUM_MEMORY_USAGE_MAX_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_MEMORY_USAGE_MAX_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_memory_usage_per_connection_max_mb {:.2}\n",
            global.max_memory_per_connection_mb
        ));

        prometheus_output.push_str(STRATUM_MEMORY_USAGE_MIN_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_MEMORY_USAGE_MIN_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_memory_usage_per_connection_min_mb {:.2}\n",
            global.min_memory_per_connection_mb
        ));

        prometheus_output.push_str(STRATUM_CPU_UTILIZATION_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_CPU_UTILIZATION_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_cpu_utilization_current_percent {:.2}\n",
            global.cpu_utilization
        ));

        prometheus_output.push_str(STRATUM_NETWORK_BANDWIDTH_RX_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_NETWORK_BANDWIDTH_RX_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_network_bandwidth_rx_bytes_per_sec {}\n",
            global.network_bandwidth_rx_bps
        ));

        prometheus_output.push_str(STRATUM_NETWORK_BANDWIDTH_TX_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_NETWORK_BANDWIDTH_TX_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_network_bandwidth_tx_bytes_per_sec {}\n",
            global.network_bandwidth_tx_bps
        ));

        prometheus_output.push_str(STRATUM_CONNECTION_MEMORY_TOTAL_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_CONNECTION_MEMORY_TOTAL_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_connection_memory_total_bytes {}\n",
            global.connection_memory_total
        ));

        prometheus_output.push_str(STRATUM_CONNECTION_MEMORY_PEAK_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_CONNECTION_MEMORY_PEAK_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_connection_memory_peak_bytes {}\n",
            global.connection_memory_peak
        ));

        // Additional missing critical metrics to match alert rules
        prometheus_output.push_str(STRATUM_CONNECTION_DURATION_AVG_MS_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_CONNECTION_DURATION_AVG_MS_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_connection_duration_avg_ms {:.2}\n",
            global.avg_connection_duration_ms
        ));

        prometheus_output.push_str(STRATUM_IDLE_TIME_GAUGE_MS_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_IDLE_TIME_GAUGE_MS_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_idle_time_gauge_ms {:.2}\n",
            global.idle_time_ms
        ));

        prometheus_output.push_str(STRATUM_RECONNECTION_ATTEMPTS_COUNTER_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_RECONNECTION_ATTEMPTS_COUNTER_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_reconnection_attempts_counter {}\n",
            global.reconnection_attempts
        ));

        prometheus_output.push_str(STRATUM_CONNECTION_ESTABLISHED_COUNTER_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_CONNECTION_ESTABLISHED_COUNTER_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_connection_established_counter {}\n",
            global.connections_established
        ));

        prometheus_output.push_str(STRATUM_CONNECTION_CLOSED_COUNTER_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_CONNECTION_CLOSED_COUNTER_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_connection_closed_counter {}\n",
            global.connections_closed
        ));

        // Missing mining metrics needed by alert rules
        prometheus_output.push_str(STRATUM_PROTOCOL_CONVERSION_AVG_SUCCESS_RATE_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_PROTOCOL_CONVERSION_AVG_SUCCESS_RATE_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_protocol_conversion_avg_success_rate {:.6}\n",
            global.protocol_conversion_success_rate
        ));

        prometheus_output.push_str(STRATUM_SHARE_ACCEPTANCE_AVG_RATE_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_SHARE_ACCEPTANCE_AVG_RATE_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_share_acceptance_avg_rate {:.6}\n",
            acceptance_rate
        ));

        prometheus_output.push_str(STRATUM_DIFFICULTY_ADJUSTMENTS_COUNTER_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_DIFFICULTY_ADJUSTMENTS_COUNTER_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_difficulty_adjustments_counter {}\n",
            global.difficulty_adjustments
        ));

        prometheus_output.push_str(STRATUM_SHARES_PER_MINUTE_GAUGE_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_SHARES_PER_MINUTE_GAUGE_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_shares_per_minute_gauge {}\n",
            global.shares_per_minute
        ));

        prometheus_output.push_str(STRATUM_STALE_SHARES_COUNTER_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_STALE_SHARES_COUNTER_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_stale_shares_counter {}\n",
            global.stale_shares
        ));

        prometheus_output.push_str(STRATUM_DUPLICATE_SHARES_COUNTER_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_DUPLICATE_SHARES_COUNTER_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_duplicate_shares_counter {}\n",
            global.duplicate_submissions
        ));

        // Job distribution latency metrics with proper naming
        prometheus_output.push_str(STRATUM_JOB_DISTRIBUTION_LATENCY_AVG_MS_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_JOB_DISTRIBUTION_LATENCY_AVG_MS_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_job_distribution_latency_avg_ms {:.2}\n",
            global.avg_job_distribution_latency_ms
        ));

        prometheus_output.push_str(STRATUM_JOB_DISTRIBUTION_LATENCY_MAX_MS_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_JOB_DISTRIBUTION_LATENCY_MAX_MS_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_job_distribution_latency_max_ms {:.2}\n",
            global.max_job_distribution_latency_ms
        ));

        prometheus_output.push_str(STRATUM_JOB_DISTRIBUTION_LATENCY_MIN_MS_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_JOB_DISTRIBUTION_LATENCY_MIN_MS_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_job_distribution_latency_min_ms {:.2}\n",
            global.min_job_distribution_latency_ms
        ));

        // Missing error metrics with proper _total suffix
        prometheus_output.push_str(STRATUM_TIMEOUT_ERRORS_TOTAL_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_TIMEOUT_ERRORS_TOTAL_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_timeout_errors_total {}\n",
            global.timeout_errors
        ));

        prometheus_output.push_str(STRATUM_PROTOCOL_PARSE_ERRORS_TOTAL_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_PROTOCOL_PARSE_ERRORS_TOTAL_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_protocol_parse_errors_total {}\n",
            global.protocol_parse_errors
        ));

        prometheus_output.push_str(STRATUM_VALIDATION_ERRORS_TOTAL_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_VALIDATION_ERRORS_TOTAL_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_validation_errors_total {}\n",
            global.validation_errors
        ));

        prometheus_output.push_str(STRATUM_SECURITY_VIOLATION_ERRORS_TOTAL_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_SECURITY_VIOLATION_ERRORS_TOTAL_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_security_violation_errors_total {}\n",
            global.security_violation_errors
        ));

        prometheus_output.push_str(STRATUM_RESOURCE_EXHAUSTION_ERRORS_TOTAL_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_RESOURCE_EXHAUSTION_ERRORS_TOTAL_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_resource_exhaustion_errors_total {}\n",
            global.resource_exhaustion_errors
        ));

        prometheus_output.push_str(STRATUM_INTERNAL_ERRORS_TOTAL_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_INTERNAL_ERRORS_TOTAL_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_internal_errors_total {}\n",
            global.internal_errors
        ));

        // Additional resource utilization metrics
        prometheus_output.push_str(STRATUM_MEMORY_EFFICIENCY_RATIO_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_MEMORY_EFFICIENCY_RATIO_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_memory_efficiency_ratio {:.6}\n",
            global.memory_efficiency_ratio
        ));

        prometheus_output.push_str(STRATUM_RESOURCE_PRESSURE_EVENTS_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_RESOURCE_PRESSURE_EVENTS_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_resource_pressure_events {}\n",
            global.resource_pressure_events
        ));

        // Missing protocol detection metrics
        prometheus_output.push_str(STRATUM_HTTP_CONNECT_REQUESTS_ALT_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_HTTP_CONNECT_REQUESTS_ALT_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_http_connect_requests {}\n",
            global.http_connect_requests
        ));

        prometheus_output.push_str(STRATUM_DIRECT_STRATUM_CONNECTIONS_ALT_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_DIRECT_STRATUM_CONNECTIONS_ALT_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_direct_stratum_connections {}\n",
            global.direct_stratum_connections
        ));

        // Basic protocol metrics needed by alert rules
        prometheus_output.push_str(STRATUM_MESSAGES_RECEIVED_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_MESSAGES_RECEIVED_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_messages_received {}\n",
            global.messages_received
        ));

        prometheus_output.push_str(STRATUM_SUBMISSIONS_RECEIVED_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_SUBMISSIONS_RECEIVED_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_submissions_received {}\n",
            global.submissions_received
        ));

        prometheus_output.push_str(STRATUM_SUBMISSIONS_ACCEPTED_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_SUBMISSIONS_ACCEPTED_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_submissions_accepted {}\n",
            global.submissions_accepted
        ));

        // Authentication metrics
        prometheus_output.push_str(STRATUM_AUTH_SUCCESSES_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_AUTH_SUCCESSES_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_auth_successes {}\n",
            global.auth_successes
        ));

        prometheus_output.push_str(STRATUM_AUTH_FAILURES_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_AUTH_FAILURES_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!("stratum_auth_failures {}\n", global.auth_failures));
        prometheus_output.push_str(STRATUM_AUTH_ATTEMPTS_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_AUTH_ATTEMPTS_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!("stratum_auth_attempts {}\n", global.auth_attempts));

        prometheus_output.push_str(STRATUM_AUTH_SUCCESSES_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_AUTH_SUCCESSES_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_auth_successes {}\n",
            global.auth_successes
        ));

        prometheus_output.push_str(STRATUM_AUTH_FAILURES_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_AUTH_FAILURES_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!("stratum_auth_failures {}\n", global.auth_failures));

        // Legacy submission metrics (existing)
        prometheus_output.push_str(STRATUM_TOTAL_SUBMISSIONS_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_TOTAL_SUBMISSIONS_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_total_submissions {}\n",
            global.submissions_received
        ));

        prometheus_output.push_str(STRATUM_ACCEPTED_SUBMISSIONS_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_ACCEPTED_SUBMISSIONS_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_accepted_submissions {}\n",
            global.submissions_accepted
        ));

        prometheus_output.push_str(STRATUM_REJECTED_SUBMISSIONS_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_REJECTED_SUBMISSIONS_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_rejected_submissions {}\n",
            global.submissions_rejected
        ));

        // Legacy security metrics (existing)
        prometheus_output.push_str(STRATUM_SECURITY_VIOLATIONS_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_SECURITY_VIOLATIONS_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_security_violations {}\n",
            global.security_violations
        ));

        prometheus_output.push_str(STRATUM_RATE_LIMIT_HITS_HELP);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(STRATUM_RATE_LIMIT_HITS_TYPE);
        prometheus_output.push_str("\n");
        prometheus_output.push_str(&format!(
            "stratum_rate_limit_hits {}\n",
            global.rate_limit_hits
        ));

        // User metrics with proper TYPE declarations
        let users = stratum_metrics.get_all_user_metrics();
        if !users.is_empty() {
            prometheus_output.push_str(STRATUM_USER_TOTAL_SUBMISSIONS_HELP);
            prometheus_output.push_str("\n");
            prometheus_output.push_str(STRATUM_USER_TOTAL_SUBMISSIONS_TYPE);
            prometheus_output.push_str("\n");
            prometheus_output.push_str(STRATUM_USER_ACCEPTED_SUBMISSIONS_HELP);
            prometheus_output.push_str("\n");
            prometheus_output.push_str(STRATUM_USER_ACCEPTED_SUBMISSIONS_TYPE);
            prometheus_output.push_str("\n");
            prometheus_output.push_str(STRATUM_USER_REJECTED_SUBMISSIONS_HELP);
            prometheus_output.push_str("\n");
            prometheus_output.push_str(STRATUM_USER_REJECTED_SUBMISSIONS_TYPE);
            prometheus_output.push_str("\n");
            prometheus_output.push_str(STRATUM_USER_HASHRATE_HELP);
            prometheus_output.push_str("\n");
            prometheus_output.push_str(STRATUM_USER_HASHRATE_TYPE);
            prometheus_output.push_str("\n");
            prometheus_output.push_str(STRATUM_USER_CONNECTIONS_HELP);
            prometheus_output.push_str("\n");
            prometheus_output.push_str(STRATUM_USER_CONNECTIONS_TYPE);
            prometheus_output.push_str("\n");
            prometheus_output.push_str(STRATUM_USER_MESSAGES_RECEIVED_HELP);
            prometheus_output.push_str("\n");
            prometheus_output.push_str(STRATUM_USER_MESSAGES_RECEIVED_TYPE);
            prometheus_output.push_str("\n");
            prometheus_output.push_str(STRATUM_USER_MESSAGES_SENT_HELP);
            prometheus_output.push_str("\n");
            prometheus_output.push_str(STRATUM_USER_MESSAGES_SENT_TYPE);
            prometheus_output.push_str("\n");

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
                    user_id,
                    0.0 // TODO: calculate from submission rate
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
        .route("/", get(health_check));

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
