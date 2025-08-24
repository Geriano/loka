use anyhow::Result;
use sentry::{ClientOptions, configure_scope};
use std::env;
use tracing::info;

use crate::config::SentryConfig;

/// Initialize Sentry with configuration from environment variables or config
pub fn init_sentry(config: Option<&SentryConfig>) -> Result<Option<sentry::ClientInitGuard>> {
    // Try to get DSN from config first, then environment variable
    let dsn = config
        .map(|c| c.dsn.clone())
        .or_else(|| env::var("SENTRY_DSN").ok())
        .filter(|dsn| !dsn.is_empty());

    if let Some(dsn) = dsn {
        info!("🔧 Initializing Sentry error tracking");

        // Get environment from config or environment variable
        let environment = config
            .and_then(|c| c.environment.clone())
            .or_else(|| env::var("SENTRY_ENVIRONMENT").ok())
            .unwrap_or_else(|| {
                if cfg!(debug_assertions) {
                    "development".to_string()
                } else {
                    "production".to_string()
                }
            });

        // Get sample rates from config or use defaults
        let sample_rate = config
            .and_then(|c| c.sample_rate)
            .or_else(|| {
                env::var("SENTRY_SAMPLE_RATE")
                    .ok()
                    .and_then(|s| s.parse().ok())
            })
            .unwrap_or(1.0);

        let traces_sample_rate = config
            .and_then(|c| c.traces_sample_rate)
            .or_else(|| {
                env::var("SENTRY_TRACES_SAMPLE_RATE")
                    .ok()
                    .and_then(|s| s.parse().ok())
            })
            .unwrap_or(1.0);

        let debug = config
            .and_then(|c| c.debug)
            .or_else(|| env::var("SENTRY_DEBUG").ok().and_then(|s| s.parse().ok()))
            .unwrap_or(false);

        let max_breadcrumbs = config
            .and_then(|c| c.max_breadcrumbs)
            .or_else(|| {
                env::var("SENTRY_MAX_BREADCRUMBS")
                    .ok()
                    .and_then(|s| s.parse().ok())
            })
            .unwrap_or(100);

        let attach_stacktrace = config.and_then(|c| c.attach_stacktrace).unwrap_or(true);

        // Get release from config, environment, or Cargo package version
        let release = config
            .and_then(|c| c.release.clone())
            .or_else(|| env::var("SENTRY_RELEASE").ok())
            .unwrap_or_else(|| format!("loka-stratum@{}", env!("CARGO_PKG_VERSION")));

        let client_options = ClientOptions {
            dsn: Some(dsn.parse()?),
            environment: Some(environment.clone().into()),
            sample_rate,
            traces_sample_rate,
            debug,
            max_breadcrumbs,
            attach_stacktrace,
            release: Some(release.clone().into()),
            ..Default::default()
        };

        let guard = sentry::init(client_options);

        // Configure Sentry scope with mining-specific context
        configure_scope(|scope| {
            scope.set_tag("service", "loka-stratum");
            scope.set_tag("component", "mining-proxy");
            scope.set_tag("protocol", "stratum-v1");
            scope.set_tag("version", env!("CARGO_PKG_VERSION"));
        });

        info!(
            "✅ Sentry initialized - Environment: {}, Release: {}",
            environment, release
        );
        Ok(Some(guard))
    } else {
        info!("ℹ️  Sentry DSN not configured, skipping error tracking initialization");
        Ok(None)
    }
}

/// Create Sentry tracing layer for structured logging integration
/// Note: This is a placeholder for future integration - returns None for now
pub fn create_sentry_layer() -> Option<()> {
    // TODO: Implement proper Sentry tracing layer integration
    // This requires complex type system work that will be addressed in subtask 10.4
    None
}

/// Create Sentry tower layer for HTTP request tracing  
/// Note: This is a placeholder for future integration - returns None for now
pub fn create_sentry_tower_layer() -> Option<()> {
    // TODO: Implement proper Sentry tower layer integration
    // This requires complex type system work that will be addressed in subtask 10.4
    None
}

/// Add mining-specific context to current Sentry scope
pub fn add_mining_context(
    pool_id: Option<&str>,
    worker_id: Option<&str>,
    connection_id: Option<&str>,
) {
    configure_scope(|scope| {
        if let Some(pool_id) = pool_id {
            scope.set_tag("pool_id", pool_id);
        }
        if let Some(worker_id) = worker_id {
            scope.set_tag("worker_id", worker_id);
        }
        if let Some(connection_id) = connection_id {
            scope.set_tag("connection_id", connection_id);
        }
    });
}

/// Add performance transaction for mining operations
pub fn start_mining_transaction(name: &str, op: &str) -> sentry::TransactionOrSpan {
    let ctx = sentry::TransactionContext::new(name, op);
    sentry::TransactionOrSpan::Transaction(sentry::start_transaction(ctx))
}

/// Helper macro for instrumenting functions with Sentry transactions
#[macro_export]
macro_rules! sentry_transaction {
    ($name:expr, $op:expr, $block:block) => {{
        let _transaction = $crate::sentry::start_mining_transaction($name, $op);
        $block
    }};
}

/// Helper function to capture mining-related errors
pub fn capture_mining_error(error: &anyhow::Error, operation: &str, pool_id: Option<&str>) {
    configure_scope(|scope| {
        scope.set_tag("operation", operation);
        if let Some(pool_id) = pool_id {
            scope.set_tag("pool_id", pool_id);
        }
        scope.set_level(Some(sentry::Level::Error));
    });

    // Capture anyhow error by converting to message
    let error_msg = format!("{error:#}");
    sentry::capture_message(&error_msg, sentry::Level::Error);
}

/// Helper function to add breadcrumbs for mining operations
pub fn add_mining_breadcrumb(message: &str, category: &str) {
    sentry::add_breadcrumb(sentry::Breadcrumb {
        message: Some(message.to_string()),
        category: Some(category.to_string()),
        level: sentry::Level::Info,
        ..Default::default()
    });
}
