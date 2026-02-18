//! Configuration structures for protocol integration

use std::time::Duration;

use crate::types::{MessageFormat, TransportType};
use toadstool_common::auth::ServiceAuthConfig;

/// Protocol client configuration
#[derive(Debug, Clone)]
pub struct ProtocolConfig {
    /// Service identifier for this client
    pub service_id: String,

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
            service_id: format!("toadstool-{}", uuid::Uuid::new_v4()),
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

// ============================================================================
// Authentication
// ============================================================================
//
// Using canonical ServiceAuthConfig from toadstool_common::auth
// This provides unified authentication across all ToadStool services
//
// For backward compatibility, you can use the type alias:
// pub type AuthConfig = ServiceAuthConfig;
//
// ============================================================================

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
pub use toadstool::config_bases::RetryConfig;

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
mod tests {
    use super::*;

    #[test]
    fn test_protocol_config_default() {
        let config = ProtocolConfig::default();
        assert!(!config.service_id.is_empty());
        assert_eq!(config.default_format, MessageFormat::Json);
        assert_eq!(config.supported_transports.len(), 2);
        assert_eq!(config.request_timeout, Duration::from_secs(30));
        assert!(config.auth_config.is_none());
    }

    #[test]
    fn test_connection_pool_config_default() {
        let config = ConnectionPoolConfig::default();
        assert_eq!(config.max_connections_per_service, 10);
        assert_eq!(config.idle_timeout, Duration::from_secs(300));
        assert_eq!(config.keep_alive_interval, Duration::from_secs(30));
        assert_eq!(config.max_concurrent_requests, 100);
    }

    #[test]
    fn test_routing_config_default() {
        let config = RoutingConfig::default();
        assert!(matches!(
            config.default_strategy,
            RoutingStrategy::RoundRobin
        ));
        assert!(config.load_balancing.health_check_enabled);
        assert!(config.circuit_breaker.is_none());
    }

    #[test]
    fn test_load_balancing_config_default() {
        let config = LoadBalancingConfig::default();
        assert!(config.health_check_enabled);
        assert_eq!(config.health_check_interval, Duration::from_secs(30));
        assert_eq!(config.unhealthy_threshold, 3);
        assert_eq!(config.healthy_threshold, 2);
    }

    #[test]
    fn test_health_config_default() {
        let config = HealthConfig::default();
        assert_eq!(config.endpoint, "/health");
    }

    #[test]
    fn test_discovery_type_variants() {
        let static_discovery = DiscoveryType::Static;
        let dns_discovery = DiscoveryType::Dns;
        let consul_discovery = DiscoveryType::Consul;
        let etcd_discovery = DiscoveryType::Etcd;
        let k8s_discovery = DiscoveryType::Kubernetes;
        let custom_discovery = DiscoveryType::Custom("my-discovery".to_string());

        // All variants should be Debug
        assert!(format!("{:?}", static_discovery).contains("Static"));
        assert!(format!("{:?}", dns_discovery).contains("Dns"));
        assert!(format!("{:?}", consul_discovery).contains("Consul"));
        assert!(format!("{:?}", etcd_discovery).contains("Etcd"));
        assert!(format!("{:?}", k8s_discovery).contains("Kubernetes"));
        assert!(format!("{:?}", custom_discovery).contains("Custom"));
    }

    #[test]
    fn test_routing_strategy_variants() {
        let strategies = vec![
            RoutingStrategy::RoundRobin,
            RoutingStrategy::Random,
            RoutingStrategy::LeastConnections,
            RoutingStrategy::Weighted,
            RoutingStrategy::Sticky,
            RoutingStrategy::Custom("custom-strategy".to_string()),
        ];

        assert_eq!(strategies.len(), 6);
    }

    #[test]
    fn test_circuit_breaker_config() {
        let config = CircuitBreakerConfig {
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(60),
            success_threshold: 2,
        };

        assert_eq!(config.failure_threshold, 5);
        assert_eq!(config.recovery_timeout, Duration::from_secs(60));
        assert_eq!(config.success_threshold, 2);
    }

    #[test]
    fn test_service_discovery_config() {
        let config = ServiceDiscoveryConfig {
            discovery_type: DiscoveryType::Consul,
            registry_endpoint: Some("http://consul.local:8500".to_string()),
            registration_ttl: Duration::from_secs(300),
            refresh_interval: Duration::from_secs(60),
            auto_register: true,
        };

        assert!(matches!(config.discovery_type, DiscoveryType::Consul));
        assert_eq!(
            config.registry_endpoint,
            Some("http://consul.local:8500".to_string())
        );
        assert!(config.auto_register);
    }

    #[test]
    fn test_protocol_config_clone() {
        let config1 = ProtocolConfig::default();
        let config2 = config1.clone();

        assert_eq!(config1.default_format, config2.default_format);
        assert_eq!(config1.request_timeout, config2.request_timeout);
    }

    #[test]
    fn test_connection_pool_config_clone() {
        let config1 = ConnectionPoolConfig::default();
        let config2 = config1.clone();

        assert_eq!(
            config1.max_connections_per_service,
            config2.max_connections_per_service
        );
        assert_eq!(config1.idle_timeout, config2.idle_timeout);
    }

    #[test]
    fn test_routing_config_clone() {
        let config1 = RoutingConfig::default();
        let config2 = config1.clone();

        assert_eq!(
            config1.load_balancing.health_check_enabled,
            config2.load_balancing.health_check_enabled
        );
    }

    #[test]
    fn test_load_balancing_thresholds() {
        let config = LoadBalancingConfig {
            health_check_enabled: false,
            health_check_interval: Duration::from_secs(10),
            unhealthy_threshold: 5,
            healthy_threshold: 3,
        };

        assert!(!config.health_check_enabled);
        assert_eq!(config.unhealthy_threshold, 5);
        assert_eq!(config.healthy_threshold, 3);
    }

    #[test]
    fn test_health_config_custom_endpoint() {
        let config = HealthConfig {
            base: toadstool_common::config_bases::HealthCheckConfig::default(),
            endpoint: "/api/health".to_string(),
        };

        assert_eq!(config.endpoint, "/api/health");
    }

    #[test]
    fn test_discovery_type_custom() {
        let custom = DiscoveryType::Custom("my-service-mesh".to_string());
        assert!(matches!(custom, DiscoveryType::Custom(_)));
    }

    #[test]
    fn test_routing_strategy_custom() {
        let custom = RoutingStrategy::Custom("geo-based".to_string());
        assert!(matches!(custom, RoutingStrategy::Custom(_)));
    }
}
