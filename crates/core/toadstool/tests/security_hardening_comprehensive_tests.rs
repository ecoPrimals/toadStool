// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for Security Hardening Module
//!
//! Week 18 Sprint 7: Security hardening tests
//! Target: 12.36% → 60% coverage (~40 tests)

use std::collections::HashMap;
use std::time::Duration;
use toadstool::security_hardening::*;

// ============================================================================
// SecurityHardeningConfig Tests (5 tests)
// ============================================================================

#[test]
fn test_security_hardening_config_default() {
    let config = SecurityHardeningConfig::default();

    assert!(config.enable_input_validation);
    assert!(config.enable_rate_limiting);
    assert!(config.enable_audit_logging);
    assert!(config.enable_intrusion_detection);
    assert_eq!(config.rate_limiting.max_requests_per_minute, 60);
}

#[test]
fn test_security_hardening_config_custom() {
    let config = SecurityHardeningConfig {
        enable_input_validation: false,
        enable_rate_limiting: true,
        enable_audit_logging: true,
        enable_intrusion_detection: false,
        rate_limiting: RateLimitingConfig::default(),
        audit_config: AuditConfig::default(),
        intrusion_detection: IntrusionDetectionConfig::default(),
        validation_rules: ValidationRules::default(),
    };

    assert!(!config.enable_input_validation);
    assert!(!config.enable_intrusion_detection);
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
    assert!(!config.enable_audit_logging);
    assert!(!config.enable_intrusion_detection);
}

#[test]
fn test_security_hardening_config_clone() {
    let config1 = SecurityHardeningConfig::default();
    let config2 = config1.clone();

    assert_eq!(
        config1.enable_input_validation,
        config2.enable_input_validation
    );
    assert_eq!(config1.enable_rate_limiting, config2.enable_rate_limiting);
}

#[test]
fn test_security_hardening_config_debug() {
    let config = SecurityHardeningConfig::default();
    let debug_str = format!("{config:?}");

    assert!(debug_str.contains("SecurityHardeningConfig"));
    assert!(debug_str.contains("enable_input_validation"));
}

// ============================================================================
// RateLimitingConfig Tests (6 tests)
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
fn test_rate_limiting_config_strict() {
    let config = RateLimitingConfig {
        max_requests_per_minute: 10,
        max_requests_per_hour: 600,
        max_requests_per_day: 10000,
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
        max_requests_per_day: 1_000_000,
        sliding_window: Duration::from_secs(120),
        burst_allowance: 100,
    };

    assert_eq!(config.max_requests_per_minute, 1000);
    assert_eq!(config.burst_allowance, 100);
}

#[test]
fn test_rate_limiting_config_different_windows() {
    let windows = vec![
        Duration::from_secs(30),
        Duration::from_secs(60),
        Duration::from_secs(120),
        Duration::from_secs(300),
    ];

    for window in windows {
        let config = RateLimitingConfig {
            max_requests_per_minute: 60,
            max_requests_per_hour: 3600,
            max_requests_per_day: 86400,
            sliding_window: window,
            burst_allowance: 10,
        };
        assert_eq!(config.sliding_window, window);
    }
}

#[test]
fn test_rate_limiting_config_clone() {
    let config1 = RateLimitingConfig::default();
    let config2 = config1.clone();

    assert_eq!(
        config1.max_requests_per_minute,
        config2.max_requests_per_minute
    );
    assert_eq!(config1.burst_allowance, config2.burst_allowance);
}

#[test]
fn test_rate_limiting_config_debug() {
    let config = RateLimitingConfig::default();
    let debug_str = format!("{config:?}");

    assert!(debug_str.contains("RateLimitingConfig"));
    assert!(debug_str.contains("max_requests_per_minute"));
}

// ============================================================================
// AuditConfig Tests (5 tests)
// ============================================================================

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
fn test_audit_config_with_file() {
    let config = AuditConfig {
        structured_logging: true,
        log_level: "debug".to_string(),
        retention_days: 90,
        log_file_path: Some("/var/log/toadstool/audit.log".to_string()),
        remote_endpoint: None,
    };

    assert_eq!(
        config.log_file_path,
        Some("/var/log/toadstool/audit.log".to_string())
    );
    assert_eq!(config.retention_days, 90);
}

#[test]
fn test_audit_config_with_remote() {
    let config = AuditConfig {
        structured_logging: true,
        log_level: "warn".to_string(),
        retention_days: 7,
        log_file_path: None,
        remote_endpoint: Some("https://logging.example.com".to_string()),
    };

    assert_eq!(
        config.remote_endpoint,
        Some("https://logging.example.com".to_string())
    );
}

#[test]
fn test_audit_config_different_log_levels() {
    let levels = vec!["trace", "debug", "info", "warn", "error"];

    for level in levels {
        let config = AuditConfig {
            structured_logging: true,
            log_level: level.to_string(),
            retention_days: 30,
            log_file_path: None,
            remote_endpoint: None,
        };
        assert_eq!(config.log_level, level);
    }
}

#[test]
fn test_audit_config_clone_debug() {
    let config1 = AuditConfig::default();
    let config2 = config1.clone();

    assert_eq!(config1.log_level, config2.log_level);
    assert_eq!(config1.retention_days, config2.retention_days);

    let debug_str = format!("{config1:?}");
    assert!(debug_str.contains("AuditConfig"));
}

// ============================================================================
// IntrusionDetectionConfig Tests (5 tests)
// ============================================================================

#[allow(clippy::float_cmp)]
#[test]
fn test_intrusion_detection_config_default() {
    let config = IntrusionDetectionConfig::default();

    assert_eq!(config.anomaly_threshold, 0.8);
    assert_eq!(config.activity_window, Duration::from_secs(300));
    assert_eq!(config.auto_ban_threshold, 10);
    assert_eq!(config.ban_duration, Duration::from_secs(3600));
    assert_eq!(config.allowed_ips.len(), 2);
    assert!(config.allowed_ips.contains(&"127.0.0.1".to_string()));
}

#[allow(clippy::float_cmp)]
#[test]
fn test_intrusion_detection_config_strict() {
    let config = IntrusionDetectionConfig {
        anomaly_threshold: 0.5,
        activity_window: Duration::from_secs(60),
        auto_ban_threshold: 3,
        ban_duration: Duration::from_secs(7200),
        allowed_ips: vec![],
    };

    assert_eq!(config.anomaly_threshold, 0.5);
    assert_eq!(config.auto_ban_threshold, 3);
    assert!(config.allowed_ips.is_empty());
}

#[allow(clippy::float_cmp)]
#[test]
fn test_intrusion_detection_config_permissive() {
    let config = IntrusionDetectionConfig {
        anomaly_threshold: 0.95,
        activity_window: Duration::from_secs(600),
        auto_ban_threshold: 50,
        ban_duration: Duration::from_secs(300),
        allowed_ips: vec![
            "127.0.0.1".to_string(),
            "::1".to_string(),
            "192.168.1.0/24".to_string(),
        ],
    };

    assert_eq!(config.anomaly_threshold, 0.95);
    assert_eq!(config.auto_ban_threshold, 50);
    assert_eq!(config.allowed_ips.len(), 3);
}

#[allow(clippy::float_cmp)]
#[test]
fn test_intrusion_detection_config_different_thresholds() {
    let thresholds = vec![0.5, 0.6, 0.7, 0.8, 0.9, 0.95];

    for threshold in thresholds {
        let config = IntrusionDetectionConfig {
            anomaly_threshold: threshold,
            activity_window: Duration::from_secs(300),
            auto_ban_threshold: 10,
            ban_duration: Duration::from_secs(3600),
            allowed_ips: vec![],
        };
        assert_eq!(config.anomaly_threshold, threshold);
    }
}

#[allow(clippy::float_cmp)]
#[test]
fn test_intrusion_detection_config_clone_debug() {
    let config1 = IntrusionDetectionConfig::default();
    let config2 = config1.clone();

    assert_eq!(config1.anomaly_threshold, config2.anomaly_threshold);
    assert_eq!(config1.auto_ban_threshold, config2.auto_ban_threshold);

    let debug_str = format!("{config1:?}");
    assert!(debug_str.contains("IntrusionDetectionConfig"));
}

// ============================================================================
// ValidationRules Tests (4 tests)
// ============================================================================

#[test]
fn test_validation_rules_default() {
    let rules = ValidationRules::default();

    assert_eq!(rules.max_input_length, 1024 * 1024); // 1MB = 1048576
    assert!(rules.allowed_characters.is_none());
    assert!(!rules.blocked_patterns.is_empty());
    assert!(!rules.sql_injection_patterns.is_empty());
}

#[test]
fn test_validation_rules_strict() {
    let rules = ValidationRules {
        max_input_length: 1000,
        allowed_characters: Some("^[a-zA-Z0-9_-]+$".to_string()),
        blocked_patterns: vec![
            "<script>".to_string(),
            "javascript:".to_string(),
            "onclick=".to_string(),
        ],
        sql_injection_patterns: vec!["' OR '1'='1".to_string(), "; DROP TABLE".to_string()],
        xss_patterns: vec!["<script>".to_string(), "onerror=".to_string()],
        command_injection_patterns: vec!["; rm -rf".to_string(), "| cat".to_string()],
    };

    assert_eq!(rules.max_input_length, 1000);
    assert!(rules.allowed_characters.is_some());
    assert_eq!(rules.blocked_patterns.len(), 3);
}

#[test]
fn test_validation_rules_different_lengths() {
    let lengths = vec![100, 1000, 10_000, 100_000, 1_000_000];

    for length in lengths {
        let rules = ValidationRules {
            max_input_length: length,
            allowed_characters: None,
            blocked_patterns: vec![],
            sql_injection_patterns: vec![],
            xss_patterns: vec![],
            command_injection_patterns: vec![],
        };
        assert_eq!(rules.max_input_length, length);
    }
}

#[test]
fn test_validation_rules_clone_debug() {
    let rules1 = ValidationRules::default();
    let rules2 = rules1.clone();

    assert_eq!(rules1.max_input_length, rules2.max_input_length);

    let debug_str = format!("{rules1:?}");
    assert!(debug_str.contains("ValidationRules"));
}

// ============================================================================
// SecurityEventType Tests (4 tests)
// ============================================================================

#[test]
fn test_security_event_type_variants() {
    let events = [
        SecurityEventType::AuthenticationAttempt,
        SecurityEventType::AuthorizationFailure,
        SecurityEventType::InputValidationFailure,
        SecurityEventType::RateLimitExceeded,
        SecurityEventType::SuspiciousActivity,
        SecurityEventType::IntrusionAttempt,
        SecurityEventType::PolicyViolation,
        SecurityEventType::CapabilityAbuse,
    ];

    assert_eq!(events.len(), 8);
}

#[test]
fn test_security_event_type_clone() {
    let event1 = SecurityEventType::AuthenticationAttempt;
    let event2 = event1.clone();

    match (event1, event2) {
        (SecurityEventType::AuthenticationAttempt, SecurityEventType::AuthenticationAttempt) => {}
        _ => panic!("Clone failed"),
    }
}

#[test]
fn test_security_event_type_debug() {
    let events = vec![
        SecurityEventType::RateLimitExceeded,
        SecurityEventType::SuspiciousActivity,
        SecurityEventType::IntrusionAttempt,
    ];

    for event in events {
        let debug_str = format!("{event:?}");
        assert!(!debug_str.is_empty());
    }
}

#[test]
fn test_security_event_type_all_variants() {
    let _ = SecurityEventType::AuthenticationAttempt;
    let _ = SecurityEventType::AuthorizationFailure;
    let _ = SecurityEventType::InputValidationFailure;
    let _ = SecurityEventType::RateLimitExceeded;
    let _ = SecurityEventType::SuspiciousActivity;
    let _ = SecurityEventType::IntrusionAttempt;
    let _ = SecurityEventType::PolicyViolation;
    let _ = SecurityEventType::CapabilityAbuse;

    // All variants compile and are valid
}

// ============================================================================
// SecuritySeverity Tests (5 tests)
// ============================================================================

#[test]
fn test_security_severity_variants() {
    let severities = [
        SecuritySeverity::Low,
        SecuritySeverity::Medium,
        SecuritySeverity::High,
        SecuritySeverity::Critical,
    ];

    assert_eq!(severities.len(), 4);
}

#[test]
fn test_security_severity_ordering() {
    assert!(SecuritySeverity::Low < SecuritySeverity::Medium);
    assert!(SecuritySeverity::Medium < SecuritySeverity::High);
    assert!(SecuritySeverity::High < SecuritySeverity::Critical);
}

#[test]
fn test_security_severity_equality() {
    assert_eq!(SecuritySeverity::Low, SecuritySeverity::Low);
    assert_eq!(SecuritySeverity::Critical, SecuritySeverity::Critical);
    assert_ne!(SecuritySeverity::Low, SecuritySeverity::High);
}

#[test]
fn test_security_severity_clone() {
    let sev1 = SecuritySeverity::High;
    let sev2 = sev1.clone();

    assert_eq!(sev1, sev2);
}

#[test]
fn test_security_severity_debug() {
    let severities = [
        SecuritySeverity::Low,
        SecuritySeverity::Medium,
        SecuritySeverity::High,
        SecuritySeverity::Critical,
    ];

    for severity in severities {
        let debug_str = format!("{severity:?}");
        assert!(!debug_str.is_empty());
    }
}

// ============================================================================
// ActivityType Tests (3 tests)
// ============================================================================

#[test]
fn test_activity_type_variants() {
    let activities = [
        ActivityType::Request,
        ActivityType::FailedAttempt,
        ActivityType::SuspiciousPattern,
    ];

    assert_eq!(activities.len(), 3);
}

#[test]
fn test_activity_type_clone() {
    let act1 = ActivityType::Request;
    let act2 = act1.clone();

    match (act1, act2) {
        (ActivityType::Request, ActivityType::Request) => {}
        _ => panic!("Clone failed"),
    }
}

#[test]
fn test_activity_type_debug() {
    let activities = [
        ActivityType::Request,
        ActivityType::FailedAttempt,
        ActivityType::SuspiciousPattern,
    ];

    for activity in activities {
        let debug_str = format!("{activity:?}");
        assert!(!debug_str.is_empty());
    }
}

// ============================================================================
// SecurityAuditLogger Tests (3 tests)
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_security_audit_logger_creation() {
    let config = AuditConfig::default();
    let logger = SecurityAuditLogger::new(config);

    let events = logger.get_recent_events(10).await;
    assert_eq!(events.len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_security_audit_logger_log_event() {
    let config = AuditConfig::default();
    let logger = SecurityAuditLogger::new(config);

    let event = SecurityAuditEvent {
        id: uuid::Uuid::new_v4(),
        event_type: SecurityEventType::AuthenticationAttempt,
        timestamp: std::time::SystemTime::now(),
        client_id: Some("client-1".to_string()),
        ip_address: Some("127.0.0.1".to_string()),
        user_agent: Some("test-agent".to_string()),
        details: HashMap::new(),
        severity: SecuritySeverity::Low,
    };

    logger.log_event(event).await;

    let events = logger.get_recent_events(10).await;
    assert_eq!(events.len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_security_audit_logger_multiple_events() {
    let config = AuditConfig::default();
    let logger = SecurityAuditLogger::new(config);

    for i in 0..5 {
        let event = SecurityAuditEvent {
            id: uuid::Uuid::new_v4(),
            event_type: SecurityEventType::RateLimitExceeded,
            timestamp: std::time::SystemTime::now(),
            client_id: Some(format!("client-{i}")),
            ip_address: Some(format!("192.168.1.{i}")),
            user_agent: Some("test-agent".to_string()),
            details: HashMap::new(),
            severity: SecuritySeverity::Medium,
        };
        logger.log_event(event).await;
    }

    let events = logger.get_recent_events(10).await;
    assert_eq!(events.len(), 5);

    let limited_events = logger.get_recent_events(3).await;
    assert_eq!(limited_events.len(), 3);
}

// ============================================================================
// IntrusionDetectionSystem Tests (8 tests)
// ============================================================================

#[tokio::test]
async fn test_intrusion_detection_system_new() {
    let config = IntrusionDetectionConfig::default();
    let ids = IntrusionDetectionSystem::new(config);
    assert!(!ids.is_banned("client-1").await);
}

#[tokio::test]
async fn test_intrusion_detection_record_request_activity() {
    let config = IntrusionDetectionConfig::default();
    let ids = IntrusionDetectionSystem::new(config);
    ids.record_activity("client-req", ActivityType::Request)
        .await;
    ids.record_activity("client-req", ActivityType::Request)
        .await;
    assert!(!ids.is_banned("client-req").await);
}

#[tokio::test]
async fn test_intrusion_detection_record_failed_attempt() {
    let config = IntrusionDetectionConfig::default();
    let ids = IntrusionDetectionSystem::new(config);
    ids.record_activity("client-fail", ActivityType::FailedAttempt)
        .await;
    assert!(!ids.is_banned("client-fail").await);
}

#[tokio::test]
async fn test_intrusion_detection_record_suspicious_pattern() {
    let config = IntrusionDetectionConfig::default();
    let ids = IntrusionDetectionSystem::new(config);
    ids.record_activity("client-susp", ActivityType::SuspiciousPattern)
        .await;
    assert!(!ids.is_banned("client-susp").await);
}

#[tokio::test]
async fn test_intrusion_detection_ban_client() {
    let config = IntrusionDetectionConfig::default();
    let ids = IntrusionDetectionSystem::new(config);
    ids.ban_client("bad-client", Duration::from_secs(3600), "Manual ban")
        .await;
    assert!(ids.is_banned("bad-client").await);
}

#[tokio::test]
async fn test_intrusion_detection_ban_expires() {
    let config = IntrusionDetectionConfig {
        ban_duration: Duration::from_millis(50),
        ..IntrusionDetectionConfig::default()
    };
    let ids = IntrusionDetectionSystem::new(config);
    ids.ban_client("temp-banned", Duration::from_millis(50), "Short ban")
        .await;
    assert!(ids.is_banned("temp-banned").await);
    // Wait for ban duration using interval (proper async pattern for time-based wait)
    let mut interval = tokio::time::interval(Duration::from_millis(60));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    interval.tick().await;
    interval.tick().await;
    assert!(!ids.is_banned("temp-banned").await);
}

#[tokio::test]
async fn test_intrusion_detection_auto_ban_threshold() {
    let config = IntrusionDetectionConfig {
        auto_ban_threshold: 3,
        anomaly_threshold: 10.0,
        ban_duration: Duration::from_secs(60),
        ..IntrusionDetectionConfig::default()
    };
    let ids = IntrusionDetectionSystem::new(config);
    for _ in 0..3 {
        ids.record_activity("auto-ban-client", ActivityType::FailedAttempt)
            .await;
    }
    assert!(ids.is_banned("auto-ban-client").await);
}

#[tokio::test]
async fn test_intrusion_detection_anomaly_threshold_triggers_ban() {
    let config = IntrusionDetectionConfig {
        anomaly_threshold: 0.5,
        auto_ban_threshold: 100,
        ban_duration: Duration::from_secs(60),
        ..IntrusionDetectionConfig::default()
    };
    let ids = IntrusionDetectionSystem::new(config);
    for _ in 0..3 {
        ids.record_activity("anomaly-client", ActivityType::SuspiciousPattern)
            .await;
    }
    assert!(ids.is_banned("anomaly-client").await);
}

// ============================================================================
// Test Coverage Summary
// ============================================================================

#[test]
fn test_security_hardening_coverage_summary() {
    println!("=== Security Hardening Test Coverage ===");
    println!("SecurityHardeningConfig Tests:    5 tests");
    println!("RateLimitingConfig Tests:         6 tests");
    println!("AuditConfig Tests:                5 tests");
    println!("IntrusionDetectionConfig Tests:   5 tests");
    println!("ValidationRules Tests:            4 tests");
    println!("SecurityEventType Tests:          4 tests");
    println!("SecuritySeverity Tests:           5 tests");
    println!("ActivityType Tests:               3 tests");
    println!("SecurityAuditLogger Tests:        3 tests");
    println!("────────────────────────────────────────");
    println!("Total:                           40 tests");
    println!("Module Coverage: 12.36% → Target 60%");
    println!("=========================================");
}
