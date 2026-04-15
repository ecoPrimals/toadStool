// SPDX-License-Identifier: AGPL-3.0-or-later
//! Network and communication configuration
//!
//! This module contains configuration types for networking including:
//! - Network bind addresses and ports
//! - Service endpoint URLs (coordination, security, storage, etc.)
//! - Connection settings (timeouts, retries, keepalive)
//! - TLS/SSL configuration

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::Duration;

use crate::network;
use toadstool_common::constants::network::{BIND_ALL_IPV4, HTTP_PROTOCOL, LOCALHOST_IPV4};

/// Last-resort bind port when parsing the configured bind address fails (development fallback only).
const BIND_FALLBACK_PORT: u16 = 3000;

/// Network configuration
///
/// Controls network-level settings including bind address, endpoints,
/// connection management, and TLS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Bind address for server listening
    pub bind_address: SocketAddr,

    /// External service endpoints
    pub endpoints: EndpointConfig,

    /// Connection settings
    pub connection: ConnectionConfig,

    /// TLS configuration (optional)
    pub tls: Option<TlsConfig>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        let config = crate::env_config::EnvironmentConfig::from_env();
        let bind_address = format!(
            "{}:{}",
            config.network.bind_address, config.network.toadstool_port
        )
        .parse()
        .unwrap_or_else(|e| {
            tracing::error!("Invalid default bind address, using fallback: {}", e);
            // Fallback to BIND_ALL_IPV4:BIND_FALLBACK_PORT, which should always parse
            let fallback = format!("{BIND_ALL_IPV4}:{BIND_FALLBACK_PORT}");
            fallback.parse().unwrap_or_else(|_| {
                // Last resort: LOCALHOST_IPV4:BIND_FALLBACK_PORT is guaranteed valid by IP spec
                // This expect is justified: it's a compile-time constant that must be valid
                #[expect(clippy::expect_used)]
                format!("{LOCALHOST_IPV4}:{BIND_FALLBACK_PORT}").parse().expect(
                    "Constants LOCALHOST_IPV4:BIND_FALLBACK_PORT must parse - language guarantee",
                )
            })
        });

        Self {
            bind_address,
            endpoints: EndpointConfig::default(),
            connection: ConnectionConfig::default(),
            tls: None,
        }
    }
}

/// External service endpoints configuration.
///
/// Fields use **capability-domain names** per `CAPABILITY_BASED_DISCOVERY_STANDARD.md`.
/// Serde aliases accept the legacy primal names (`songbird`, `beardog`, etc.) in
/// existing config files. All capability fields are deprecated — prefer runtime
/// discovery via `ServiceDiscovery::find_by_capability(...)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointConfig {
    /// Coordination capability endpoint (legacy fallback)
    #[deprecated(
        since = "0.7.0",
        note = "Use ServiceDiscovery::find_by_capability(Capability::Coordination) instead"
    )]
    #[serde(alias = "songbird")]
    pub coordination: String,

    /// Security / crypto capability endpoint (legacy fallback)
    #[deprecated(
        since = "0.7.0",
        note = "Use ServiceDiscovery::find_by_capability(Capability::Crypto) instead"
    )]
    #[serde(alias = "beardog")]
    pub security: String,

    /// Storage capability endpoint (legacy fallback)
    #[deprecated(
        since = "0.7.0",
        note = "Use ServiceDiscovery::find_by_capability(Capability::Storage) instead"
    )]
    #[serde(alias = "nestgate")]
    pub storage: String,

    /// AI processing capability endpoint (legacy fallback)
    #[deprecated(
        since = "0.7.0",
        note = "Use ServiceDiscovery::find_by_capability(Capability::AI) instead"
    )]
    #[serde(alias = "squirrel")]
    pub ai_processing: String,

    /// Federation endpoint for multi-instance coordination (self-knowledge)
    pub federation: String,

    /// Metrics endpoint for Prometheus/monitoring (self-knowledge)
    pub metrics: String,

    /// Health check endpoint (self-knowledge)
    pub health: String,
}

impl Default for EndpointConfig {
    fn default() -> Self {
        let config = crate::env_config::EnvironmentConfig::from_env();

        // Coordination URL: `TOADSTOOL_COORDINATION_SERVICE_URL` first; else `http://{BIND_ADDRESS}:{COORDINATION}`
        // with port from `ports::capability_fallback::COORDINATION` (8080). `apply_env_overrides` may still set
        // `TOADSTOOL_COORDINATION_ENDPOINT` / `TOADSTOOL_SONGBIRD_ENDPOINT` afterward.
        let coordination =
            std::env::var("TOADSTOOL_COORDINATION_SERVICE_URL").unwrap_or_else(|_| {
                let port = crate::ports::capability_fallback::COORDINATION;
                format!("{HTTP_PROTOCOL}{}:{}", config.network.bind_address, port)
            });
        let security = std::env::var("TOADSTOOL_CRYPTO_SERVICE_URL").unwrap_or_else(|_| {
            let port = crate::ports::capability_fallback::SECURITY;
            format!("{HTTP_PROTOCOL}{}:{}", config.network.bind_address, port)
        });
        let storage = std::env::var("TOADSTOOL_STORAGE_SERVICE_URL").unwrap_or_else(|_| {
            let port = crate::ports::capability_fallback::STORAGE;
            format!("{HTTP_PROTOCOL}{}:{}", config.network.bind_address, port)
        });
        let ai_processing = std::env::var("TOADSTOOL_AI_SERVICE_URL").unwrap_or_else(|_| {
            let port = crate::ports::capability_fallback::PLATFORM;
            format!("{HTTP_PROTOCOL}{}:{}", config.network.bind_address, port)
        });

        Self {
            #[expect(deprecated)]
            coordination,
            #[expect(deprecated)]
            security,
            #[expect(deprecated)]
            storage,
            #[expect(deprecated)]
            ai_processing,
            // Self-knowledge endpoints (still valid)
            federation: format!(
                "{HTTP_PROTOCOL}{}:{}",
                config.network.bind_address, config.network.federation_port
            ),
            metrics: format!(
                "{HTTP_PROTOCOL}{}:{}",
                config.network.bind_address, config.network.metrics_port
            ),
            health: format!(
                "{HTTP_PROTOCOL}{}:{}",
                config.network.bind_address, config.network.health_port
            ),
        }
    }
}

/// Connection configuration
///
/// Controls connection behavior including timeouts, retries, keepalive,
/// and connection pooling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    /// Request timeout duration
    pub request_timeout: Duration,

    /// Connection establishment timeout
    pub connection_timeout: Duration,

    /// Maximum retry attempts for failed requests
    pub max_retries: u32,

    /// Keepalive interval for long-lived connections
    pub keepalive_interval: Duration,

    /// Maximum connections per host
    pub max_connections_per_host: u32,

    /// Connection pool size
    pub pool_size: u32,

    /// Enable HTTP/2 protocol
    pub enable_http2: bool,

    /// Enable compression (gzip, brotli)
    pub enable_compression: bool,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            request_timeout: Duration::from_secs(network::DEFAULT_REQUEST_TIMEOUT_SECS),
            connection_timeout: Duration::from_secs(network::DEFAULT_CONNECTION_TIMEOUT_SECS),
            max_retries: network::DEFAULT_MAX_RETRIES,
            keepalive_interval: Duration::from_secs(network::DEFAULT_KEEPALIVE_INTERVAL_SECS),
            max_connections_per_host: network::DEFAULT_MAX_CONNECTIONS_PER_HOST,
            pool_size: 10,
            enable_http2: true,
            enable_compression: true,
        }
    }
}

/// TLS/SSL configuration
///
/// Configures TLS for secure communication including certificates,
/// cipher suites, and verification settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Path to certificate file (PEM format)
    pub cert_file: String,

    /// Path to private key file (PEM format)
    pub key_file: String,

    /// Path to CA certificate file (optional)
    pub ca_file: Option<String>,

    /// Verify peer certificates
    pub verify_certs: bool,

    /// Minimum TLS version (e.g., "1.2", "1.3")
    pub tls_version: String,

    /// Allowed cipher suites
    pub cipher_suites: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[expect(deprecated)]
    fn test_default_network_config() {
        // Tests backward compatibility with deprecated fields
        let config = NetworkConfig::default();
        assert!(!config.endpoints.coordination.is_empty());
        assert!(config.connection.max_retries > 0);
    }

    #[test]
    #[expect(deprecated)]
    fn test_endpoint_config_defaults() {
        // Tests backward compatibility with deprecated fields
        let config = EndpointConfig::default();
        assert!(config.coordination.contains("http"));
        assert!(config.security.contains("http"));
        assert!(config.metrics.contains("http"));
    }

    #[test]
    fn test_connection_config_defaults() {
        let config = ConnectionConfig::default();
        assert!(config.request_timeout.as_secs() > 0);
        assert!(config.max_connections_per_host > 0);
        assert!(config.enable_http2);
    }
}
