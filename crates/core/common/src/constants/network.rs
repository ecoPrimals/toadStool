// SPDX-License-Identifier: AGPL-3.0-only
//! Network-related constants
//!
//! ToadStool uses JSON-RPC over Unix domain sockets per wateringHole standards.
//! This module holds bind/loopback literals, protocol prefixes for occasional
//! URL composition in integration code, and ToadStool self-knowledge ports only.

// ============================================================================
// ToadStool self-knowledge ports
// ============================================================================

/// Default metrics/monitoring port (Prometheus-style scrape target).
pub const METRICS_PORT: u16 = 9090;

/// Default BYOB (Bring Your Own Biome) coordinator/daemon port
pub const BYOB_DEFAULT_PORT: u16 = 8084;

// ============================================================================
// Address constants
// ============================================================================

/// Localhost `IPv4` address
pub const LOCALHOST_IPV4: &str = "127.0.0.1";

/// Localhost `IPv6` address
pub const LOCALHOST_IPV6: &str = "::1";

/// Bind to all interfaces `IPv4`
pub const BIND_ALL_IPV4: &str = "0.0.0.0";

/// Bind to any IPv4 interface with OS-assigned port (e.g. for mDNS socket)
pub const BIND_ANY: &str = "0.0.0.0:0";

/// Bind to all interfaces `IPv6`
pub const BIND_ALL_IPV6: &str = "::";

/// Default hostname
pub const DEFAULT_HOSTNAME: &str = "localhost";

// ============================================================================
// Protocol prefixes (used when composing URLs in cross-layer integration)
// ============================================================================

/// HTTP protocol prefix
pub const HTTP_PROTOCOL: &str = "http://";

/// HTTPS protocol prefix
pub const HTTPS_PROTOCOL: &str = "https://";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toadstool_ports() {
        assert_eq!(METRICS_PORT, 9090);
        assert_eq!(BYOB_DEFAULT_PORT, 8084);
    }

    #[test]
    fn test_address_constants() {
        assert_eq!(LOCALHOST_IPV4, "127.0.0.1");
        assert_eq!(LOCALHOST_IPV6, "::1");
        assert_eq!(BIND_ALL_IPV4, "0.0.0.0");
        assert_eq!(BIND_ANY, "0.0.0.0:0");
        assert_eq!(BIND_ALL_IPV6, "::");
        assert_eq!(DEFAULT_HOSTNAME, "localhost");
    }

    #[test]
    fn test_protocol_constants() {
        assert_eq!(HTTP_PROTOCOL, "http://");
        assert_eq!(HTTPS_PROTOCOL, "https://");
    }

    #[test]
    fn test_protocol_format() {
        assert!(HTTP_PROTOCOL.ends_with("://"));
        assert!(HTTPS_PROTOCOL.ends_with("://"));
    }
}
