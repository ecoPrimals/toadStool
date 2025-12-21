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
// Ecosystem Service Ports
// ============================================================================

/// Songbird (distributed messaging) default port
pub const SONGBIRD_PORT: u16 = 5000;

/// Songbird alternative port
pub const SONGBIRD_ALT_PORT: u16 = 5001;

/// BearDog (security) default port
pub const BEARDOG_PORT: u16 = 6000;

/// Squirrel (resource management) default port
pub const SQUIRREL_PORT: u16 = 7000;

/// NestGate (storage) default port
pub const NESTGATE_PORT: u16 = 8000;

// ============================================================================
// Common Infrastructure Ports
// ============================================================================

/// Redis default port
pub const REDIS_PORT: u16 = 6379;

/// `PostgreSQL` default port
pub const POSTGRES_PORT: u16 = 5432;

/// `MongoDB` default port
pub const MONGODB_PORT: u16 = 27017;

/// Prometheus default port
pub const PROMETHEUS_PORT: u16 = 9090;

/// Grafana default port
pub const GRAFANA_PORT: u16 = 3000;

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
