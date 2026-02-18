//! Network environment configuration.
//!
//! # Self-Knowledge Architecture
//!
//! ToadStool only knows itself. `TOADSTOOL_*` variables govern its own ports and
//! identity. Ports for other primals are deprecated legacy fields; the modern
//! approach is `RuntimeDiscovery::discover_capability(...)`.

use serde::{Deserialize, Serialize};

use super::loader::EnvConfigLoader;

/// Network configuration loaded from environment variables.
///
/// ## Self-Knowledge fields (valid):
/// - `toadstool_*` — our own ports and identity
/// - `bind_address` — where we listen
/// - `external_hostname` — how we identify ourselves
///
/// ## Legacy fields (deprecated):
/// - `songbird_port`, `beardog_port`, etc. — use `RuntimeDiscovery` instead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkEnvConfig {
    // ── Self-knowledge ──────────────────────────────────────────────────────
    /// ToadStool API port
    pub toadstool_port: u16,
    /// Federation port
    pub federation_port: u16,
    /// Metrics port
    pub metrics_port: u16,
    /// Health check port
    pub health_port: u16,
    /// WebSocket port
    pub websocket_port: u16,
    /// Bind address
    pub bind_address: String,
    /// External hostname (our identity)
    pub external_hostname: String,

    // ── Outbound connection behaviour ────────────────────────────────────────
    /// Enable TLS for outbound connections
    pub tls_enabled: bool,
    /// Connection timeout (seconds)
    pub connection_timeout_secs: u64,
    /// Request timeout (seconds)
    pub request_timeout_secs: u64,
    /// Max retries for failed requests
    pub max_retries: u32,
    /// Max connections per remote host
    pub max_connections_per_host: u32,

    // ── Legacy (deprecated) ──────────────────────────────────────────────────
    /// ⚠️ DEPRECATED — use `RuntimeDiscovery::discover_capability(&Capability::Coordination)`
    #[deprecated(
        since = "0.3.0",
        note = "Use RuntimeDiscovery for capability-based service discovery"
    )]
    pub songbird_port: u16,

    /// ⚠️ DEPRECATED — use `RuntimeDiscovery::discover_capability(&Capability::Authentication)`
    #[deprecated(
        since = "0.3.0",
        note = "Use RuntimeDiscovery for capability-based service discovery"
    )]
    pub beardog_port: u16,

    /// ⚠️ DEPRECATED — use `RuntimeDiscovery::discover_capability(&Capability::Storage)`
    #[deprecated(
        since = "0.3.0",
        note = "Use RuntimeDiscovery for capability-based service discovery"
    )]
    pub nestgate_port: u16,

    /// ⚠️ DEPRECATED — use `RuntimeDiscovery::discover_capability(&Capability::MCP)`
    #[deprecated(
        since = "0.3.0",
        note = "Use RuntimeDiscovery for capability-based service discovery"
    )]
    pub squirrel_port: u16,
}

impl NetworkEnvConfig {
    /// Load network configuration from environment variables.
    #[must_use]
    #[allow(deprecated)]
    pub fn from_env() -> Self {
        let loader = EnvConfigLoader::new();
        let ext = EnvConfigLoader::with_prefix(""); // unprefixed for other primals

        Self {
            songbird_port: ext.get_u16(
                "SONGBIRD_PORT",
                crate::defaults::network::COORDINATION_FALLBACK_PORT,
            ),
            beardog_port: ext.get_u16(
                "BEARDOG_PORT",
                crate::defaults::network::SECURITY_FALLBACK_PORT,
            ),
            nestgate_port: ext.get_u16(
                "NESTGATE_PORT",
                crate::defaults::network::STORAGE_FALLBACK_PORT,
            ),
            squirrel_port: ext.get_u16("SQUIRREL_PORT", crate::defaults::network::AI_FALLBACK_PORT),
            toadstool_port: loader.get_u16("TOADSTOOL_PORT", crate::defaults::network::API_PORT),
            federation_port: loader
                .get_u16("FEDERATION_PORT", crate::defaults::network::FEDERATION_PORT),
            metrics_port: loader.get_u16("METRICS_PORT", crate::defaults::network::METRICS_PORT),
            health_port: loader.get_u16("HEALTH_PORT", crate::defaults::network::DISCOVERY_PORT),
            websocket_port: loader
                .get_u16("WEBSOCKET_PORT", crate::defaults::network::WEBSOCKET_PORT),
            bind_address: loader.get_string("BIND_ADDRESS", "127.0.0.1"),
            external_hostname: loader.get_string("EXTERNAL_HOSTNAME", "localhost"),
            tls_enabled: loader.get_bool("TLS_ENABLED", false),
            connection_timeout_secs: loader.get_u64("CONNECTION_TIMEOUT_SECS", 10),
            request_timeout_secs: loader.get_u64("REQUEST_TIMEOUT_SECS", 30),
            max_retries: loader.get_u32("MAX_RETRIES", 3),
            max_connections_per_host: loader.get_u32("MAX_CONNECTIONS_PER_HOST", 100),
        }
    }

    // ── Self-knowledge endpoints ─────────────────────────────────────────────

    /// Our own API endpoint.
    #[must_use]
    pub fn toadstool_endpoint(&self) -> String {
        format!("http://{}:{}", self.external_hostname, self.toadstool_port)
    }

    /// Our federation endpoint.
    #[must_use]
    pub fn federation_endpoint(&self) -> String {
        format!("http://{}:{}", self.external_hostname, self.federation_port)
    }

    /// Our metrics endpoint.
    #[must_use]
    pub fn metrics_endpoint(&self) -> String {
        format!("http://{}:{}", self.external_hostname, self.metrics_port)
    }

    /// Our health endpoint.
    #[must_use]
    pub fn health_endpoint(&self) -> String {
        format!("http://{}:{}", self.external_hostname, self.health_port)
    }

    // ── Legacy endpoints (deprecated) ────────────────────────────────────────

    /// ⚠️ DEPRECATED — use `RuntimeDiscovery::discover_capability(&Capability::Coordination)`
    #[deprecated(
        since = "0.3.0",
        note = "Use RuntimeDiscovery::discover_capability(&Capability::Coordination)"
    )]
    #[must_use]
    pub fn songbird_endpoint(&self) -> String {
        format!("http://{}:{}", self.bind_address, 8080)
    }

    /// ⚠️ DEPRECATED — use `RuntimeDiscovery::discover_capability(&Capability::Authentication)`
    #[deprecated(
        since = "0.3.0",
        note = "Use RuntimeDiscovery::discover_capability(&Capability::Authentication)"
    )]
    #[must_use]
    pub fn beardog_endpoint(&self) -> String {
        format!("http://{}:{}", self.bind_address, 8081)
    }

    /// ⚠️ DEPRECATED — use `RuntimeDiscovery::discover_capability(&Capability::Storage)`
    #[deprecated(
        since = "0.3.0",
        note = "Use RuntimeDiscovery::discover_capability(&Capability::Storage)"
    )]
    #[must_use]
    pub fn nestgate_endpoint(&self) -> String {
        format!("http://{}:{}", self.bind_address, 8082)
    }

    /// ⚠️ DEPRECATED — use `RuntimeDiscovery::discover_capability(&Capability::MCP)`
    #[deprecated(
        since = "0.3.0",
        note = "Use RuntimeDiscovery::discover_capability(&Capability::MCP)"
    )]
    #[must_use]
    pub fn squirrel_endpoint(&self) -> String {
        format!("http://{}:{}", self.bind_address, 8083)
    }
}
