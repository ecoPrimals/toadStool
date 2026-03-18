// SPDX-License-Identifier: AGPL-3.0-or-later
//! Configuration constants for ToadStool
//!
//! # ⚠️ DEPRECATION NOTICE
//!
//! **Hardcoded ports are being phased out** in favor of runtime discovery.
//! Use `RuntimeDiscovery` with `Capability` for modern primal-agnostic code.
//!
//! This module contains legacy well-known constants. For new code, use the
//! capability-based discovery system to find primals at runtime.

/// **DEPRECATED**: Use `toadstool_config::ports::capability_fallback` for port defaults,
/// or `RuntimeDiscovery::discover_capability()` for production.
#[deprecated(
    since = "0.3.0",
    note = "Use toadstool_config::ports::capability_fallback for port defaults."
)]
pub mod ports {
    use crate::ports::capability_fallback;

    #[deprecated(note = "Use ports::capability_fallback::COORDINATION")]
    pub const SONGBIRD: u16 = capability_fallback::COORDINATION;

    #[deprecated(note = "Use ports::capability_fallback::SECURITY")]
    pub const BEARDOG: u16 = capability_fallback::SECURITY;

    #[deprecated(note = "Use ports::capability_fallback::STORAGE")]
    pub const NESTGATE: u16 = capability_fallback::STORAGE;

    #[deprecated(note = "Use defaults::network::API_PORT for ToadStool self-config")]
    pub const TOADSTOOL: u16 = 0;

    #[deprecated(note = "Use ports::capability_fallback::PLATFORM")]
    pub const SQUIRREL: u16 = capability_fallback::PLATFORM;
}

/// Capability-based service identifiers (preferred)
///
/// Use these for discovery and routing. Re-exports from `toadstool_common::interned_strings::capabilities`.
pub mod capabilities {
    pub use toadstool_common::constants::PRIMAL_NAME;
    pub use toadstool_common::interned_strings::capabilities::{
        COORDINATION, CRYPTO, INTELLIGENCE, STORAGE,
    };

    /// All known capability identifiers
    pub const ALL: &[&str] = &[COORDINATION, CRYPTO, STORAGE, PRIMAL_NAME, INTELLIGENCE];
}

/// Default network configuration
pub mod network {
    /// Default bind address (all interfaces)
    pub const DEFAULT_BIND_ADDR: &str = "0.0.0.0";

    /// Localhost address
    pub const LOCALHOST: &str = "127.0.0.1";

    /// `IPv6` localhost
    pub const LOCALHOST_V6: &str = "::1";

    /// Default HTTP scheme
    pub const HTTP_SCHEME: &str = "http";

    /// Default HTTPS scheme
    pub const HTTPS_SCHEME: &str = "https";
}

/// Timeout constants (in seconds)
pub mod timeouts {
    /// Default operation timeout
    pub const DEFAULT: u64 = 30;

    /// Short operation timeout
    pub const SHORT: u64 = 5;

    /// Long operation timeout
    pub const LONG: u64 = 120;

    /// Network discovery timeout
    pub const DISCOVERY: u64 = 10;

    /// Health check timeout
    pub const HEALTH_CHECK: u64 = 5;
}

// Compile-time validation of timeout ordering
// These assertions are evaluated during compilation, catching configuration errors early
const _: () = assert!(timeouts::SHORT < timeouts::DEFAULT);
const _: () = assert!(timeouts::DEFAULT < timeouts::LONG);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(deprecated)] // Testing legacy constants during migration
    fn test_port_constants() {
        assert_eq!(ports::SONGBIRD, 8080);
        assert_eq!(ports::BEARDOG, 8081);
        assert_eq!(ports::NESTGATE, 8082); // Consolidated with ports::discovery_fallback
    }

    #[test]
    fn test_capability_constants() {
        assert_eq!(capabilities::COORDINATION, "coordination");
        assert_eq!(capabilities::CRYPTO, "crypto");
        assert_eq!(capabilities::ALL.len(), 5);
    }

    #[test]
    fn test_timeout_constants() {
        // Verify timeout ordering at compile time
        const _: () = assert!(timeouts::SHORT < timeouts::DEFAULT);
        const _: () = assert!(timeouts::DEFAULT < timeouts::LONG);
    }
}

// Compile-time validation of timeout ordering moved before test module
// These assertions are evaluated during compilation, catching configuration errors early
