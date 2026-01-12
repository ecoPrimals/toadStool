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

// ============================================================================
// REMOVED: Primal-Specific Ports
// ============================================================================
//
// **INFANT DISCOVERY PATTERN**: ToadStool knows only itself.
// Other primals (songbird, nestgate, beardog, squirrel) are discovered at runtime
// via capability-based discovery. No hardcoded ports for other services.
//
// See: `primal_identity.rs` for self-knowledge implementation
// See: `discovery_defaults.rs` for runtime discovery fallbacks
// See: `HARDCODING_ELIMINATION_PLAN_JAN9_2026.md` for full migration plan
//
// Migration:
//   Before: let url = format!("http://localhost:{}", SONGBIRD_PORT);
//   After:  let service = discovery.find_service_by_capability(
//               Capability::Coordination(CoordinationCapability::ServiceDiscovery)
//           ).await?;
//           let url = service.endpoint();
//
// ============================================================================

// ============================================================================
// Vendor Service Fallback Defaults
// ============================================================================
//
// These are FALLBACK DEFAULTS for vendor services (redis, postgres, etc.)
// when explicit discovery fails. They should NOT be used as primary configuration.
// Prefer environment variables or discovery mechanisms.
//

/// Redis default fallback port (prefer discovery or environment variable)
#[deprecated(note = "Use discovery or REDIS_URL environment variable instead")]
pub const REDIS_FALLBACK_PORT: u16 = 6379;

/// `PostgreSQL` default fallback port (prefer discovery or environment variable)
#[deprecated(note = "Use discovery or DATABASE_URL environment variable instead")]
pub const POSTGRES_FALLBACK_PORT: u16 = 5432;

/// `MongoDB` default fallback port (prefer discovery or environment variable)
#[deprecated(note = "Use discovery or MONGODB_URL environment variable instead")]
pub const MONGODB_FALLBACK_PORT: u16 = 27017;

/// Prometheus default fallback port (prefer discovery or environment variable)
#[deprecated(note = "Use discovery or PROMETHEUS_URL environment variable instead")]
pub const PROMETHEUS_FALLBACK_PORT: u16 = 9090;

/// Grafana default fallback port (prefer discovery or environment variable)
#[deprecated(note = "Use discovery or GRAFANA_URL environment variable instead")]
pub const GRAFANA_FALLBACK_PORT: u16 = 3000;

/// Consul default fallback port (prefer discovery or CONSUL_HTTP_ADDR environment variable)
#[deprecated(note = "Use discovery or CONSUL_HTTP_ADDR environment variable instead")]
pub const CONSUL_FALLBACK_PORT: u16 = 8500;

/// etcd default fallback port (prefer discovery or environment variable)
#[deprecated(note = "Use discovery or ETCD_ENDPOINTS environment variable instead")]
pub const ETCD_FALLBACK_PORT: u16 = 2379;

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
pub const WS_PROTOCOL: &str = "ws://";

/// Secure `WebSocket` protocol prefix
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
pub fn ws_url(host: &str, port: u16) -> String {
    format!("{WS_PROTOCOL}{host}:{port}")
}

/// Build secure `WebSocket` URL from host and port
#[must_use]
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
/// 1. CONSUL_HTTP_ADDR environment variable (full URL)
/// 2. CONSUL_HOST + CONSUL_PORT environment variables
/// 3. Fallback to localhost:8500
#[must_use]
#[allow(deprecated)]
pub fn consul_http_addr() -> String {
    std::env::var("CONSUL_HTTP_ADDR").unwrap_or_else(|_| {
        let host = std::env::var("CONSUL_HOST").unwrap_or_else(|_| DEFAULT_HOSTNAME.to_string());
        let port = std::env::var("CONSUL_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(CONSUL_FALLBACK_PORT);
        http_url(&host, port)
    })
}

/// Get etcd endpoints from environment or default
///
/// Priority:
/// 1. ETCD_ENDPOINTS environment variable (comma-separated URLs)
/// 2. ETCD_HOST + ETCD_PORT environment variables
/// 3. Fallback to localhost:2379
#[must_use]
#[allow(deprecated)]
pub fn etcd_endpoints() -> String {
    std::env::var("ETCD_ENDPOINTS").unwrap_or_else(|_| {
        let host = std::env::var("ETCD_HOST").unwrap_or_else(|_| DEFAULT_HOSTNAME.to_string());
        let port = std::env::var("ETCD_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(ETCD_FALLBACK_PORT);
        http_url(&host, port)
    })
}
