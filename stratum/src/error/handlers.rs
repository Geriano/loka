use async_trait::async_trait;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use super::types::{ErrorAction, ErrorSeverity, Result, StratumError};

/// Trait for handling errors in the Stratum system
#[async_trait]
pub trait ErrorHandler: Send + Sync {
    /// Handle an error and return the recommended action
    async fn handle_error(&self, error: &StratumError, context: &ErrorContext) -> ErrorAction;

    /// Check if the error handler can handle this specific error type
    fn can_handle(&self, error: &StratumError) -> bool;

    /// Get the priority of this error handler (higher values = higher priority)
    fn priority(&self) -> u32 {
        100
    }
}

/// Context information for error handling
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub component: String,
    pub operation: String,
    pub remote_addr: Option<std::net::SocketAddr>,
    pub user_id: Option<String>,
    pub request_id: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub attempt_count: u32,
    pub max_attempts: u32,
}

impl ErrorContext {
    pub fn new(component: &str, operation: &str) -> Self {
        Self {
            component: component.to_string(),
            operation: operation.to_string(),
            remote_addr: None,
            user_id: None,
            request_id: None,
            timestamp: chrono::Utc::now(),
            attempt_count: 0,
            max_attempts: 3,
        }
    }

    pub fn with_remote_addr(mut self, addr: std::net::SocketAddr) -> Self {
        self.remote_addr = Some(addr);
        self
    }

    pub fn with_user(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }

    pub fn with_attempt_info(mut self, attempt_count: u32, max_attempts: u32) -> Self {
        self.attempt_count = attempt_count;
        self.max_attempts = max_attempts;
        self
    }

    pub fn should_retry(&self) -> bool {
        self.attempt_count < self.max_attempts
    }
}

/// Default error handler that implements basic retry logic
#[derive(Debug)]
pub struct DefaultErrorHandler {
    max_retries: u32,
    base_delay: Duration,
    max_delay: Duration,
}

impl DefaultErrorHandler {
    pub fn new() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
        }
    }

    pub fn with_retry_config(
        mut self,
        max_retries: u32,
        base_delay: Duration,
        max_delay: Duration,
    ) -> Self {
        self.max_retries = max_retries;
        self.base_delay = base_delay;
        self.max_delay = max_delay;
        self
    }

    /// Calculate exponential backoff delay
    #[allow(unused)]
    fn calculate_delay(&self, attempt: u32) -> Duration {
        let delay = self.base_delay * 2_u32.pow(attempt.saturating_sub(1));
        std::cmp::min(delay, self.max_delay)
    }
}

impl Default for DefaultErrorHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ErrorHandler for DefaultErrorHandler {
    async fn handle_error(&self, error: &StratumError, context: &ErrorContext) -> ErrorAction {
        let severity = error.severity();

        // Log the error with appropriate level
        match severity {
            ErrorSeverity::Critical => {
                error!(
                    error = %error,
                    component = context.component,
                    operation = context.operation,
                    remote_addr = ?context.remote_addr,
                    user_id = ?context.user_id,
                    "Critical error occurred"
                );
            }
            ErrorSeverity::High => {
                error!(
                    error = %error,
                    component = context.component,
                    operation = context.operation,
                    "High severity error"
                );
            }
            ErrorSeverity::Medium => {
                warn!(
                    error = %error,
                    component = context.component,
                    operation = context.operation,
                    "Medium severity error"
                );
            }
            ErrorSeverity::Low => {
                debug!(
                    error = %error,
                    component = context.component,
                    operation = context.operation,
                    "Low severity error"
                );
            }
        }

        // Return error's recommended action, but override for retryable errors
        let recommended = error.recommended_action();

        match recommended {
            ErrorAction::Retry { .. } if context.should_retry() && error.is_retryable() => {
                ErrorAction::Retry {
                    max_attempts: self.max_retries,
                    base_delay: self.base_delay,
                    max_delay: self.max_delay,
                }
            }
            _ => recommended,
        }
    }

    fn can_handle(&self, _error: &StratumError) -> bool {
        true // Default handler can handle any error
    }

    fn priority(&self) -> u32 {
        0 // Lowest priority
    }
}

/// Network-specific error handler with connection management
#[derive(Debug)]
pub struct NetworkErrorHandler {
    connection_timeout: Duration,
    max_connection_retries: u32,
}

impl NetworkErrorHandler {
    pub fn new() -> Self {
        Self {
            connection_timeout: Duration::from_secs(30),
            max_connection_retries: 3,
        }
    }

    pub fn with_connection_config(mut self, timeout: Duration, max_retries: u32) -> Self {
        self.connection_timeout = timeout;
        self.max_connection_retries = max_retries;
        self
    }
}

impl Default for NetworkErrorHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ErrorHandler for NetworkErrorHandler {
    async fn handle_error(&self, error: &StratumError, context: &ErrorContext) -> ErrorAction {
        match error {
            StratumError::Network { message, .. } => {
                warn!(
                    message = message,
                    component = context.component,
                    remote_addr = ?context.remote_addr,
                    "Network error detected"
                );

                if context.attempt_count < self.max_connection_retries {
                    ErrorAction::Retry {
                        max_attempts: self.max_connection_retries,
                        base_delay: Duration::from_millis(500),
                        max_delay: Duration::from_secs(10),
                    }
                } else {
                    ErrorAction::Terminate
                }
            }

            StratumError::ConnectionTimeout { .. } => {
                info!(
                    component = context.component,
                    timeout = ?self.connection_timeout,
                    "Connection timeout, will retry with longer timeout"
                );

                ErrorAction::Retry {
                    max_attempts: 2,
                    base_delay: Duration::from_secs(1),
                    max_delay: Duration::from_secs(5),
                }
            }

            StratumError::ConnectionLimitExceeded { current, max } => {
                warn!(
                    current = current,
                    max = max,
                    "Connection limit exceeded, terminating connection"
                );
                ErrorAction::Terminate
            }

            _ => ErrorAction::Escalate,
        }
    }

    fn can_handle(&self, error: &StratumError) -> bool {
        matches!(
            error,
            StratumError::Network { .. }
                | StratumError::Connection { .. }
                | StratumError::ConnectionTimeout { .. }
                | StratumError::ConnectionLimitExceeded { .. }
        )
    }

    fn priority(&self) -> u32 {
        200 // High priority for network errors
    }
}

/// Security-focused error handler
#[derive(Debug)]
pub struct SecurityErrorHandler {
    ban_duration: Duration,
    max_violations: u32,
}

impl SecurityErrorHandler {
    pub fn new() -> Self {
        Self {
            ban_duration: Duration::from_secs(300), // 5 minutes
            max_violations: 5,
        }
    }

    pub fn with_security_config(mut self, ban_duration: Duration, max_violations: u32) -> Self {
        self.ban_duration = ban_duration;
        self.max_violations = max_violations;
        self
    }
}

impl Default for SecurityErrorHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ErrorHandler for SecurityErrorHandler {
    async fn handle_error(&self, error: &StratumError, context: &ErrorContext) -> ErrorAction {
        match error {
            StratumError::SecurityViolation { message, severity } => {
                error!(
                    message = message,
                    severity = %severity,
                    component = context.component,
                    remote_addr = ?context.remote_addr,
                    user_id = ?context.user_id,
                    "Security violation detected"
                );

                // Immediate termination for security violations
                ErrorAction::Terminate
            }

            StratumError::Authentication {
                reason,
                username,
                remote_addr,
            } => {
                warn!(
                    reason = reason,
                    username = ?username,
                    remote_addr = ?remote_addr,
                    "Authentication failure"
                );

                // For auth failures, implement progressive backoff
                if context.attempt_count >= 3 {
                    ErrorAction::CircuitBreaker {
                        duration: self.ban_duration,
                        message: "Too many authentication failures".to_string(),
                    }
                } else {
                    ErrorAction::Retry {
                        max_attempts: 3,
                        base_delay: Duration::from_secs(1),
                        max_delay: Duration::from_secs(10),
                    }
                }
            }

            StratumError::RateLimitExceeded {
                limit,
                window,
                retry_after,
            } => {
                info!(
                    limit = limit,
                    window = ?window,
                    retry_after = ?retry_after,
                    remote_addr = ?context.remote_addr,
                    "Rate limit exceeded"
                );

                ErrorAction::CircuitBreaker {
                    duration: *retry_after,
                    message: "Rate limit exceeded".to_string(),
                }
            }

            _ => ErrorAction::Escalate,
        }
    }

    fn can_handle(&self, error: &StratumError) -> bool {
        matches!(
            error,
            StratumError::SecurityViolation { .. }
                | StratumError::Authentication { .. }
                | StratumError::Authorization { .. }
                | StratumError::RateLimitExceeded { .. }
                | StratumError::ValidationFailed { .. }
        )
    }

    fn priority(&self) -> u32 {
        300 // Highest priority for security errors
    }
}

/// Composite error handler that delegates to specialized handlers
pub struct CompositeErrorHandler {
    handlers: Vec<Arc<dyn ErrorHandler>>,
}

impl std::fmt::Debug for CompositeErrorHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompositeErrorHandler")
            .field("handlers", &format!("{} handlers", self.handlers.len()))
            .finish()
    }
}

impl CompositeErrorHandler {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    pub fn with_handler(mut self, handler: Arc<dyn ErrorHandler>) -> Self {
        self.handlers.push(handler);
        // Sort by priority (highest first)
        self.handlers
            .sort_by_key(|h| std::cmp::Reverse(h.priority()));
        self
    }

    pub fn with_default_handlers() -> Self {
        Self::new()
            .with_handler(Arc::new(SecurityErrorHandler::new()))
            .with_handler(Arc::new(NetworkErrorHandler::new()))
            .with_handler(Arc::new(DefaultErrorHandler::new()))
    }
}

impl Default for CompositeErrorHandler {
    fn default() -> Self {
        Self::with_default_handlers()
    }
}

#[async_trait]
impl ErrorHandler for CompositeErrorHandler {
    async fn handle_error(&self, error: &StratumError, context: &ErrorContext) -> ErrorAction {
        // Find the first handler that can handle this error
        for handler in &self.handlers {
            if handler.can_handle(error) {
                let action = handler.handle_error(error, context).await;

                debug!(
                    handler = std::any::type_name_of_val(handler.as_ref()),
                    action = ?action,
                    error = %error,
                    "Error handled by specialized handler"
                );

                return action;
            }
        }

        // If no handler can handle it, terminate by default
        warn!(
            error = %error,
            "No handler found for error, terminating"
        );
        ErrorAction::Terminate
    }

    fn can_handle(&self, error: &StratumError) -> bool {
        self.handlers.iter().any(|h| h.can_handle(error))
    }

    fn priority(&self) -> u32 {
        self.handlers
            .iter()
            .map(|h| h.priority())
            .max()
            .unwrap_or(0)
    }
}

/// Error recovery utility for implementing retry logic with exponential backoff
pub struct ErrorRecovery {
    handler: Arc<dyn ErrorHandler>,
}

impl ErrorRecovery {
    pub fn new(handler: Arc<dyn ErrorHandler>) -> Self {
        Self { handler }
    }

    /// Execute an operation with automatic error recovery
    pub async fn execute_with_recovery<F, Fut, T>(
        &self,
        operation: F,
        context: ErrorContext,
    ) -> Result<T>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<T>> + Send,
        T: Send,
    {
        let mut current_context = context;
        let start_time = Instant::now();

        loop {
            current_context.attempt_count += 1;

            match operation().await {
                Ok(result) => {
                    if current_context.attempt_count > 1 {
                        info!(
                            component = current_context.component,
                            operation = current_context.operation,
                            attempts = current_context.attempt_count,
                            duration = ?start_time.elapsed(),
                            "Operation succeeded after retry"
                        );
                    }
                    return Ok(result);
                }
                Err(error) => {
                    let action = self.handler.handle_error(&error, &current_context).await;

                    match action {
                        ErrorAction::Retry {
                            max_attempts,
                            base_delay,
                            max_delay,
                        } => {
                            if current_context.attempt_count >= max_attempts {
                                error!(
                                    error = %error,
                                    attempts = current_context.attempt_count,
                                    max_attempts = max_attempts,
                                    "Max retry attempts exceeded"
                                );
                                return Err(error);
                            }

                            // Calculate delay with exponential backoff
                            let attempt = current_context.attempt_count.saturating_sub(1);
                            let delay = std::cmp::min(base_delay * 2_u32.pow(attempt), max_delay);

                            debug!(
                                attempt = current_context.attempt_count,
                                max_attempts = max_attempts,
                                delay = ?delay,
                                "Retrying operation after error"
                            );

                            sleep(delay).await;
                            continue;
                        }

                        ErrorAction::CircuitBreaker { duration, message } => {
                            warn!(
                                duration = ?duration,
                                message = message,
                                "Circuit breaker activated"
                            );

                            sleep(duration).await;
                            return Err(error);
                        }

                        ErrorAction::Terminate => {
                            debug!("Error handler recommended termination");
                            return Err(error);
                        }

                        ErrorAction::Ignore => {
                            warn!(
                                error = %error,
                                "Error ignored by handler"
                            );
                            // Return a default value or continue - this depends on the operation
                            return Err(error);
                        }

                        ErrorAction::Escalate => {
                            error!(
                                error = %error,
                                "Error escalated - no recovery possible"
                            );
                            return Err(error);
                        }

                        ErrorAction::Fallback { message } => {
                            warn!(message = message, "Falling back to default behavior");
                            return Err(error);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;

    #[tokio::test]
    async fn test_default_error_handler() {
        let handler = DefaultErrorHandler::new();
        let context = ErrorContext::new("test", "test_operation");

        let error = StratumError::Network {
            message: "Connection refused".to_string(),
            source: None,
        };

        let action = handler.handle_error(&error, &context).await;

        match action {
            ErrorAction::Retry { max_attempts, .. } => {
                assert_eq!(max_attempts, 3);
            }
            _ => panic!("Expected retry action"),
        }
    }

    #[tokio::test]
    async fn test_security_error_handler() {
        let handler = SecurityErrorHandler::new();
        let context = ErrorContext::new("auth", "authenticate")
            .with_remote_addr("127.0.0.1:1234".parse().unwrap());

        let error = StratumError::Authentication {
            reason: "Invalid password".to_string(),
            username: Some("test_user".to_string()),
            remote_addr: Some("127.0.0.1:1234".parse().unwrap()),
        };

        let action = handler.handle_error(&error, &context).await;

        match action {
            ErrorAction::Retry { max_attempts, .. } => {
                assert_eq!(max_attempts, 3);
            }
            _ => panic!("Expected retry action"),
        }
    }

    #[tokio::test]
    async fn test_composite_error_handler() {
        let handler = CompositeErrorHandler::with_default_handlers();
        let context = ErrorContext::new("network", "connect");

        let error = StratumError::Network {
            message: "Connection timeout".to_string(),
            source: None,
        };

        let action = handler.handle_error(&error, &context).await;

        // Should be handled by NetworkErrorHandler
        match action {
            ErrorAction::Retry { .. } | ErrorAction::Terminate => {
                // Either is acceptable
            }
            _ => panic!("Expected retry or terminate action"),
        }
    }
}
