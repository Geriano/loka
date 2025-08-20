use std::collections::HashMap;
use dashmap::DashMap;
use std::net::IpAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use async_trait::async_trait;
use serde_json::Value;
use tracing::{debug, warn};

use super::types::{StratumError, Result, SecuritySeverity};

/// Validation context for requests
#[derive(Debug, Clone)]
pub struct ValidationContext {
    pub remote_addr: Option<std::net::SocketAddr>,
    pub user_id: Option<String>,
    pub request_id: Option<String>,
    pub method: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub headers: HashMap<String, String>,
}

impl ValidationContext {
    pub fn new() -> Self {
        Self {
            remote_addr: None,
            user_id: None,
            request_id: None,
            method: None,
            timestamp: chrono::Utc::now(),
            headers: HashMap::new(),
        }
    }
    
    pub fn with_remote_addr(mut self, addr: std::net::SocketAddr) -> Self {
        self.remote_addr = Some(addr);
        self
    }
    
    pub fn with_method(mut self, method: String) -> Self {
        self.method = Some(method);
        self
    }
    
    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }
}

impl Default for ValidationContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for validating messages and requests
#[async_trait]
pub trait MessageValidator: Send + Sync {
    /// Validate a message and return any validation errors
    async fn validate(&self, message: &Value, context: &ValidationContext) -> Result<()>;
    
    /// Get the name of this validator for logging
    fn name(&self) -> &str;
    
    /// Check if this validator should run for the given message type
    fn should_validate(&self, method: Option<&str>) -> bool {
        true // Default: validate all messages
    }
}

/// JSON structure validator
#[derive(Debug, Clone)]
pub struct JsonStructureValidator {
    max_depth: usize,
    max_string_length: usize,
    max_array_length: usize,
    max_object_properties: usize,
}

impl JsonStructureValidator {
    pub fn new() -> Self {
        Self {
            max_depth: 10,
            max_string_length: 1024,
            max_array_length: 100,
            max_object_properties: 50,
        }
    }
    
    pub fn with_limits(
        mut self,
        max_depth: usize,
        max_string_length: usize,
        max_array_length: usize,
        max_object_properties: usize,
    ) -> Self {
        self.max_depth = max_depth;
        self.max_string_length = max_string_length;
        self.max_array_length = max_array_length;
        self.max_object_properties = max_object_properties;
        self
    }
    
    fn validate_value(&self, value: &Value, depth: usize) -> Result<()> {
        if depth > self.max_depth {
            return Err(StratumError::ValidationFailed {
                field: "json_structure".to_string(),
                message: format!("JSON depth exceeds limit of {}", self.max_depth),
                value: None,
            });
        }
        
        match value {
            Value::String(s) => {
                if s.len() > self.max_string_length {
                    return Err(StratumError::ValidationFailed {
                        field: "string_length".to_string(),
                        message: format!("String length {} exceeds limit of {}", 
                                       s.len(), self.max_string_length),
                        value: Some(format!("{}...", &s[..50.min(s.len())])),
                    });
                }
            },
            Value::Array(arr) => {
                if arr.len() > self.max_array_length {
                    return Err(StratumError::ValidationFailed {
                        field: "array_length".to_string(),
                        message: format!("Array length {} exceeds limit of {}", 
                                       arr.len(), self.max_array_length),
                        value: None,
                    });
                }
                
                for item in arr {
                    self.validate_value(item, depth + 1)?;
                }
            },
            Value::Object(obj) => {
                if obj.len() > self.max_object_properties {
                    return Err(StratumError::ValidationFailed {
                        field: "object_properties".to_string(),
                        message: format!("Object properties {} exceed limit of {}", 
                                       obj.len(), self.max_object_properties),
                        value: None,
                    });
                }
                
                for (_, val) in obj {
                    self.validate_value(val, depth + 1)?;
                }
            },
            _ => {} // Numbers, booleans, null are always valid
        }
        
        Ok(())
    }
}

impl Default for JsonStructureValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MessageValidator for JsonStructureValidator {
    async fn validate(&self, message: &Value, context: &ValidationContext) -> Result<()> {
        self.validate_value(message, 0)
    }
    
    fn name(&self) -> &str {
        "JsonStructureValidator"
    }
}

/// Stratum protocol validator
#[derive(Debug, Clone)]
pub struct StratumProtocolValidator;

impl StratumProtocolValidator {
    pub fn new() -> Self {
        Self
    }
    
    fn validate_required_fields(&self, obj: &serde_json::Map<String, Value>) -> Result<()> {
        // Check for required JSON-RPC fields
        if !obj.contains_key("id") {
            return Err(StratumError::ValidationFailed {
                field: "id".to_string(),
                message: "Missing required 'id' field".to_string(),
                value: None,
            });
        }
        
        if !obj.contains_key("method") && !obj.contains_key("result") && !obj.contains_key("error") {
            return Err(StratumError::ValidationFailed {
                field: "message_type".to_string(),
                message: "Message must have 'method' (request) or 'result'/'error' (response)".to_string(),
                value: None,
            });
        }
        
        Ok(())
    }
    
    fn validate_method(&self, method: &str, params: &Value) -> Result<()> {
        match method {
            "mining.authorize" => {
                let params = params.as_array().ok_or_else(|| StratumError::ValidationFailed {
                    field: "params".to_string(),
                    message: "mining.authorize params must be an array".to_string(),
                    value: Some(params.to_string()),
                })?;
                
                if params.len() != 2 {
                    return Err(StratumError::ValidationFailed {
                        field: "params".to_string(),
                        message: "mining.authorize requires exactly 2 parameters".to_string(),
                        value: Some(format!("got {} parameters", params.len())),
                    });
                }
                
                // Validate username format
                let username = params[0].as_str().ok_or_else(|| StratumError::ValidationFailed {
                    field: "params[0]".to_string(),
                    message: "Username must be a string".to_string(),
                    value: Some(params[0].to_string()),
                })?;
                
                if username.is_empty() || username.len() > 100 {
                    return Err(StratumError::ValidationFailed {
                        field: "username".to_string(),
                        message: "Username must be between 1-100 characters".to_string(),
                        value: Some(username.to_string()),
                    });
                }
                
                // Check for dangerous characters
                if username.contains(['<', '>', '"', '\'', '&', '\0']) {
                    return Err(StratumError::SecurityViolation {
                        message: "Username contains potentially dangerous characters".to_string(),
                        severity: SecuritySeverity::Medium,
                    });
                }
            },
            
            "mining.submit" => {
                let params = params.as_array().ok_or_else(|| StratumError::ValidationFailed {
                    field: "params".to_string(),
                    message: "mining.submit params must be an array".to_string(),
                    value: Some(params.to_string()),
                })?;
                
                if params.len() != 5 {
                    return Err(StratumError::ValidationFailed {
                        field: "params".to_string(),
                        message: "mining.submit requires exactly 5 parameters".to_string(),
                        value: Some(format!("got {} parameters", params.len())),
                    });
                }
                
                // Validate hex string parameters
                for (i, param) in params.iter().skip(1).enumerate() {
                    let hex_str = param.as_str().ok_or_else(|| StratumError::ValidationFailed {
                        field: format!("params[{}]", i + 1),
                        message: "Submit parameters must be strings".to_string(),
                        value: Some(param.to_string()),
                    })?;
                    
                    if !hex_str.chars().all(|c| c.is_ascii_hexdigit()) {
                        return Err(StratumError::ValidationFailed {
                            field: format!("params[{}]", i + 1),
                            message: "Submit parameter must be valid hex string".to_string(),
                            value: Some(hex_str.to_string()),
                        });
                    }
                }
            },
            
            "mining.subscribe" => {
                if let Some(params) = params.as_array() {
                    if params.len() > 2 {
                        return Err(StratumError::ValidationFailed {
                            field: "params".to_string(),
                            message: "mining.subscribe accepts at most 2 parameters".to_string(),
                            value: Some(format!("got {} parameters", params.len())),
                        });
                    }
                }
            },
            
            // Accept other standard mining methods
            "mining.extranonce.subscribe" | 
            "mining.configure" | 
            "mining.capabilities" => {
                // These are valid but we don't validate their specific parameters yet
            },
            
            _ => {
                warn!(method = method, "Unknown mining method");
                // Don't fail on unknown methods to allow for protocol extensions
            }
        }
        
        Ok(())
    }
}

impl Default for StratumProtocolValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MessageValidator for StratumProtocolValidator {
    async fn validate(&self, message: &Value, context: &ValidationContext) -> Result<()> {
        let obj = message.as_object().ok_or_else(|| StratumError::ValidationFailed {
            field: "message".to_string(),
            message: "Message must be a JSON object".to_string(),
            value: Some(message.to_string()),
        })?;
        
        self.validate_required_fields(obj)?;
        
        // Validate method-specific parameters
        if let Some(method) = obj.get("method").and_then(|v| v.as_str()) {
            let params = obj.get("params").unwrap_or(&Value::Null);
            self.validate_method(method, params)?;
        }
        
        Ok(())
    }
    
    fn name(&self) -> &str {
        "StratumProtocolValidator"
    }
    
    fn should_validate(&self, method: Option<&str>) -> bool {
        method.map(|m| m.starts_with("mining.")).unwrap_or(false)
    }
}

/// Rate limiting validator to prevent abuse
#[derive(Debug)]
pub struct RateLimitValidator {
    // IP-based rate limiting
    ip_requests: DashMap<IpAddr, Vec<Instant>>,
    max_requests_per_minute: u32,
    max_requests_per_second: u32,
    ban_duration: Duration,
    cleanup_interval: Duration,
    last_cleanup: AtomicU64, // Unix timestamp in seconds
}

impl RateLimitValidator {
    pub fn new() -> Self {
        Self {
            ip_requests: DashMap::new(),
            max_requests_per_minute: 60,
            max_requests_per_second: 5,
            ban_duration: Duration::from_secs(300), // 5 minutes
            cleanup_interval: Duration::from_secs(60),
            last_cleanup: AtomicU64::new(SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()),
        }
    }
    
    pub fn with_limits(
        mut self,
        max_per_second: u32,
        max_per_minute: u32,
        ban_duration: Duration,
    ) -> Self {
        self.max_requests_per_second = max_per_second;
        self.max_requests_per_minute = max_per_minute;
        self.ban_duration = ban_duration;
        self
    }
    
    fn cleanup_old_entries(&self) {
        let now_secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        let last_cleanup = self.last_cleanup.load(Ordering::Relaxed);
        
        if now_secs.saturating_sub(last_cleanup) < self.cleanup_interval.as_secs() {
            return;
        }
        
        let cutoff = Instant::now() - Duration::from_secs(60);
        
        self.ip_requests.retain(|_, timestamps| {
            timestamps.retain(|&t| t > cutoff);
            !timestamps.is_empty()
        });
        
        self.last_cleanup.store(now_secs, Ordering::Relaxed);
        debug!("Cleaned up rate limit entries, {} IPs tracked", self.ip_requests.len());
    }
    
    fn check_rate_limit(&self, ip: IpAddr) -> Result<()> {
        self.cleanup_old_entries();
        
        let now = Instant::now();
        let mut requests = self.ip_requests.entry(ip).or_insert_with(Vec::new);
        
        // Count requests in the last second
        let second_ago = now - Duration::from_secs(1);
        let recent_requests = requests.iter().filter(|&&t| t > second_ago).count();
        
        if recent_requests >= self.max_requests_per_second as usize {
            return Err(StratumError::RateLimitExceeded {
                limit: self.max_requests_per_second as u64,
                window: Duration::from_secs(1),
                retry_after: Duration::from_secs(1),
            });
        }
        
        // Count requests in the last minute
        let minute_ago = now - Duration::from_secs(60);
        let minute_requests = requests.iter().filter(|&&t| t > minute_ago).count();
        
        if minute_requests >= self.max_requests_per_minute as usize {
            return Err(StratumError::RateLimitExceeded {
                limit: self.max_requests_per_minute as u64,
                window: Duration::from_secs(60),
                retry_after: self.ban_duration,
            });
        }
        
        // Record this request
        requests.push(now);
        Ok(())
    }
}

impl Default for RateLimitValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MessageValidator for RateLimitValidator {
    async fn validate(&self, _message: &Value, context: &ValidationContext) -> Result<()> {
        if let Some(addr) = context.remote_addr {
            self.check_rate_limit(addr.ip())?;
        }
        Ok(())
    }
    
    fn name(&self) -> &str {
        "RateLimitValidator"
    }
}

/// Composite validator that runs multiple validators
pub struct CompositeValidator {
    validators: Vec<Box<dyn MessageValidator>>,
}

impl std::fmt::Debug for CompositeValidator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompositeValidator")
            .field("validators", &format!("{} validators", self.validators.len()))
            .finish()
    }
}

impl CompositeValidator {
    pub fn new() -> Self {
        Self {
            validators: Vec::new(),
        }
    }
    
    pub fn add_validator(mut self, validator: Box<dyn MessageValidator>) -> Self {
        self.validators.push(validator);
        self
    }
    
    pub fn with_default_validators() -> Self {
        Self::new()
            .add_validator(Box::new(JsonStructureValidator::new()))
            .add_validator(Box::new(StratumProtocolValidator::new()))
            .add_validator(Box::new(RateLimitValidator::new()))
    }
}

impl Default for CompositeValidator {
    fn default() -> Self {
        Self::with_default_validators()
    }
}

#[async_trait]
impl MessageValidator for CompositeValidator {
    async fn validate(&self, message: &Value, context: &ValidationContext) -> Result<()> {
        let method = context.method.as_deref();
        
        for validator in &self.validators {
            if validator.should_validate(method) {
                if let Err(error) = validator.validate(message, context).await {
                    warn!(
                        validator = validator.name(),
                        error = %error,
                        method = ?method,
                        remote_addr = ?context.remote_addr,
                        "Validation failed"
                    );
                    return Err(error);
                }
            }
        }
        
        Ok(())
    }
    
    fn name(&self) -> &str {
        "CompositeValidator"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    
    #[tokio::test]
    async fn test_json_structure_validator() {
        let validator = JsonStructureValidator::new();
        let context = ValidationContext::new();
        
        // Valid JSON
        let valid_message = json!({
            "id": 1,
            "method": "mining.authorize",
            "params": ["user", "pass"]
        });
        
        assert!(validator.validate(&valid_message, &context).await.is_ok());
        
        // Invalid - too deep nesting
        let deep_json = json!({
            "a": {"b": {"c": {"d": {"e": {"f": {"g": {"h": {"i": {"j": {"k": 1}}}}}}}}}}
        });
        
        let validator_shallow = JsonStructureValidator::new().with_limits(5, 1024, 100, 50);
        assert!(validator_shallow.validate(&deep_json, &context).await.is_err());
    }
    
    #[tokio::test]
    async fn test_stratum_protocol_validator() {
        let validator = StratumProtocolValidator::new();
        let context = ValidationContext::new();
        
        // Valid authorize message
        let auth_message = json!({
            "id": 1,
            "method": "mining.authorize",
            "params": ["user", "pass"]
        });
        
        assert!(validator.validate(&auth_message, &context).await.is_ok());
        
        // Invalid - missing required fields
        let invalid_message = json!({
            "method": "mining.authorize"
            // missing id and params
        });
        
        assert!(validator.validate(&invalid_message, &context).await.is_err());
        
        // Invalid - dangerous username
        let dangerous_message = json!({
            "id": 1,
            "method": "mining.authorize",
            "params": ["<script>alert('xss')</script>", "pass"]
        });
        
        assert!(validator.validate(&dangerous_message, &context).await.is_err());
    }
    
    #[tokio::test]
    async fn test_rate_limit_validator() {
        let validator = RateLimitValidator::new().with_limits(2, 10, Duration::from_secs(60));
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let context = ValidationContext::new()
            .with_remote_addr(SocketAddr::new(ip, 1234));
        
        let message = json!({"id": 1, "method": "test"});
        
        // First two requests should pass
        assert!(validator.validate(&message, &context).await.is_ok());
        assert!(validator.validate(&message, &context).await.is_ok());
        
        // Third request should be rate limited
        let result = validator.validate(&message, &context).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StratumError::RateLimitExceeded { .. }));
    }
    
    #[tokio::test]
    async fn test_composite_validator() {
        let validator = CompositeValidator::with_default_validators();
        let context = ValidationContext::new()
            .with_method("mining.authorize".to_string());
        
        // Valid message should pass all validators
        let valid_message = json!({
            "id": 1,
            "method": "mining.authorize",
            "params": ["user", "pass"]
        });
        
        assert!(validator.validate(&valid_message, &context).await.is_ok());
        
        // Invalid message should fail
        let invalid_message = json!({
            "id": 1,
            "method": "mining.authorize",
            "params": ["", "pass"] // empty username
        });
        
        assert!(validator.validate(&invalid_message, &context).await.is_err());
    }
}