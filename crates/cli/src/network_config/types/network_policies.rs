// SPDX-License-Identifier: AGPL-3.0-or-later

//! Network policies configuration types.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Network policies configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPoliciesConfig {
    /// Enable network policies
    pub enabled: bool,
    /// Default policy (allow, deny)
    pub default_policy: String,
    /// Ingress rules
    pub ingress_rules: Vec<IngressRule>,
    /// Egress rules
    pub egress_rules: Vec<EgressRule>,
    /// Service mesh policies
    pub service_mesh_policies: Vec<ServiceMeshPolicy>,
}

/// Ingress rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngressRule {
    /// Rule name
    pub name: String,
    /// Source selectors
    pub from: Vec<NetworkSelector>,
    /// Port specifications
    pub ports: Vec<NetworkPort>,
    /// Action (allow, deny)
    pub action: String,
    /// Priority
    pub priority: u32,
}

/// Egress rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EgressRule {
    /// Rule name
    pub name: String,
    /// Destination selectors
    pub to: Vec<NetworkSelector>,
    /// Port specifications
    pub ports: Vec<NetworkPort>,
    /// Action (allow, deny)
    pub action: String,
    /// Priority
    pub priority: u32,
}

/// Network selector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSelector {
    /// Selector type (ip, cidr, service, label)
    pub selector_type: String,
    /// Selector value
    pub value: String,
}

/// Network port
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPort {
    /// Port number
    pub port: u16,
    /// Protocol (tcp, udp, sctp)
    pub protocol: String,
    /// End port (for ranges)
    pub end_port: Option<u16>,
}

/// Service mesh policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMeshPolicy {
    /// Policy name
    pub name: String,
    /// Policy type (traffic, security, observability)
    pub policy_type: String,
    /// Selector
    pub selector: HashMap<String, String>,
    /// Configuration
    pub config: HashMap<String, serde_json::Value>,
}
