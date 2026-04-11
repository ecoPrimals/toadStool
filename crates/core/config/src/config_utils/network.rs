// SPDX-License-Identifier: AGPL-3.0-or-later
//! Network configuration utilities
//!
//! Port constants, network configuration helpers, endpoints, and connection settings.

use std::collections::HashMap;
use std::time::Duration;
use toadstool_common::constants::network::DEFAULT_HOSTNAME;

use crate::env_config::{EnvConfigLoader, NetworkEnvConfig};
use crate::network;

/// Get primal port by capability/env name (capability-based lookup)
///
/// **Purpose**: Defaults for initial connection discovery before runtime discovery.
/// Used when capability-based discovery (coordination service, mDNS) is not yet available.
///
/// **Production**: Prefer `RuntimeDiscovery::discover_capability()` instead.
/// This is the cold-start bootstrap path only.
///
/// **Env pattern**: Prefer `TOADSTOOL_{CAPABILITY}_PORT` and `{CAPABILITY}_PORT` (e.g.
/// `TOADSTOOL_COORDINATION_PORT`, `COORDINATION_PORT`), then legacy `{PRIMAL_NAME}_PORT`
/// (legacy env names) via [`crate::ports::resolve_capability_port`].
#[must_use]
#[deprecated(
    since = "0.92.0",
    note = "Use capability-based discovery via infant_discovery instead of primal-name port lookup"
)]
pub fn get_primal_default_port(primal_name: &str) -> u16 {
    use crate::ports::{capability_fallback, resolve_capability_port};

    let (capability, fallback) = match primal_name {
        // Legacy primal names map to wateringHole capabilities; port env resolution
        // prefers TOADSTOOL_{CAPABILITY}_PORT / {CAPABILITY}_PORT, then *_PORT legacy names.
        "SONGBIRD" => ("COORDINATION", capability_fallback::COORDINATION),
        "BEARDOG" => ("SECURITY", capability_fallback::SECURITY),
        "NESTGATE" => ("STORAGE", capability_fallback::STORAGE),
        // Legacy intelligence / platform processing mapping (see TOADSTOOL_INTELLIGENCE_* env vars)
        "SQUIRREL" => ("PLATFORM", capability_fallback::PLATFORM),
        "BIOMEOS" => ("ECOSYSTEM", capability_fallback::ECOSYSTEM),
        _ => return crate::defaults::network::API_PORT,
    };

    resolve_capability_port(capability, fallback)
}

/// Get ToadStool port from environment or default
#[must_use]
pub fn get_toadstool_port() -> u16 {
    let loader = EnvConfigLoader::with_prefix("");
    loader.get_u16("TOADSTOOL_PORT", crate::defaults::network::API_PORT)
}

/// Get federation port from environment or default
#[must_use]
pub fn get_federation_port() -> u16 {
    let config = crate::env_config::EnvironmentConfig::from_env();
    let loader = EnvConfigLoader::new();
    loader.get_u16("FEDERATION_PORT", config.network.federation_port)
}

/// Get metrics port from environment or default
#[must_use]
pub fn get_metrics_port() -> u16 {
    let config = crate::env_config::EnvironmentConfig::from_env();
    let loader = EnvConfigLoader::new();
    loader.get_u16("METRICS_PORT", config.network.metrics_port)
}

/// Get health check port from environment or default
#[must_use]
pub fn get_health_port() -> u16 {
    let config = crate::env_config::EnvironmentConfig::from_env();
    let loader = EnvConfigLoader::new();
    loader.get_u16("HEALTH_PORT", config.network.health_port)
}

/// Get events port from environment or default (JSON-RPC event streaming)
#[must_use]
pub fn get_events_port() -> u16 {
    let config = crate::env_config::EnvironmentConfig::from_env();
    let loader = EnvConfigLoader::new();
    loader.get_u16("EVENTS_PORT", config.network.events_port)
}

/// Get bind address from environment or default
#[must_use]
pub fn get_bind_address() -> String {
    let loader = EnvConfigLoader::with_prefix("");
    loader.get_string(
        "BIND_ADDRESS",
        crate::defaults::network::BIND_ADDRESS_DEFAULT,
    )
}

/// Get external hostname from environment or default
#[must_use]
pub fn get_external_hostname() -> String {
    let loader = EnvConfigLoader::new();
    loader.get_string("EXTERNAL_HOSTNAME", DEFAULT_HOSTNAME)
}

/// Get ToadStool endpoint from environment or default
#[must_use]
pub fn get_toadstool_endpoint() -> String {
    let net_config = NetworkEnvConfig::from_env();
    net_config.toadstool_endpoint()
}

/// Get request timeout from environment or default
#[must_use]
pub fn get_request_timeout() -> Duration {
    let loader = EnvConfigLoader::new();
    loader.get_duration(
        "REQUEST_TIMEOUT_SECS",
        Duration::from_secs(network::DEFAULT_REQUEST_TIMEOUT_SECS),
    )
}

/// Get connection timeout from environment or default
#[must_use]
pub fn get_connection_timeout() -> Duration {
    let loader = EnvConfigLoader::new();
    loader.get_duration(
        "CONNECTION_TIMEOUT_SECS",
        Duration::from_secs(network::DEFAULT_CONNECTION_TIMEOUT_SECS),
    )
}

/// Get max retries from environment or default
#[must_use]
pub fn get_max_retries() -> u32 {
    let loader = EnvConfigLoader::new();
    loader.get_u32("MAX_RETRIES", network::DEFAULT_MAX_RETRIES)
}

/// Get max connections per host from environment or default
#[must_use]
pub fn get_max_connections_per_host() -> u32 {
    let loader = EnvConfigLoader::new();
    loader.get_u32(
        "MAX_CONNECTIONS_PER_HOST",
        network::DEFAULT_MAX_CONNECTIONS_PER_HOST,
    )
}

/// Get keepalive interval from environment or default
#[must_use]
pub fn get_keepalive_interval() -> Duration {
    let loader = EnvConfigLoader::new();
    loader.get_duration(
        "KEEPALIVE_INTERVAL_SECS",
        Duration::from_secs(network::DEFAULT_KEEPALIVE_INTERVAL_SECS),
    )
}

/// Get all service ports as a map
#[must_use]
pub fn get_service_ports() -> HashMap<String, u16> {
    use toadstool_common::constants::primal_identity::PRIMAL_NAME;

    let mut ports = HashMap::new();
    ports.insert(PRIMAL_NAME.to_string(), get_toadstool_port());
    ports.insert("federation".to_string(), get_federation_port());
    ports.insert("metrics".to_string(), get_metrics_port());
    ports.insert("health".to_string(), get_health_port());
    ports.insert("events".to_string(), get_events_port());
    ports
}

/// Get all service endpoints as a map
#[must_use]
pub fn get_service_endpoints() -> HashMap<String, String> {
    use toadstool_common::constants::primal_identity::PRIMAL_NAME;

    let mut endpoints = HashMap::new();
    endpoints.insert(PRIMAL_NAME.to_string(), get_toadstool_endpoint());
    endpoints
}

/// Get container port range from environment or default
#[must_use]
pub fn get_container_port_range() -> (u16, u16) {
    let loader = EnvConfigLoader::new();
    let start = loader.get_u16(
        "CONTAINER_PORT_START",
        crate::defaults::ports::CONTAINER_START,
    );
    let end = loader.get_u16("CONTAINER_PORT_END", crate::defaults::ports::CONTAINER_END);
    (start, end)
}

/// Get port allocation range from environment or default
#[must_use]
pub fn get_port_allocation_range() -> (u16, u16) {
    let loader = EnvConfigLoader::new();
    let start = loader.get_u16("PORT_RANGE_START", crate::defaults::ports::RANGE_START);
    let end = loader.get_u16("PORT_RANGE_END", crate::defaults::ports::RANGE_END);
    (start, end)
}
