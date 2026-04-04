// SPDX-License-Identifier: AGPL-3.0-only
//! Network configuration utilities
//!
//! **Runtime configuration**: Prefer [`crate::env_config::EnvironmentConfig::from_env()`]
//! or [`crate::config_utils::ConfigUtils`] for environment-aware values (bind address,
//! timeouts, self-service ports, and federation).
//!
//! **Discovery**: For locating peer services, use capability-based discovery
//! (for example [`crate::primal_capabilities`] and runtime discovery) rather than
//! hardcoded host/port pairs.
//!
//! This module keeps shared numeric defaults used by [`crate::config_utils`] and
//! [`default_federation_address`], plus [`get_bind_host`] for simple bind-address resolution.

// Note: Primal-named endpoint helpers were removed in favor of capability-based discovery
// and `ConfigUtils` for ToadStool's own bind/API settings.

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

/// Get bind host from environment or default
///
/// # Self-Knowledge: ToadStool's own bind address
#[must_use]
pub fn get_bind_host() -> String {
    std::env::var("BIND_ADDRESS")
        .unwrap_or_else(|_| toadstool_common::constants::network::LOCALHOST_IPV4.to_string())
}
