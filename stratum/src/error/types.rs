use std::time::Duration;
use thiserror::Error;
use serde::{Deserialize, Serialize};

/// Comprehensive error types for the Stratum proxy system
#[derive(Error, Debug)]
pub enum StratumError {
    // Network-related errors
    #[error("Network error: {message}")]
    Network { 
        message: String,
        #[source]
        source: Option<Box<StratumError>>,
    },
    
    #[error("Connection error: {message}")]
    Connection { 
        message: String,
        remote_addr: Option<std::net::SocketAddr>,
    },
    
    #[error("Connection limit exceeded: {current}/{max}")]
    ConnectionLimitExceeded { current: usize, max: usize },
    
    #[error("Connection timeout: {timeout:?}")]
    ConnectionTimeout { timeout: Duration },
    
    // Protocol-related errors
    #[error("Protocol error: {message}")]
    Protocol { 
        message: String,
        method: Option<String>,
        request_id: Option<i64>,
    },
    
    #[error("Invalid message format: {message}")]
    InvalidMessageFormat { 
        message: String,
        raw_data: Option<String>,
    },
    
    #[error("Unsupported protocol version: {version}")]
    UnsupportedProtocolVersion { version: String },
    
    #[error("Message too large: {size} bytes (max: {max_size} bytes)")]
    MessageTooLarge { size: usize, max_size: usize },
    
    // Authentication-related errors
    #[error("Authentication failed: {reason}")]
    Authentication { 
        reason: String,
        username: Option<String>,
        remote_addr: Option<std::net::SocketAddr>,
    },
    
    #[error("Authorization failed: insufficient permissions")]
    Authorization { 
        required_permission: String,
        current_permissions: Vec<String>,
    },
    
    #[error("Session expired for user: {username}")]
    SessionExpired { 
        username: String,
        expired_at: chrono::DateTime<chrono::Utc>,
    },
    
    #[error("Invalid credentials format")]
    InvalidCredentials,
    
    // Job-related errors
    #[error("Job error: {message}")]
    Job { 
        message: String,
        job_id: Option<String>,
    },
    
    #[error("Job not found: {job_id}")]
    JobNotFound { job_id: String },
    
    #[error("Job expired: {job_id} (expired at: {expired_at})")]
    JobExpired { 
        job_id: String,
        expired_at: chrono::DateTime<chrono::Utc>,
    },
    
    #[error("Invalid job parameters: {message}")]
    InvalidJobParameters { message: String },
    
    // Submission-related errors
    #[error("Submission error: {message}")]
    Submission { 
        message: String,
        submission_id: Option<String>,
        job_id: Option<String>,
    },
    
    #[error("Duplicate submission: {submission_id}")]
    DuplicateSubmission { submission_id: String },
    
    #[error("Submission rejected: {reason}")]
    SubmissionRejected { 
        reason: String,
        submission_id: String,
        job_id: String,
    },
    
    #[error("Invalid share: {message}")]
    InvalidShare { 
        message: String,
        difficulty: Option<u64>,
    },
    
    // Storage-related errors
    #[error("Storage error: {message}")]
    Storage { 
        message: String,
        operation: Option<String>,
    },
    
    #[error("Storage capacity exceeded: {current}/{max}")]
    StorageCapacityExceeded { current: usize, max: usize },
    
    #[error("Data corruption detected: {message}")]
    DataCorruption { message: String },
    
    // Rate limiting and security errors
    #[error("Rate limit exceeded: {limit} requests per {window:?}")]
    RateLimitExceeded { 
        limit: u64,
        window: Duration,
        retry_after: Duration,
    },
    
    #[error("Security violation: {message}")]
    SecurityViolation { 
        message: String,
        severity: SecuritySeverity,
    },
    
    #[error("Validation failed: {field} - {message}")]
    ValidationFailed { 
        field: String,
        message: String,
        value: Option<String>,
    },
    
    // Configuration errors
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),
    
    // External service errors
    #[error("JSON parsing error: {message}")]
    JsonParsing { message: String },
    
    #[error("Internal error: {message}")]
    Internal { message: String },
    
    // Task management errors
    #[error("Task error: {message}")]
    Task { 
        message: String,
        task_id: Option<String>,
    },
    
    #[error("Task timeout: {timeout:?}")]
    TaskTimeout { timeout: Duration },
    
    #[error("Task cancelled: {task_id}")]
    TaskCancelled { task_id: String },
    
    // Resource management errors
    #[error("Resource exhausted: {resource}")]
    ResourceExhausted { resource: String },
    
    #[error("Resource unavailable: {resource}")]
    ResourceUnavailable { resource: String },
}

/// Security violation severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecuritySeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for SecuritySeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecuritySeverity::Low => write!(f, "LOW"),
            SecuritySeverity::Medium => write!(f, "MEDIUM"),
            SecuritySeverity::High => write!(f, "HIGH"),
            SecuritySeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Configuration-specific errors
#[derive(Error, Debug, Clone)]
pub enum ConfigError {
    #[error("Invalid port: {port} (must be between 1 and 65535)")]
    InvalidPort { port: u16 },
    
    #[error("Invalid connection limit: {limit} (must be > 0)")]
    InvalidConnectionLimit { limit: usize },
    
    #[error("Invalid duration: {field} = {duration:?} (must be > 0)")]
    InvalidDuration { 
        field: String,
        duration: Duration,
    },
    
    #[error("Missing required field: {field}")]
    MissingField { field: String },
    
    #[error("Invalid configuration format: {message}")]
    InvalidFormat { message: String },
    
    #[error("Configuration file not found: {path}")]
    FileNotFound { path: String },
    
    #[error("Permission denied accessing config file: {path}")]
    PermissionDenied { path: String },
    
    #[error("Environment variable error: {variable} - {message}")]
    EnvironmentVariable { 
        variable: String,
        message: String,
    },
    
    #[error("Configuration validation failed: {message}")]
    ValidationFailed { message: String },
    
    #[error("Incompatible configuration version: {version} (expected: {expected})")]
    IncompatibleVersion { 
        version: String,
        expected: String,
    },
}

/// Error action recommendations for error recovery
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorAction {
    /// Retry the operation with exponential backoff
    Retry { 
        max_attempts: u32,
        base_delay: Duration,
        max_delay: Duration,
    },
    /// Terminate the current operation/connection
    Terminate,
    /// Ignore the error and continue
    Ignore,
    /// Escalate to a higher level handler
    Escalate,
    /// Fallback to a default behavior
    Fallback { message: String },
    /// Circuit breaker - temporarily disable functionality
    CircuitBreaker { 
        duration: Duration,
        message: String,
    },
}

impl StratumError {
    /// Create a network error from std::io::Error
    pub fn from_io_error(err: std::io::Error) -> Self {
        StratumError::Network {
            message: err.to_string(),
            source: None,
        }
    }
    
    /// Create a JSON parsing error
    pub fn from_json_error(err: serde_json::Error) -> Self {
        StratumError::JsonParsing {
            message: err.to_string(),
        }
    }
    
    /// Create an internal error from any error type
    pub fn from_internal<E>(err: E) -> Self 
    where 
        E: std::error::Error + Send + Sync + 'static,
    {
        StratumError::Internal {
            message: err.to_string(),
        }
    }
    
    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(self, 
            StratumError::Network { .. } |
            StratumError::ConnectionTimeout { .. } |
            StratumError::Storage { .. } |
            StratumError::TaskTimeout { .. } |
            StratumError::ResourceUnavailable { .. }
        )
    }
    
    /// Check if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        !matches!(self,
            StratumError::Config(_) |
            StratumError::SecurityViolation { .. } |
            StratumError::DataCorruption { .. } |
            StratumError::InvalidCredentials |
            StratumError::UnsupportedProtocolVersion { .. }
        )
    }
    
    /// Get the severity level of this error
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            StratumError::SecurityViolation { severity: SecuritySeverity::Critical, .. } |
            StratumError::DataCorruption { .. } => ErrorSeverity::Critical,
            
            StratumError::SecurityViolation { .. } |
            StratumError::ConnectionLimitExceeded { .. } |
            StratumError::StorageCapacityExceeded { .. } => ErrorSeverity::High,
            
            StratumError::Authentication { .. } |
            StratumError::Authorization { .. } |
            StratumError::RateLimitExceeded { .. } |
            StratumError::ValidationFailed { .. } => ErrorSeverity::Medium,
            
            _ => ErrorSeverity::Low,
        }
    }
    
    /// Get recommended error action
    pub fn recommended_action(&self) -> ErrorAction {
        match self {
            // Retryable network errors
            StratumError::Network { .. } => ErrorAction::Retry {
                max_attempts: 3,
                base_delay: Duration::from_millis(100),
                max_delay: Duration::from_secs(5),
            },
            
            StratumError::ConnectionTimeout { .. } => ErrorAction::Retry {
                max_attempts: 2,
                base_delay: Duration::from_millis(500),
                max_delay: Duration::from_secs(2),
            },
            
            // Terminable errors
            StratumError::SecurityViolation { .. } |
            StratumError::ConnectionLimitExceeded { .. } => ErrorAction::Terminate,
            
            // Rate limiting
            StratumError::RateLimitExceeded { retry_after, .. } => {
                ErrorAction::CircuitBreaker {
                    duration: *retry_after,
                    message: "Rate limit exceeded".to_string(),
                }
            },
            
            // Validation errors can be escalated
            StratumError::ValidationFailed { .. } => ErrorAction::Escalate,
            
            // Configuration errors are not recoverable
            StratumError::Config(_) => ErrorAction::Terminate,
            
            // Default to retry for most other errors
            _ => ErrorAction::Retry {
                max_attempts: 1,
                base_delay: Duration::from_millis(50),
                max_delay: Duration::from_millis(50),
            },
        }
    }
    
    /// Create a context-aware error with additional information
    pub fn with_context(self, context: &str) -> Self {
        match self {
            StratumError::Internal { message } => StratumError::Internal {
                message: format!("{}: {}", context, message),
            },
            other => other,
        }
    }
}

impl From<serde_json::Error> for StratumError {
    fn from(err: serde_json::Error) -> Self {
        Self::from_json_error(err)
    }
}

impl From<std::io::Error> for StratumError {
    fn from(err: std::io::Error) -> Self {
        Self::from_io_error(err)
    }
}

/// Error severity levels for logging and alerting
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    Low = 0,
    Medium = 1,
    High = 2,
    Critical = 3,
}

impl std::fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorSeverity::Low => write!(f, "LOW"),
            ErrorSeverity::Medium => write!(f, "MEDIUM"),
            ErrorSeverity::High => write!(f, "HIGH"),
            ErrorSeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Result type alias for the Stratum system
pub type Result<T> = std::result::Result<T, StratumError>;