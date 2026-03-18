// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use toadstool::security::SecurityContext;

use super::policy::SecurityPolicy;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthRequest {
    pub service_id: String,
    pub service_type: String,
    pub capabilities: Vec<String>,
    pub security_context: SecurityContext,
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: std::time::SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub scope: Vec<String>,
    pub security_level: String,
    pub policies: Vec<SecurityPolicy>,
}

impl AuthResponse {
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

    pub fn is_standalone(&self) -> bool {
        self.access_token == "standalone"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthzRequest {
    pub access_token: String,
    pub resource: String,
    pub action: String,
    pub context: HashMap<String, serde_json::Value>,
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: std::time::SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthzResponse {
    pub allowed: bool,
    pub reason: Option<String>,
    pub policies_applied: Vec<String>,
    pub security_recommendations: Vec<String>,
    pub audit_id: String,
}
