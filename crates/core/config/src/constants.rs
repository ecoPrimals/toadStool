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

/// # ⚠️ DEPRECATED: Ecosystem primal service names
///
/// **`WateringHole` Sovereignty**: Discover by CAPABILITY, not by hardcoded name.
/// Scan for what a service CAN DO, not what it IS CALLED.
///
/// **Use `RuntimeDiscovery` instead** for primal-agnostic, capability-based discovery.
///
/// These hardcoded names assume:
/// - Fixed primal names (not extensible)
/// - Known ecosystem topology (not dynamic)
/// - Centralized naming (not federated)
///
/// # Modern Alternative
///
/// ```rust,ignore
/// use toadstool_common::{RuntimeDiscovery, Capability};
///
/// // OLD (hardcoded name):
/// // let service_name = constants::primals::SONGBIRD;
///
/// // NEW (discover by capability):
/// let discovery = RuntimeDiscovery::new(client);
/// let services = discovery
///     .discover_capability(&Capability::Coordination)
///     .await?;
/// // Returns whatever coordinator is available, regardless of name
/// ```
///
/// **Philosophy**: ToadStool should only know about itself. Discover others at runtime.
#[deprecated(
    since = "0.3.0",
    note = "Use RuntimeDiscovery::discover_capability() for primal-agnostic service discovery. \
            Hardcoded primal names violate self-knowledge principle. See module docs."
)]
pub mod primals {
    #[allow(deprecated)]
    use toadstool_common::interned_strings::primals as canonical;

    /// Songbird service mesh identifier
    /// **DEPRECATED**: Discover coordinator by capability, not hardcoded name
    #[allow(deprecated)]
    pub const SONGBIRD: &str = canonical::SONGBIRD;

    /// BearDog security service identifier
    /// **DEPRECATED**: Discover security service by capability, not hardcoded name
    #[allow(deprecated)]
    pub const BEARDOG: &str = canonical::BEARDOG;

    /// NestGate storage service identifier
    /// **DEPRECATED**: Discover storage service by capability, not hardcoded name
    #[allow(deprecated)]
    pub const NESTGATE: &str = canonical::NESTGATE;

    /// ToadStool compute service identifier
    /// **This is self-knowledge** - the only canonical name for THIS primal.
    #[allow(deprecated)]
    pub const TOADSTOOL: &str = canonical::TOADSTOOL;

    /// Squirrel MCP platform identifier
    /// **DEPRECATED**: Discover MCP platform by capability, not hardcoded name
    #[allow(deprecated)]
    pub const SQUIRREL: &str = canonical::SQUIRREL;

    /// All known primal identifiers
    /// **DEPRECATED**: Cannot enumerate dynamic ecosystem - discover at runtime
    pub const ALL: &[&str] = &[SONGBIRD, BEARDOG, NESTGATE, TOADSTOOL, SQUIRREL];
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
    #[allow(deprecated)] // Testing legacy constants during migration
    fn test_primal_constants() {
        assert_eq!(primals::SONGBIRD, "songbird");
        assert_eq!(primals::BEARDOG, "beardog");
        assert_eq!(primals::ALL.len(), 5);
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
