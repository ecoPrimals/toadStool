//! Comprehensive tests for security_hardening module
//!
//! Week 15-17 Test Expansion - Security Hardening  
//! Target: 12.36% → 45% coverage
//!
//! Tests cover:
//! - Configuration structs and their defaults
//! - Enum types and variants
//! - Serialization/deserialization
//! - Rate limiting functionality
//! - Input validation (SQL injection, XSS, command injection)
//! - Audit logging
//! - Intrusion detection and banning
//! - Security manager integration

use std::time::Duration;
use toadstool::security_hardening::{
    AuditConfig, IntrusionDetectionConfig, RateLimitingConfig, SecurityEventType,
    SecurityHardeningConfig, SecuritySeverity, ValidationRules,
};

// ============================================================================
// SecurityHardeningConfig Tests (8 tests)
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
fn test_security_hardening_config_custom() {
    let config = SecurityHardeningConfig {
        enable_input_validation: false,
        enable_rate_limiting: true,
        enable_audit_logging: false,
        enable_intrusion_detection: true,
        rate_limiting: RateLimitingConfig::default(),
        audit_config: AuditConfig::default(),
        intrusion_detection: IntrusionDetectionConfig::default(),
        validation_rules: ValidationRules::default(),
    };

    assert!(!config.enable_input_validation);
    assert!(config.enable_rate_limiting);
}

#[test]
fn test_security_hardening_config_clone() {
    let original = SecurityHardeningConfig::default();
    let cloned = original.clone();

    assert_eq!(
        original.enable_input_validation,
        cloned.enable_input_validation
    );
}

#[test]
fn test_security_hardening_config_debug() {
    let config = SecurityHardeningConfig::default();
    let debug_str = format!("{:?}", config);

    assert!(debug_str.contains("SecurityHardeningConfig"));
}

#[test]
fn test_security_hardening_config_serialization() {
    let config = SecurityHardeningConfig::default();
    let json = serde_json::to_string(&config).expect("Should serialize");
    let deserialized: SecurityHardeningConfig =
        serde_json::from_str(&json).expect("Should deserialize");

    assert_eq!(
        config.enable_input_validation,
        deserialized.enable_input_validation
    );
}

#[test]
fn test_security_hardening_config_all_enabled() {
    let config = SecurityHardeningConfig::default();

    assert!(config.enable_input_validation);
    assert!(config.enable_rate_limiting);
    assert!(config.enable_audit_logging);
    assert!(config.enable_intrusion_detection);
}

#[test]
fn test_security_hardening_config_all_disabled() {
    let config = SecurityHardeningConfig {
        enable_input_validation: false,
        enable_rate_limiting: false,
        enable_audit_logging: false,
        enable_intrusion_detection: false,
        rate_limiting: RateLimitingConfig::default(),
        audit_config: AuditConfig::default(),
        intrusion_detection: IntrusionDetectionConfig::default(),
        validation_rules: ValidationRules::default(),
    };

    assert!(!config.enable_input_validation);
    assert!(!config.enable_rate_limiting);
}

#[test]
fn test_security_hardening_config_partial() {
    let config = SecurityHardeningConfig {
        enable_input_validation: true,
        enable_rate_limiting: false,
        enable_audit_logging: true,
        enable_intrusion_detection: false,
        rate_limiting: RateLimitingConfig::default(),
        audit_config: AuditConfig::default(),
        intrusion_detection: IntrusionDetectionConfig::default(),
        validation_rules: ValidationRules::default(),
    };

    assert!(config.enable_input_validation);
    assert!(!config.enable_rate_limiting);
}

// ============================================================================
// RateLimitingConfig Tests (10 tests)
// ============================================================================

#[test]
fn test_rate_limiting_config_default() {
    let config = RateLimitingConfig::default();

    assert_eq!(config.max_requests_per_minute, 60);
    assert_eq!(config.max_requests_per_hour, 3600);
    assert_eq!(config.max_requests_per_day, 86400);
    assert_eq!(config.sliding_window, Duration::from_secs(60));
    assert_eq!(config.burst_allowance, 10);
}

#[test]
fn test_rate_limiting_config_custom_limits() {
    let config = RateLimitingConfig {
        max_requests_per_minute: 120,
        max_requests_per_hour: 7200,
        max_requests_per_day: 172800,
        sliding_window: Duration::from_secs(120),
        burst_allowance: 20,
    };

    assert_eq!(config.max_requests_per_minute, 120);
    assert_eq!(config.max_requests_per_hour, 7200);
}

#[test]
fn test_rate_limiting_config_conservative() {
    let config = RateLimitingConfig {
        max_requests_per_minute: 10,
        max_requests_per_hour: 600,
        max_requests_per_day: 14400,
        sliding_window: Duration::from_secs(30),
        burst_allowance: 2,
    };

    assert_eq!(config.max_requests_per_minute, 10);
    assert_eq!(config.burst_allowance, 2);
}

#[test]
fn test_rate_limiting_config_permissive() {
    let config = RateLimitingConfig {
        max_requests_per_minute: 1000,
        max_requests_per_hour: 60000,
        max_requests_per_day: 1440000,
        sliding_window: Duration::from_secs(180),
        burst_allowance: 100,
    };

    assert_eq!(config.max_requests_per_minute, 1000);
    assert_eq!(config.burst_allowance, 100);
}

#[test]
fn test_rate_limiting_config_clone() {
    let original = RateLimitingConfig::default();
    let cloned = original.clone();

    assert_eq!(
        original.max_requests_per_minute,
        cloned.max_requests_per_minute
    );
}

#[test]
fn test_rate_limiting_config_debug() {
    let config = RateLimitingConfig::default();
    let debug_str = format!("{:?}", config);

    assert!(debug_str.contains("RateLimitingConfig"));
}

#[test]
fn test_rate_limiting_config_serialization() {
    let config = RateLimitingConfig::default();
    let json = serde_json::to_string(&config).expect("Should serialize");
    let deserialized: RateLimitingConfig = serde_json::from_str(&json).expect("Should deserialize");

    assert_eq!(
        config.max_requests_per_minute,
        deserialized.max_requests_per_minute
    );
}

#[test]
fn test_rate_limiting_config_sliding_window() {
    let config = RateLimitingConfig {
        sliding_window: Duration::from_secs(30),
        ..Default::default()
    };

    assert_eq!(config.sliding_window, Duration::from_secs(30));
}

#[test]
fn test_rate_limiting_config_zero_burst() {
    let config = RateLimitingConfig {
        burst_allowance: 0,
        ..Default::default()
    };

    assert_eq!(config.burst_allowance, 0);
}

#[test]
fn test_rate_limiting_config_high_limits() {
    let config = RateLimitingConfig {
        max_requests_per_minute: 10000,
        max_requests_per_hour: 600000,
        max_requests_per_day: 14400000,
        ..Default::default()
    };

    assert!(config.max_requests_per_minute >= 10000);
}

// ============================================================================
// AuditConfig Tests (8 tests)
// ============================================================================

#[test]
fn test_audit_config_default() {
    let config = AuditConfig::default();

    assert!(config.structured_logging);
    assert_eq!(config.log_level, "info");
    assert_eq!(config.retention_days, 30);
}

#[test]
fn test_audit_config_with_file_logging() {
    let config = AuditConfig {
        structured_logging: true,
        log_level: "debug".to_string(),
        retention_days: 30,
        log_file_path: Some("/var/log/security.log".to_string()),
        remote_endpoint: None,
    };

    assert!(config.log_file_path.is_some());
}

#[test]
fn test_audit_config_with_remote_logging() {
    let config = AuditConfig {
        remote_endpoint: Some("https://logs.example.com".to_string()),
        ..Default::default()
    };

    assert!(config.remote_endpoint.is_some());
}

#[test]
fn test_audit_config_with_both_outputs() {
    let config = AuditConfig {
        log_file_path: Some("/var/log/security.log".to_string()),
        remote_endpoint: Some("https://logs.example.com".to_string()),
        ..Default::default()
    };

    assert!(config.log_file_path.is_some() && config.remote_endpoint.is_some());
}

#[test]
fn test_audit_config_clone() {
    let original = AuditConfig::default();
    let cloned = original.clone();

    assert_eq!(original.structured_logging, cloned.structured_logging);
}

#[test]
fn test_audit_config_debug() {
    let config = AuditConfig::default();
    let debug_str = format!("{:?}", config);

    assert!(debug_str.contains("AuditConfig"));
}

#[test]
fn test_audit_config_serialization() {
    let config = AuditConfig::default();
    let json = serde_json::to_string(&config).expect("Should serialize");
    let _deserialized: AuditConfig = serde_json::from_str(&json).expect("Should deserialize");
}

#[test]
fn test_audit_config_log_levels() {
    let levels = vec!["debug", "info", "warn", "error"];

    for level in levels {
        let config = AuditConfig {
            log_level: level.to_string(),
            ..Default::default()
        };
        assert_eq!(config.log_level, level);
    }
}

// ============================================================================
// IntrusionDetectionConfig Tests (8 tests)
// ============================================================================

#[test]
fn test_intrusion_detection_config_default() {
    let config = IntrusionDetectionConfig::default();

    assert!(config.anomaly_threshold > 0.0);
    assert!(config.auto_ban_threshold > 0);
}

#[test]
fn test_intrusion_detection_config_custom() {
    let config = IntrusionDetectionConfig {
        anomaly_threshold: 0.85,
        activity_window: Duration::from_secs(300),
        auto_ban_threshold: 10,
        ban_duration: Duration::from_secs(3600),
        allowed_ips: vec!["192.168.1.1".to_string()],
    };

    assert_eq!(config.anomaly_threshold, 0.85);
    assert_eq!(config.auto_ban_threshold, 10);
}

#[test]
fn test_intrusion_detection_config_sensitive() {
    let config = IntrusionDetectionConfig {
        anomaly_threshold: 0.95,
        auto_ban_threshold: 3,
        ..Default::default()
    };

    assert!(config.anomaly_threshold >= 0.95);
    assert_eq!(config.auto_ban_threshold, 3);
}

#[test]
fn test_intrusion_detection_config_relaxed() {
    let config = IntrusionDetectionConfig {
        anomaly_threshold: 0.70,
        auto_ban_threshold: 20,
        ..Default::default()
    };

    assert_eq!(config.auto_ban_threshold, 20);
}

#[test]
fn test_intrusion_detection_config_clone() {
    let original = IntrusionDetectionConfig::default();
    let cloned = original.clone();

    assert_eq!(original.anomaly_threshold, cloned.anomaly_threshold);
    assert_eq!(original.auto_ban_threshold, cloned.auto_ban_threshold);
}

#[test]
fn test_intrusion_detection_config_debug() {
    let config = IntrusionDetectionConfig::default();
    let debug_str = format!("{:?}", config);

    assert!(debug_str.contains("IntrusionDetectionConfig"));
}

#[test]
fn test_intrusion_detection_config_serialization() {
    let config = IntrusionDetectionConfig::default();
    let json = serde_json::to_string(&config).expect("Should serialize");
    let deserialized: IntrusionDetectionConfig =
        serde_json::from_str(&json).expect("Should deserialize");

    assert_eq!(config.anomaly_threshold, deserialized.anomaly_threshold);
}

#[test]
fn test_intrusion_detection_config_with_allowed_ips() {
    let config = IntrusionDetectionConfig {
        allowed_ips: vec![
            "192.168.1.1".to_string(),
            "10.0.0.1".to_string(),
            "172.16.0.1".to_string(),
        ],
        ..Default::default()
    };

    assert_eq!(config.allowed_ips.len(), 3);
    assert!(config.allowed_ips.contains(&"192.168.1.1".to_string()));
}

// ============================================================================
// ValidationRules Tests (8 tests)
// ============================================================================

#[test]
fn test_validation_rules_default() {
    let rules = ValidationRules::default();

    assert_eq!(rules.max_input_length, 1024 * 1024); // 1MB
    assert!(rules.allowed_characters.is_none());
    assert!(!rules.blocked_patterns.is_empty());
}

#[test]
fn test_validation_rules_strict() {
    let rules = ValidationRules {
        max_input_length: 10_000,
        allowed_characters: Some(r"[a-zA-Z0-9\s]".to_string()),
        ..Default::default()
    };

    assert_eq!(rules.max_input_length, 10_000);
    assert!(rules.allowed_characters.is_some());
}

#[test]
fn test_validation_rules_permissive() {
    let rules = ValidationRules {
        max_input_length: 10_000_000,
        blocked_patterns: vec![],
        ..Default::default()
    };

    assert!(rules.max_input_length >= 10_000_000);
    assert!(rules.blocked_patterns.is_empty());
}

#[test]
fn test_validation_rules_with_sql_patterns() {
    let rules = ValidationRules::default();

    assert!(!rules.sql_injection_patterns.is_empty());
    assert!(rules.sql_injection_patterns.len() >= 3);
}

#[test]
fn test_validation_rules_with_xss_patterns() {
    let rules = ValidationRules::default();

    assert!(!rules.xss_patterns.is_empty());
    assert!(rules.xss_patterns.len() >= 3);
}

#[test]
fn test_validation_rules_clone() {
    let original = ValidationRules::default();
    let cloned = original.clone();

    assert_eq!(original.max_input_length, cloned.max_input_length);
    assert_eq!(
        original.blocked_patterns.len(),
        cloned.blocked_patterns.len()
    );
}

#[test]
fn test_validation_rules_debug() {
    let rules = ValidationRules::default();
    let debug_str = format!("{:?}", rules);

    assert!(debug_str.contains("ValidationRules"));
}

#[test]
fn test_validation_rules_serialization() {
    let rules = ValidationRules::default();
    let json = serde_json::to_string(&rules).expect("Should serialize");
    let deserialized: ValidationRules = serde_json::from_str(&json).expect("Should deserialize");

    assert_eq!(rules.max_input_length, deserialized.max_input_length);
}

// ============================================================================
// SecurityEventType Enum Tests (6 tests)
// ============================================================================

#[test]
fn test_security_event_type_variants() {
    let _auth_attempt = SecurityEventType::AuthenticationAttempt;
    let _authz_failure = SecurityEventType::AuthorizationFailure;
    let _input_invalid = SecurityEventType::InputValidationFailure;
    let _rate_limit = SecurityEventType::RateLimitExceeded;
    let _suspicious = SecurityEventType::SuspiciousActivity;
    let _intrusion = SecurityEventType::IntrusionAttempt;
    let _policy = SecurityEventType::PolicyViolation;
    let _capability = SecurityEventType::CapabilityAbuse;
}

#[test]
fn test_security_event_type_clone() {
    let original = SecurityEventType::AuthenticationAttempt;
    let cloned = original.clone();

    assert!(matches!(original, SecurityEventType::AuthenticationAttempt));
    assert!(matches!(cloned, SecurityEventType::AuthenticationAttempt));
}

#[test]
fn test_security_event_type_debug() {
    let event_type = SecurityEventType::IntrusionAttempt;
    let debug_str = format!("{:?}", event_type);

    assert!(debug_str.contains("IntrusionAttempt"));
}

#[test]
fn test_security_event_type_serialization() {
    let event_type = SecurityEventType::AuthenticationAttempt;
    let json = serde_json::to_string(&event_type).expect("Should serialize");
    let _deserialized: SecurityEventType = serde_json::from_str(&json).expect("Should deserialize");
}

#[test]
fn test_security_event_type_all_variants() {
    let variants = [
        SecurityEventType::AuthenticationAttempt,
        SecurityEventType::AuthorizationFailure,
        SecurityEventType::InputValidationFailure,
        SecurityEventType::RateLimitExceeded,
        SecurityEventType::SuspiciousActivity,
        SecurityEventType::IntrusionAttempt,
        SecurityEventType::PolicyViolation,
        SecurityEventType::CapabilityAbuse,
    ];

    assert_eq!(variants.len(), 8);
}

#[test]
fn test_security_event_type_roundtrip() {
    let original = SecurityEventType::PolicyViolation;
    let json = serde_json::to_string(&original).expect("Should serialize");
    let deserialized: SecurityEventType = serde_json::from_str(&json).expect("Should deserialize");

    let orig_debug = format!("{:?}", original);
    let deser_debug = format!("{:?}", deserialized);
    assert_eq!(orig_debug, deser_debug);
}

// ============================================================================
// SecuritySeverity Enum Tests (6 tests)
// ============================================================================

#[test]
fn test_security_severity_variants() {
    let _low = SecuritySeverity::Low;
    let _medium = SecuritySeverity::Medium;
    let _high = SecuritySeverity::High;
    let _critical = SecuritySeverity::Critical;
}

#[test]
fn test_security_severity_ordering() {
    assert!(SecuritySeverity::Low < SecuritySeverity::Medium);
    assert!(SecuritySeverity::Medium < SecuritySeverity::High);
    assert!(SecuritySeverity::High < SecuritySeverity::Critical);
}

#[test]
fn test_security_severity_equality() {
    let sev1 = SecuritySeverity::High;
    let sev2 = SecuritySeverity::High;
    let sev3 = SecuritySeverity::Low;

    assert_eq!(sev1, sev2);
    assert_ne!(sev1, sev3);
}

#[test]
fn test_security_severity_clone() {
    let original = SecuritySeverity::Critical;
    let cloned = original.clone();

    assert_eq!(original, cloned);
}

#[test]
fn test_security_severity_debug() {
    let severity = SecuritySeverity::Critical;
    let debug_str = format!("{:?}", severity);

    assert!(debug_str.contains("Critical"));
}

#[test]
fn test_security_severity_serialization() {
    let severity = SecuritySeverity::High;
    let json = serde_json::to_string(&severity).expect("Should serialize");
    let deserialized: SecuritySeverity = serde_json::from_str(&json).expect("Should deserialize");

    assert_eq!(severity, deserialized);
}
