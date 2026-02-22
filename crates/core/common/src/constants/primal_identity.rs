//! Primal Identity Constants — Self-Knowledge Module
//!
//! ToadStool only has knowledge of itself. Other primals are discovered at
//! runtime through capability-based discovery (Songbird, BirdSong, mDNS).
//!
//! This module defines:
//! - **Self-knowledge**: ToadStool's own identity, app name, socket paths
//! - **Capability identifiers**: Well-known capability strings used for discovery
//! - **Well-known service roles**: The *roles* primals can fill (not their names)
//!
//! ## Design Principles
//!
//! 1. **Self-knowledge only**: ToadStool knows its own name, version, and
//!    capabilities. It never hardcodes another primal's name or port.
//! 2. **Capability-based discovery**: When ToadStool needs crypto, it asks for
//!    `capability::CRYPTO_PROVIDER`, not "beardog".
//! 3. **Runtime resolution**: Socket paths, ports, and endpoints for external
//!    primals come from discovery, environment variables, or config — never
//!    from compiled-in constants.
//!
//! ## Migration
//!
//! ```text
//! Before: get_socket_path_for_service("beardog")
//! After:  discovery.resolve_by_capability(capability::CRYPTO_PROVIDER)
//! ```

// ==========================================================================
// Self-Knowledge: This Primal's Identity
// ==========================================================================

/// This primal's canonical name.
///
/// Used for socket paths, config directories, IPC registration, and log
/// prefixes. Every occurrence of the string `"toadstool"` in production
/// code should reference this constant.
pub const PRIMAL_NAME: &str = "toadstool";

/// Human-readable display name (used in UI, logs, user-agent strings).
pub const PRIMAL_DISPLAY_NAME: &str = "ToadStool";

/// Primal description for capability advertisements.
pub const PRIMAL_DESCRIPTION: &str = "Universal compute orchestrator";

// ==========================================================================
// Well-Known Capability Identifiers (for runtime discovery)
// ==========================================================================

/// Capability identifiers used to discover ecosystem services at runtime.
///
/// ToadStool never asks for a primal *by name*. It asks for a primal
/// that provides a particular *capability*. The discovery layer (Songbird,
/// BirdSong, mDNS, or environment fallback) resolves the capability to a
/// concrete endpoint.
pub mod capability {
    /// Cryptographic operations (key derivation, encryption, signing).
    pub const CRYPTO_PROVIDER: &str = "crypto.provider";

    /// Service discovery and coordination.
    pub const SERVICE_DISCOVERY: &str = "discovery.coordination";

    /// Encrypted multicast discovery protocol.
    pub const BEACON_PROTOCOL: &str = "discovery.beacon";

    /// High-performance network gateway with zero-copy I/O.
    pub const NETWORK_GATEWAY: &str = "network.gateway";

    /// Persistent key–value and structured storage.
    pub const STORAGE_PROVIDER: &str = "storage.provider";

    /// System health monitoring and orchestration.
    pub const SYSTEM_MONITOR: &str = "system.monitor";

    /// AI/ML model serving and inference.
    pub const ML_INFERENCE: &str = "ml.inference";

    /// GPU/NPU compute execution.
    pub const COMPUTE_EXECUTION: &str = "compute.execution";
}

// ==========================================================================
// Well-Known Platform Paths
// ==========================================================================

/// biomeOS platform directory name.
///
/// This is a *platform convention*, not primal-specific knowledge. It is
/// analogous to knowing that Linux uses `/etc` — ToadStool must know the
/// standard platform directory names to function on a biomeOS system.
pub const PLATFORM_DIR: &str = "biomeos";

// ==========================================================================
// Well-Known Audience/Issuer Identifiers
// ==========================================================================

/// Standard audience values for token validation.
///
/// When running on a biomeOS platform, tokens may be scoped to a set of
/// well-known audience identifiers. These are part of the *platform protocol*,
/// not primal-specific knowledge.
pub mod audience {
    /// This primal's audience claim.
    pub const SELF_AUDIENCE: &str = super::PRIMAL_NAME;

    /// Platform-wide audience (accepted by all primals).
    pub const PLATFORM_AUDIENCE: &str = "biomeos";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn self_knowledge_is_consistent() {
        assert_eq!(PRIMAL_NAME, "toadstool");
        assert!(PRIMAL_DISPLAY_NAME.to_lowercase().contains(PRIMAL_NAME));
    }

    #[test]
    fn capabilities_are_namespaced() {
        assert!(capability::CRYPTO_PROVIDER.contains('.'));
        assert!(capability::SERVICE_DISCOVERY.contains('.'));
        assert!(capability::NETWORK_GATEWAY.contains('.'));
        assert!(capability::STORAGE_PROVIDER.contains('.'));
    }

    #[test]
    fn audience_self_matches_identity() {
        assert_eq!(audience::SELF_AUDIENCE, PRIMAL_NAME);
    }
}
