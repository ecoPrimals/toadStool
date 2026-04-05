// SPDX-License-Identifier: AGPL-3.0-or-later
//! Network environment configuration.
//!
//! # Self-Knowledge Architecture
//!
//! ToadStool only knows itself. `TOADSTOOL_*` variables govern its own ports and
//! identity. Optional capability ports (`coordination_port`, etc.) are bootstrap
//! hints for outbound connections; the preferred approach is
//! `RuntimeDiscovery::discover_capability(...)`.

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
/// ## Capability-oriented ports (optional bootstrap):
/// - `coordination_port`, `security_port`, `storage_port`, `ai_processing_port` —
///   use `RuntimeDiscovery` for capability-based service discovery in production.
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

    // ── Capability bootstrap ports (peer services) ─────────────────────────
    /// Coordination service port (`RuntimeDiscovery::discover_capability(&Capability::Coordination)`)
    #[serde(alias = "songbird_port")]
    pub coordination_port: u16,

    /// Security / PKI port (`RuntimeDiscovery::discover_capability(&Capability::Authentication)`)
    #[serde(alias = "beardog_port")]
    pub security_port: u16,

    /// Storage port (`RuntimeDiscovery::discover_capability(&Capability::Storage)`)
    #[serde(alias = "nestgate_port")]
    pub storage_port: u16,

    /// AI / platform processing port (`RuntimeDiscovery::discover_capability(&Capability::MCP)`)
    #[serde(alias = "squirrel_port")]
    pub ai_processing_port: u16,

    /// BiomeOS primary port (ecosystem discovery)
    pub biomeos_port: u16,
}

impl NetworkEnvConfig {
    /// Load network configuration from environment variables.
    #[must_use]
    pub fn from_env() -> Self {
        use crate::ports::{capability_fallback, resolve_capability_port};

        let loader = EnvConfigLoader::new();

        Self {
            coordination_port: resolve_capability_port(
                "COORDINATION",
                capability_fallback::COORDINATION,
            ),
            security_port: resolve_capability_port("SECURITY", capability_fallback::SECURITY),
            storage_port: resolve_capability_port("STORAGE", capability_fallback::STORAGE),
            ai_processing_port: resolve_capability_port("PLATFORM", capability_fallback::PLATFORM),
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

    // ── Outbound capability endpoints ───────────────────────────────────────

    /// Coordination service endpoint (client-side; port 0 = use capability discovery).
    #[must_use]
    pub fn coordination_endpoint(&self) -> String {
        format!("http://{}:{}", DEFAULT_HOSTNAME, self.coordination_port)
    }

    /// Security / PKI endpoint (client-side; port 0 = use capability discovery).
    #[must_use]
    pub fn security_endpoint(&self) -> String {
        format!("http://{}:{}", DEFAULT_HOSTNAME, self.security_port)
    }

    /// Storage endpoint (client-side; port 0 = use capability discovery).
    #[must_use]
    pub fn storage_endpoint(&self) -> String {
        format!("http://{}:{}", DEFAULT_HOSTNAME, self.storage_port)
    }

    /// AI / platform processing endpoint (client-side; port 0 = use capability discovery).
    #[must_use]
    pub fn ai_processing_endpoint(&self) -> String {
        format!("http://{}:{}", DEFAULT_HOSTNAME, self.ai_processing_port)
    }
}
