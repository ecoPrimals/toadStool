//! Configuration constants for ToadStool
//!
//! # ⚠️ DEPRECATION NOTICE
//!
//! **Hardcoded ports are being phased out** in favor of runtime discovery.
//! Use `RuntimeDiscovery` with `Capability` for modern primal-agnostic code.
//!
//! This module contains legacy well-known constants. For new code, use the
//! capability-based discovery system to find primals at runtime.

/// # ⚠️ DEPRECATED: Default network ports for ecosystem primals
///
/// **Use `RuntimeDiscovery` instead** for primal-agnostic, runtime-discovered services.
///
/// These hardcoded ports assume:
/// - Primals run on specific ports (brittle)
/// - Single-instance deployments (not scalable)
/// - Fixed port assignments (conflicts in multi-instance)
///
/// # Modern Alternative
///
/// ```rust,ignore
/// use toadstool_common::{RuntimeDiscovery, Capability};
///
/// // OLD (hardcoded):
/// // let songbird_port = constants::ports::SONGBIRD; // 8080
///
/// // NEW (discovered):
/// let discovery = RuntimeDiscovery::new(client);
/// let coordinators = discovery
///     .discover_capability(&Capability::Coordination)
///     .await?;
/// let coordinator_endpoint = &coordinators[0].endpoint;
/// ```
///
/// See: `toadstool-common::RuntimeDiscovery` for capability-based discovery
#[deprecated(
    since = "0.3.0",
    note = "Use RuntimeDiscovery::discover_capability() for primal-agnostic service discovery. \
            Hardcoded ports are being eliminated. See module docs for migration examples."
)]
pub mod ports {
    /// Default Songbird service mesh port
    /// **DEPRECATED**: Use `RuntimeDiscovery` to find coordinator at runtime
    pub const SONGBIRD: u16 = 8080;

    /// Default BearDog security service port
    /// **DEPRECATED**: Use `RuntimeDiscovery` to find security service at runtime
    pub const BEARDOG: u16 = 8081;

    /// Default NestGate storage service port
    /// **DEPRECATED**: Use `RuntimeDiscovery` to find storage service at runtime
    pub const NESTGATE: u16 = 9000;

    /// Default ToadStool compute service port
    /// **DEPRECATED**: Use `RuntimeDiscovery` to find compute service at runtime
    pub const TOADSTOOL: u16 = 7000;

    /// Default Squirrel MCP platform port
    /// **DEPRECATED**: Use `RuntimeDiscovery` to find MCP platform at runtime
    pub const SQUIRREL: u16 = 6000;
}

/// # ⚠️ DEPRECATED: Ecosystem primal service names
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
    /// Songbird service mesh identifier
    /// **DEPRECATED**: Discover coordinator by capability, not hardcoded name
    pub const SONGBIRD: &str = "songbird";

    /// BearDog security service identifier  
    /// **DEPRECATED**: Discover security service by capability, not hardcoded name
    pub const BEARDOG: &str = "beardog";

    /// NestGate storage service identifier
    /// **DEPRECATED**: Discover storage service by capability, not hardcoded name
    pub const NESTGATE: &str = "nestgate";

    /// ToadStool compute service identifier
    /// **This is self-knowledge** - acceptable for identifying ourselves
    pub const TOADSTOOL: &str = "toadstool";

    /// Squirrel MCP platform identifier
    /// **DEPRECATED**: Discover MCP platform by capability, not hardcoded name
    pub const SQUIRREL: &str = "squirrel";

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

    /// IPv6 localhost
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
        assert_eq!(ports::NESTGATE, 9000);
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
