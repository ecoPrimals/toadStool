//! Configuration constants for ToadStool
//!
//! This module contains well-known constants used throughout the system.
//! Following modern Rust idioms, these are centralized for maintainability.

/// Default network ports for ecosystem primals
pub mod ports {
    /// Default Songbird service mesh port
    pub const SONGBIRD: u16 = 8080;

    /// Default BearDog security service port
    pub const BEARDOG: u16 = 8081;

    /// Default NestGate storage service port
    pub const NESTGATE: u16 = 9000;

    /// Default ToadStool compute service port
    pub const TOADSTOOL: u16 = 7000;

    /// Default Squirrel MCP platform port
    pub const SQUIRREL: u16 = 6000;
}

/// Ecosystem primal service names
pub mod primals {
    /// Songbird service mesh identifier
    pub const SONGBIRD: &str = "songbird";

    /// BearDog security service identifier
    pub const BEARDOG: &str = "beardog";

    /// NestGate storage service identifier
    pub const NESTGATE: &str = "nestgate";

    /// ToadStool compute service identifier
    pub const TOADSTOOL: &str = "toadstool";

    /// Squirrel MCP platform identifier
    pub const SQUIRREL: &str = "squirrel";

    /// All known primal identifiers
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_constants() {
        assert_eq!(ports::SONGBIRD, 8080);
        assert_eq!(ports::BEARDOG, 8081);
        assert_eq!(ports::NESTGATE, 9000);
    }

    #[test]
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
