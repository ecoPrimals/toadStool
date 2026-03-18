// SPDX-License-Identifier: AGPL-3.0-or-later
//! BearDog security policy and audit types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use toadstool::security::SecurityContext;

/// Security policy from BearDog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    /// Policy identifier
    pub id: String,
    /// Human-readable policy name
    pub name: String,
    /// Policy description
    pub description: String,
    /// Condition-action rules
    pub rules: Vec<PolicyRule>,
    /// Enforcement level (strict, warn, etc.)
    pub enforcement_level: String,
    /// Policy creation timestamp
    #[serde(with = "toadstool_common::system_time_serde")]
    pub created_at: std::time::SystemTime,
}

/// Single condition-action rule within a security policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    /// Condition expression
    pub condition: String,
    /// Action when condition matches (allow, deny, etc.)
    pub action: String,
    /// Rule parameters
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Security audit event for compliance logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAuditEvent {
    /// Unique event identifier
    pub event_id: String,
    /// Event type (auth, authz, validation, etc.)
    pub event_type: String,
    /// Service that triggered the event
    pub service_id: String,
    /// User ID if applicable
    pub user_id: Option<String>,
    /// Resource accessed
    pub resource: String,
    /// Action performed
    pub action: String,
    /// Outcome (allowed, denied)
    pub result: String,
    /// Security context at time of event
    pub security_context: SecurityContext,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Event timestamp
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: std::time::SystemTime,
}
