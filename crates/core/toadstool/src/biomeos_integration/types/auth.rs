// SPDX-License-Identifier: AGPL-3.0-or-later
//! Authentication, authorization, and security policy types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Biome security configuration (auth, policies, network).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeSecurity {
    /// Enable `BearDog` integration
    pub security: bool,
    /// Security policies
    pub policies: Vec<SecurityPolicy>,
    /// Network policies
    pub network_policies: Vec<NetworkPolicy>,
    /// Authentication settings
    pub authentication: AuthenticationConfig,
    /// Authorization settings
    pub authorization: AuthorizationConfig,
}

/// Security policy definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    /// Policy name
    pub name: String,
    /// Policy type
    pub policy_type: String,
    /// Policy rules
    pub rules: Vec<PolicyRule>,
}

/// Network policy definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPolicy {
    /// Policy name
    pub name: String,
    /// Ingress rules
    pub ingress: Vec<NetworkRule>,
    /// Egress rules
    pub egress: Vec<NetworkRule>,
}

/// Network rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRule {
    /// Allowed sources/destinations
    pub from: Vec<String>,
    /// Allowed ports
    pub ports: Vec<u16>,
    /// Protocol
    pub protocol: String,
}

/// Policy rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    /// Rule name
    pub name: String,
    /// Rule action (allow, deny)
    pub action: String,
    /// Rule conditions
    pub conditions: HashMap<String, serde_json::Value>,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationConfig {
    /// Authentication methods
    pub methods: Vec<String>,
    /// Token settings
    pub token: Option<TokenConfig>,
    /// `OAuth` settings
    pub oauth: Option<OAuthConfig>,
}

/// Authorization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationConfig {
    /// Authorization method
    pub method: String,
    /// Role-based access control
    pub rbac: Option<RBACConfig>,
    /// Policy-based access control
    pub pbac: Option<PBACConfig>,
}

/// Token configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenConfig {
    /// Token type
    pub token_type: String,
    /// Token lifetime
    pub lifetime: Duration,
    /// Refresh settings
    pub refresh: Option<TokenRefreshConfig>,
}

/// Token refresh configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRefreshConfig {
    /// Enable token refresh
    pub enabled: bool,
    /// Refresh interval
    pub interval: Duration,
}

/// `OAuth` configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    /// `OAuth` provider
    pub provider: String,
    /// Client ID
    pub client_id: String,
    /// `OAuth` scopes
    pub scopes: Vec<String>,
}

/// RBAC configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RBACConfig {
    /// Enable RBAC
    pub enabled: bool,
    /// Roles
    pub roles: Vec<Role>,
    /// Role bindings
    pub role_bindings: Vec<RoleBinding>,
}

/// PBAC configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PBACConfig {
    /// Enable PBAC
    pub enabled: bool,
    /// Policies
    pub policies: Vec<String>,
}

/// Role definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    /// Role name
    pub name: String,
    /// Permissions
    pub permissions: Vec<String>,
}

/// Role binding definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleBinding {
    /// Binding name
    pub name: String,
    /// Role name
    pub role: String,
    /// Subjects
    pub subjects: Vec<String>,
}
