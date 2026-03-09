// SPDX-License-Identifier: AGPL-3.0-only
//! Security audit logging
//!
//! Extracted from security_hardening.rs for modularity (Feb 14, 2026).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
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
