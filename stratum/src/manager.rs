use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use tokio::task::JoinHandle;

use crate::Config;
use crate::services::database::DatabaseService;
use crate::services::metrics::{AtomicMetrics, MetricsConfig, MetricsService};
use crate::{auth, job, submission};

/// Central coordinator for all stratum proxy subsystems.
///
/// The `Manager` serves as the main orchestrator, providing unified access to:
/// - Authentication and authorization services
/// - Mining job distribution and management
/// - Share submission tracking and validation
/// - Database operations and persistence
/// - Metrics collection and monitoring
/// - Background task coordination
///
/// # Architecture
///
/// The manager follows a composition pattern, where each major subsystem
/// is represented by a specialized manager with its own responsibilities:
///
/// ```text
/// ┌─────────────────────────────────────────────┐
/// │                 Manager                     │
/// ├─────────────┬─────────────┬─────────────────┤
/// │ AuthManager │ JobManager  │ SubmissionMgr   │
/// ├─────────────┼─────────────┼─────────────────┤
/// │ Database    │ Metrics     │ Background Tasks│
/// └─────────────┴─────────────┴─────────────────┘
/// ```
///
/// # Examples
///
/// ```rust
/// use loka_stratum::{Config, Manager};
/// use loka_stratum::services::database::DatabaseService;
/// use std::sync::Arc;
///
/// # async fn example() -> loka_stratum::Result<()> {
/// let config = Arc::new(Config::default());
/// let database = Arc::new(DatabaseService::new("sqlite::memory:").await?);
/// let manager = Manager::new(config, database)?;
///
/// // Access subsystems through the manager
/// let auth = manager.auth();
/// let jobs = manager.jobs();
/// let submissions = manager.submissions();
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct Manager {
    /// Proxy configuration settings
    config: Arc<Config>,
    /// Authentication and session management
    auth: Arc<auth::Manager>,
    /// Mining job distribution and tracking
    jobs: Arc<job::Manager>,
    /// Share submission validation and recording
    submissions: Arc<submission::Manager>,
    /// Database service for persistence
    database: Arc<DatabaseService>,
    /// Metrics collection and monitoring
    metrics: Arc<MetricsService>,
    /// Background task handles for cleanup
    tasks: Vec<Arc<JoinHandle<()>>>,
    /// Connection lifecycle tracking for metrics
    connection_starts: Arc<dashmap::DashMap<SocketAddr, Instant>>,
}

impl Manager {
    /// Creates a new Manager instance with all subsystems initialized.
    ///
    /// This constructor sets up all the necessary components:
    /// - Initializes authentication manager for miner sessions
    /// - Creates job manager for mining work distribution
    /// - Sets up submission manager for share tracking
    /// - Configures metrics service with database integration
    /// - Prepares connection tracking for lifecycle metrics
    ///
    /// # Arguments
    /// * `config` - Shared configuration for all subsystems
    /// * `database` - Database service for persistence operations
    ///
    /// # Returns
    /// * `Ok(Manager)` - Successfully initialized manager
    /// * `Err(StratumError)` - Configuration or initialization error
    ///
    /// # Examples
    ///
    /// ```rust
    /// use loka_stratum::{Config, Manager};
    /// use loka_stratum::services::database::DatabaseService;
    /// use std::sync::Arc;
    ///
    /// # async fn create_manager() -> loka_stratum::Result<()> {
    /// let config = Arc::new(Config::load().await?);
    /// let database = Arc::new(DatabaseService::new("sqlite::memory:").await?);
    ///
    /// let manager = Manager::new(config, database)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(
        config: Arc<Config>,
        database: Arc<DatabaseService>,
    ) -> Result<Self, crate::error::StratumError> {
        // Create metrics service with database integration
        let metrics_config = MetricsConfig::default();
        let atomic_metrics = Arc::new(AtomicMetrics::new());
        let metrics = Arc::new(
            MetricsService::with_config(atomic_metrics, metrics_config)
                .with_database_service(database.clone())
        );

        Ok(Self {
            auth: Arc::new(auth::Manager::default()),
            jobs: Arc::new(job::Manager::with_metrics(&config, metrics.clone())),
            submissions: Arc::new(submission::Manager::with_metrics(
                database.clone(),
                config.pool.id,
                metrics.clone(),
            )),
            config,
            database,
            metrics,
            tasks: Vec::new(),
            connection_starts: Arc::new(dashmap::DashMap::new()),
        })
    }

    /// Returns a reference to the proxy configuration.
    ///
    /// Provides access to all proxy settings including server binding,
    /// pool configuration, rate limiting, and feature flags.
    pub fn config(&self) -> &Arc<Config> {
        &self.config
    }

    /// Returns a reference to the authentication manager.
    ///
    /// Used for managing miner sessions, authentication state,
    /// and user/worker credential validation.
    pub fn auth(&self) -> &Arc<auth::Manager> {
        &self.auth
    }

    /// Returns a reference to the job manager.
    ///
    /// Handles mining job distribution, job lifecycle management,
    /// and work assignment to authenticated miners.
    pub fn jobs(&self) -> &Arc<job::Manager> {
        &self.jobs
    }

    /// Returns a reference to the submission manager.
    ///
    /// Manages share submissions, validation, acceptance tracking,
    /// and integration with the mining pool backend.
    pub fn submissions(&self) -> &Arc<submission::Manager> {
        &self.submissions
    }

    /// Returns a reference to the database service.
    ///
    /// Provides persistent storage for miners, pools, submissions,
    /// earnings, and other operational data.
    pub fn database(&self) -> &Arc<DatabaseService> {
        &self.database
    }

    /// Returns a reference to the metrics service.
    ///
    /// Enables high-performance metrics collection, monitoring,
    /// and integration with Prometheus/Grafana stacks.
    pub fn metrics(&self) -> &Arc<MetricsService> {
        &self.metrics
    }

    /// Get the pool ID from config
    pub fn get_pool_id(&self) -> uuid::Uuid {
        self.config.pool.id
    }

    /// Connection lifecycle tracking methods (Task 8.1)
    pub fn connection_established(&self, addr: &SocketAddr) {
        let now = Instant::now();
        self.connection_starts.insert(*addr, now);
        self.metrics.record_connection_established_event(0.0); // 0.0 indicates we don't have timing yet
        tracing::debug!("Connection established tracked for {}", addr);
    }

    pub fn connection_reconnect_attempt(&self, addr: &SocketAddr) {
        self.metrics.record_reconnection_attempt_event();
        tracing::debug!("Reconnection attempt tracked for {}", addr);
    }

    pub fn terminated(&self, addr: &SocketAddr) {
        // Record connection closure with duration if we have the start time
        if let Some((_, start_time)) = self.connection_starts.remove(addr) {
            let duration = start_time.elapsed();
            let duration_ms = duration.as_secs_f64() * 1000.0;
            self.metrics.record_connection_closed_event(duration_ms);
            tracing::debug!("Connection {} closed after {:.2}ms", addr, duration_ms);
        }

        self.auth.terminated(addr);
    }

    pub fn update_connection_idle_time(&self, addr: &SocketAddr, idle_time_ms: f64) {
        self.metrics.update_idle_time(idle_time_ms);
        tracing::trace!("Idle time updated for {}: {:.2}ms", addr, idle_time_ms);
    }

    /// Protocol detection tracking methods (Task 8.2)
    pub fn protocol_http_request_detected(&self, addr: &SocketAddr) {
        self.metrics.record_http_request_event();
        tracing::debug!("HTTP request detected for {}", addr);
    }

    pub fn protocol_stratum_request_detected(&self, addr: &SocketAddr) {
        self.metrics.record_stratum_request_event();
        tracing::debug!("Stratum request detected for {}", addr);
    }

    pub fn protocol_detection_success(&self, addr: &SocketAddr, protocol: &str) {
        self.metrics.record_protocol_detection_success_event();
        tracing::debug!("Protocol {} detection success for {}", protocol, addr);
    }

    pub fn protocol_detection_failure(&self, addr: &SocketAddr) {
        self.metrics.record_protocol_detection_failure_event();
        tracing::warn!("Protocol detection failure for {}", addr);
    }

    pub fn http_connect_request(&self, addr: &SocketAddr, path: Option<&str>) {
        self.metrics.record_http_connect_request_event();
        if let Some(path) = path {
            tracing::info!("HTTP CONNECT request with path '{}' for {}", path, addr);
        } else {
            tracing::info!("HTTP CONNECT request for {}", addr);
        }
    }

    pub fn direct_stratum_connection(&self, addr: &SocketAddr) {
        self.metrics.record_direct_stratum_connection_event();
        tracing::debug!("Direct Stratum connection for {}", addr);
    }

    pub fn protocol_conversion_success(&self, addr: &SocketAddr, success_rate: f64) {
        self.metrics
            .record_protocol_conversion_success_event(success_rate);
        tracing::debug!(
            "Protocol conversion success for {} with rate {:.2}%",
            addr,
            success_rate * 100.0
        );
    }

    pub fn protocol_conversion_error(&self, addr: &SocketAddr, error: &str) {
        self.metrics.record_protocol_conversion_error_event();
        tracing::warn!("Protocol conversion error for {}: {}", addr, error);
    }
}

impl Drop for Manager {
    fn drop(&mut self) {
        for task in self.tasks.drain(..) {
            task.abort();
        }
    }
}
