// SPDX-License-Identifier: AGPL-3.0-only

//! Cross-primal security configuration types.

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Cross-primal security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossPrimalSecurityConfig {
    /// Enable cross-primal security
    pub enabled: bool,
    /// Authentication requirements
    pub authentication: AuthenticationConfig,
    /// Authorization policies
    pub authorization: AuthorizationConfig,
    /// Network isolation
    pub network_isolation: NetworkIsolationConfig,
    /// Audit logging
    pub audit_logging: AuditLoggingConfig,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationConfig {
    /// Authentication method (jwt, oauth2, mtls, pki)
    pub method: String,
    /// Token validation
    pub token_validation: TokenValidationConfig,
    /// Certificate validation
    pub certificate_validation: CertificateValidationConfig,
    /// PKI security service integration (legacy field name: beardog_integration)
    pub beardog_integration: BearDogIntegrationConfig,
}

/// Token validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenValidationConfig {
    /// Issuer validation
    pub validate_issuer: bool,
    /// Audience validation
    pub validate_audience: bool,
    /// Expiration validation
    pub validate_expiration: bool,
    /// Signature validation
    pub validate_signature: bool,
    /// Clock skew tolerance
    pub clock_skew: Duration,
}

/// Certificate validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateValidationConfig {
    /// Validate certificate chain
    pub validate_chain: bool,
    /// Validate certificate expiration
    pub validate_expiration: bool,
    /// Validate certificate usage
    pub validate_usage: bool,
    /// Trusted CA certificates
    pub trusted_cas: Vec<String>,
}

/// PKI security service integration configuration (legacy name: BearDogIntegrationConfig).
///
/// Prefer [`PkiSecurityConfig`] alias for new code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BearDogIntegrationConfig {
    /// Enable PKI security integration
    pub enabled: bool,
    /// PKI security service endpoint (discovered at runtime)
    pub endpoint: String,
    /// Authentication token
    pub auth_token: Option<String>,
    /// Signature verification
    pub signature_verification: bool,
    /// Crypto-lock integration
    pub crypto_lock: bool,
}

/// Capability-based alias — prefer for new code.
pub type PkiSecurityConfig = BearDogIntegrationConfig;

/// Authorization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationConfig {
    /// Authorization model (rbac, abac, policy)
    pub model: String,
    /// Policy engine
    pub policy_engine: PolicyEngineConfig,
    /// Role definitions
    pub roles: Vec<RoleDefinition>,
    /// Permission matrix
    pub permissions: HashMap<String, Vec<String>>,
}

/// Policy engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEngineConfig {
    /// Engine type (opa, casbin, native)
    pub engine_type: String,
    /// Policy files
    pub policy_files: Vec<String>,
    /// Policy endpoints
    pub policy_endpoints: Vec<String>,
    /// Evaluation cache
    pub evaluation_cache: bool,
}

/// Role definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleDefinition {
    /// Role name
    pub name: String,
    /// Role description
    pub description: String,
    /// Permissions
    pub permissions: Vec<String>,
    /// Inheritance
    pub inherits: Vec<String>,
}

/// Network isolation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkIsolationConfig {
    /// Enable network isolation
    pub enabled: bool,
    /// Isolation level (none, basic, strict, paranoid)
    pub isolation_level: String,
    /// Allowed networks
    pub allowed_networks: Vec<String>,
    /// Blocked networks
    pub blocked_networks: Vec<String>,
    /// Firewall rules
    pub firewall_rules: Vec<FirewallRule>,
}

/// Firewall rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirewallRule {
    /// Rule name
    pub name: String,
    /// Rule action (allow, deny, log)
    pub action: String,
    /// Source
    pub source: String,
    /// Destination
    pub destination: String,
    /// Protocol
    pub protocol: String,
    /// Port range
    pub port_range: Option<String>,
    /// Priority
    pub priority: u32,
}

/// Audit logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLoggingConfig {
    /// Enable audit logging
    pub enabled: bool,
    /// Log level
    pub log_level: String,
    /// Log format
    pub log_format: String,
    /// Log destinations
    pub destinations: Vec<LogDestination>,
    /// Retention policy
    pub retention: RetentionPolicy,
}

/// Log destination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogDestination {
    /// Destination type (file, syslog, elasticsearch, s3)
    pub destination_type: String,
    /// Destination configuration
    pub config: HashMap<String, serde_json::Value>,
    /// Enabled
    pub enabled: bool,
}

/// Retention policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// Retention duration
    pub duration: Duration,
    /// Compression enabled
    pub compression: bool,
    /// Archive location
    pub archive_location: Option<String>,
}
