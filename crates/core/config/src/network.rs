// SPDX-License-Identifier: AGPL-3.0-only
//! Network configuration utilities
//!
//! **Migration Note**: For runtime configuration, use `EnvironmentConfig::from_env()`
//! or the `ConfigUtils` helper methods. These provide environment variable support
//! and better defaults.
//!
//! **Recommended**:
//! - `EnvironmentConfig::from_env()` - Full configuration with env var support
//! - `ConfigUtils::get_songbird_port()` - Individual port getters with env var fallback

// Note: Old hardcoded constants were removed in 0.6.0 to encourage use of
// EnvironmentConfig and ConfigUtils which provide:
// - Environment variable override support
// - Consistent defaults from toadstool_config::defaults
// - Better testability and configuration management

/// Default request timeout in seconds
pub const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 30;

/// Default connection timeout in seconds
pub const DEFAULT_CONNECTION_TIMEOUT_SECS: u64 = 10;

/// Default max retry attempts
pub const DEFAULT_MAX_RETRIES: u32 = 3;

/// Default keepalive interval in seconds
pub const DEFAULT_KEEPALIVE_INTERVAL_SECS: u64 = 30;

/// Default max connections per host
pub const DEFAULT_MAX_CONNECTIONS_PER_HOST: u32 = 100;

/// Generate default Songbird endpoint (fallback only)
///
/// ⚠️ **DEPRECATED**: Use capability-based discovery instead.
/// This function is kept only for backward compatibility and test fixtures.
#[deprecated(
    since = "0.7.0",
    note = "Use ServiceDiscovery::find_by_capability(Capability::Coordination) instead"
)]
#[must_use]
pub fn default_songbird_endpoint() -> String {
    let config = crate::env_config::EnvironmentConfig::from_env();
    #[allow(deprecated)]
    let port = crate::ports::fallback::SONGBIRD;
    format!("http://{}:{}", config.network.bind_address, port)
}

/// Generate default `BearDog` endpoint (fallback only)
///
/// ⚠️ **DEPRECATED**: Use capability-based discovery instead.
#[deprecated(
    since = "0.7.0",
    note = "Use ServiceDiscovery::find_by_capability(Capability::Crypto) instead"
)]
#[must_use]
pub fn default_beardog_endpoint() -> String {
    let config = crate::env_config::EnvironmentConfig::from_env();
    format!("http://{}:{}", config.network.bind_address, 8081)
}

/// Generate default `NestGate` endpoint (fallback only)
///
/// ⚠️ **DEPRECATED**: Use capability-based discovery instead.
#[deprecated(
    since = "0.7.0",
    note = "Use ServiceDiscovery::find_by_capability(Capability::Storage) instead"
)]
#[must_use]
pub fn default_nestgate_endpoint() -> String {
    let config = crate::env_config::EnvironmentConfig::from_env();
    format!("http://{}:{}", config.network.bind_address, 8082)
}

/// Generate default Squirrel MCP endpoint (fallback only)
///
/// ⚠️ **DEPRECATED**: Use capability-based discovery instead.
#[deprecated(
    since = "0.7.0",
    note = "Use ServiceDiscovery::find_by_capability(Capability::AI) instead"
)]
#[must_use]
pub fn default_squirrel_endpoint() -> String {
    let config = crate::env_config::EnvironmentConfig::from_env();
    format!("http://{}:{}", config.network.bind_address, 8083)
}

/// Generate default `ToadStool` API endpoint (self-knowledge)
///
/// ⚠️ **DEPRECATED**: Use `PrimalIdentity` for self-knowledge instead.
#[deprecated(
    since = "0.7.0",
    note = "Use PrimalIdentity to get own endpoint instead"
)]
#[must_use]
pub fn default_toadstool_endpoint() -> String {
    let config = crate::env_config::EnvironmentConfig::from_env();
    format!(
        "http://{}:{}",
        config.network.bind_address, config.network.toadstool_port
    )
}

/// Generate default federation address
#[must_use]
pub fn default_federation_address() -> std::net::SocketAddr {
    let config = crate::env_config::EnvironmentConfig::from_env();
    format!(
        "{}:{}",
        config.network.bind_address, config.network.federation_port
    )
    .parse()
    .unwrap_or_else(|_| {
        tracing::error!("Invalid default federation address configuration");
        std::net::SocketAddr::from(([127, 0, 0, 1], config.network.federation_port))
    })
}

// ===== LEGACY PORT FUNCTIONS (DEPRECATED) =====
// These functions are kept for backward compatibility but should not be used in new code.
// Use capability-based discovery instead via `toadstool_common::runtime_discovery`.
//
// **Migration Path**:
// 1. Old: `network::get_songbird_endpoint()` - Hardcoded service name
// 2. New: `ServiceDiscovery::find_by_capability(Capability::Coordination)` - Capability-based
//
// See DOCUMENTATION.md for the self-knowledge migration path.

/// Get Songbird port from environment or default
///
/// ⚠️ **DEPRECATED**: Use capability-based discovery instead.
/// This function will be removed in a future version.
///
/// # Self-Knowledge Principle
///
/// Checks `SONGBIRD_PORT` (not `TOADSTOOL_SONGBIRD_PORT`) - other primals manage their own env
#[deprecated(
    since = "0.7.0",
    note = "Use ServiceDiscovery::find_by_capability(Capability::Coordination) instead"
)]
#[must_use]
#[allow(deprecated)]
pub fn get_songbird_port() -> u16 {
    crate::config_utils::ConfigUtils::get_primal_default_port("SONGBIRD")
}

/// Get `BearDog` port from environment or default
///
/// ⚠️ **DEPRECATED**: Use capability-based discovery instead.
///
/// # Self-Knowledge Principle
///
/// Checks `BEARDOG_PORT` (not `TOADSTOOL_BEARDOG_PORT`) - other primals manage their own env
#[deprecated(
    since = "0.7.0",
    note = "Use ServiceDiscovery::find_by_capability(Capability::Crypto) instead"
)]
#[must_use]
#[allow(deprecated)]
pub fn get_beardog_port() -> u16 {
    crate::config_utils::ConfigUtils::get_primal_default_port("BEARDOG")
}

/// Get `NestGate` port from environment or default
///
/// # ⚠️ DEPRECATED - Use Capability-Based Discovery
///
/// Modern pattern: Discover storage services by capability.
///
/// # Self-Knowledge Principle
///
/// Checks `NESTGATE_PORT` (not `TOADSTOOL_NESTGATE_PORT`) - other primals manage their own env
#[deprecated(
    since = "0.7.0",
    note = "Use RuntimeDiscovery::discover_capability(Capability::Storage) for service discovery"
)]
#[must_use]
#[allow(deprecated)]
pub fn get_nestgate_port() -> u16 {
    crate::config_utils::ConfigUtils::get_primal_default_port("NESTGATE")
}

/// Get Squirrel MCP port from environment or default
///
/// # ⚠️ DEPRECATED - Use Capability-Based Discovery
#[deprecated(
    since = "0.7.0",
    note = "Use RuntimeDiscovery::discover_capability(Capability::AI) for service discovery"
)]
#[must_use]
#[allow(deprecated)]
pub fn get_squirrel_port() -> u16 {
    crate::config_utils::ConfigUtils::get_primal_default_port("SQUIRREL")
}

/// Get `ToadStool` API port from environment or default
///
/// ⚠️ **DEPRECATED**: Use self-knowledge via `PrimalIdentity` instead.
#[deprecated(
    since = "0.7.0",
    note = "Use PrimalIdentity to get own endpoint instead"
)]
#[must_use]
pub fn get_toadstool_port() -> u16 {
    std::env::var("TOADSTOOL_PORT")
        .or_else(|_| std::env::var("TOADSTOOL_API_PORT"))
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(crate::defaults::network::API_PORT)
}

/// Get bind host from environment or default
///
/// # Self-Knowledge: ToadStool's own bind address
#[must_use]
pub fn get_bind_host() -> String {
    std::env::var("BIND_ADDRESS")
        .unwrap_or_else(|_| toadstool_common::constants::network::LOCALHOST_IPV4.to_string())
}

/// Generate Songbird endpoint from environment configuration
///
/// ⚠️ **DEPRECATED**: Use capability-based discovery instead.
#[deprecated(
    since = "0.7.0",
    note = "Use ServiceDiscovery::find_by_capability(Capability::Coordination) instead"
)]
#[must_use]
#[allow(deprecated)]
pub fn get_songbird_endpoint() -> String {
    format!("http://{}:{}", get_bind_host(), get_songbird_port())
}

/// Generate `BearDog` endpoint from environment configuration
///
/// ⚠️ **DEPRECATED**: Use capability-based discovery instead.
#[deprecated(
    since = "0.7.0",
    note = "Use ServiceDiscovery::find_by_capability(Capability::Crypto) instead"
)]
#[must_use]
#[allow(deprecated)]
pub fn get_beardog_endpoint() -> String {
    format!("http://{}:{}", get_bind_host(), get_beardog_port())
}

/// Generate `NestGate` endpoint from environment configuration
///
/// ⚠️ **DEPRECATED**: Use capability-based discovery instead.
#[deprecated(
    since = "0.7.0",
    note = "Use ServiceDiscovery::find_by_capability(Capability::Storage) instead"
)]
#[must_use]
#[allow(deprecated)]
pub fn get_nestgate_endpoint() -> String {
    format!("http://{}:{}", get_bind_host(), get_nestgate_port())
}

/// Generate Squirrel MCP endpoint from environment configuration
///
/// ⚠️ **DEPRECATED**: Use capability-based discovery instead.
#[deprecated(
    since = "0.7.0",
    note = "Use ServiceDiscovery::find_by_capability(Capability::AI) instead"
)]
#[must_use]
#[allow(deprecated)]
pub fn get_squirrel_endpoint() -> String {
    format!("http://{}:{}", get_bind_host(), get_squirrel_port())
}

/// Generate `ToadStool` API endpoint from environment configuration
///
/// ⚠️ **DEPRECATED**: Use self-knowledge via `PrimalIdentity` instead.
#[deprecated(
    since = "0.7.0",
    note = "Use PrimalIdentity to get own endpoint instead"
)]
#[must_use]
#[allow(deprecated)]
pub fn get_toadstool_endpoint() -> String {
    format!("http://{}:{}", get_bind_host(), get_toadstool_port())
}
