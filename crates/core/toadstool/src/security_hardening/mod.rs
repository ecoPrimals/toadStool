// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Security Hardening Module
//!
//! This module provides production-ready security hardening features for `ToadStool`:
//! - Input validation and sanitization
//! - Rate limiting and `DDoS` protection
//! - Audit logging and intrusion detection
//! - Security context validation
//!
//! # Architecture (Feb 14, 2026 Refactor)
//!
//! Split into focused submodules for maintainability:
//! - `config` - Configuration types
//! - `rate_limiter` - Rate limiting engine
//! - `input_validator` - Input validation and sanitization
//! - `audit` - Security event logging
//! - `intrusion` - Intrusion detection system

pub mod audit;
pub mod config;
pub mod input_validator;
pub mod intrusion;
pub mod rate_limiter;

// Re-export all public types for backward compatibility
pub use audit::{SecurityAuditEvent, SecurityAuditLogger, SecurityEventType, SecuritySeverity};
pub use config::{
    AuditConfig, IntrusionDetectionConfig, RateLimitingConfig, SecurityHardeningConfig,
    ValidationRules,
};
pub use input_validator::InputValidator;
pub use intrusion::{ActivityType, IntrusionDetectionSystem};
pub use rate_limiter::RateLimiter;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use uuid::Uuid;

use crate::security::SecurityContext;
use crate::{ToadStoolError, ToadStoolResult};

/// Security hardening manager
///
/// Orchestrates all security hardening components.
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
    ///
    /// # Errors
    ///
    /// Returns error if the client is banned, rate limited, or the context fails validation.
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
                    timestamp: SystemTime::now(),
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
                    timestamp: SystemTime::now(),
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
    ///
    /// # Errors
    ///
    /// Returns error if validation is enabled and the input violates rules.
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
                    timestamp: SystemTime::now(),
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
    use crate::security::SecurityContext;
    use std::time::Duration;
    use toadstool_common::constants::network::LOCALHOST_IPV4;

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
            timestamp: SystemTime::now(),
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
            burst_allowance: 1,
        };
        let limiter = RateLimiter::new(config);

        // Both clients should be allowed
        assert!(limiter.check_rate_limit("client-1").await.unwrap());
        assert!(limiter.check_rate_limit("client-2").await.unwrap());

        // Client 1 should still have requests
        assert!(limiter.check_rate_limit("client-1").await.unwrap());

        // Client 1 at limit
        assert!(!limiter.check_rate_limit("client-1").await.unwrap());

        // Client 2 should still have requests
        assert!(limiter.check_rate_limit("client-2").await.unwrap());
    }

    // ============================================================================
    // InputValidator tests
    // ============================================================================

    #[test]
    fn test_input_validator_new() {
        let rules = ValidationRules::default();
        let _validator = InputValidator::new(rules);
    }

    #[test]
    fn test_input_validator_valid_input() {
        let rules = ValidationRules::default();
        let validator = InputValidator::new(rules);
        assert!(validator.validate_input("Hello, world!").is_ok());
    }

    #[test]
    fn test_input_validator_rejects_xss() {
        let rules = ValidationRules::default();
        let validator = InputValidator::new(rules);
        assert!(
            validator
                .validate_input("<script>alert('xss')</script>")
                .is_err()
        );
    }

    #[test]
    fn test_input_validator_sanitize() {
        let rules = ValidationRules::default();
        let validator = InputValidator::new(rules);
        let sanitized = validator.sanitize_input("<script>");
        assert!(!sanitized.contains('<'));
        assert!(sanitized.contains("&lt;"));
    }

    // ============================================================================
    // SecurityAuditLogger tests
    // ============================================================================

    #[tokio::test]
    async fn test_audit_logger_new() {
        let config = AuditConfig::default();
        let _logger = SecurityAuditLogger::new(config);
    }

    #[tokio::test]
    async fn test_audit_logger_log_event() {
        let config = AuditConfig::default();
        let logger = SecurityAuditLogger::new(config);

        let event = SecurityAuditEvent {
            id: Uuid::new_v4(),
            event_type: SecurityEventType::AuthenticationAttempt,
            timestamp: SystemTime::now(),
            client_id: Some("client-1".to_string()),
            ip_address: None,
            user_agent: None,
            details: HashMap::new(),
            severity: SecuritySeverity::Low,
        };

        logger.log_event(event).await;

        let events = logger.get_recent_events(10).await;
        assert_eq!(events.len(), 1);
    }

    // ============================================================================
    // IntrusionDetectionSystem tests
    // ============================================================================

    #[tokio::test]
    async fn test_ids_new() {
        let config = IntrusionDetectionConfig::default();
        let _ids = IntrusionDetectionSystem::new(config);
    }

    #[tokio::test]
    async fn test_ids_record_activity() {
        let config = IntrusionDetectionConfig::default();
        let ids = IntrusionDetectionSystem::new(config);

        ids.record_activity("client-1", ActivityType::Request).await;

        // Client should not be banned after single request
        assert!(!ids.is_banned("client-1").await);
    }

    // ============================================================================
    // SecurityHardeningManager tests
    // ============================================================================

    #[test]
    fn test_security_hardening_manager_new() {
        let config = SecurityHardeningConfig::default();
        let _manager = SecurityHardeningManager::new(config);
    }

    #[test]
    fn test_security_hardening_manager_validate_input() {
        let config = SecurityHardeningConfig::default();
        let manager = SecurityHardeningManager::new(config);
        assert!(manager.validate_input("Hello, world!").is_ok());
    }

    #[test]
    fn test_security_hardening_manager_sanitize_input() {
        let config = SecurityHardeningConfig::default();
        let manager = SecurityHardeningManager::new(config);
        let sanitized = manager.sanitize_input("<script>");
        assert!(!sanitized.contains('<'));
    }

    #[tokio::test]
    async fn test_security_hardening_manager_check_context() {
        let config = SecurityHardeningConfig::default();
        let manager = SecurityHardeningManager::new(config);

        let context = SecurityContext {
            isolation_level: crate::security::IsolationLevel::Standard,
            capabilities: vec![crate::security::Capability::Read],
            user_context: None,
            network_security: crate::security::NetworkSecurity::default(),
            filesystem_security: crate::security::FilesystemSecurity {
                read_only: true,
                ..Default::default()
            },
        };

        let result = manager.check_security_context("client-1", &context).await;
        assert!(result.is_ok());
    }
}
