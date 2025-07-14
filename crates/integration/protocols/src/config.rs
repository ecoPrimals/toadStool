//! Configuration structures for protocol integration

use std::time::Duration;

use crate::types::{AuthType, MessageFormat, TransportType};

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
    pub auth_config: Option<AuthConfig>,

    /// Request timeout
    pub request_timeout: Duration,

    /// Connection pool configuration
    pub connection_pool: ConnectionPoolConfig,

    /// Service discovery configuration
    pub discovery_config: Option<DiscoveryConfig>,

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
            supported_transports: vec![TransportType::Http, TransportType::WebSocket],
            auth_config: None,
            request_timeout: Duration::from_secs(30),
            connection_pool: ConnectionPoolConfig::default(),
            discovery_config: None,
            routing_config: RoutingConfig::default(),
            health_config: HealthConfig::default(),
        }
    }
}

/// Authentication configuration
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// Authentication type
    pub auth_type: AuthType,

    /// Token for authentication
    pub token: Option<String>,

    /// Certificate path for mTLS
    pub cert_path: Option<String>,

    /// Private key path for mTLS
    pub key_path: Option<String>,

    /// CA certificate path
    pub ca_path: Option<String>,
}

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

/// Service discovery configuration
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
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

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum retry attempts
    pub max_attempts: u32,

    /// Base retry delay
    pub base_delay: Duration,

    /// Maximum retry delay
    pub max_delay: Duration,

    /// Exponential backoff multiplier
    pub backoff_multiplier: f64,

    /// Jitter enabled
    pub jitter_enabled: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            jitter_enabled: true,
        }
    }
}

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

/// Health check configuration
#[derive(Debug, Clone)]
pub struct HealthConfig {
    /// Health check enabled
    pub enabled: bool,

    /// Health check interval
    pub interval: Duration,

    /// Health check timeout
    pub timeout: Duration,

    /// Health check endpoint
    pub endpoint: String,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval: Duration::from_secs(30),
            timeout: Duration::from_secs(5),
            endpoint: "/health".to_string(),
        }
    }
}
