// SPDX-License-Identifier: AGPL-3.0-or-later
//! Security Integration Module
//!
//! **Design Philosophy**:
//! - Runtime discovery: No hardcoded URLs or endpoints
//! - Capability-based: Discover Security by encryption capability
//! - Self-knowledge: Toadstool knows it needs crypto, not that The security service provides it
//! - Graceful degradation: Works without Security (fallback to local crypto)

pub mod client;
pub mod client_evolved;
pub mod crypto_dispatch;
pub mod discovery;
#[cfg(test)]
mod tests;
pub mod types;

pub use client::SecurityClient;
pub use crypto_dispatch::DistributedCryptoProvider;
pub use discovery::SecurityDiscovery;
pub use types::{
    EncryptionRequest, EncryptionResponse, KeyManagementRequest, KeyManagementResponse,
    SecurityCapability, SecurityEndpoint,
};

/// security service discovery configuration
///
/// **Design**: No hardcoded endpoints, discover at runtime
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Enable auto-discovery
    pub auto_discover: bool,

    /// Discovery timeout (milliseconds)
    pub discovery_timeout_ms: u64,

    /// Preferred service location
    pub preferred_location: ServiceLocation,

    /// Fallback to local crypto if Security unavailable
    pub fallback_enabled: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            auto_discover: true,
            discovery_timeout_ms: crate::common::defaults::DISCOVERY_TIMEOUT_MS,
            preferred_location: ServiceLocation::Local,
            fallback_enabled: true,
        }
    }
}

/// Service location preference
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceLocation {
    /// Prefer local Security instance
    Local,
    /// Prefer network security service
    Network,
    /// Any available
    Any,
}
