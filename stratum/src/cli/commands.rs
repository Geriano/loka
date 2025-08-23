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
        #[cfg(feature = "mock-pool")]
        crate::cli::Commands::MockPool {
            bind,
            config,
            accept_rate,
            job_interval,
            difficulty,
            vardiff,
            latency,
            error_rate,
        } => {
            start_mock_pool(
                bind,
                config,
                accept_rate,
                job_interval,
                difficulty,
                vardiff,
                latency,
                error_rate,
            )
            .await
        }
        #[cfg(feature = "mock-miner")]
        crate::cli::Commands::MockMiner {
            pool,
            config,
            workers,
            hashrate,
            username,
            password,
            worker_prefix,
            share_interval,
            stale_rate,
            invalid_rate,
            duration,
            log_mining,
        } => {
            start_mock_miner(
                pool,
                config,
                workers,
                hashrate,
                username,
                password,
                worker_prefix,
                share_interval,
                stale_rate,
                invalid_rate,
                duration,
                log_mining,
            )
            .await
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

        // Global connection metrics
        prometheus_output.push_str(&format!(
            "# TYPE stratum_total_connections counter\nstratum_total_connections {}\n",
            global.total_connections
        ));
        prometheus_output.push_str(&format!(
            "# TYPE stratum_active_connections gauge\nstratum_active_connections {}\n",
            global.active_connections
        ));
        prometheus_output.push_str(&format!(
            "# TYPE stratum_connection_errors counter\nstratum_connection_errors {}\n",
            global.connection_errors
        ));

        // Protocol metrics
        prometheus_output.push_str(&format!(
            "# TYPE stratum_messages_received counter\nstratum_messages_received {}\n",
            global.messages_received
        ));
        prometheus_output.push_str(&format!(
            "# TYPE stratum_messages_sent counter\nstratum_messages_sent {}\n",
            global.messages_sent
        ));
        prometheus_output.push_str(&format!(
            "# TYPE stratum_bytes_received counter\nstratum_bytes_received {}\n",
            global.bytes_received
        ));
        prometheus_output.push_str(&format!(
            "# TYPE stratum_bytes_sent counter\nstratum_bytes_sent {}\n",
            global.bytes_sent
        ));

        // Auth metrics
        prometheus_output.push_str(&format!(
            "# TYPE stratum_auth_attempts counter\nstratum_auth_attempts {}\n",
            global.auth_attempts
        ));
        prometheus_output.push_str(&format!(
            "# TYPE stratum_auth_successes counter\nstratum_auth_successes {}\n",
            global.auth_successes
        ));
        prometheus_output.push_str(&format!(
            "# TYPE stratum_auth_failures counter\nstratum_auth_failures {}\n",
            global.auth_failures
        ));

        // Submission metrics
        prometheus_output.push_str(&format!(
            "# TYPE stratum_submissions_received counter\nstratum_submissions_received {}\n",
            global.submissions_received
        ));
        prometheus_output.push_str(&format!(
            "# TYPE stratum_submissions_accepted counter\nstratum_submissions_accepted {}\n",
            global.submissions_accepted
        ));
        prometheus_output.push_str(&format!(
            "# TYPE stratum_submissions_rejected counter\nstratum_submissions_rejected {}\n",
            global.submissions_rejected
        ));

        // Security metrics
        prometheus_output.push_str(&format!(
            "# TYPE stratum_security_violations counter\nstratum_security_violations {}\n",
            global.security_violations
        ));
        prometheus_output.push_str(&format!(
            "# TYPE stratum_rate_limit_hits counter\nstratum_rate_limit_hits {}\n",
            global.rate_limit_hits
        ));

        // User metrics
        let users = stratum_metrics.get_all_user_metrics();
        for user in users.iter().take(100) {
            // Limit to 100 users to avoid huge responses
            let user_id = sanitize_label_value(&user.user_id);
            prometheus_output.push_str(&format!(
                "stratum_user_submissions_received{{user=\"{}\"}} {}\n",
                user_id, user.submissions_received
            ));
            prometheus_output.push_str(&format!(
                "stratum_user_submissions_accepted{{user=\"{}\"}} {}\n",
                user_id, user.submissions_accepted
            ));
            prometheus_output.push_str(&format!(
                "stratum_user_submissions_rejected{{user=\"{}\"}} {}\n",
                user_id, user.submissions_rejected
            ));
            prometheus_output.push_str(&format!(
                "stratum_user_hashrate{{user=\"{}\"}} {}\n",
                user_id, user.hashrate_estimate
            ));
            prometheus_output.push_str(&format!(
                "stratum_user_connections{{user=\"{}\"}} {}\n",
                user_id, user.connections
            ));
            prometheus_output.push_str(&format!(
                "stratum_user_messages_received{{user=\"{}\"}} {}\n",
                user_id, user.messages_received
            ));
            prometheus_output.push_str(&format!(
                "stratum_user_messages_sent{{user=\"{}\"}} {}\n",
                user_id, user.messages_sent
            ));
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
    use tracing_subscriber::EnvFilter;

    let log_level = match args.verbose {
        0 => &args.log_level,
        1 => "debug",
        _ => "trace",
    };

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));

    // For now, use simple logging setup
    // TODO: Implement different log formats when JSON support is available
    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    Ok(())
}

#[cfg(feature = "mock-pool")]
async fn start_mock_pool(
    bind: String,
    config_path: Option<std::path::PathBuf>,
    accept_rate: f64,
    job_interval: u64,
    difficulty: u64,
    vardiff: bool,
    latency: u64,
    error_rate: f64,
) -> Result<()> {
    use crate::mock::{MockConfig, MockPool};

    info!("Starting mock mining pool on {}", bind);

    // Load or create config
    let mut config = if let Some(path) = config_path {
        let content = tokio::fs::read_to_string(path).await?;
        toml::from_str(&content)?
    } else {
        MockConfig::default()
    };

    // Override with CLI arguments
    config.accept_rate = accept_rate;
    config.job_interval_secs = job_interval;
    config.initial_difficulty = difficulty;
    config.vardiff_enabled = vardiff;
    config.latency_ms = latency;
    config.error_rate = error_rate;

    // Start the mock pool
    let pool = MockPool::new(config);
    let handle = pool.start(&bind).await?;

    info!("Mock pool started successfully");
    info!("Configuration:");
    info!("  Accept rate: {:.1}%", accept_rate * 100.0);
    info!("  Job interval: {}s", job_interval);
    info!("  Initial difficulty: {}", difficulty);
    info!(
        "  Vardiff: {}",
        if vardiff { "enabled" } else { "disabled" }
    );
    info!("  Latency: {}ms", latency);
    info!("  Error rate: {:.1}%", error_rate * 100.0);
    info!("");
    info!("Press Ctrl+C to stop the mock pool");

    // Wait for shutdown signal
    tokio::select! {
        _ = signal::ctrl_c() => {
            info!("Received shutdown signal");
        }
    }

    info!("Shutting down mock pool...");
    handle.shutdown().await?;
    info!("Mock pool shutdown completed");

    Ok(())
}

#[cfg(feature = "mock-miner")]
async fn start_mock_miner(
    pool: String,
    config_path: Option<std::path::PathBuf>,
    workers: usize,
    hashrate: f64,
    username: String,
    password: String,
    worker_prefix: String,
    share_interval: f64,
    stale_rate: f64,
    invalid_rate: f64,
    duration: u64,
    log_mining: bool,
) -> Result<()> {
    use crate::miner::{MinerConfig, MinerSimulator};

    info!("Starting mock miner targeting {}", pool);

    // Load or create config
    let mut config = if let Some(path) = config_path {
        let content = tokio::fs::read_to_string(path).await?;
        toml::from_str(&content)?
    } else {
        MinerConfig::default()
    };

    // Override with CLI arguments
    config.pool_address = pool;
    config.workers = workers;
    config.hashrate_mhs = hashrate;
    config.username = username;
    config.password = password;
    config.worker_prefix = worker_prefix;
    config.share_interval_secs = share_interval;
    config.stale_rate = stale_rate;
    config.invalid_rate = invalid_rate;
    config.duration_secs = duration;
    config.log_mining = log_mining;

    info!("Configuration:");
    info!("  Pool address: {}", config.pool_address);
    info!("  Workers: {}", config.workers);
    info!("  Hashrate per worker: {:.2} MH/s", config.hashrate_mhs);
    info!("  Total hashrate: {:.2} MH/s", config.total_hashrate());
    info!("  Username: {}", config.username);
    info!("  Share interval: {:.1}s", config.share_interval_secs);
    info!("  Stale rate: {:.1}%", config.stale_rate * 100.0);
    info!("  Invalid rate: {:.1}%", config.invalid_rate * 100.0);

    if let Some(duration) = config.simulation_duration() {
        info!("  Duration: {:.1}s", duration.as_secs_f64());
    } else {
        info!("  Duration: infinite");
    }

    info!("");

    // Start the simulation
    let simulator = MinerSimulator::new(config);
    simulator.run().await?;

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
