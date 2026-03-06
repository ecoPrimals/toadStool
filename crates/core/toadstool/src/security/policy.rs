// SPDX-License-Identifier: AGPL-3.0-or-later
//! Security policies and audit configuration

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::types::{Capability, FilesystemSecurity, IsolationLevel, NetworkSecurity};

/// Security settings for runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySettings {
    /// Default isolation level
    pub default_isolation_level: IsolationLevel,
    /// Default capabilities
    pub default_capabilities: Vec<Capability>,
    /// Security policies
    pub security_policies: HashMap<String, SecurityPolicy>,
    /// Audit settings
    pub audit_settings: AuditSettings,
}

impl Default for SecuritySettings {
    fn default() -> Self {
        Self {
            default_isolation_level: IsolationLevel::Standard,
            default_capabilities: vec![Capability::Execute],
            security_policies: HashMap::new(),
            audit_settings: AuditSettings::default(),
        }
    }
}

/// Security policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    /// Policy name
    pub name: String,
    /// Policy version
    pub version: String,
    /// Isolation level
    pub isolation_level: IsolationLevel,
    /// Allowed capabilities
    pub allowed_capabilities: Vec<Capability>,
    /// Denied capabilities
    pub denied_capabilities: Vec<Capability>,
    /// Network restrictions
    pub network_restrictions: NetworkSecurity,
    /// File system restrictions
    pub filesystem_restrictions: FilesystemSecurity,
}

/// Audit settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditSettings {
    /// Enable audit logging
    pub enabled: bool,
    /// Audit log level
    pub log_level: String,
    /// Audit events to log
    pub events: Vec<AuditEvent>,
}

impl Default for AuditSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            log_level: "info".to_string(),
            events: vec![
                AuditEvent::ExecutionStart,
                AuditEvent::ExecutionEnd,
                AuditEvent::SecurityViolation,
            ],
        }
    }
}

/// Audit events
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuditEvent {
    /// Execution started
    ExecutionStart,
    /// Execution ended
    ExecutionEnd,
    /// Security violation occurred
    SecurityViolation,
    /// Capability used
    CapabilityUsed,
    /// Network access
    NetworkAccess,
    /// File system access
    FilesystemAccess,
}
