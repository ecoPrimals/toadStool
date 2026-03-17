// SPDX-License-Identifier: AGPL-3.0-only
//! Network-related constants
//!
//! Centralized network configuration values including ports, addresses,
//! and protocol defaults.

// ============================================================================
// Standard Service Ports
// ============================================================================

/// Default ToadStool server HTTP port
pub const DEFAULT_HTTP_PORT: u16 = 8080;

/// Default ToadStool server HTTPS port
pub const DEFAULT_HTTPS_PORT: u16 = 8443;

/// Alternative HTTP port for development
pub const DEV_HTTP_PORT: u16 = 3000;

/// Alternative HTTP port for testing
pub const TEST_HTTP_PORT: u16 = 9000;

/// Default `WebSocket` port
pub const DEFAULT_WS_PORT: u16 = 8081;

/// Default metrics/monitoring port
pub const METRICS_PORT: u16 = 9090;

/// Default health check port
pub const HEALTH_CHECK_PORT: u16 = 8082;

/// Default gRPC port for coordination services
pub const DEFAULT_GRPC_PORT: u16 = 50051;

/// Default coordination discovery fallback port
/// Overridable via `TOADSTOOL_COORDINATION_ENDPOINT` env var
pub const COORDINATION_FALLBACK_PORT: u16 = 50051;

/// Default BYOB (Bring Your Own Biome) coordinator/daemon port
pub const BYOB_DEFAULT_PORT: u16 = 8084;

// Primal-specific ports were removed — ToadStool discovers other primals
// at runtime via capability-based discovery (see `primal_identity.rs`,
// `discovery_defaults.rs`).

// Vendor service fallback ports (Redis, Postgres, etc.) were removed.
// Use env-based discovery: REDIS_URL, DATABASE_URL, MONGODB_URL, PROMETHEUS_URL,
// GRAFANA_URL, CONSUL_HTTP_ADDR, ETCD_ENDPOINTS.

// ============================================================================
// Address Constants
// ============================================================================

/// Localhost `IPv4` address
pub const LOCALHOST_IPV4: &str = "127.0.0.1";

/// Localhost `IPv6` address
pub const LOCALHOST_IPV6: &str = "::1";

/// Bind to all interfaces `IPv4`
pub const BIND_ALL_IPV4: &str = "0.0.0.0";

/// Bind to all interfaces `IPv6`
pub const BIND_ALL_IPV6: &str = "::";

/// Default hostname
pub const DEFAULT_HOSTNAME: &str = "localhost";

// ============================================================================
// Protocol Constants
// ============================================================================

/// HTTP protocol prefix
pub const HTTP_PROTOCOL: &str = "http://";

/// HTTPS protocol prefix
pub const HTTPS_PROTOCOL: &str = "https://";

/// `WebSocket` protocol prefix
#[deprecated(
    since = "0.5.0",
    note = "WebSocket is deprecated. Use JSON-RPC 2.0 polling instead."
)]
pub const WS_PROTOCOL: &str = "ws://";

/// Secure `WebSocket` protocol prefix
#[deprecated(
    since = "0.5.0",
    note = "WebSocket is deprecated. Use JSON-RPC 2.0 polling instead."
)]
pub const WSS_PROTOCOL: &str = "wss://";

// ============================================================================
// Helper Functions
// ============================================================================

/// Build HTTP URL from host and port
#[must_use]
pub fn http_url(host: &str, port: u16) -> String {
    format!("{HTTP_PROTOCOL}{host}:{port}")
}

/// Build HTTPS URL from host and port
#[must_use]
pub fn https_url(host: &str, port: u16) -> String {
    format!("{HTTPS_PROTOCOL}{host}:{port}")
}

/// Build `WebSocket` URL from host and port
#[must_use]
#[allow(deprecated)]
#[deprecated(
    since = "0.5.0",
    note = "WebSocket is deprecated. Use JSON-RPC 2.0 polling instead."
)]
pub fn ws_url(host: &str, port: u16) -> String {
    format!("{WS_PROTOCOL}{host}:{port}")
}

/// Build secure `WebSocket` URL from host and port
#[must_use]
#[allow(deprecated)]
#[deprecated(
    since = "0.5.0",
    note = "WebSocket is deprecated. Use JSON-RPC 2.0 polling instead."
)]
pub fn wss_url(host: &str, port: u16) -> String {
    format!("{WSS_PROTOCOL}{host}:{port}")
}

/// Default localhost HTTP URL
#[must_use]
pub fn default_http_url() -> String {
    http_url(LOCALHOST_IPV4, DEFAULT_HTTP_PORT)
}

/// Default localhost HTTPS URL
#[must_use]
pub fn default_https_url() -> String {
    https_url(LOCALHOST_IPV4, DEFAULT_HTTPS_PORT)
}

// ============================================================================
// Service Discovery URL Helpers (Deep Debt Compliant)
// ============================================================================

/// Get Consul HTTP address from environment or default
///
/// Priority:
/// 1. `CONSUL_HTTP_ADDR` environment variable (full URL)
/// 2. `CONSUL_HOST` + `CONSUL_PORT` environment variables
/// 3. `TOADSTOOL_CONSUL_DEFAULT_ADDR` (full URL override for fallback)
/// 4. Fallback to localhost:8500
#[must_use]
pub fn consul_http_addr() -> String {
    std::env::var("CONSUL_HTTP_ADDR").unwrap_or_else(|_| {
        std::env::var("TOADSTOOL_CONSUL_DEFAULT_ADDR").unwrap_or_else(|_| {
            let host =
                std::env::var("CONSUL_HOST").unwrap_or_else(|_| DEFAULT_HOSTNAME.to_string());
            let port = std::env::var("CONSUL_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8500);
            http_url(&host, port)
        })
    })
}

/// Get etcd endpoints from environment or default
///
/// Priority:
/// 1. `ETCD_ENDPOINTS` environment variable (comma-separated URLs)
/// 2. `ETCD_HOST` + `ETCD_PORT` environment variables
/// 3. `TOADSTOOL_ETCD_DEFAULT_ENDPOINTS` (full URL override for fallback)
/// 4. Fallback to localhost:2379
#[must_use]
pub fn etcd_endpoints() -> String {
    std::env::var("ETCD_ENDPOINTS").unwrap_or_else(|_| {
        std::env::var("TOADSTOOL_ETCD_DEFAULT_ENDPOINTS").unwrap_or_else(|_| {
            let host = std::env::var("ETCD_HOST").unwrap_or_else(|_| DEFAULT_HOSTNAME.to_string());
            let port = std::env::var("ETCD_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(2379);
            http_url(&host, port)
        })
    })
}

#[cfg(test)]
#[allow(unsafe_code)] // env::set_var/remove_var are unsafe in Rust 2024; test-only usage
mod tests {
    use super::*;

    #[test]
    fn test_default_ports() {
        assert_eq!(DEFAULT_HTTP_PORT, 8080);
        assert_eq!(DEFAULT_HTTPS_PORT, 8443);
        assert_eq!(DEV_HTTP_PORT, 3000);
        assert_eq!(TEST_HTTP_PORT, 9000);
        assert_eq!(DEFAULT_WS_PORT, 8081);
        assert_eq!(METRICS_PORT, 9090);
        assert_eq!(HEALTH_CHECK_PORT, 8082);
    }

    #[test]
    fn test_address_constants() {
        assert_eq!(LOCALHOST_IPV4, "127.0.0.1");
        assert_eq!(LOCALHOST_IPV6, "::1");
        assert_eq!(BIND_ALL_IPV4, "0.0.0.0");
        assert_eq!(BIND_ALL_IPV6, "::");
        assert_eq!(DEFAULT_HOSTNAME, "localhost");
    }

    #[test]
    #[allow(deprecated)]
    fn test_protocol_constants() {
        assert_eq!(HTTP_PROTOCOL, "http://");
        assert_eq!(HTTPS_PROTOCOL, "https://");
        assert_eq!(WS_PROTOCOL, "ws://");
        assert_eq!(WSS_PROTOCOL, "wss://");
    }

    #[test]
    #[allow(deprecated)]
    fn test_protocol_format() {
        assert!(HTTP_PROTOCOL.ends_with("://"));
        assert!(HTTPS_PROTOCOL.ends_with("://"));
        assert!(WS_PROTOCOL.ends_with("://"));
        assert!(WSS_PROTOCOL.ends_with("://"));
    }

    #[test]
    fn test_http_url() {
        assert_eq!(http_url("localhost", 8080), "http://localhost:8080");
        assert_eq!(http_url("127.0.0.1", 3000), "http://127.0.0.1:3000");
    }

    #[test]
    fn test_https_url() {
        assert_eq!(https_url("example.com", 443), "https://example.com:443");
    }

    #[test]
    #[allow(deprecated)]
    fn test_ws_url() {
        assert_eq!(ws_url("localhost", 8081), "ws://localhost:8081");
    }

    #[test]
    #[allow(deprecated)]
    fn test_wss_url() {
        assert_eq!(wss_url("example.com", 443), "wss://example.com:443");
    }

    #[test]
    fn test_default_http_url() {
        let url = default_http_url();
        assert_eq!(url, "http://127.0.0.1:8080");
    }

    #[test]
    fn test_default_https_url() {
        let url = default_https_url();
        assert_eq!(url, "https://127.0.0.1:8443");
    }

    #[test]
    fn test_consul_http_addr_default() {
        // SAFETY: Test-only; sequential test execution
        unsafe {
            std::env::remove_var("CONSUL_HTTP_ADDR");
            std::env::remove_var("CONSUL_HOST");
            std::env::remove_var("CONSUL_PORT");
        }
        let addr = consul_http_addr();
        assert!(addr.starts_with("http://"));
        assert!(addr.contains(":8500"));
    }

    #[test]
    fn test_consul_http_addr_from_env() {
        // SAFETY: Test-only; sequential test execution
        unsafe { std::env::set_var("CONSUL_HTTP_ADDR", "http://consul.example.com:8500") };
        let addr = consul_http_addr();
        assert_eq!(addr, "http://consul.example.com:8500");
        unsafe { std::env::remove_var("CONSUL_HTTP_ADDR") };
    }

    #[test]
    fn test_etcd_endpoints_default() {
        // SAFETY: Test-only; sequential test execution
        unsafe {
            std::env::remove_var("ETCD_ENDPOINTS");
            std::env::remove_var("ETCD_HOST");
            std::env::remove_var("ETCD_PORT");
        }
        let endpoints = etcd_endpoints();
        assert!(endpoints.starts_with("http://"));
        assert!(endpoints.contains(":2379"));
    }

    #[test]
    fn test_etcd_endpoints_from_env() {
        // SAFETY: Test-only; sequential test execution
        unsafe { std::env::set_var("ETCD_ENDPOINTS", "http://etcd.example.com:2379") };
        let endpoints = etcd_endpoints();
        assert_eq!(endpoints, "http://etcd.example.com:2379");
        unsafe { std::env::remove_var("ETCD_ENDPOINTS") };
    }

    #[test]
    fn test_etcd_endpoints_host_port_env() {
        // SAFETY: Test-only; sequential test execution
        unsafe {
            std::env::remove_var("ETCD_ENDPOINTS");
            std::env::set_var("ETCD_HOST", "etcd.local");
            std::env::set_var("ETCD_PORT", "2379");
        }
        let endpoints = etcd_endpoints();
        assert_eq!(endpoints, "http://etcd.local:2379");
        unsafe {
            std::env::remove_var("ETCD_HOST");
            std::env::remove_var("ETCD_PORT");
        }
    }

    #[test]
    fn test_consul_toadstool_default_addr() {
        // SAFETY: Test-only; sequential test execution
        unsafe {
            std::env::remove_var("CONSUL_HTTP_ADDR");
            std::env::remove_var("CONSUL_HOST");
            std::env::remove_var("CONSUL_PORT");
            std::env::set_var(
                "TOADSTOOL_CONSUL_DEFAULT_ADDR",
                "http://consul.override:8600",
            );
        }
        let addr = consul_http_addr();
        assert_eq!(addr, "http://consul.override:8600");
        unsafe { std::env::remove_var("TOADSTOOL_CONSUL_DEFAULT_ADDR") };
    }

    #[test]
    fn test_etcd_toadstool_default_endpoints() {
        // SAFETY: Test-only; sequential test execution
        unsafe {
            std::env::remove_var("ETCD_ENDPOINTS");
            std::env::remove_var("ETCD_HOST");
            std::env::remove_var("ETCD_PORT");
            std::env::set_var(
                "TOADSTOOL_ETCD_DEFAULT_ENDPOINTS",
                "http://etcd.override:2380",
            );
        }
        let endpoints = etcd_endpoints();
        assert_eq!(endpoints, "http://etcd.override:2380");
        unsafe { std::env::remove_var("TOADSTOOL_ETCD_DEFAULT_ENDPOINTS") };
    }
}
