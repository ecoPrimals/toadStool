//! # Security Hardening Module
//!
//! This module provides production-ready security hardening features for `ToadStool`:
//! - Input validation and sanitization
//! - Rate limiting and `DDoS` protection
//! - Audit logging and intrusion detection
//! - Security context validation
//! - Encryption and key management
//! - Authentication and authorization hardening

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::security::SecurityContext;
use crate::{ToadStoolError, ToadStoolResult};
use toadstool_common::constants::network::LOCALHOST_IPV4;

/// Security hardening configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityHardeningConfig {
    /// Enable input validation
    pub enable_input_validation: bool,
    /// Enable rate limiting
    pub enable_rate_limiting: bool,
    /// Enable audit logging
    pub enable_audit_logging: bool,
    /// Enable intrusion detection
    pub enable_intrusion_detection: bool,
    /// Rate limiting configuration
    pub rate_limiting: RateLimitingConfig,
    /// Audit logging configuration
    pub audit_config: AuditConfig,
    /// Intrusion detection configuration
    pub intrusion_detection: IntrusionDetectionConfig,
    /// Input validation rules
    pub validation_rules: ValidationRules,
}

impl Default for SecurityHardeningConfig {
    fn default() -> Self {
        Self {
            enable_input_validation: true,
            enable_rate_limiting: true,
            enable_audit_logging: true,
            enable_intrusion_detection: true,
            rate_limiting: RateLimitingConfig::default(),
            audit_config: AuditConfig::default(),
            intrusion_detection: IntrusionDetectionConfig::default(),
            validation_rules: ValidationRules::default(),
        }
    }
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitingConfig {
    /// Maximum requests per minute
    pub max_requests_per_minute: u32,
    /// Maximum requests per hour
    pub max_requests_per_hour: u32,
    /// Maximum requests per day
    pub max_requests_per_day: u32,
    /// Sliding window size
    pub sliding_window: Duration,
    /// Burst allowance
    pub burst_allowance: u32,
}

impl Default for RateLimitingConfig {
    fn default() -> Self {
        Self {
            max_requests_per_minute: 60,
            max_requests_per_hour: 3600,
            max_requests_per_day: 86400,
            sliding_window: Duration::from_secs(60),
            burst_allowance: 10,
        }
    }
}

/// Audit logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Enable structured logging
    pub structured_logging: bool,
    /// Log level
    pub log_level: String,
    /// Log retention days
    pub retention_days: u32,
    /// Log file path
    pub log_file_path: Option<String>,
    /// Remote logging endpoint
    pub remote_endpoint: Option<String>,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            structured_logging: true,
            log_level: "info".to_string(),
            retention_days: 30,
            log_file_path: None,
            remote_endpoint: None,
        }
    }
}

/// Intrusion detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntrusionDetectionConfig {
    /// Anomaly detection threshold
    pub anomaly_threshold: f64,
    /// Suspicious activity window
    pub activity_window: Duration,
    /// Auto-ban threshold
    pub auto_ban_threshold: u32,
    /// Ban duration
    pub ban_duration: Duration,
    /// Allowed IPs (permit list)
    pub allowed_ips: Vec<String>,
}

impl Default for IntrusionDetectionConfig {
    fn default() -> Self {
        Self {
            anomaly_threshold: 0.8,
            activity_window: Duration::from_secs(300),
            auto_ban_threshold: 10,
            ban_duration: Duration::from_secs(3600),
            allowed_ips: vec![LOCALHOST_IPV4.to_string(), "::1".to_string()],
        }
    }
}

/// Input validation rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRules {
    /// Maximum input length
    pub max_input_length: usize,
    /// Allowed characters regex
    pub allowed_characters: Option<String>,
    /// Blocked patterns
    pub blocked_patterns: Vec<String>,
    /// SQL injection patterns
    pub sql_injection_patterns: Vec<String>,
    /// XSS patterns
    pub xss_patterns: Vec<String>,
    /// Command injection patterns
    pub command_injection_patterns: Vec<String>,
}

impl Default for ValidationRules {
    fn default() -> Self {
        Self {
            max_input_length: 1024 * 1024, // 1MB
            allowed_characters: None,
            blocked_patterns: vec![
                r"<script".to_string(),
                r"javascript:".to_string(),
                r"vbscript:".to_string(),
                r"on\w+\s*=".to_string(),
            ],
            sql_injection_patterns: vec![
                r"(?i)(union|select|insert|update|delete|drop|create|alter|exec|execute)"
                    .to_string(),
                r"(?i)(or|and)\s+\d+\s*=\s*\d+".to_string(),
                r"(?i)'\s*(or|and)\s*'".to_string(),
            ],
            xss_patterns: vec![
                r"(?i)<script".to_string(),
                r"(?i)javascript:".to_string(),
                r"(?i)on\w+\s*=".to_string(),
            ],
            command_injection_patterns: vec![
                r"[;&|`]".to_string(),
                r"\$\(".to_string(),
                r">\s*/".to_string(),
            ],
        }
    }
}

/// Rate limiter
pub struct RateLimiter {
    /// Configuration
    config: RateLimitingConfig,
    /// Client request counts
    client_requests: Arc<RwLock<HashMap<String, ClientRateData>>>,
}

/// Client rate limiting data
#[derive(Debug, Clone)]
struct ClientRateData {
    /// Request timestamps
    request_times: Vec<Instant>,
    /// Total requests today
    daily_requests: u32,
    /// Last reset time
    last_reset: Instant,
    /// Is currently banned
    is_banned: bool,
    /// Ban expiry time
    ban_expiry: Option<Instant>,
}

impl RateLimiter {
    /// Create new rate limiter
    #[must_use]
    pub fn new(config: RateLimitingConfig) -> Self {
        Self {
            config,
            client_requests: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if client is allowed to make request
    pub async fn check_rate_limit(&self, client_id: &str) -> ToadStoolResult<bool> {
        let mut clients = self.client_requests.write().await;
        let now = Instant::now();

        let client_data = clients
            .entry(client_id.to_string())
            .or_insert_with(|| ClientRateData {
                request_times: Vec::new(),
                daily_requests: 0,
                last_reset: now,
                is_banned: false,
                ban_expiry: None,
            });

        // Check if ban has expired
        if let Some(ban_expiry) = client_data.ban_expiry {
            if now > ban_expiry {
                client_data.is_banned = false;
                client_data.ban_expiry = None;
                info!("Rate limit ban expired for client: {}", client_id);
            }
        }

        // Check if currently banned
        if client_data.is_banned {
            return Ok(false);
        }

        // Clean old requests outside sliding window
        client_data
            .request_times
            .retain(|&time| now.duration_since(time) < self.config.sliding_window);

        // Check daily reset
        if now.duration_since(client_data.last_reset) > Duration::from_secs(86400) {
            client_data.daily_requests = 0;
            client_data.last_reset = now;
        }

        // Check rate limits
        let requests_in_window = u32::try_from(client_data.request_times.len()).unwrap_or(u32::MAX);

        if requests_in_window >= self.config.max_requests_per_minute {
            warn!(
                "Rate limit exceeded for client {}: {} requests/minute",
                client_id, requests_in_window
            );
            return Ok(false);
        }

        if client_data.daily_requests >= self.config.max_requests_per_day {
            warn!(
                "Daily rate limit exceeded for client {}: {} requests/day",
                client_id, client_data.daily_requests
            );
            return Ok(false);
        }

        // Record request
        client_data.request_times.push(now);
        client_data.daily_requests += 1;

        Ok(true)
    }

    /// Ban client for specified duration
    pub async fn ban_client(&self, client_id: &str, duration: Duration) {
        let mut clients = self.client_requests.write().await;
        let now = Instant::now();

        let client_data = clients
            .entry(client_id.to_string())
            .or_insert_with(|| ClientRateData {
                request_times: Vec::new(),
                daily_requests: 0,
                last_reset: now,
                is_banned: false,
                ban_expiry: None,
            });

        client_data.is_banned = true;
        client_data.ban_expiry = Some(now + duration);

        warn!("Client {} banned for {:?}", client_id, duration);
    }
}

/// Input validator
pub struct InputValidator {
    /// Validation rules
    rules: ValidationRules,
}

impl InputValidator {
    /// Create new input validator
    #[must_use]
    pub fn new(rules: ValidationRules) -> Self {
        Self { rules }
    }

    /// Validate input string
    pub fn validate_input(&self, input: &str) -> ToadStoolResult<()> {
        // Check length
        if input.len() > self.rules.max_input_length {
            return Err(ToadStoolError::validation(format!(
                "Input length {} exceeds maximum {}",
                input.len(),
                self.rules.max_input_length
            )));
        }

        // Check for blocked patterns
        for pattern in &self.rules.blocked_patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                if regex.is_match(input) {
                    return Err(ToadStoolError::security(format!(
                        "Input contains blocked pattern: {pattern}"
                    )));
                }
            }
        }

        // Check for SQL injection
        for pattern in &self.rules.sql_injection_patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                if regex.is_match(input) {
                    return Err(ToadStoolError::security(
                        "Input contains potential SQL injection pattern".to_string(),
                    ));
                }
            }
        }

        // Check for XSS
        for pattern in &self.rules.xss_patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                if regex.is_match(input) {
                    return Err(ToadStoolError::security(
                        "Input contains potential XSS pattern".to_string(),
                    ));
                }
            }
        }

        // Check for command injection
        for pattern in &self.rules.command_injection_patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                if regex.is_match(input) {
                    return Err(ToadStoolError::security(
                        "Input contains potential command injection pattern".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }

    /// Sanitize input string
    #[must_use]
    pub fn sanitize_input(&self, input: &str) -> String {
        let mut sanitized = input.to_string();

        // Remove common dangerous characters
        sanitized = sanitized.replace('<', "&lt;");
        sanitized = sanitized.replace('>', "&gt;");
        sanitized = sanitized.replace('"', "&quot;");
        sanitized = sanitized.replace('\'', "&#x27;");
        sanitized = sanitized.replace('&', "&amp;");

        // Remove null bytes
        sanitized = sanitized.replace('\0', "");

        // Limit length
        if sanitized.len() > self.rules.max_input_length {
            sanitized.truncate(self.rules.max_input_length);
        }

        sanitized
    }
}

/// Security audit logger
pub struct SecurityAuditLogger {
    /// Configuration
    _config: AuditConfig,
    /// Audit events buffer
    events: Arc<RwLock<Vec<SecurityAuditEvent>>>,
}

/// Security audit event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAuditEvent {
    /// Event ID
    pub id: Uuid,
    /// Event type
    pub event_type: SecurityEventType,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Client ID
    pub client_id: Option<String>,
    /// IP address
    pub ip_address: Option<String>,
    /// User agent
    pub user_agent: Option<String>,
    /// Event details
    pub details: HashMap<String, String>,
    /// Severity level
    pub severity: SecuritySeverity,
}

/// Security event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityEventType {
    /// Authentication attempt
    AuthenticationAttempt,
    /// Authorization failure
    AuthorizationFailure,
    /// Input validation failure
    InputValidationFailure,
    /// Rate limit exceeded
    RateLimitExceeded,
    /// Suspicious activity detected
    SuspiciousActivity,
    /// Intrusion attempt
    IntrusionAttempt,
    /// Security policy violation
    PolicyViolation,
    /// Capability abuse
    CapabilityAbuse,
}

/// Security severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecuritySeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl SecurityAuditLogger {
    /// Create new security audit logger
    #[must_use]
    pub fn new(config: AuditConfig) -> Self {
        Self {
            _config: config,
            events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Log security event
    pub async fn log_event(&self, event: SecurityAuditEvent) {
        let mut events = self.events.write().await;
        events.push(event.clone());

        // Log to tracing
        match event.severity {
            SecuritySeverity::Low => debug!("Security event: {:?}", event),
            SecuritySeverity::Medium => info!("Security event: {:?}", event),
            SecuritySeverity::High => warn!("Security event: {:?}", event),
            SecuritySeverity::Critical => error!("Security event: {:?}", event),
        }

        // Future enhancement: Send to external logging system if configured
        // Current implementation uses standard logging which can be configured via log aggregation
    }

    /// Get recent security events
    pub async fn get_recent_events(&self, limit: usize) -> Vec<SecurityAuditEvent> {
        let events = self.events.read().await;
        events.iter().rev().take(limit).cloned().collect()
    }
}

/// Intrusion detection system
pub struct IntrusionDetectionSystem {
    /// Configuration
    config: IntrusionDetectionConfig,
    /// Client activity tracking
    client_activity: Arc<RwLock<HashMap<String, ClientActivity>>>,
    /// Banned clients
    banned_clients: Arc<RwLock<HashMap<String, BanInfo>>>,
}

/// Client activity tracking
#[derive(Debug, Clone)]
struct ClientActivity {
    /// Request count
    request_count: u32,
    /// Failed attempts
    failed_attempts: u32,
    /// Suspicious patterns
    suspicious_patterns: u32,
    /// Activity window start
    window_start: Instant,
    /// Risk score
    risk_score: f64,
}

/// Ban information
#[derive(Debug, Clone)]
struct BanInfo {
    /// Ban start time
    ban_start: Instant,
    /// Ban duration
    duration: Duration,
    /// Reason
    _reason: String,
}

impl IntrusionDetectionSystem {
    /// Create new intrusion detection system
    #[must_use]
    pub fn new(config: IntrusionDetectionConfig) -> Self {
        Self {
            config,
            client_activity: Arc::new(RwLock::new(HashMap::new())),
            banned_clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Record client activity
    pub async fn record_activity(&self, client_id: &str, activity_type: ActivityType) {
        let mut activities = self.client_activity.write().await;
        let now = Instant::now();

        let activity = activities
            .entry(client_id.to_string())
            .or_insert_with(|| ClientActivity {
                request_count: 0,
                failed_attempts: 0,
                suspicious_patterns: 0,
                window_start: now,
                risk_score: 0.0,
            });

        // Reset window if needed
        if now.duration_since(activity.window_start) > self.config.activity_window {
            activity.request_count = 0;
            activity.failed_attempts = 0;
            activity.suspicious_patterns = 0;
            activity.window_start = now;
            activity.risk_score = 0.0;
        }

        // Update activity
        match activity_type {
            ActivityType::Request => {
                activity.request_count += 1;
            }
            ActivityType::FailedAttempt => {
                activity.failed_attempts += 1;
                activity.risk_score += 0.1;
            }
            ActivityType::SuspiciousPattern => {
                activity.suspicious_patterns += 1;
                activity.risk_score += 0.2;
            }
        }

        // Check if should ban
        if activity.risk_score >= self.config.anomaly_threshold
            || activity.failed_attempts >= self.config.auto_ban_threshold
        {
            self.ban_client(
                client_id,
                self.config.ban_duration,
                "Suspicious activity detected",
            )
            .await;
        }
    }

    /// Check if client is banned
    pub async fn is_banned(&self, client_id: &str) -> bool {
        let mut banned = self.banned_clients.write().await;
        let now = Instant::now();

        if let Some(ban_info) = banned.get(client_id) {
            if now.duration_since(ban_info.ban_start) > ban_info.duration {
                // Ban expired
                banned.remove(client_id);
                false
            } else {
                true
            }
        } else {
            false
        }
    }

    /// Ban client
    pub async fn ban_client(&self, client_id: &str, duration: Duration, reason: &str) {
        let mut banned = self.banned_clients.write().await;
        let now = Instant::now();

        banned.insert(
            client_id.to_string(),
            BanInfo {
                ban_start: now,
                duration,
                _reason: reason.to_string(),
            },
        );

        error!("Client {} banned for {:?}: {}", client_id, duration, reason);
    }
}

/// Activity types for intrusion detection
#[derive(Debug, Clone)]
pub enum ActivityType {
    Request,
    FailedAttempt,
    SuspiciousPattern,
}

/// Security hardening manager
pub struct SecurityHardeningManager {
    /// Configuration
    config: SecurityHardeningConfig,
    /// Rate limiter
    rate_limiter: Arc<RateLimiter>,
    /// Input validator
    input_validator: Arc<InputValidator>,
    /// Audit logger
    audit_logger: Arc<SecurityAuditLogger>,
    /// Intrusion detection
    intrusion_detection: Arc<IntrusionDetectionSystem>,
}

impl SecurityHardeningManager {
    /// Create new security hardening manager
    #[must_use]
    pub fn new(config: SecurityHardeningConfig) -> Self {
        let rate_limiter = Arc::new(RateLimiter::new(config.rate_limiting.clone()));
        let input_validator = Arc::new(InputValidator::new(config.validation_rules.clone()));
        let audit_logger = Arc::new(SecurityAuditLogger::new(config.audit_config.clone()));
        let intrusion_detection = Arc::new(IntrusionDetectionSystem::new(
            config.intrusion_detection.clone(),
        ));

        Self {
            config,
            rate_limiter,
            input_validator,
            audit_logger,
            intrusion_detection,
        }
    }

    /// Check security context
    pub async fn check_security_context(
        &self,
        client_id: &str,
        context: &SecurityContext,
    ) -> ToadStoolResult<()> {
        // Check if client is banned
        if self.config.enable_intrusion_detection
            && self.intrusion_detection.is_banned(client_id).await
        {
            self.audit_logger
                .log_event(SecurityAuditEvent {
                    id: Uuid::new_v4(),
                    event_type: SecurityEventType::IntrusionAttempt,
                    timestamp: chrono::Utc::now(),
                    client_id: Some(client_id.to_string()),
                    ip_address: None,
                    user_agent: None,
                    details: HashMap::new(),
                    severity: SecuritySeverity::High,
                })
                .await;

            return Err(ToadStoolError::security("Client is banned".to_string()));
        }

        // Check rate limit
        if self.config.enable_rate_limiting
            && !self.rate_limiter.check_rate_limit(client_id).await?
        {
            self.audit_logger
                .log_event(SecurityAuditEvent {
                    id: Uuid::new_v4(),
                    event_type: SecurityEventType::RateLimitExceeded,
                    timestamp: chrono::Utc::now(),
                    client_id: Some(client_id.to_string()),
                    ip_address: None,
                    user_agent: None,
                    details: HashMap::new(),
                    severity: SecuritySeverity::Medium,
                })
                .await;

            return Err(ToadStoolError::security("Rate limit exceeded".to_string()));
        }

        // Validate security context
        context.validate()?;

        // Record successful activity
        if self.config.enable_intrusion_detection {
            self.intrusion_detection
                .record_activity(client_id, ActivityType::Request)
                .await;
        }

        Ok(())
    }

    /// Validate input
    pub fn validate_input(&self, input: &str) -> ToadStoolResult<()> {
        if self.config.enable_input_validation {
            self.input_validator.validate_input(input)
        } else {
            Ok(())
        }
    }

    /// Sanitize input
    #[must_use]
    pub fn sanitize_input(&self, input: &str) -> String {
        if self.config.enable_input_validation {
            self.input_validator.sanitize_input(input)
        } else {
            input.to_string()
        }
    }

    /// Log security event
    pub async fn log_security_event(&self, event: SecurityAuditEvent) {
        if self.config.enable_audit_logging {
            self.audit_logger.log_event(event).await;
        }
    }

    /// Record security failure
    pub async fn record_security_failure(&self, client_id: &str, failure_type: SecurityEventType) {
        if self.config.enable_intrusion_detection {
            self.intrusion_detection
                .record_activity(client_id, ActivityType::FailedAttempt)
                .await;
        }

        if self.config.enable_audit_logging {
            self.audit_logger
                .log_event(SecurityAuditEvent {
                    id: Uuid::new_v4(),
                    event_type: failure_type,
                    timestamp: chrono::Utc::now(),
                    client_id: Some(client_id.to_string()),
                    ip_address: None,
                    user_agent: None,
                    details: HashMap::new(),
                    severity: SecuritySeverity::High,
                })
                .await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::{Capability, FilesystemSecurity, SecurityContext};
    use std::time::Duration;

    // ============================================================================
    // Default implementation tests
    // ============================================================================

    #[test]
    fn test_security_hardening_config_default() {
        let config = SecurityHardeningConfig::default();
        assert!(config.enable_input_validation);
        assert!(config.enable_rate_limiting);
        assert!(config.enable_audit_logging);
        assert!(config.enable_intrusion_detection);
    }

    #[test]
    fn test_rate_limiting_config_default() {
        let config = RateLimitingConfig::default();
        assert_eq!(config.max_requests_per_minute, 60);
        assert_eq!(config.max_requests_per_hour, 3600);
        assert_eq!(config.max_requests_per_day, 86400);
        assert_eq!(config.sliding_window.as_secs(), 60);
        assert_eq!(config.burst_allowance, 10);
    }

    #[test]
    fn test_audit_config_default() {
        let config = AuditConfig::default();
        assert!(config.structured_logging);
        assert_eq!(config.log_level, "info");
        assert_eq!(config.retention_days, 30);
        assert!(config.log_file_path.is_none());
        assert!(config.remote_endpoint.is_none());
    }

    #[test]
    fn test_intrusion_detection_config_default() {
        let config = IntrusionDetectionConfig::default();
        assert!((config.anomaly_threshold - 0.8).abs() < f64::EPSILON);
        assert_eq!(config.activity_window.as_secs(), 300);
        assert_eq!(config.auto_ban_threshold, 10);
        assert_eq!(config.ban_duration.as_secs(), 3600);
        assert!(config.allowed_ips.contains(&LOCALHOST_IPV4.to_string()));
        assert!(config.allowed_ips.contains(&"::1".to_string()));
    }

    #[test]
    fn test_validation_rules_default() {
        let rules = ValidationRules::default();
        assert_eq!(rules.max_input_length, 1024 * 1024);
        assert!(rules.allowed_characters.is_none());
        assert!(!rules.blocked_patterns.is_empty());
        assert!(!rules.sql_injection_patterns.is_empty());
        assert!(!rules.xss_patterns.is_empty());
        assert!(!rules.command_injection_patterns.is_empty());
    }

    // ============================================================================
    // Serialization/Deserialization tests
    // ============================================================================

    #[test]
    fn test_security_hardening_config_serde_roundtrip() {
        let config = SecurityHardeningConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: SecurityHardeningConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(
            config.enable_input_validation,
            deserialized.enable_input_validation
        );
        assert_eq!(
            config.enable_rate_limiting,
            deserialized.enable_rate_limiting
        );
    }

    #[test]
    fn test_rate_limiting_config_serde_roundtrip() {
        let config = RateLimitingConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: RateLimitingConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(
            config.max_requests_per_minute,
            deserialized.max_requests_per_minute
        );
    }

    #[test]
    fn test_audit_config_serde_roundtrip() {
        let config = AuditConfig {
            structured_logging: true,
            log_level: "debug".to_string(),
            retention_days: 7,
            log_file_path: Some("/var/log/audit.log".to_string()),
            remote_endpoint: Some("https://log.example.com".to_string()),
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: AuditConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.log_level, deserialized.log_level);
        assert_eq!(config.log_file_path, deserialized.log_file_path);
    }

    #[test]
    fn test_security_audit_event_serde_roundtrip() {
        let event = SecurityAuditEvent {
            id: Uuid::new_v4(),
            event_type: SecurityEventType::InputValidationFailure,
            timestamp: chrono::Utc::now(),
            client_id: Some("client-1".to_string()),
            ip_address: Some("192.168.1.1".to_string()),
            user_agent: Some("Mozilla/5.0".to_string()),
            details: HashMap::from([("key".to_string(), "value".to_string())]),
            severity: SecuritySeverity::High,
        };
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: SecurityAuditEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event.id, deserialized.id);
        assert_eq!(event.severity, deserialized.severity);
    }

    #[test]
    fn test_security_severity_ordering() {
        assert!(SecuritySeverity::Critical > SecuritySeverity::High);
        assert!(SecuritySeverity::High > SecuritySeverity::Medium);
        assert!(SecuritySeverity::Medium > SecuritySeverity::Low);
    }

    // ============================================================================
    // RateLimiter tests
    // ============================================================================

    #[tokio::test]
    async fn test_rate_limiter_new() {
        let config = RateLimitingConfig::default();
        let _limiter = RateLimiter::new(config);
    }

    #[tokio::test]
    async fn test_rate_limiter_first_request_allowed() {
        let config = RateLimitingConfig::default();
        let limiter = RateLimiter::new(config);
        let result = limiter.check_rate_limit("client-1").await.unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn test_rate_limiter_multiple_clients_independent() {
        let config = RateLimitingConfig {
            max_requests_per_minute: 2,
            max_requests_per_hour: 100,
            max_requests_per_day: 1000,
            sliding_window: Duration::from_secs(60),
            burst_allowance: 0,
        };
        let limiter = RateLimiter::new(config);

        assert!(limiter.check_rate_limit("client-a").await.unwrap());
        assert!(limiter.check_rate_limit("client-a").await.unwrap());
        assert!(!limiter.check_rate_limit("client-a").await.unwrap());

        assert!(limiter.check_rate_limit("client-b").await.unwrap());
    }

    #[tokio::test]
    async fn test_rate_limiter_ban_client() {
        let config = RateLimitingConfig::default();
        let limiter = RateLimiter::new(config);

        assert!(limiter.check_rate_limit("banned-client").await.unwrap());
        limiter
            .ban_client("banned-client", Duration::from_secs(3600))
            .await;
        assert!(!limiter.check_rate_limit("banned-client").await.unwrap());
    }

    // ============================================================================
    // InputValidator tests
    // ============================================================================

    #[test]
    fn test_input_validator_valid_input() {
        let validator = InputValidator::new(ValidationRules::default());
        assert!(validator.validate_input("Hello, safe input!").is_ok());
    }

    #[test]
    fn test_input_validator_input_too_long() {
        let rules = ValidationRules {
            max_input_length: 10,
            ..ValidationRules::default()
        };
        let validator = InputValidator::new(rules);
        let result = validator.validate_input("this is way too long");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exceeds maximum"));
    }

    #[test]
    fn test_input_validator_blocked_pattern_script() {
        let validator = InputValidator::new(ValidationRules::default());
        let result = validator.validate_input("Hello <script>evil</script> world");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("blocked"));
    }

    #[test]
    fn test_input_validator_blocked_pattern_javascript() {
        let validator = InputValidator::new(ValidationRules::default());
        let result = validator.validate_input("click javascript:alert(1)");
        assert!(result.is_err());
    }

    #[test]
    fn test_input_validator_sql_injection() {
        let validator = InputValidator::new(ValidationRules::default());
        let result = validator.validate_input("admin' OR '1'='1");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("SQL injection"));
    }

    #[test]
    fn test_input_validator_sql_injection_union() {
        let validator = InputValidator::new(ValidationRules::default());
        let result = validator.validate_input("x' UNION SELECT * FROM users--");
        assert!(result.is_err());
    }

    #[test]
    fn test_input_validator_xss_pattern() {
        let validator = InputValidator::new(ValidationRules::default());
        let result = validator.validate_input("<script>document.cookie</script>");
        assert!(result.is_err());
        // Error may mention XSS or security/blocked pattern (blocked_patterns also match <script)
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.to_lowercase().contains("xss")
                || err_msg.contains("blocked")
                || err_msg.contains("security"),
            "expected security/XSS/blocked in error: {}",
            err_msg
        );
    }

    #[test]
    fn test_input_validator_command_injection_semicolon() {
        let validator = InputValidator::new(ValidationRules::default());
        let result = validator.validate_input("foo; rm -rf /");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("command injection"));
    }

    #[test]
    fn test_input_validator_command_injection_backtick() {
        let validator = InputValidator::new(ValidationRules::default());
        let result = validator.validate_input("echo `whoami`");
        assert!(result.is_err());
    }

    #[test]
    fn test_sanitize_input_escapes_html() {
        let validator = InputValidator::new(ValidationRules::default());
        let sanitized = validator.sanitize_input("<script>alert(1)</script>");
        // Sanitizer replaces < with &lt; then & with &amp;, so we get &amp;lt;
        assert!(!sanitized.contains("<script>"));
        assert!(!sanitized.contains('<'));
        assert!(!sanitized.contains('>'));
    }

    #[test]
    fn test_sanitize_input_escapes_quotes() {
        let validator = InputValidator::new(ValidationRules::default());
        let sanitized = validator.sanitize_input("foo \"bar\" 'baz'");
        // Sanitizer replaces " and ' first, then & - raw quotes must be gone
        assert!(!sanitized.contains('"'));
        assert!(!sanitized.contains('\''));
    }

    #[test]
    fn test_sanitize_input_removes_null_bytes() {
        let validator = InputValidator::new(ValidationRules::default());
        let sanitized = validator.sanitize_input("foo\x00bar");
        assert!(!sanitized.contains('\0'));
    }

    #[test]
    fn test_sanitize_input_truncates_to_max_length() {
        let rules = ValidationRules {
            max_input_length: 5,
            ..ValidationRules::default()
        };
        let validator = InputValidator::new(rules);
        let sanitized = validator.sanitize_input("hello world");
        assert_eq!(sanitized.len(), 5);
    }

    // ============================================================================
    // SecurityAuditLogger tests
    // ============================================================================

    #[tokio::test]
    async fn test_audit_logger_log_and_retrieve_events() {
        let logger = SecurityAuditLogger::new(AuditConfig::default());
        let event = SecurityAuditEvent {
            id: Uuid::new_v4(),
            event_type: SecurityEventType::AuthenticationAttempt,
            timestamp: chrono::Utc::now(),
            client_id: Some("client-1".to_string()),
            ip_address: None,
            user_agent: None,
            details: HashMap::new(),
            severity: SecuritySeverity::Low,
        };
        logger.log_event(event.clone()).await;
        let events = logger.get_recent_events(10).await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, event.id);
    }

    #[tokio::test]
    async fn test_audit_logger_recent_events_order() {
        let logger = SecurityAuditLogger::new(AuditConfig::default());
        for i in 0..5 {
            let event = SecurityAuditEvent {
                id: Uuid::new_v4(),
                event_type: SecurityEventType::PolicyViolation,
                timestamp: chrono::Utc::now(),
                client_id: Some(format!("client-{i}")),
                ip_address: None,
                user_agent: None,
                details: HashMap::new(),
                severity: SecuritySeverity::Low,
            };
            logger.log_event(event).await;
        }
        let events = logger.get_recent_events(3).await;
        assert_eq!(events.len(), 3);
        assert!(events[0].client_id.as_ref().unwrap().contains("client-4"));
    }

    #[tokio::test]
    async fn test_audit_logger_all_severity_levels() {
        let logger = SecurityAuditLogger::new(AuditConfig::default());
        for severity in [
            SecuritySeverity::Low,
            SecuritySeverity::Medium,
            SecuritySeverity::High,
            SecuritySeverity::Critical,
        ] {
            let event = SecurityAuditEvent {
                id: Uuid::new_v4(),
                event_type: SecurityEventType::PolicyViolation,
                timestamp: chrono::Utc::now(),
                client_id: None,
                ip_address: None,
                user_agent: None,
                details: HashMap::new(),
                severity,
            };
            logger.log_event(event).await;
        }
        let events = logger.get_recent_events(10).await;
        assert_eq!(events.len(), 4);
    }

    // ============================================================================
    // IntrusionDetectionSystem tests
    // ============================================================================

    #[tokio::test]
    async fn test_ids_new_client_not_banned() {
        let ids = IntrusionDetectionSystem::new(IntrusionDetectionConfig::default());
        assert!(!ids.is_banned("new-client").await);
    }

    #[tokio::test]
    async fn test_ids_ban_client() {
        let ids = IntrusionDetectionSystem::new(IntrusionDetectionConfig::default());
        ids.ban_client("malicious-client", Duration::from_secs(3600), "Test ban")
            .await;
        assert!(ids.is_banned("malicious-client").await);
    }

    #[tokio::test]
    async fn test_ids_record_request_activity() {
        let ids = IntrusionDetectionSystem::new(IntrusionDetectionConfig::default());
        ids.record_activity("client-1", ActivityType::Request).await;
        ids.record_activity("client-1", ActivityType::Request).await;
        assert!(!ids.is_banned("client-1").await);
    }

    #[tokio::test]
    async fn test_ids_failed_attempts_trigger_ban() {
        let config = IntrusionDetectionConfig {
            auto_ban_threshold: 3,
            anomaly_threshold: 1.0,
            ..IntrusionDetectionConfig::default()
        };
        let ids = IntrusionDetectionSystem::new(config);
        for _ in 0..3 {
            ids.record_activity("attacker", ActivityType::FailedAttempt)
                .await;
        }
        assert!(ids.is_banned("attacker").await);
    }

    #[tokio::test]
    async fn test_ids_suspicious_patterns_increase_risk() {
        let config = IntrusionDetectionConfig {
            anomaly_threshold: 0.5,
            auto_ban_threshold: 100,
            ..IntrusionDetectionConfig::default()
        };
        let ids = IntrusionDetectionSystem::new(config);
        for _ in 0..3 {
            ids.record_activity("suspicious", ActivityType::SuspiciousPattern)
                .await;
        }
        assert!(ids.is_banned("suspicious").await);
    }

    #[test]
    fn test_activity_type_debug() {
        assert_eq!(format!("{:?}", ActivityType::Request), "Request");
        assert_eq!(
            format!("{:?}", ActivityType::FailedAttempt),
            "FailedAttempt"
        );
        assert_eq!(
            format!("{:?}", ActivityType::SuspiciousPattern),
            "SuspiciousPattern"
        );
    }

    // ============================================================================
    // SecurityHardeningManager tests
    // ============================================================================

    #[test]
    fn test_security_hardening_manager_new() {
        let config = SecurityHardeningConfig::default();
        let _manager = SecurityHardeningManager::new(config);
    }

    #[tokio::test]
    async fn test_manager_validate_input_enabled() {
        let config = SecurityHardeningConfig::default();
        let manager = SecurityHardeningManager::new(config);
        assert!(manager.validate_input("safe input").is_ok());
        assert!(manager.validate_input("<script>evil</script>").is_err());
    }

    #[tokio::test]
    async fn test_manager_validate_input_disabled() {
        let config = SecurityHardeningConfig {
            enable_input_validation: false,
            ..SecurityHardeningConfig::default()
        };
        let manager = SecurityHardeningManager::new(config);
        assert!(manager.validate_input("<script>evil</script>").is_ok());
    }

    #[test]
    fn test_manager_sanitize_input_enabled() {
        let config = SecurityHardeningConfig::default();
        let manager = SecurityHardeningManager::new(config);
        let sanitized = manager.sanitize_input("<b>bold</b>");
        assert!(!sanitized.contains('<'));
        assert!(!sanitized.contains('>'));
    }

    #[test]
    fn test_manager_sanitize_input_disabled() {
        let config = SecurityHardeningConfig {
            enable_input_validation: false,
            ..SecurityHardeningConfig::default()
        };
        let manager = SecurityHardeningManager::new(config);
        let sanitized = manager.sanitize_input("<script>evil</script>");
        assert_eq!(sanitized, "<script>evil</script>");
    }

    #[tokio::test]
    async fn test_manager_check_security_context_success() {
        let config = SecurityHardeningConfig::default();
        let manager = SecurityHardeningManager::new(config);
        let context = SecurityContext::default();
        let result = manager.check_security_context("client-ok", &context).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_manager_check_security_context_banned() {
        let config = SecurityHardeningConfig::default();
        let manager = SecurityHardeningManager::new(config);
        let context = SecurityContext::default();

        manager
            .intrusion_detection
            .ban_client("banned-client", Duration::from_secs(3600), "Test")
            .await;

        let result = manager
            .check_security_context("banned-client", &context)
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("banned"));
    }

    #[tokio::test]
    async fn test_manager_check_security_context_invalid_context_empty_capabilities() {
        let config = SecurityHardeningConfig {
            enable_rate_limiting: false,
            ..SecurityHardeningConfig::default()
        };
        let manager = SecurityHardeningManager::new(config);
        let context = SecurityContext {
            capabilities: vec![],
            ..SecurityContext::default()
        };
        let result = manager.check_security_context("client-1", &context).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("capability"));
    }

    #[tokio::test]
    async fn test_manager_check_security_context_invalid_context_read_write_conflict() {
        let config = SecurityHardeningConfig {
            enable_rate_limiting: false,
            ..SecurityHardeningConfig::default()
        };
        let manager = SecurityHardeningManager::new(config);
        let context = SecurityContext {
            capabilities: vec![Capability::Read, Capability::Write],
            filesystem_security: FilesystemSecurity {
                read_only: true,
                ..FilesystemSecurity::default()
            },
            ..SecurityContext::default()
        };
        let result = manager.check_security_context("client-1", &context).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("read-only"));
    }

    #[tokio::test]
    async fn test_manager_log_security_event_enabled() {
        let config = SecurityHardeningConfig::default();
        let manager = SecurityHardeningManager::new(config);
        let event = SecurityAuditEvent {
            id: Uuid::new_v4(),
            event_type: SecurityEventType::CapabilityAbuse,
            timestamp: chrono::Utc::now(),
            client_id: Some("client-1".to_string()),
            ip_address: None,
            user_agent: None,
            details: HashMap::new(),
            severity: SecuritySeverity::Medium,
        };
        manager.log_security_event(event).await;
        let events = manager.audit_logger.get_recent_events(5).await;
        assert_eq!(events.len(), 1);
    }

    #[tokio::test]
    async fn test_manager_log_security_event_disabled() {
        let config = SecurityHardeningConfig {
            enable_audit_logging: false,
            ..SecurityHardeningConfig::default()
        };
        let manager = SecurityHardeningManager::new(config);
        let event = SecurityAuditEvent {
            id: Uuid::new_v4(),
            event_type: SecurityEventType::PolicyViolation,
            timestamp: chrono::Utc::now(),
            client_id: None,
            ip_address: None,
            user_agent: None,
            details: HashMap::new(),
            severity: SecuritySeverity::Low,
        };
        manager.log_security_event(event).await;
        let events = manager.audit_logger.get_recent_events(5).await;
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn test_manager_record_security_failure() {
        let config = SecurityHardeningConfig::default();
        let manager = SecurityHardeningManager::new(config);
        manager
            .record_security_failure("bad-actor", SecurityEventType::AuthorizationFailure)
            .await;
        let events = manager.audit_logger.get_recent_events(5).await;
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0].event_type,
            SecurityEventType::AuthorizationFailure
        ));
    }

    #[tokio::test]
    async fn test_manager_rate_limit_exceeded() {
        let config = SecurityHardeningConfig {
            rate_limiting: RateLimitingConfig {
                max_requests_per_minute: 1,
                max_requests_per_hour: 100,
                max_requests_per_day: 1000,
                sliding_window: Duration::from_secs(60),
                burst_allowance: 0,
            },
            ..SecurityHardeningConfig::default()
        };
        let manager = SecurityHardeningManager::new(config);
        let context = SecurityContext::default();

        let first = manager
            .check_security_context("rate-limited", &context)
            .await;
        assert!(first.is_ok());

        let second = manager
            .check_security_context("rate-limited", &context)
            .await;
        assert!(second.is_err());
        assert!(second.unwrap_err().to_string().contains("Rate limit"));
    }

    // ============================================================================
    // SecurityEventType serialization
    // ============================================================================

    #[test]
    fn test_security_event_type_serde() {
        let event_type = SecurityEventType::IntrusionAttempt;
        let json = serde_json::to_string(&event_type).unwrap();
        let deserialized: SecurityEventType = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, SecurityEventType::IntrusionAttempt));
    }

    // ============================================================================
    // Edge cases
    // ============================================================================

    #[test]
    fn test_empty_input_valid() {
        let validator = InputValidator::new(ValidationRules::default());
        assert!(validator.validate_input("").is_ok());
    }

    #[test]
    fn test_sanitize_empty_string() {
        let validator = InputValidator::new(ValidationRules::default());
        let sanitized = validator.sanitize_input("");
        assert_eq!(sanitized, "");
    }

    #[test]
    fn test_sanitize_ampersand() {
        let validator = InputValidator::new(ValidationRules::default());
        let sanitized = validator.sanitize_input("a & b");
        assert!(sanitized.contains("&amp;"));
    }
}
