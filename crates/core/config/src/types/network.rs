//! Network and communication configuration
//!
//! This module contains configuration types for networking including:
//! - Network bind addresses and ports
//! - Service endpoint URLs (Songbird, BearDog, NestGate, etc.)
//! - Connection settings (timeouts, retries, keepalive)
//! - TLS/SSL configuration

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::Duration;

use crate::network;
use toadstool_common::constants::network::{BIND_ALL_IPV4, DEV_HTTP_PORT, LOCALHOST_IPV4};

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
            // Fallback to BIND_ALL_IPV4:DEV_HTTP_PORT, which should always parse
            let fallback = format!("{BIND_ALL_IPV4}:{DEV_HTTP_PORT}");
            match fallback.parse() {
                Ok(addr) => addr,
                Err(_) => {
                    // Last resort: LOCALHOST_IPV4:DEV_HTTP_PORT is guaranteed valid by IP spec
                    // This expect is justified: it's a compile-time constant that must be valid
                    #[allow(clippy::expect_used)]
                    format!("{LOCALHOST_IPV4}:{DEV_HTTP_PORT}").parse().expect(
                        "Constants LOCALHOST_IPV4:DEV_HTTP_PORT must parse - language guarantee",
                    )
                }
            }
        });

        Self {
            bind_address,
            endpoints: EndpointConfig::default(),
            connection: ConnectionConfig::default(),
            tls: None,
        }
    }
}

/// External service endpoints configuration
///
/// ⚠️ **ARCHITECTURE EVOLUTION**: This struct is transitioning to capability-based discovery.
///
/// **Legacy Mode** (current): Hardcoded endpoint fields (songbird, beardog, etc.)
/// **Future Mode**: Discovery-based lookup via `ServiceDiscovery`
///
/// These hardcoded fields are kept for backward compatibility but will be deprecated.
/// New code should use `toadstool_common::runtime_discovery::ServiceDiscovery` to find
/// services by capability, not by name.
///
/// **Migration Path**:
/// 1. Old: `config.endpoints.songbird` - Hardcoded primal name
/// 2. New: `ServiceDiscovery::find_by_capability(Capability::Coordination).await?` - Capability-based
///
/// See `DOCUMENTATION.md` for the self-knowledge migration path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointConfig {
    /// Songbird coordination service endpoint (LEGACY - use discovery)
    ///
    /// ⚠️ DEPRECATED: Use `ServiceDiscovery::find_by_capability(Capability::Coordination)`
    #[deprecated(
        since = "0.7.0",
        note = "Use ServiceDiscovery::find_by_capability(Capability::Coordination) instead"
    )]
    pub songbird: String,

    /// BearDog cryptography service endpoint (LEGACY - use discovery)
    ///
    /// ⚠️ DEPRECATED: Use `ServiceDiscovery::find_by_capability(Capability::Crypto)`
    #[deprecated(
        since = "0.7.0",
        note = "Use ServiceDiscovery::find_by_capability(Capability::Crypto) instead"
    )]
    pub beardog: String,

    /// NestGate storage service endpoint (LEGACY - use discovery)
    ///
    /// ⚠️ DEPRECATED: Use `ServiceDiscovery::find_by_capability(Capability::Storage)`
    #[deprecated(
        since = "0.7.0",
        note = "Use ServiceDiscovery::find_by_capability(Capability::Storage) instead"
    )]
    pub nestgate: String,

    /// Squirrel MCP platform endpoint (LEGACY - use discovery)
    ///
    /// ⚠️ DEPRECATED: Use `ServiceDiscovery::find_by_capability(Capability::AI)`
    #[deprecated(
        since = "0.7.0",
        note = "Use ServiceDiscovery::find_by_capability(Capability::AI) instead"
    )]
    pub squirrel: String,

    /// Federation endpoint for multi-instance coordination
    /// This is still valid as it's self-knowledge (our own federation endpoint)
    pub federation: String,

    /// Metrics endpoint for Prometheus/monitoring
    /// This is still valid as it's self-knowledge (our own metrics endpoint)
    pub metrics: String,

    /// Health check endpoint
    /// This is still valid as it's self-knowledge (our own health endpoint)
    pub health: String,
}

impl Default for EndpointConfig {
    fn default() -> Self {
        let config = crate::env_config::EnvironmentConfig::from_env();

        // ✅ DEEP DEBT EVOLUTION: Capability-based discovery instead of hardcoded endpoints
        // Check environment variables first, then fall back to localhost defaults
        let songbird = std::env::var("TOADSTOOL_COORDINATION_SERVICE_URL").unwrap_or_else(|_| {
            #[allow(deprecated)]
            let port = crate::ports::fallback::SONGBIRD;
            format!("http://{}:{}", config.network.bind_address, port)
        });
        let beardog = std::env::var("TOADSTOOL_CRYPTO_SERVICE_URL").unwrap_or_else(|_| {
            #[allow(deprecated)]
            let port = crate::ports::fallback::BEARDOG;
            format!("http://{}:{}", config.network.bind_address, port)
        });
        let nestgate = std::env::var("TOADSTOOL_STORAGE_SERVICE_URL").unwrap_or_else(|_| {
            #[allow(deprecated)]
            let port = crate::ports::fallback::NESTGATE;
            format!("http://{}:{}", config.network.bind_address, port)
        });
        let squirrel = std::env::var("TOADSTOOL_AI_SERVICE_URL").unwrap_or_else(|_| {
            #[allow(deprecated)]
            let port = crate::ports::fallback::SQUIRREL;
            format!("http://{}:{}", config.network.bind_address, port)
        });

        Self {
            // Capability-based endpoints - discovered via environment
            #[allow(deprecated)]
            songbird,
            #[allow(deprecated)]
            beardog,
            #[allow(deprecated)]
            nestgate,
            #[allow(deprecated)]
            squirrel,
            // Self-knowledge endpoints (still valid)
            federation: format!(
                "http://{}:{}",
                config.network.bind_address, config.network.federation_port
            ),
            metrics: format!(
                "http://{}:{}",
                config.network.bind_address, config.network.metrics_port
            ),
            health: format!(
                "http://{}:{}",
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
    #[allow(deprecated)]
    fn test_default_network_config() {
        // Tests backward compatibility with deprecated fields
        let config = NetworkConfig::default();
        assert!(!config.endpoints.songbird.is_empty());
        assert!(config.connection.max_retries > 0);
    }

    #[test]
    #[allow(deprecated)]
    fn test_endpoint_config_defaults() {
        // Tests backward compatibility with deprecated fields
        let config = EndpointConfig::default();
        assert!(config.songbird.contains("http"));
        assert!(config.beardog.contains("http"));
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
