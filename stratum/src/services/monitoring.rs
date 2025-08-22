use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use tokio::sync::{RwLock, broadcast};
use tokio::time::interval;
use tracing::{error, info, warn};

use crate::error::{Result, StratumError};
use crate::services::metrics::AtomicMetrics;
use crate::services::performance::SystemMetrics;
use crate::services::performance::{PerformanceProfiler, ResourceMonitor};

/// Performance monitoring and alerting service
pub struct MonitoringService {
    /// Metrics collectors
    metrics: Arc<AtomicMetrics>,
    resource_monitor: Arc<ResourceMonitor>,
    performance_profiler: Arc<PerformanceProfiler>,

    /// Alert configuration
    #[allow(dead_code)]
    alert_config: AlertConfig,
    alert_sender: broadcast::Sender<Alert>,

    /// Monitoring state
    is_running: Arc<AtomicBool>,
    monitoring_tasks: Arc<RwLock<Vec<tokio::task::JoinHandle<()>>>>,

    /// Thresholds and rules
    alert_rules: Arc<RwLock<Vec<AlertRule>>>,
    threshold_breaches: Arc<DashMap<String, ThresholdBreach>>,
}

impl MonitoringService {
    /// Create a new monitoring service
    pub fn new(
        metrics: Arc<AtomicMetrics>,
        resource_monitor: Arc<ResourceMonitor>,
        performance_profiler: Arc<PerformanceProfiler>,
    ) -> Self {
        let (alert_sender, _) = broadcast::channel(1000);

        Self {
            metrics,
            resource_monitor,
            performance_profiler,
            alert_config: AlertConfig::default(),
            alert_sender,
            is_running: Arc::new(AtomicBool::new(false)),
            monitoring_tasks: Arc::new(RwLock::new(Vec::new())),
            alert_rules: Arc::new(RwLock::new(Vec::new())),
            threshold_breaches: Arc::new(DashMap::new()),
        }
    }

    /// Start the monitoring service
    pub async fn start(&self) -> Result<()> {
        if self.is_running.swap(true, Ordering::SeqCst) {
            return Err(StratumError::Internal {
                message: "Monitoring service is already running".to_string(),
            });
        }

        // Initialize default alert rules
        self.initialize_default_rules().await;

        // Start monitoring tasks
        let mut tasks = self.monitoring_tasks.write().await;

        // Resource monitoring task
        tasks.push(self.spawn_resource_monitoring());

        // Performance monitoring task
        tasks.push(self.spawn_performance_monitoring());

        // Alert processing task
        tasks.push(self.spawn_alert_processing());

        // Health check task
        tasks.push(self.spawn_health_check());

        info!(
            "Performance monitoring service started with {} alert rules",
            self.alert_rules.read().await.len()
        );
        Ok(())
    }

    /// Stop the monitoring service
    pub async fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);

        // Cancel all monitoring tasks
        let mut tasks = self.monitoring_tasks.write().await;
        for task in tasks.drain(..) {
            task.abort();
        }

        info!("Performance monitoring service stopped");
    }

    /// Subscribe to alerts
    pub fn subscribe_to_alerts(&self) -> broadcast::Receiver<Alert> {
        self.alert_sender.subscribe()
    }

    /// Add a custom alert rule
    pub async fn add_alert_rule(&self, rule: AlertRule) {
        let mut rules = self.alert_rules.write().await;
        rules.push(rule);
        info!("Added alert rule: {}", rules.last().unwrap().name);
    }

    /// Get current monitoring status
    pub async fn get_status(&self) -> MonitoringStatus {
        let active_alerts = self.get_active_alerts().await;
        let rules_count = self.alert_rules.read().await.len();
        let breaches_count = self.threshold_breaches.len();

        MonitoringStatus {
            is_running: self.is_running.load(Ordering::SeqCst),
            active_alerts: active_alerts.len(),
            total_rules: rules_count,
            threshold_breaches: breaches_count,
            uptime: Duration::from_secs(0), // Implement proper uptime tracking
        }
    }

    /// Get performance report
    pub async fn get_performance_report(&self) -> PerformanceReport {
        let system_metrics = self
            .resource_monitor
            .update_metrics()
            .await
            .unwrap_or_else(|_| SystemMetrics {
                cpu_usage: 0.0,
                memory_used: 0,
                memory_total: 0,
                memory_usage_percent: 0.0,
                disk_usage: Vec::new(),
                network_usage: Vec::new(),
                timestamp: std::time::SystemTime::now(),
            });

        let app_metrics = self.metrics.snapshot();
        let profiling_stats = self.performance_profiler.get_all_stats().await;

        PerformanceReport {
            system_metrics,
            application_metrics: app_metrics,
            profiling_stats,
            timestamp: Instant::now(),
        }
    }

    /// Initialize default monitoring rules
    async fn initialize_default_rules(&self) {
        let default_rules = vec![
            // CPU usage alert
            AlertRule {
                id: "cpu_high".to_string(),
                name: "High CPU Usage".to_string(),
                description: "CPU usage is above threshold".to_string(),
                condition: AlertCondition::CpuUsage { threshold: 80.0 },
                severity: AlertSeverity::Warning,
                cooldown: Duration::from_secs(300),
                enabled: true,
            },
            // Memory usage alert
            AlertRule {
                id: "memory_high".to_string(),
                name: "High Memory Usage".to_string(),
                description: "Memory usage is above threshold".to_string(),
                condition: AlertCondition::MemoryUsage { threshold: 85.0 },
                severity: AlertSeverity::Warning,
                cooldown: Duration::from_secs(300),
                enabled: true,
            },
            // Connection rate alert
            AlertRule {
                id: "connection_rate_high".to_string(),
                name: "High Connection Rate".to_string(),
                description: "Connection rate is above normal".to_string(),
                condition: AlertCondition::ConnectionRate { threshold: 1000 },
                severity: AlertSeverity::Info,
                cooldown: Duration::from_secs(120),
                enabled: true,
            },
            // Error rate alert
            AlertRule {
                id: "error_rate_high".to_string(),
                name: "High Error Rate".to_string(),
                description: "Error rate is above acceptable threshold".to_string(),
                condition: AlertCondition::ErrorRate { threshold: 5.0 },
                severity: AlertSeverity::Critical,
                cooldown: Duration::from_secs(60),
                enabled: true,
            },
        ];

        let mut rules = self.alert_rules.write().await;
        rules.extend(default_rules);
    }

    /// Spawn resource monitoring task
    fn spawn_resource_monitoring(&self) -> tokio::task::JoinHandle<()> {
        let is_running = Arc::clone(&self.is_running);
        let resource_monitor = Arc::clone(&self.resource_monitor);
        let alert_rules = Arc::clone(&self.alert_rules);
        let alert_sender = self.alert_sender.clone();
        let threshold_breaches = Arc::clone(&self.threshold_breaches);

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30));

            while is_running.load(Ordering::SeqCst) {
                interval.tick().await;

                if let Ok(system_metrics) = resource_monitor.update_metrics().await {
                    let rules = alert_rules.read().await;

                    for rule in rules.iter() {
                        if !rule.enabled {
                            continue;
                        }

                        if let Some(alert) =
                            Self::check_system_alert_condition(rule, &system_metrics).await
                        {
                            if Self::should_fire_alert(rule, &threshold_breaches).await {
                                let _ = alert_sender.send(alert);
                                Self::record_threshold_breach(rule, &threshold_breaches).await;
                            }
                        }
                    }
                }
            }
        })
    }

    /// Spawn performance monitoring task
    fn spawn_performance_monitoring(&self) -> tokio::task::JoinHandle<()> {
        let is_running = Arc::clone(&self.is_running);
        let performance_profiler = Arc::clone(&self.performance_profiler);
        let alert_sender = self.alert_sender.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(60));

            while is_running.load(Ordering::SeqCst) {
                interval.tick().await;

                let stats = performance_profiler.get_all_stats().await;

                // Check for slow operations
                for (operation, op_stats) in stats {
                    if op_stats.p99 > Duration::from_millis(1000) {
                        // 1 second threshold
                        let alert = Alert {
                            id: format!("slow_operation_{}", operation),
                            rule_id: "performance_slow_operation".to_string(),
                            title: format!("Slow Operation: {}", operation),
                            description: format!(
                                "Operation '{}' has P99 latency of {:?}",
                                operation, op_stats.p99
                            ),
                            severity: AlertSeverity::Warning,
                            timestamp: Instant::now(),
                            metadata: vec![
                                ("operation".to_string(), operation),
                                ("p99_ms".to_string(), op_stats.p99.as_millis().to_string()),
                                ("avg_ms".to_string(), op_stats.avg.as_millis().to_string()),
                            ]
                            .into_iter()
                            .collect(),
                        };

                        let _ = alert_sender.send(alert);
                    }
                }
            }
        })
    }

    /// Spawn alert processing task
    fn spawn_alert_processing(&self) -> tokio::task::JoinHandle<()> {
        let mut alert_receiver = self.alert_sender.subscribe();
        let is_running = Arc::clone(&self.is_running);

        tokio::spawn(async move {
            while is_running.load(Ordering::SeqCst) {
                match alert_receiver.recv().await {
                    Ok(alert) => {
                        Self::process_alert(alert).await;
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        warn!("Alert processing lagged, some alerts may have been missed");
                    }
                }
            }
        })
    }

    /// Spawn health check task
    fn spawn_health_check(&self) -> tokio::task::JoinHandle<()> {
        let is_running = Arc::clone(&self.is_running);
        let metrics = Arc::clone(&self.metrics);
        let alert_sender = self.alert_sender.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(120)); // Every 2 minutes

            while is_running.load(Ordering::SeqCst) {
                interval.tick().await;

                let health_status = Self::check_application_health(&metrics).await;

                if !health_status.is_healthy {
                    let alert = Alert {
                        id: "health_check_failed".to_string(),
                        rule_id: "health_check".to_string(),
                        title: "Application Health Check Failed".to_string(),
                        description: health_status.issues.join("; "),
                        severity: AlertSeverity::Critical,
                        timestamp: Instant::now(),
                        metadata: HashMap::new(),
                    };

                    let _ = alert_sender.send(alert);
                }
            }
        })
    }

    /// Check if alert should be fired (respects cooldown)
    async fn should_fire_alert(
        rule: &AlertRule,
        threshold_breaches: &Arc<DashMap<String, ThresholdBreach>>,
    ) -> bool {
        if let Some(last_breach) = threshold_breaches.get(&rule.id) {
            last_breach.timestamp.elapsed() >= rule.cooldown
        } else {
            true
        }
    }

    /// Record threshold breach
    async fn record_threshold_breach(
        rule: &AlertRule,
        threshold_breaches: &Arc<DashMap<String, ThresholdBreach>>,
    ) {
        threshold_breaches.insert(
            rule.id.clone(),
            ThresholdBreach {
                rule_id: rule.id.clone(),
                timestamp: Instant::now(),
            },
        );
    }

    /// Check system alert conditions
    async fn check_system_alert_condition(
        rule: &AlertRule,
        system_metrics: &SystemMetrics,
    ) -> Option<Alert> {
        match &rule.condition {
            AlertCondition::CpuUsage { threshold } => {
                if system_metrics.cpu_usage > *threshold {
                    Some(Alert {
                        id: format!("{}_{}", rule.id, Instant::now().elapsed().as_secs()),
                        rule_id: rule.id.clone(),
                        title: rule.name.clone(),
                        description: format!(
                            "CPU usage is {:.1}% (threshold: {:.1}%)",
                            system_metrics.cpu_usage, threshold
                        ),
                        severity: rule.severity.clone(),
                        timestamp: Instant::now(),
                        metadata: [
                            (
                                "cpu_usage".to_string(),
                                system_metrics.cpu_usage.to_string(),
                            ),
                            ("threshold".to_string(), threshold.to_string()),
                        ]
                        .iter()
                        .cloned()
                        .collect(),
                    })
                } else {
                    None
                }
            }

            AlertCondition::MemoryUsage { threshold } => {
                if system_metrics.memory_usage_percent > *threshold {
                    Some(Alert {
                        id: format!("{}_{}", rule.id, Instant::now().elapsed().as_secs()),
                        rule_id: rule.id.clone(),
                        title: rule.name.clone(),
                        description: format!(
                            "Memory usage is {:.1}% (threshold: {:.1}%)",
                            system_metrics.memory_usage_percent, threshold
                        ),
                        severity: rule.severity.clone(),
                        timestamp: Instant::now(),
                        metadata: [
                            (
                                "memory_usage_percent".to_string(),
                                system_metrics.memory_usage_percent.to_string(),
                            ),
                            ("threshold".to_string(), threshold.to_string()),
                        ]
                        .iter()
                        .cloned()
                        .collect(),
                    })
                } else {
                    None
                }
            }

            _ => None, // Other conditions handled elsewhere
        }
    }

    /// Process an alert
    async fn process_alert(alert: Alert) {
        match alert.severity {
            AlertSeverity::Critical => {
                error!("CRITICAL ALERT: {} - {}", alert.title, alert.description);
            }
            AlertSeverity::Warning => {
                warn!("WARNING: {} - {}", alert.title, alert.description);
            }
            AlertSeverity::Info => {
                info!("INFO: {} - {}", alert.title, alert.description);
            }
        }

        // In production, you might:
        // - Send notifications via email/Slack/PagerDuty
        // - Store alerts in database
        // - Update monitoring dashboards
        // - Trigger automated remediation
    }

    /// Check application health
    async fn check_application_health(metrics: &AtomicMetrics) -> HealthStatus {
        let mut issues = Vec::new();
        let snapshot = metrics.snapshot();

        // Check for error rates
        let total_messages = snapshot.messages_received;
        let total_errors =
            snapshot.protocol_errors + snapshot.connection_errors + snapshot.auth_failures;

        if total_messages > 0 {
            let error_rate = (total_errors as f64 / total_messages as f64) * 100.0;
            if error_rate > 10.0 {
                issues.push(format!("High error rate: {:.1}%", error_rate));
            }
        }

        // Check for stalled connections
        if snapshot.active_connections == 0 && snapshot.total_connections > 0 {
            issues.push("No active connections despite having total connections".to_string());
        }

        // Check for memory leaks (simplified)
        if snapshot.active_connections > 10000 {
            issues.push("Unusually high number of active connections".to_string());
        }

        HealthStatus {
            is_healthy: issues.is_empty(),
            issues,
            checked_at: Instant::now(),
        }
    }

    /// Get active alerts
    async fn get_active_alerts(&self) -> Vec<Alert> {
        // In a real implementation, you'd maintain a list of active alerts
        // For now, return empty list
        Vec::new()
    }
}

// Data structures

#[derive(Debug, Clone)]
pub struct AlertConfig {
    pub max_alerts_per_minute: u32,
    pub alert_retention_hours: u32,
    pub enable_email_notifications: bool,
    pub enable_slack_notifications: bool,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            max_alerts_per_minute: 60,
            alert_retention_hours: 24,
            enable_email_notifications: false,
            enable_slack_notifications: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AlertRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub condition: AlertCondition,
    pub severity: AlertSeverity,
    pub cooldown: Duration,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub enum AlertCondition {
    CpuUsage { threshold: f32 },
    MemoryUsage { threshold: f64 },
    ConnectionRate { threshold: u64 },
    ErrorRate { threshold: f64 },
    ResponseTime { threshold: Duration },
    Custom { expression: String },
}

#[derive(Debug, Clone)]
pub enum AlertSeverity {
    Critical,
    Warning,
    Info,
}

#[derive(Debug, Clone)]
pub struct Alert {
    pub id: String,
    pub rule_id: String,
    pub title: String,
    pub description: String,
    pub severity: AlertSeverity,
    pub timestamp: Instant,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug)]
struct ThresholdBreach {
    #[allow(dead_code)]
    rule_id: String,
    timestamp: Instant,
}

#[derive(Debug, Clone)]
pub struct MonitoringStatus {
    pub is_running: bool,
    pub active_alerts: usize,
    pub total_rules: usize,
    pub threshold_breaches: usize,
    pub uptime: Duration,
}

#[derive(Debug, Clone)]
pub struct PerformanceReport {
    pub system_metrics: SystemMetrics,
    pub application_metrics: crate::services::metrics::MetricsSnapshot,
    pub profiling_stats: HashMap<String, crate::services::performance::OperationStats>,
    pub timestamp: Instant,
}

#[derive(Debug)]
struct HealthStatus {
    is_healthy: bool,
    issues: Vec<String>,
    #[allow(dead_code)]
    checked_at: Instant,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::metrics::AtomicMetrics;
    use crate::services::performance::{PerformanceProfiler, ResourceMonitor};

    #[tokio::test]
    async fn test_monitoring_service_creation() {
        let metrics = Arc::new(AtomicMetrics::new());
        let resource_monitor = Arc::new(ResourceMonitor::new(100));
        let performance_profiler = Arc::new(PerformanceProfiler::new(1000));

        let monitoring = MonitoringService::new(metrics, resource_monitor, performance_profiler);

        let status = monitoring.get_status().await;
        assert!(!status.is_running);
    }

    #[test]
    fn test_alert_rule_creation() {
        let rule = AlertRule {
            id: "test_rule".to_string(),
            name: "Test Rule".to_string(),
            description: "A test alert rule".to_string(),
            condition: AlertCondition::CpuUsage { threshold: 80.0 },
            severity: AlertSeverity::Warning,
            cooldown: Duration::from_secs(300),
            enabled: true,
        };

        assert_eq!(rule.id, "test_rule");
        assert!(rule.enabled);
    }
}
