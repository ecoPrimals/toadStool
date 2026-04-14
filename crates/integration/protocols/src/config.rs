// SPDX-License-Identifier: AGPL-3.0-or-later
//! Configuration structures for protocol integration

use std::sync::Arc;
use std::time::Duration;

use crate::types::{MessageFormat, TransportType};
use toadstool_common::auth::ServiceAuthConfig;
use toadstool_common::constants::network::HTTP_PROTOCOL;

/// Protocol client configuration
#[derive(Debug, Clone)]
pub struct ProtocolConfig {
    /// Service identifier for this client (`Arc<str>` = zero-copy clone)
    pub service_id: Arc<str>,

    /// Default message format
    pub default_format: MessageFormat,

    /// Supported transport types
    pub supported_transports: Vec<TransportType>,

    /// Authentication configuration  
    pub auth_config: Option<ServiceAuthConfig>,

    /// Request timeout
    pub request_timeout: Duration,

    /// Connection pool configuration
    pub connection_pool: ConnectionPoolConfig,

    /// Service discovery configuration
    pub discovery_config: Option<ServiceDiscoveryConfig>,

    /// Message routing configuration
    pub routing_config: RoutingConfig,

    /// Health check configuration
    pub health_config: HealthConfig,
}

impl Default for ProtocolConfig {
    fn default() -> Self {
        Self {
            service_id: Arc::from(format!("toadstool-{}", uuid::Uuid::new_v4())),
            default_format: MessageFormat::Json,
            supported_transports: vec![TransportType::Http, TransportType::TRpc], // WebSocket removed — use JSON-RPC 2.0
            auth_config: None,
            request_timeout: Duration::from_secs(30),
            connection_pool: ConnectionPoolConfig::default(),
            discovery_config: None,
            routing_config: RoutingConfig::default(),
            health_config: HealthConfig::default(),
        }
    }
}

// Authentication uses canonical `ServiceAuthConfig` from `toadstool_common::auth`.

/// Connection pool configuration
#[derive(Debug, Clone)]
pub struct ConnectionPoolConfig {
    /// Maximum number of connections per service
    pub max_connections_per_service: u32,

    /// Connection idle timeout
    pub idle_timeout: Duration,

    /// Connection keep-alive interval
    pub keep_alive_interval: Duration,

    /// Maximum concurrent requests
    pub max_concurrent_requests: u32,
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            max_connections_per_service: 10,
            idle_timeout: Duration::from_secs(300),
            keep_alive_interval: Duration::from_secs(30),
            max_concurrent_requests: 100,
        }
    }
}

/// Example Consul registry hostname for documentation and tests only.
///
/// Production and integration tests that need a real endpoint must use
/// `service_registry_url()` or set `SERVICE_REGISTRY_URL` / `CONSUL_HTTP_ADDR`.
pub const SAMPLE_CONSUL_REGISTRY_HOST: &str = "consul.local";

/// Example Consul HTTP API port paired with [`SAMPLE_CONSUL_REGISTRY_HOST`].
pub const SAMPLE_CONSUL_REGISTRY_HTTP_PORT: u16 = 8500;

/// Build the sample Consul registry URL (documentation/tests; not a deployment default).
#[must_use]
pub fn sample_consul_registry_url() -> String {
    format!(
        "{HTTP_PROTOCOL}{}:{}",
        SAMPLE_CONSUL_REGISTRY_HOST, SAMPLE_CONSUL_REGISTRY_HTTP_PORT
    )
}

/// Get service registry URL from environment or capability discovery.
///
/// Priority: `SERVICE_REGISTRY_URL` → `CONSUL_HTTP_ADDR` (no deployment default; set env in integration tests).
#[must_use]
pub fn service_registry_url() -> String {
    if let Ok(url) = std::env::var("SERVICE_REGISTRY_URL") {
        return url;
    }
    if let Ok(addr) = std::env::var("CONSUL_HTTP_ADDR") {
        if addr.starts_with("http://") || addr.starts_with("https://") {
            return addr;
        }
        return format!("{HTTP_PROTOCOL}{addr}");
    }
    String::new()
}

/// Service discovery configuration for protocols
#[derive(Debug, Clone)]
pub struct ServiceDiscoveryConfig {
    /// Discovery mechanism
    pub discovery_type: DiscoveryType,

    /// Registry endpoint
    pub registry_endpoint: Option<String>,

    /// Service registration TTL
    pub registration_ttl: Duration,

    /// Discovery refresh interval
    pub refresh_interval: Duration,

    /// Enable automatic registration
    pub auto_register: bool,
}

impl ServiceDiscoveryConfig {
    /// Create Consul discovery config with endpoint from env (`SERVICE_REGISTRY_URL`, `CONSUL_HTTP_ADDR`).
    #[must_use]
    pub fn consul_default() -> Self {
        Self {
            discovery_type: DiscoveryType::Consul,
            registry_endpoint: Some(service_registry_url()),
            registration_ttl: Duration::from_secs(300),
            refresh_interval: Duration::from_secs(60),
            auto_register: true,
        }
    }
}

/// Service discovery types
#[derive(Debug, Clone)]
pub enum DiscoveryType {
    /// Static configuration
    Static,
    /// DNS-based discovery
    Dns,
    /// Consul service discovery
    Consul,
    /// etcd service discovery
    Etcd,
    /// Kubernetes service discovery
    Kubernetes,
    /// Custom discovery mechanism
    Custom(String),
}

/// Message routing configuration
#[derive(Debug, Clone)]
pub struct RoutingConfig {
    /// Default routing strategy
    pub default_strategy: RoutingStrategy,

    /// Load balancing configuration
    pub load_balancing: LoadBalancingConfig,

    /// Retry configuration
    pub retry_config: RetryConfig,

    /// Circuit breaker configuration
    pub circuit_breaker: Option<CircuitBreakerConfig>,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            default_strategy: RoutingStrategy::RoundRobin,
            load_balancing: LoadBalancingConfig::default(),
            retry_config: RetryConfig::default(),
            circuit_breaker: None,
        }
    }
}

/// Routing strategy options
#[derive(Debug, Clone)]
pub enum RoutingStrategy {
    /// Round-robin routing
    RoundRobin,
    /// Random routing
    Random,
    /// Least connections routing
    LeastConnections,
    /// Weighted routing
    Weighted,
    /// Sticky session routing
    Sticky,
    /// Custom routing strategy
    Custom(String),
}

/// Load balancing configuration
#[derive(Debug, Clone)]
pub struct LoadBalancingConfig {
    /// Health check enabled
    pub health_check_enabled: bool,

    /// Health check interval
    pub health_check_interval: Duration,

    /// Unhealthy threshold
    pub unhealthy_threshold: u32,

    /// Healthy threshold
    pub healthy_threshold: u32,
}

impl Default for LoadBalancingConfig {
    fn default() -> Self {
        Self {
            health_check_enabled: true,
            health_check_interval: Duration::from_secs(30),
            unhealthy_threshold: 3,
            healthy_threshold: 2,
        }
    }
}

// Note: Using base RetryConfig from toadstool for consistency
// If domain-specific retry logic is needed, use composition with base pattern
pub use toadstool_common::config_bases::RetryConfig;

// Base RetryConfig already has a Default implementation
// Fields: max_retries, base_delay, max_delay, backoff_multiplier, jitter_percent

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Failure threshold to open circuit
    pub failure_threshold: u32,

    /// Recovery timeout
    pub recovery_timeout: Duration,

    /// Success threshold to close circuit
    pub success_threshold: u32,
}

/// Health check configuration for protocol clients
///
/// Extends the base health check configuration with protocol-specific endpoint.
/// Uses composition pattern for config reuse.
#[derive(Debug, Clone)]
pub struct HealthConfig {
    /// Base health check configuration (enabled, interval, timeout, thresholds, retries)
    pub base: toadstool_common::config_bases::HealthCheckConfig,

    /// Health check endpoint path
    pub endpoint: String,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            base: toadstool_common::config_bases::HealthCheckConfig::default(),
            endpoint: "/health".to_string(),
        }
    }
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
