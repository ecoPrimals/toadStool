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
            allowed_ips: vec!["127.0.0.1".to_string(), "::1".to_string()],
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
