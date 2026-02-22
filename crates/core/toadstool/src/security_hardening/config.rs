//! Security hardening configuration types
//!
//! Extracted from security_hardening.rs for modularity (Feb 14, 2026).

use std::time::Duration;

use serde::{Deserialize, Serialize};
use toadstool_common::constants::network::LOCALHOST_IPV4;
use toadstool_common::constants::timeouts;

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
            sliding_window: timeouts::HEARTBEAT_INTERVAL,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_hardening_config_default() {
        let config = SecurityHardeningConfig::default();
        assert!(config.enable_input_validation);
        assert!(config.enable_rate_limiting);
        assert!(config.enable_audit_logging);
    }

    #[test]
    fn test_rate_limiting_config_default() {
        let config = RateLimitingConfig::default();
        assert_eq!(config.max_requests_per_minute, 60);
        assert_eq!(config.max_requests_per_hour, 3600);
    }

    #[test]
    fn test_audit_config_default() {
        let config = AuditConfig::default();
        assert!(config.structured_logging);
        assert_eq!(config.log_level, "info");
    }

    #[test]
    fn test_intrusion_detection_config_default() {
        let config = IntrusionDetectionConfig::default();
        assert!((config.anomaly_threshold - 0.8).abs() < f64::EPSILON);
        assert!(config.allowed_ips.contains(&"127.0.0.1".to_string()));
    }

    #[test]
    fn test_validation_rules_default() {
        let rules = ValidationRules::default();
        assert_eq!(rules.max_input_length, 1024 * 1024);
        assert!(!rules.blocked_patterns.is_empty());
    }

    #[test]
    fn test_config_serde_roundtrip() {
        let config = SecurityHardeningConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let decoded: SecurityHardeningConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(
            config.enable_input_validation,
            decoded.enable_input_validation
        );
    }
}
