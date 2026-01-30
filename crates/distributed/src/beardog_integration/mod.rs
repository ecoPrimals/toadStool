//! BearDog Integration Module
//!
//! **Design Philosophy**:
//! - Runtime discovery: No hardcoded URLs or endpoints
//! - Capability-based: Discover BearDog by encryption capability
//! - Self-knowledge: Toadstool knows it needs crypto, not that BearDog provides it
//! - Graceful degradation: Works without BearDog (fallback to local crypto)

pub mod client;
pub mod client_evolved;
pub mod types;

pub use client::{BearDogClient, BearDogDiscovery};
pub use types::{
    BearDogCapability, BearDogEndpoint, EncryptionRequest, EncryptionResponse,
    KeyManagementRequest, KeyManagementResponse,
};

/// BearDog service discovery configuration
///
/// **Design**: No hardcoded endpoints, discover at runtime
#[derive(Debug, Clone)]
pub struct BearDogConfig {
    /// Enable auto-discovery
    pub auto_discover: bool,

    /// Discovery timeout (milliseconds)
    pub discovery_timeout_ms: u64,

    /// Preferred service location
    pub preferred_location: ServiceLocation,

    /// Fallback to local crypto if BearDog unavailable
    pub fallback_enabled: bool,
}

impl Default for BearDogConfig {
    fn default() -> Self {
        Self {
            auto_discover: true,
            discovery_timeout_ms: 5000,
            preferred_location: ServiceLocation::Local,
            fallback_enabled: true,
        }
    }
}

/// Service location preference
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceLocation {
    /// Prefer local BearDog instance
    Local,
    /// Prefer network BearDog service
    Network,
    /// Any available
    Any,
}
