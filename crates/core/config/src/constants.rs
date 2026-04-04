// SPDX-License-Identifier: AGPL-3.0-only
//! Configuration constants for ToadStool
//!
//! # ⚠️ DEPRECATION NOTICE
//!
//! **Hardcoded ports are being phased out** in favor of runtime discovery.
//! Use `RuntimeDiscovery` with `Capability` for modern primal-agnostic code.
//!
//! This module contains legacy well-known constants. For new code, use the
//! capability-based discovery system to find primals at runtime.

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
    fn test_capability_fallback_ports() {
        use crate::ports::capability_fallback;
        assert_eq!(capability_fallback::COORDINATION, 8080);
        assert_eq!(capability_fallback::SECURITY, 8081);
        assert_eq!(capability_fallback::STORAGE, 8082);
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
