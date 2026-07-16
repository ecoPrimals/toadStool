// SPDX-License-Identifier: AGPL-3.0-or-later
//! Crypto Integration Module - Vendor-Agnostic Cryptographic Services
//!
//! **Design Philosophy (Infant Discovery)**:
//! - ✅ Zero hardcoding: Discovers crypto services by capability, not by name
//! - ✅ Self-knowledge: ToadStool knows it needs crypto, not which provider implements it
//! - ✅ Multi-vendor: Works with Security, HashiCorp Vault, AWS KMS, Azure Key Vault, etc.
//! - ✅ Runtime discovery: Uses mDNS, service registries, or environment configuration
//! - ✅ Graceful degradation: Falls back to local crypto if no service available
//!
//! ## Migration from security
//!
//! This module replaces `security` with a capability-based approach:
//!
//! **Before (hardcoded)**:
//! ```ignore
//! use crate::security::{SecurityClient, SecurityDiscovery};
//! let discovery = SecurityDiscovery::new(config);
//! let endpoints = discovery.discover().await?;
//! ```
//!
//! **After (capability-based)**:
//! ```ignore
//! use crate::crypto_integration::{CryptoServiceClient, CryptoServiceDiscovery};
//! use toadstool_common::primal_identity::{Capability, CryptoCapability};
//!
//! let discovery = CryptoServiceDiscovery::new(config);
//! let service = discovery
//!     .discover_by_capability(Capability::Crypto(CryptoCapability::Encryption))
//!     .await?;
//! ```
//!
//! ## Supported Providers
//!
//! Any service advertising crypto capabilities will work:
//! - Security (ecoPrimals native)
//! - HashiCorp Vault
//! - AWS KMS / Secrets Manager
//! - Azure Key Vault
//! - Google Cloud KMS
//! - CyberArk
//! - Thales HSM
//! - Local crypto (fallback)

pub mod client;
pub mod types;

pub use client::{CryptoServiceClient, CryptoServiceDiscovery};
pub use types::{
    CryptoOperation, CryptoRequest, CryptoResponse, EncryptionAlgorithm, KeyType,
    PermissionResponse, RevocationRequest, SecurityLevel, SignatureRequest, SignatureResponse,
    ValidationResponse, VerificationRequest, VerificationResponse, encryption_algorithm_from_wire,
};

/// Crypto service discovery configuration
///
/// **Design**: No hardcoded endpoints, discover at runtime
#[derive(Debug, Clone)]
pub struct CryptoServiceConfig {
    /// Enable auto-discovery
    pub auto_discover: bool,

    /// Discovery timeout (milliseconds)
    pub discovery_timeout_ms: u64,

    /// Preferred service location
    pub preferred_location: ServiceLocation,

    /// Fallback to local crypto if no service available
    pub fallback_enabled: bool,

    /// Required capabilities (filter discovered services)
    pub required_capabilities: Vec<toadstool_common::primal_identity::CryptoCapability>,
}

impl Default for CryptoServiceConfig {
    fn default() -> Self {
        use toadstool_common::primal_identity::CryptoCapability;

        Self {
            auto_discover: true,
            discovery_timeout_ms: crate::common::defaults::DISCOVERY_TIMEOUT_MS,
            preferred_location: ServiceLocation::Any,
            fallback_enabled: true,
            required_capabilities: vec![
                CryptoCapability::Encryption,
                CryptoCapability::KeyManagement,
            ],
        }
    }
}

/// Service location preference
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceLocation {
    /// Prefer local service instance
    Local,
    /// Prefer network service
    Network,
    /// Any available
    Any,
}
