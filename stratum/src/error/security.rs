use async_trait::async_trait;
use dashmap::DashMap;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, error, warn};

use super::types::{Result, SecuritySeverity, StratumError};
use super::validation::{MessageValidator, ValidationContext};

/// Security policy configuration
#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    /// Maximum connections per IP
    pub max_connections_per_ip: u32,
    /// Maximum authentication attempts per IP per hour
    pub max_auth_attempts_per_hour: u32,
    /// Maximum requests per second per IP
    pub max_requests_per_second: u32,
    /// Maximum message size in bytes
    pub max_message_size: usize,
    /// Banned IP addresses
    pub banned_ips: HashSet<IpAddr>,
    /// Allowed IP ranges (if empty, all IPs allowed)
    pub allowed_ip_ranges: Vec<IpRange>,
    /// Enable geolocation-based filtering
    pub enable_geo_filtering: bool,
    /// Allowed countries (ISO 3166-1 alpha-2 codes)
    pub allowed_countries: HashSet<String>,
    /// Enable suspicious pattern detection
    pub enable_pattern_detection: bool,
    /// Block time for security violations (in seconds)
    pub violation_block_time: Duration,
}

impl SecurityPolicy {
    pub fn new() -> Self {
        Self {
            max_connections_per_ip: 10,
            max_auth_attempts_per_hour: 20,
            max_requests_per_second: 50,
            max_message_size: 8192, // 8KB
            banned_ips: HashSet::new(),
            allowed_ip_ranges: Vec::new(),
            enable_geo_filtering: false,
            allowed_countries: HashSet::new(),
            enable_pattern_detection: true,
            violation_block_time: Duration::from_secs(300), // 5 minutes
        }
    }

    pub fn strict() -> Self {
        let mut policy = Self::new();
        policy.max_connections_per_ip = 5;
        policy.max_auth_attempts_per_hour = 10;
        policy.max_requests_per_second = 20;
        policy.max_message_size = 4096;
        policy.enable_pattern_detection = true;
        policy.violation_block_time = Duration::from_secs(600); // 10 minutes
        policy
    }

    pub fn permissive() -> Self {
        let mut policy = Self::new();
        policy.max_connections_per_ip = 50;
        policy.max_auth_attempts_per_hour = 100;
        policy.max_requests_per_second = 200;
        policy.max_message_size = 16384;
        policy.enable_pattern_detection = false;
        policy.violation_block_time = Duration::from_secs(60);
        policy
    }

    pub fn add_banned_ip(&mut self, ip: IpAddr) {
        self.banned_ips.insert(ip);
    }

    pub fn add_allowed_ip_range(&mut self, range: IpRange) {
        self.allowed_ip_ranges.push(range);
    }

    pub fn add_allowed_country(&mut self, country_code: &str) {
        self.allowed_countries.insert(country_code.to_uppercase());
    }
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self::new()
    }
}

/// IP address range for filtering
#[derive(Debug, Clone)]
pub struct IpRange {
    pub network: IpAddr,
    pub prefix_len: u8,
}

impl IpRange {
    pub fn new(network: IpAddr, prefix_len: u8) -> Self {
        Self {
            network,
            prefix_len,
        }
    }

    pub fn contains(&self, ip: &IpAddr) -> bool {
        match (self.network, ip) {
            (IpAddr::V4(net), IpAddr::V4(addr)) => {
                let net_bits = u32::from(net);
                let addr_bits = u32::from(*addr);
                let mask = (!0u32) << (32 - self.prefix_len);
                (net_bits & mask) == (addr_bits & mask)
            }
            (IpAddr::V6(net), IpAddr::V6(addr)) => {
                let net_bits = u128::from(net);
                let addr_bits = u128::from(*addr);
                let mask = (!0u128) << (128 - self.prefix_len);
                (net_bits & mask) == (addr_bits & mask)
            }
            _ => false, // IPv4 vs IPv6 mismatch
        }
    }
}

/// Security violation record
#[derive(Debug, Clone)]
pub struct SecurityViolation {
    pub ip: IpAddr,
    pub violation_type: ViolationType,
    pub timestamp: Instant,
    pub details: String,
    pub severity: SecuritySeverity,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ViolationType {
    ExcessiveConnections,
    ExcessiveAuthAttempts,
    RateLimitExceeded,
    BannedIP,
    UnauthorizedCountry,
    SuspiciousPattern,
    MessageTooLarge,
    InvalidCredentials,
    MalformedRequest,
}

impl std::fmt::Display for ViolationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ViolationType::ExcessiveConnections => write!(f, "excessive_connections"),
            ViolationType::ExcessiveAuthAttempts => write!(f, "excessive_auth_attempts"),
            ViolationType::RateLimitExceeded => write!(f, "rate_limit_exceeded"),
            ViolationType::BannedIP => write!(f, "banned_ip"),
            ViolationType::UnauthorizedCountry => write!(f, "unauthorized_country"),
            ViolationType::SuspiciousPattern => write!(f, "suspicious_pattern"),
            ViolationType::MessageTooLarge => write!(f, "message_too_large"),
            ViolationType::InvalidCredentials => write!(f, "invalid_credentials"),
            ViolationType::MalformedRequest => write!(f, "malformed_request"),
        }
    }
}

/// Security monitoring and enforcement
pub struct SecurityGuard {
    policy: SecurityPolicy,
    ip_connections: DashMap<IpAddr, u32>,
    ip_auth_attempts: DashMap<IpAddr, Vec<Instant>>,
    ip_requests: DashMap<IpAddr, Vec<Instant>>,
    violations: DashMap<String, SecurityViolation>, // Use IP+timestamp as key
    blocked_ips: DashMap<IpAddr, Instant>,
    suspicious_patterns: Vec<SuspiciousPattern>,
}

impl SecurityGuard {
    pub fn new(policy: SecurityPolicy) -> Self {
        Self {
            policy,
            ip_connections: DashMap::new(),
            ip_auth_attempts: DashMap::new(),
            ip_requests: DashMap::new(),
            violations: DashMap::new(),
            blocked_ips: DashMap::new(),
            suspicious_patterns: Self::default_suspicious_patterns(),
        }
    }

    fn default_suspicious_patterns() -> Vec<SuspiciousPattern> {
        vec![
            SuspiciousPattern::new(
                "sql_injection",
                r"(?i)(union|select|insert|delete|drop|alter|exec|script)",
                SecuritySeverity::High,
            ),
            SuspiciousPattern::new(
                "xss_attempt",
                r"(?i)(<script|javascript:|on\w+\s*=)",
                SecuritySeverity::High,
            ),
            SuspiciousPattern::new(
                "path_traversal",
                r"(\.\./|\.\.\\|%2e%2e)",
                SecuritySeverity::Medium,
            ),
            SuspiciousPattern::new(
                "command_injection",
                r"(?i)(;|\||&|\$\(|`)",
                SecuritySeverity::High,
            ),
            SuspiciousPattern::new("null_byte", r"\x00", SecuritySeverity::Medium),
            SuspiciousPattern::new("excessive_length", r".{1000,}", SecuritySeverity::Low),
        ]
    }

    /// Check if an IP is allowed to connect
    pub fn can_connect(&self, ip: IpAddr) -> Result<()> {
        // Check if IP is currently blocked
        if let Some(blocked_at) = self.blocked_ips.get(&ip) {
            if blocked_at.elapsed() < self.policy.violation_block_time {
                return Err(StratumError::SecurityViolation {
                    message: format!("IP {ip} is temporarily blocked"),
                    severity: SecuritySeverity::High,
                });
            } else {
                // Block period expired
                self.blocked_ips.remove(&ip);
            }
        }

        // Check banned IPs
        if self.policy.banned_ips.contains(&ip) {
            self.record_violation(SecurityViolation {
                ip,
                violation_type: ViolationType::BannedIP,
                timestamp: Instant::now(),
                details: "IP is permanently banned".to_string(),
                severity: SecuritySeverity::Critical,
            });

            return Err(StratumError::SecurityViolation {
                message: format!("IP {ip} is banned"),
                severity: SecuritySeverity::Critical,
            });
        }

        // Check allowed IP ranges (if configured)
        if !self.policy.allowed_ip_ranges.is_empty() {
            let allowed = self
                .policy
                .allowed_ip_ranges
                .iter()
                .any(|range| range.contains(&ip));

            if !allowed {
                self.record_violation(SecurityViolation {
                    ip,
                    violation_type: ViolationType::UnauthorizedCountry,
                    timestamp: Instant::now(),
                    details: "IP not in allowed ranges".to_string(),
                    severity: SecuritySeverity::High,
                });

                return Err(StratumError::SecurityViolation {
                    message: format!("IP {ip} not in allowed ranges"),
                    severity: SecuritySeverity::High,
                });
            }
        }

        // Check connection limit per IP
        let current_connections = self.ip_connections.get(&ip).map(|v| *v).unwrap_or(0);

        if current_connections >= self.policy.max_connections_per_ip {
            self.record_violation(SecurityViolation {
                ip,
                violation_type: ViolationType::ExcessiveConnections,
                timestamp: Instant::now(),
                details: format!(
                    "Too many connections: {}/{}",
                    current_connections, self.policy.max_connections_per_ip
                ),
                severity: SecuritySeverity::Medium,
            });

            return Err(StratumError::SecurityViolation {
                message: format!(
                    "Too many connections from IP {}: {}/{}",
                    ip, current_connections, self.policy.max_connections_per_ip
                ),
                severity: SecuritySeverity::Medium,
            });
        }

        Ok(())
    }

    /// Register a new connection from an IP
    pub fn register_connection(&self, ip: IpAddr) -> Result<()> {
        self.can_connect(ip)?;

        let new_count = self.ip_connections.entry(ip).or_insert(0).value() + 1;
        self.ip_connections.insert(ip, new_count);

        debug!("Registered connection from {}, total: {}", ip, new_count);
        Ok(())
    }

    /// Unregister a connection from an IP
    pub fn unregister_connection(&self, ip: IpAddr) {
        if let Some(mut count) = self.ip_connections.get_mut(&ip) {
            *count = count.saturating_sub(1);
            let new_count = *count;
            drop(count);
            if new_count == 0 {
                self.ip_connections.remove(&ip);
            }
        }
    }

    /// Check if an authentication attempt is allowed
    pub fn can_authenticate(&self, ip: IpAddr) -> Result<()> {
        let now = Instant::now();
        let hour_ago = now - Duration::from_secs(3600);

        let mut attempts = self.ip_auth_attempts.entry(ip).or_default();

        // Clean old attempts
        attempts.retain(|&timestamp| timestamp > hour_ago);

        if attempts.len() >= self.policy.max_auth_attempts_per_hour as usize {
            self.record_violation(SecurityViolation {
                ip,
                violation_type: ViolationType::ExcessiveAuthAttempts,
                timestamp: now,
                details: format!(
                    "Too many auth attempts: {}/{}",
                    attempts.len(),
                    self.policy.max_auth_attempts_per_hour
                ),
                severity: SecuritySeverity::Medium,
            });

            return Err(StratumError::SecurityViolation {
                message: format!("Too many authentication attempts from IP {ip}"),
                severity: SecuritySeverity::Medium,
            });
        }

        // Record this attempt
        attempts.push(now);

        Ok(())
    }

    /// Check if a request is allowed (rate limiting)
    pub fn can_make_request(&self, ip: IpAddr) -> Result<()> {
        let now = Instant::now();
        let second_ago = now - Duration::from_secs(1);

        let mut request_times = self.ip_requests.entry(ip).or_default();

        // Clean old requests
        request_times.retain(|&timestamp| timestamp > second_ago);

        if request_times.len() >= self.policy.max_requests_per_second as usize {
            self.record_violation(SecurityViolation {
                ip,
                violation_type: ViolationType::RateLimitExceeded,
                timestamp: now,
                details: format!(
                    "Rate limit exceeded: {}/{}",
                    request_times.len(),
                    self.policy.max_requests_per_second
                ),
                severity: SecuritySeverity::Low,
            });

            return Err(StratumError::RateLimitExceeded {
                limit: self.policy.max_requests_per_second as u64,
                window: Duration::from_secs(1),
                retry_after: Duration::from_secs(1),
            });
        }

        // Record this request
        request_times.push(now);

        Ok(())
    }

    /// Validate message content for security threats
    pub fn validate_message(&self, message: &str, ip: IpAddr) -> Result<()> {
        // Check message size
        if message.len() > self.policy.max_message_size {
            self.record_violation(SecurityViolation {
                ip,
                violation_type: ViolationType::MessageTooLarge,
                timestamp: Instant::now(),
                details: format!("Message too large: {} bytes", message.len()),
                severity: SecuritySeverity::Low,
            });

            return Err(StratumError::MessageTooLarge {
                size: message.len(),
                max_size: self.policy.max_message_size,
            });
        }

        // Check for suspicious patterns
        if self.policy.enable_pattern_detection {
            for pattern in &self.suspicious_patterns {
                if pattern.matches(message) {
                    self.record_violation(SecurityViolation {
                        ip,
                        violation_type: ViolationType::SuspiciousPattern,
                        timestamp: Instant::now(),
                        details: format!("Suspicious pattern detected: {}", pattern.name),
                        severity: pattern.severity,
                    });

                    return Err(StratumError::SecurityViolation {
                        message: format!(
                            "Suspicious pattern detected in message: {}",
                            pattern.name
                        ),
                        severity: pattern.severity,
                    });
                }
            }
        }

        Ok(())
    }

    /// Record a security violation
    fn record_violation(&self, violation: SecurityViolation) {
        let ip = violation.ip;
        let severity = violation.severity;

        warn!(
            ip = %ip,
            violation_type = %violation.violation_type,
            severity = %severity,
            details = violation.details,
            "Security violation detected"
        );

        // Store violation for analysis (use IP+timestamp as unique key)
        let violation_key = format!("{}_{}", ip, violation.timestamp.elapsed().as_nanos());
        self.violations.insert(violation_key, violation);

        // Clean up old violations (older than 24 hours)
        let day_ago = Instant::now() - Duration::from_secs(86400);
        self.violations.retain(|_, v| v.timestamp > day_ago);

        // Consider blocking IP for serious violations
        match severity {
            SecuritySeverity::Critical | SecuritySeverity::High => {
                self.blocked_ips.insert(ip, Instant::now());

                error!(
                    ip = %ip,
                    severity = %severity,
                    "IP temporarily blocked due to security violation"
                );
            }
            _ => {
                // For medium/low severity, check if there are too many recent violations
                let recent_violations = self
                    .violations
                    .iter()
                    .filter(|entry| {
                        let v = entry.value();
                        v.ip == ip && v.timestamp.elapsed() < Duration::from_secs(300)
                    })
                    .count();

                if recent_violations >= 5 {
                    self.blocked_ips.insert(ip, Instant::now());

                    warn!(
                        ip = %ip,
                        violations = recent_violations,
                        "IP temporarily blocked due to multiple violations"
                    );
                }
            }
        }
    }

    /// Get security statistics
    pub fn get_statistics(&self) -> SecurityStatistics {
        let mut stats_by_type = HashMap::new();
        for entry in self.violations.iter() {
            let violation = entry.value();
            *stats_by_type
                .entry(violation.violation_type.clone())
                .or_insert(0) += 1;
        }

        let connections_by_ip: HashMap<IpAddr, u32> = self
            .ip_connections
            .iter()
            .map(|entry| (*entry.key(), *entry.value()))
            .collect();

        SecurityStatistics {
            active_connections: self.ip_connections.iter().map(|entry| *entry.value()).sum(),
            total_violations: self.violations.len(),
            blocked_ips: self.blocked_ips.len(),
            violations_by_type: stats_by_type,
            connections_by_ip,
        }
    }

    /// Clear old data and blocked IPs
    pub fn cleanup(&self) {
        let now = Instant::now();

        // Clear expired blocks
        self.blocked_ips
            .retain(|_, blocked_at| blocked_at.elapsed() < self.policy.violation_block_time);

        // Clear old violations
        let day_ago = now - Duration::from_secs(86400);
        self.violations.retain(|_, v| v.timestamp > day_ago);

        // Clear old auth attempts
        let hour_ago = now - Duration::from_secs(3600);
        self.ip_auth_attempts.retain(|_, attempts| {
            attempts.retain(|&timestamp| timestamp > hour_ago);
            !attempts.is_empty()
        });

        // Clear old request records
        let minute_ago = now - Duration::from_secs(60);
        self.ip_requests.retain(|_, request_times| {
            request_times.retain(|&timestamp| timestamp > minute_ago);
            !request_times.is_empty()
        });
    }
}

/// Suspicious pattern detector
#[derive(Debug, Clone)]
struct SuspiciousPattern {
    name: String,
    regex: regex::Regex,
    severity: SecuritySeverity,
}

impl SuspiciousPattern {
    fn new(name: &str, pattern: &str, severity: SecuritySeverity) -> Self {
        Self {
            name: name.to_string(),
            regex: regex::Regex::new(pattern).unwrap(),
            severity,
        }
    }

    fn matches(&self, text: &str) -> bool {
        self.regex.is_match(text)
    }
}

/// Security statistics for monitoring
#[derive(Debug, Clone)]
pub struct SecurityStatistics {
    pub active_connections: u32,
    pub total_violations: usize,
    pub blocked_ips: usize,
    pub violations_by_type: HashMap<ViolationType, u32>,
    pub connections_by_ip: HashMap<IpAddr, u32>,
}

/// Security validator that integrates with the validation system
pub struct SecurityValidator {
    security_guard: Arc<SecurityGuard>,
}

impl SecurityValidator {
    pub fn new(security_guard: Arc<SecurityGuard>) -> Self {
        Self { security_guard }
    }
}

#[async_trait]
impl MessageValidator for SecurityValidator {
    async fn validate(&self, message: &Value, context: &ValidationContext) -> Result<()> {
        if let Some(remote_addr) = context.remote_addr {
            let ip = remote_addr.ip();

            // Check rate limits
            self.security_guard.can_make_request(ip)?;

            // Validate message content
            let message_str = message.to_string();
            self.security_guard.validate_message(&message_str, ip)?;

            // For authentication messages, check auth rate limits
            if let Some(method) = &context.method {
                if method == "mining.authorize" {
                    self.security_guard.can_authenticate(ip)?;
                }
            }
        }

        Ok(())
    }

    fn name(&self) -> &str {
        "SecurityValidator"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, SocketAddr};

    #[test]
    fn test_ip_range() {
        let range = IpRange::new(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 0)), 24);

        assert!(range.contains(&IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))));
        assert!(range.contains(&IpAddr::V4(Ipv4Addr::new(192, 168, 0, 255))));
        assert!(!range.contains(&IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))));
        assert!(!range.contains(&IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
    }

    #[test]
    fn test_security_guard_connections() {
        let policy = SecurityPolicy {
            max_connections_per_ip: 2,
            ..SecurityPolicy::default()
        };
        let guard = SecurityGuard::new(policy);
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        // Should allow first two connections
        assert!(guard.register_connection(ip).is_ok());
        assert!(guard.register_connection(ip).is_ok());

        // Third connection should be rejected
        assert!(guard.register_connection(ip).is_err());

        // After unregistering one, should allow another
        guard.unregister_connection(ip);
        assert!(guard.register_connection(ip).is_ok());
    }

    #[test]
    fn test_security_guard_banned_ip() {
        let mut policy = SecurityPolicy::default();
        let banned_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));
        policy.banned_ips.insert(banned_ip);

        let guard = SecurityGuard::new(policy);

        assert!(guard.can_connect(banned_ip).is_err());
        assert!(
            guard
                .can_connect(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)))
                .is_ok()
        );
    }

    #[test]
    fn test_suspicious_patterns() {
        let guard = SecurityGuard::new(SecurityPolicy::default());
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        // Should detect SQL injection
        assert!(guard.validate_message("SELECT * FROM users", ip).is_err());

        // Should detect XSS
        assert!(
            guard
                .validate_message("<script>alert('xss')</script>", ip)
                .is_err()
        );

        // Should allow normal messages
        assert!(
            guard
                .validate_message("{\"id\":1,\"method\":\"mining.authorize\"}", ip)
                .is_ok()
        );
    }

    #[tokio::test]
    async fn test_security_validator() {
        let guard = Arc::new(SecurityGuard::new(SecurityPolicy::strict()));
        let validator = SecurityValidator::new(guard);

        let context = ValidationContext::new()
            .with_remote_addr(SocketAddr::new(
                IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                1234,
            ))
            .with_method("mining.authorize".to_string());

        let message = serde_json::json!({
            "id": 1,
            "method": "mining.authorize",
            "params": ["user", "pass"]
        });

        assert!(validator.validate(&message, &context).await.is_ok());

        // Test malicious message
        let malicious_message = serde_json::json!({
            "id": 1,
            "method": "mining.authorize",
            "params": ["<script>alert('xss')</script>", "pass"]
        });

        assert!(
            validator
                .validate(&malicious_message, &context)
                .await
                .is_err()
        );
    }
}
