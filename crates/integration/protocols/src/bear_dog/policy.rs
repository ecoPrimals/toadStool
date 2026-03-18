// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use toadstool::security::SecurityContext;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    pub id: String,
    pub name: String,
    pub description: String,
    pub rules: Vec<PolicyRule>,
    pub enforcement_level: String,
    #[serde(with = "toadstool_common::system_time_serde")]
    pub created_at: std::time::SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub condition: String,
    pub action: String,
    pub parameters: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAuditEvent {
    pub event_id: String,
    pub event_type: String,
    pub service_id: String,
    pub user_id: Option<String>,
    pub resource: String,
    pub action: String,
    pub result: String,
    pub security_context: SecurityContext,
    pub metadata: HashMap<String, serde_json::Value>,
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: std::time::SystemTime,
}
