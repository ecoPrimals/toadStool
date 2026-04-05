// SPDX-License-Identifier: AGPL-3.0-or-later
//! Security authentication and authorization request/response types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use toadstool::security::SecurityContext;

use super::policy::SecurityPolicy;

/// Authentication request to security PKI security service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthRequest {
    /// Service identifier requesting auth
    pub service_id: String,
    /// Service type (e.g. compute, storage)
    pub service_type: String,
    /// Requested capability scopes
    pub capabilities: Vec<String>,
    /// PKI security context for validation
    pub security_context: SecurityContext,
    /// Request timestamp for replay protection
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: std::time::SystemTime,
}

/// Authentication response from security PKI security service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    /// JWT or bearer token for subsequent requests
    pub access_token: String,
    /// Token type (e.g. bearer)
    pub token_type: String,
    /// Token validity in seconds
    pub expires_in: u64,
    /// Granted capability scopes
    pub scope: Vec<String>,
    /// Assigned security level
    pub security_level: String,
    /// Active security policies
    pub policies: Vec<SecurityPolicy>,
}

impl AuthResponse {
    /// Create a standalone-mode response when Security is unavailable
    pub fn standalone() -> Self {
        Self {
            access_token: "standalone".to_string(),
            token_type: "bearer".to_string(),
            expires_in: 3600,
            scope: vec!["standalone".to_string()],
            security_level: "standard".to_string(),
            policies: vec![],
        }
    }

    /// Returns true if this is a standalone-mode response (no Security)
    pub fn is_standalone(&self) -> bool {
        self.access_token == "standalone"
    }
}

/// Authorization request for resource access check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthzRequest {
    /// Bearer token from prior auth
    pub access_token: String,
    /// Resource path being accessed
    pub resource: String,
    /// Action (read, write, execute, etc.)
    pub action: String,
    /// Additional context for policy evaluation
    pub context: HashMap<String, serde_json::Value>,
    /// Request timestamp
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: std::time::SystemTime,
}

/// Authorization decision response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthzResponse {
    /// Whether access was granted
    pub allowed: bool,
    /// Denial reason if not allowed
    pub reason: Option<String>,
    /// Policy IDs that were evaluated
    pub policies_applied: Vec<String>,
    /// Security recommendations
    pub security_recommendations: Vec<String>,
    /// Audit trail identifier
    pub audit_id: String,
}
