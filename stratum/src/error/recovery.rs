use async_trait::async_trait;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicU8;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use super::handlers::{ErrorContext, ErrorHandler};
use super::types::{ErrorAction, Result, StratumError};

/// Recovery policy configuration
#[derive(Debug, Clone)]
pub struct RecoveryPolicy {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Base delay for exponential backoff
    pub base_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Backoff multiplier (default 2.0 for exponential backoff)
    pub backoff_multiplier: f64,
    /// Maximum time to spend on retries before giving up
    pub max_total_time: Duration,
    /// Jitter factor to prevent thundering herd (0.0-1.0)
    pub jitter_factor: f64,
}

impl RecoveryPolicy {
    pub fn new() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            max_total_time: Duration::from_secs(300), // 5 minutes
            jitter_factor: 0.1,
        }
    }

    pub fn aggressive() -> Self {
        Self {
            max_retries: 5,
            base_delay: Duration::from_millis(50),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 1.5,
            max_total_time: Duration::from_secs(60),
            jitter_factor: 0.2,
        }
    }

    pub fn conservative() -> Self {
        Self {
            max_retries: 2,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            backoff_multiplier: 3.0,
            max_total_time: Duration::from_secs(600),
            jitter_factor: 0.05,
        }
    }

    pub fn immediate() -> Self {
        Self {
            max_retries: 1,
            base_delay: Duration::from_millis(10),
            max_delay: Duration::from_millis(100),
            backoff_multiplier: 1.0,
            max_total_time: Duration::from_secs(1),
            jitter_factor: 0.0,
        }
    }

    /// Calculate delay for the given attempt number
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        let base_delay_ms = self.base_delay.as_millis() as f64;
        let multiplier = self
            .backoff_multiplier
            .powi(attempt.saturating_sub(1) as i32);
        let delay_ms = base_delay_ms * multiplier;

        // Apply jitter using a simple random approach
        let jitter = if self.jitter_factor > 0.0 {
            // Simple random using system time for jitter (not cryptographically secure but sufficient)
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_nanos();
            let random_factor = ((nanos % 1000) as f64 / 500.0) - 1.0; // -1.0 to 1.0
            delay_ms * (1.0 + self.jitter_factor * random_factor)
        } else {
            delay_ms
        };

        let delay = Duration::from_millis(jitter.max(0.0) as u64);
        std::cmp::min(delay, self.max_delay)
    }
}

impl Default for RecoveryPolicy {
    fn default() -> Self {
        Self::new()
    }
}

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CircuitState {
    Closed = 0,   // Normal operation
    Open = 1,     // Failing fast
    HalfOpen = 2, // Testing if service recovered
}

/// Circuit breaker for preventing cascade failures
#[derive(Debug)]
pub struct CircuitBreaker {
    state: AtomicU8,
    failure_count: AtomicU32,
    success_count: AtomicU32,
    last_failure: AtomicU64,
    config: CircuitBreakerConfig,
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening circuit
    pub failure_threshold: u32,
    /// Time to wait before moving to half-open
    pub recovery_timeout: Duration,
    /// Number of successful requests to close circuit from half-open
    pub success_threshold: u32,
    /// Reset failure count after this time of successful operation
    pub reset_timeout: Duration,
}

impl CircuitBreakerConfig {
    pub fn new() -> Self {
        Self {
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(60),
            success_threshold: 3,
            reset_timeout: Duration::from_secs(300),
        }
    }
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: AtomicU8::new(CircuitState::Closed as u8),
            failure_count: AtomicU32::new(0),
            success_count: AtomicU32::new(0),
            last_failure: AtomicU64::new(0),
            config,
        }
    }

    /// Check if the circuit allows the request
    pub fn can_execute(&self) -> Result<()> {
        let state = match self.state.load(Ordering::Relaxed) {
            0 => CircuitState::Closed,
            1 => CircuitState::Open,
            2 => CircuitState::HalfOpen,
            _ => unreachable!(),
        };

        let last_failure = self.last_failure.load(Ordering::Relaxed);

        match state {
            CircuitState::Closed => Ok(()),
            CircuitState::Open => {
                if last_failure > 0 {
                    let last_failure_time = UNIX_EPOCH + Duration::from_nanos(last_failure);
                    let now = SystemTime::now();
                    if now.duration_since(last_failure_time).unwrap_or_default()
                        >= self.config.recovery_timeout
                    {
                        self.state
                            .store(CircuitState::HalfOpen as u8, Ordering::Relaxed);
                        self.success_count.store(0, Ordering::Relaxed); // Reset success count when transitioning to half-open
                        debug!("Circuit breaker moving to half-open state");
                        Ok(())
                    } else {
                        Err(StratumError::ResourceUnavailable {
                            resource: "Circuit breaker is open".to_string(),
                        })
                    }
                } else {
                    Ok(()) // Should not happen, but allow execution
                }
            }
            CircuitState::HalfOpen => Ok(()),
        }
    }

    /// Record a successful execution
    pub fn record_success(&self) {
        let state = match self.state.load(Ordering::Relaxed) {
            0 => CircuitState::Closed,
            1 => CircuitState::Open,
            2 => CircuitState::HalfOpen,
            _ => unreachable!(),
        };

        let last_failure = self.last_failure.load(Ordering::Relaxed);

        match state {
            CircuitState::HalfOpen => {
                let new_success_count = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;
                if new_success_count >= self.config.success_threshold {
                    self.state
                        .store(CircuitState::Closed as u8, Ordering::Relaxed);
                    self.failure_count.store(0, Ordering::Relaxed);
                    self.success_count.store(0, Ordering::Relaxed);
                    debug!("Circuit breaker closed after successful recovery");
                }
            }
            CircuitState::Closed => {
                // Reset failure count periodically on success
                if last_failure > 0 {
                    let last_failure_time = UNIX_EPOCH + Duration::from_nanos(last_failure);
                    let now = SystemTime::now();
                    if now.duration_since(last_failure_time).unwrap_or_default()
                        >= self.config.reset_timeout
                    {
                        self.failure_count.store(0, Ordering::Relaxed);
                    }
                }
            }
            _ => {}
        }
    }

    /// Record a failed execution
    pub fn record_failure(&self) {
        let state = match self.state.load(Ordering::Relaxed) {
            0 => CircuitState::Closed,
            1 => CircuitState::Open,
            2 => CircuitState::HalfOpen,
            _ => unreachable!(),
        };

        let _last_failure = self.last_failure.load(Ordering::Relaxed);

        // Increment failure count first
        let new_failure_count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        self.last_failure.store(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64,
            Ordering::Relaxed,
        );

        match state {
            CircuitState::Closed => {
                if new_failure_count >= self.config.failure_threshold {
                    self.state
                        .store(CircuitState::Open as u8, Ordering::Relaxed);
                    self.success_count.store(0, Ordering::Relaxed); // Reset success count when opening
                    warn!(
                        "Circuit breaker opened after {} failures",
                        new_failure_count
                    );
                }
            }
            CircuitState::HalfOpen => {
                self.state
                    .store(CircuitState::Open as u8, Ordering::Relaxed);
                self.success_count.store(0, Ordering::Relaxed); // Reset success count when reopening
                debug!("Circuit breaker reopened after failure in half-open state");
            }
            _ => {}
        }
    }

    pub fn state(&self) -> CircuitState {
        match self.state.load(Ordering::Relaxed) {
            0 => CircuitState::Closed,
            1 => CircuitState::Open,
            2 => CircuitState::HalfOpen,
            _ => unreachable!(),
        }
    }

    pub fn failure_count(&self) -> u32 {
        self.failure_count.load(Ordering::Relaxed)
    }
}

/// Retry statistics for monitoring with atomic operations
#[derive(Debug)]
pub struct RetryStatistics {
    pub total_attempts: AtomicU64,
    pub successful_retries: AtomicU64,
    pub failed_retries: AtomicU64,
    pub total_retry_time_nanos: AtomicU64,
    pub circuit_breaker_activations: AtomicU64,
}

impl Default for RetryStatistics {
    fn default() -> Self {
        Self {
            total_attempts: AtomicU64::new(0),
            successful_retries: AtomicU64::new(0),
            failed_retries: AtomicU64::new(0),
            total_retry_time_nanos: AtomicU64::new(0),
            circuit_breaker_activations: AtomicU64::new(0),
        }
    }
}

impl RetryStatistics {
    pub fn snapshot(&self) -> RetryStatisticsSnapshot {
        RetryStatisticsSnapshot {
            total_attempts: self.total_attempts.load(Ordering::Relaxed),
            successful_retries: self.successful_retries.load(Ordering::Relaxed),
            failed_retries: self.failed_retries.load(Ordering::Relaxed),
            total_retry_time: Duration::from_nanos(
                self.total_retry_time_nanos.load(Ordering::Relaxed),
            ),
            circuit_breaker_activations: self.circuit_breaker_activations.load(Ordering::Relaxed),
        }
    }

    pub fn reset(&self) {
        self.total_attempts.store(0, Ordering::Relaxed);
        self.successful_retries.store(0, Ordering::Relaxed);
        self.failed_retries.store(0, Ordering::Relaxed);
        self.total_retry_time_nanos.store(0, Ordering::Relaxed);
        self.circuit_breaker_activations.store(0, Ordering::Relaxed);
    }
}

/// Snapshot of retry statistics for cloning and reporting
#[derive(Debug, Clone)]
pub struct RetryStatisticsSnapshot {
    pub total_attempts: u64,
    pub successful_retries: u64,
    pub failed_retries: u64,
    pub total_retry_time: Duration,
    pub circuit_breaker_activations: u64,
}

/// Advanced recovery manager with multiple strategies
pub struct RecoveryManager {
    error_handler: Arc<dyn ErrorHandler>,
    circuit_breakers: DashMap<String, Arc<CircuitBreaker>>,
    retry_policies: HashMap<String, RecoveryPolicy>,
    statistics: RetryStatistics,
}

impl RecoveryManager {
    pub fn new(error_handler: Arc<dyn ErrorHandler>) -> Self {
        Self {
            error_handler,
            circuit_breakers: DashMap::new(),
            retry_policies: Self::default_policies(),
            statistics: RetryStatistics::default(),
        }
    }

    fn default_policies() -> HashMap<String, RecoveryPolicy> {
        let mut policies = HashMap::new();
        policies.insert("network".to_string(), RecoveryPolicy::new());
        policies.insert("database".to_string(), RecoveryPolicy::conservative());
        policies.insert("auth".to_string(), RecoveryPolicy::aggressive());
        policies.insert("critical".to_string(), RecoveryPolicy::immediate());
        policies.insert("default".to_string(), RecoveryPolicy::new());
        policies
    }

    pub fn set_policy(&mut self, category: &str, policy: RecoveryPolicy) {
        self.retry_policies.insert(category.to_string(), policy);
    }

    pub fn get_circuit_breaker(&self, key: &str) -> Arc<CircuitBreaker> {
        self.circuit_breakers
            .entry(key.to_string())
            .or_insert_with(|| Arc::new(CircuitBreaker::new(CircuitBreakerConfig::new())))
            .clone()
    }

    /// Execute operation with full recovery strategy
    pub async fn execute_with_recovery<F, Fut, T>(
        &self,
        operation: F,
        context: ErrorContext,
        category: Option<&str>,
    ) -> Result<T>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<T>> + Send,
        T: Send,
    {
        let policy = self
            .retry_policies
            .get(category.unwrap_or("default"))
            .unwrap_or(&RecoveryPolicy::new())
            .clone();

        let circuit_key = format!("{}:{}", context.component, category.unwrap_or("default"));
        let circuit_breaker = self.get_circuit_breaker(&circuit_key);

        let mut current_context = context;
        let start_time = Instant::now();
        let mut total_delay = Duration::ZERO;

        // Update statistics
        self.statistics
            .total_attempts
            .fetch_add(1, Ordering::Relaxed);

        loop {
            current_context.attempt_count += 1;

            // Check circuit breaker
            match circuit_breaker.can_execute() {
                Err(circuit_error) => {
                    warn!("Circuit breaker preventing execution: {}", circuit_error);

                    // Update statistics
                    self.statistics
                        .circuit_breaker_activations
                        .fetch_add(1, Ordering::Relaxed);

                    return Err(circuit_error);
                }
                Ok(()) => {}
            }

            // Check if we've exceeded total time budget
            if start_time.elapsed() >= policy.max_total_time {
                error!(
                    component = current_context.component,
                    operation = current_context.operation,
                    total_time = ?start_time.elapsed(),
                    max_time = ?policy.max_total_time,
                    "Recovery time budget exceeded"
                );

                circuit_breaker.record_failure();

                return Err(StratumError::TaskTimeout {
                    timeout: policy.max_total_time,
                });
            }

            // Try the operation
            match operation().await {
                Ok(result) => {
                    circuit_breaker.record_success();

                    if current_context.attempt_count > 1 {
                        info!(
                            component = current_context.component,
                            operation = current_context.operation,
                            attempts = current_context.attempt_count,
                            total_time = ?start_time.elapsed(),
                            total_delay = ?total_delay,
                            "Operation succeeded after recovery"
                        );

                        // Update statistics
                        self.statistics
                            .successful_retries
                            .fetch_add(1, Ordering::Relaxed);
                        let elapsed_nanos = start_time.elapsed().as_nanos() as u64;
                        self.statistics
                            .total_retry_time_nanos
                            .fetch_add(elapsed_nanos, Ordering::Relaxed);
                    }

                    return Ok(result);
                }
                Err(error) => {
                    circuit_breaker.record_failure();

                    // Get error handling action
                    let action = self
                        .error_handler
                        .handle_error(&error, &current_context)
                        .await;

                    match action {
                        ErrorAction::Retry { .. } => {
                            if current_context.attempt_count >= policy.max_retries {
                                error!(
                                    error = %error,
                                    attempts = current_context.attempt_count,
                                    max_retries = policy.max_retries,
                                    total_time = ?start_time.elapsed(),
                                    "Max retry attempts exceeded"
                                );

                                // Update statistics
                                self.statistics
                                    .failed_retries
                                    .fetch_add(1, Ordering::Relaxed);

                                return Err(error);
                            }

                            // Calculate delay with jitter
                            let delay = policy.calculate_delay(current_context.attempt_count);
                            total_delay += delay;

                            debug!(
                                attempt = current_context.attempt_count,
                                max_attempts = policy.max_retries,
                                delay = ?delay,
                                error = %error,
                                "Retrying operation after delay"
                            );

                            sleep(delay).await;
                            continue;
                        }

                        ErrorAction::CircuitBreaker { duration, message } => {
                            warn!(
                                duration = ?duration,
                                message = message,
                                "Circuit breaker activated by error handler"
                            );

                            // Force circuit breaker open
                            circuit_breaker.record_failure();

                            sleep(duration).await;
                            return Err(error);
                        }

                        ErrorAction::Fallback { message } => {
                            warn!(
                                error = %error,
                                message = message,
                                "Attempting fallback strategy"
                            );

                            // Try one more time with a fallback approach
                            // This would need to be implemented by the calling code
                            return Err(error);
                        }

                        ErrorAction::Terminate | ErrorAction::Escalate => {
                            debug!(
                                action = ?action,
                                error = %error,
                                "Error handler recommended termination"
                            );
                            return Err(error);
                        }

                        ErrorAction::Ignore => {
                            warn!(
                                error = %error,
                                "Error handler recommended ignoring error"
                            );
                            return Err(error);
                        }
                    }
                }
            }
        }
    }

    /// Get current retry statistics
    pub fn get_statistics(&self) -> RetryStatisticsSnapshot {
        self.statistics.snapshot()
    }

    /// Reset statistics
    pub fn reset_statistics(&self) {
        self.statistics.reset();
    }

    /// Get circuit breaker states for monitoring
    pub fn get_circuit_breaker_states(&self) -> HashMap<String, (CircuitState, u32)> {
        self.circuit_breakers
            .iter()
            .map(|r| {
                (
                    r.key().clone(),
                    (r.value().state(), r.value().failure_count()),
                )
            })
            .collect()
    }
}

/// Trait for components that can be recovered
#[async_trait]
pub trait Recoverable<T>: Send + Sync {
    async fn execute(&self) -> Result<T>;

    fn recovery_category(&self) -> &str {
        "default"
    }

    fn error_context(&self) -> ErrorContext;
}

/// Helper function to execute a recoverable operation
pub async fn execute_recoverable<T: Send>(
    recoverable: Arc<dyn Recoverable<T>>,
    recovery_manager: &RecoveryManager,
) -> Result<T> {
    // Execute with recovery using the recoverable's context and category
    let context = recoverable.error_context();
    let category = Some(recoverable.recovery_category());

    recovery_manager
        .execute_with_recovery(|| recoverable.execute(), context, category)
        .await
}

/// Macro to create recoverable operations
#[macro_export]
macro_rules! recoverable_operation {
    ($name:ident, $category:expr, $component:expr, $operation:expr, $body:block) => {
        struct $name;

        #[async_trait]
        impl Recoverable<()> for $name {
            async fn execute(&self) -> Result<()> $body

            fn recovery_category(&self) -> &str {
                $category
            }

            fn error_context(&self) -> ErrorContext {
                ErrorContext::new($component, $operation)
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::handlers::DefaultErrorHandler;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[tokio::test]
    async fn test_recovery_policy() {
        let policy = RecoveryPolicy::new();

        let delay1 = policy.calculate_delay(1);
        let delay2 = policy.calculate_delay(2);
        let delay3 = policy.calculate_delay(3);

        // Should increase with exponential backoff
        assert!(delay1 <= delay2);
        assert!(delay2 <= delay3);
        assert!(delay3 <= policy.max_delay);
    }

    #[tokio::test]
    async fn test_circuit_breaker() {
        let breaker = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 2,
            recovery_timeout: Duration::from_millis(100),
            success_threshold: 1,
            reset_timeout: Duration::from_secs(1),
        });

        // Should start closed
        assert_eq!(breaker.state(), CircuitState::Closed);
        assert!(breaker.can_execute().is_ok());

        // Record failures to open circuit
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Closed);

        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);
        assert!(breaker.can_execute().is_err());

        // Wait for recovery timeout
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert!(breaker.can_execute().is_ok());
        assert_eq!(breaker.state(), CircuitState::HalfOpen);

        // Success should close circuit
        breaker.record_success();
        assert_eq!(breaker.state(), CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_recovery_manager() {
        let handler = Arc::new(DefaultErrorHandler::new());
        let manager = RecoveryManager::new(handler);

        let attempt_count = Arc::new(AtomicU32::new(0));
        let attempt_count_clone = attempt_count.clone();

        let operation = move || {
            let count = attempt_count_clone.fetch_add(1, Ordering::SeqCst);
            async move {
                if count < 2 {
                    Err(StratumError::Network {
                        message: "Connection failed".to_string(),
                        source: None,
                    })
                } else {
                    Ok("success".to_string())
                }
            }
        };

        let context = ErrorContext::new("test", "test_operation");
        let result = manager
            .execute_with_recovery(operation, context, Some("network"))
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        assert_eq!(attempt_count.load(Ordering::SeqCst), 3); // 2 failures + 1 success
    }

    #[tokio::test]
    async fn test_recovery_failure() {
        let handler = Arc::new(DefaultErrorHandler::new());
        let mut manager = RecoveryManager::new(handler);

        // Set very limited retry policy
        manager.set_policy(
            "test",
            RecoveryPolicy {
                max_retries: 1,
                base_delay: Duration::from_millis(10),
                max_delay: Duration::from_millis(10),
                backoff_multiplier: 1.0,
                max_total_time: Duration::from_millis(100),
                jitter_factor: 0.0,
            },
        );

        let operation = || async {
            Err::<String, _>(StratumError::Network {
                message: "Always fails".to_string(),
                source: None,
            })
        };

        let context = ErrorContext::new("test", "failing_operation");
        let result = manager
            .execute_with_recovery(operation, context, Some("test"))
            .await;

        assert!(result.is_err());
    }
}
