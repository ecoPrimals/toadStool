// SPDX-License-Identifier: AGPL-3.0-only
//! Network environment configuration.
//!
//! # Self-Knowledge Architecture
//!
//! ToadStool only knows itself. `TOADSTOOL_*` variables govern its own ports and
//! identity. Ports for other primals are deprecated legacy fields; the modern
//! approach is `RuntimeDiscovery::discover_capability(...)`.

use serde::{Deserialize, Serialize};

use toadstool_common::constants::network::DEFAULT_HOSTNAME;

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
    /// Port for JSON-RPC event streaming (replaces deprecated `WebSocket`)
    pub events_port: u16,
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

    /// BiomeOS primary port (ecosystem discovery)
    pub biomeos_port: u16,
}

impl NetworkEnvConfig {
    /// Load network configuration from environment variables.
    #[must_use]
    #[allow(deprecated)] // Struct fields deprecated; from_env still needed for bootstrap
    pub fn from_env() -> Self {
        use crate::ports::{capability_fallback, resolve_capability_port};

        let loader = EnvConfigLoader::new();

        Self {
            songbird_port: resolve_capability_port(
                "COORDINATION",
                capability_fallback::COORDINATION,
            ),
            beardog_port: resolve_capability_port("SECURITY", capability_fallback::SECURITY),
            nestgate_port: resolve_capability_port("STORAGE", capability_fallback::STORAGE),
            squirrel_port: resolve_capability_port("PLATFORM", capability_fallback::PLATFORM),
            biomeos_port: resolve_capability_port(
                "ECOSYSTEM",
                capability_fallback::ECOSYSTEM_PRIMARY,
            ),
            toadstool_port: loader.get_u16("TOADSTOOL_PORT", crate::defaults::network::API_PORT),
            federation_port: loader
                .get_u16("FEDERATION_PORT", crate::defaults::network::FEDERATION_PORT),
            metrics_port: loader.get_u16("METRICS_PORT", crate::defaults::network::METRICS_PORT),
            health_port: loader.get_u16("HEALTH_PORT", crate::defaults::network::DISCOVERY_PORT),
            events_port: loader.get_u16("EVENTS_PORT", crate::defaults::network::EVENTS_PORT),
            bind_address: loader.get_string(
                "BIND_ADDRESS",
                crate::defaults::network::BIND_ADDRESS_DEFAULT,
            ),
            external_hostname: loader.get_string("EXTERNAL_HOSTNAME", DEFAULT_HOSTNAME),
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
    #[allow(deprecated)]
    pub fn songbird_endpoint(&self) -> String {
        // Client-side: connect to other primal. Port 0 = use capability discovery.
        format!("http://{}:{}", DEFAULT_HOSTNAME, self.songbird_port)
    }

    /// ⚠️ DEPRECATED — use `RuntimeDiscovery::discover_capability(&Capability::Authentication)`
    #[deprecated(
        since = "0.3.0",
        note = "Use RuntimeDiscovery::discover_capability(&Capability::Authentication)"
    )]
    #[must_use]
    #[allow(deprecated)]
    pub fn beardog_endpoint(&self) -> String {
        // Client-side: connect to other primal. Port 0 = use capability discovery.
        format!("http://{}:{}", DEFAULT_HOSTNAME, self.beardog_port)
    }

    /// ⚠️ DEPRECATED — use `RuntimeDiscovery::discover_capability(&Capability::Storage)`
    #[deprecated(
        since = "0.3.0",
        note = "Use RuntimeDiscovery::discover_capability(&Capability::Storage)"
    )]
    #[must_use]
    #[allow(deprecated)]
    pub fn nestgate_endpoint(&self) -> String {
        // Client-side: connect to other primal. Port 0 = use capability discovery.
        format!("http://{}:{}", DEFAULT_HOSTNAME, self.nestgate_port)
    }

    /// ⚠️ DEPRECATED — use `RuntimeDiscovery::discover_capability(&Capability::MCP)`
    #[deprecated(
        since = "0.3.0",
        note = "Use RuntimeDiscovery::discover_capability(&Capability::MCP)"
    )]
    #[must_use]
    #[allow(deprecated)]
    pub fn squirrel_endpoint(&self) -> String {
        // Client-side: connect to other primal. Port 0 = use capability discovery.
        format!("http://{}:{}", DEFAULT_HOSTNAME, self.squirrel_port)
    }
}
