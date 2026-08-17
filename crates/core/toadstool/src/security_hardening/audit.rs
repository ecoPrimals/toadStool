// SPDX-License-Identifier: AGPL-3.0-or-later
//! Security audit logging
//!
//! Extracted from `security_hardening.rs` for modularity (Feb 14, 2026).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use std::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::config::AuditConfig;

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
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: SystemTime,
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
    /// Low severity; informational.
    Low,
    /// Medium severity; notable event.
    Medium,
    /// High severity; potential risk.
    High,
    /// Critical severity; immediate action.
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
        self.events
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .push(event.clone());

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
        let events = self.events.read().unwrap_or_else(|e| e.into_inner());
        events.iter().rev().take(limit).cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_event(severity: SecuritySeverity, event_type: SecurityEventType) -> SecurityAuditEvent {
        SecurityAuditEvent {
            id: Uuid::new_v4(),
            event_type,
            timestamp: SystemTime::now(),
            client_id: Some("test-client".into()),
            ip_address: None,
            user_agent: None,
            details: HashMap::new(),
            severity,
        }
    }

    #[tokio::test]
    async fn empty_logger_returns_no_events() {
        let logger = SecurityAuditLogger::new(AuditConfig::default());
        assert!(logger.get_recent_events(10).await.is_empty());
    }

    #[tokio::test]
    async fn logged_events_are_retrievable() {
        let logger = SecurityAuditLogger::new(AuditConfig::default());
        let event = make_event(
            SecuritySeverity::Medium,
            SecurityEventType::AuthenticationAttempt,
        );
        let id = event.id;
        logger.log_event(event).await;

        let events = logger.get_recent_events(10).await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, id);
    }

    #[tokio::test]
    async fn recent_events_respects_limit() {
        let logger = SecurityAuditLogger::new(AuditConfig::default());
        for _ in 0..5 {
            logger
                .log_event(make_event(
                    SecuritySeverity::Low,
                    SecurityEventType::RateLimitExceeded,
                ))
                .await;
        }
        let events = logger.get_recent_events(3).await;
        assert_eq!(events.len(), 3);
    }

    #[tokio::test]
    async fn recent_events_are_most_recent_first() {
        let logger = SecurityAuditLogger::new(AuditConfig::default());
        let first = make_event(SecuritySeverity::Low, SecurityEventType::PolicyViolation);
        let first_id = first.id;
        let second = make_event(SecuritySeverity::High, SecurityEventType::IntrusionAttempt);
        let second_id = second.id;

        logger.log_event(first).await;
        logger.log_event(second).await;

        let events = logger.get_recent_events(10).await;
        assert_eq!(events[0].id, second_id);
        assert_eq!(events[1].id, first_id);
    }

    #[test]
    fn security_severity_ordering() {
        assert!(SecuritySeverity::Critical > SecuritySeverity::High);
        assert!(SecuritySeverity::High > SecuritySeverity::Medium);
        assert!(SecuritySeverity::Medium > SecuritySeverity::Low);
    }

    #[test]
    fn audit_event_serde_roundtrip() {
        let event = make_event(
            SecuritySeverity::Critical,
            SecurityEventType::CapabilityAbuse,
        );
        let json = serde_json::to_string(&event).unwrap();
        let decoded: SecurityAuditEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event.id, decoded.id);
        assert_eq!(event.severity, decoded.severity);
    }

    #[test]
    fn security_event_type_serde_roundtrip() {
        let types = [
            SecurityEventType::AuthenticationAttempt,
            SecurityEventType::AuthorizationFailure,
            SecurityEventType::InputValidationFailure,
            SecurityEventType::RateLimitExceeded,
            SecurityEventType::SuspiciousActivity,
            SecurityEventType::IntrusionAttempt,
            SecurityEventType::PolicyViolation,
            SecurityEventType::CapabilityAbuse,
        ];
        for t in types {
            let json = serde_json::to_string(&t).unwrap();
            let _decoded: SecurityEventType = serde_json::from_str(&json).unwrap();
        }
    }
}
