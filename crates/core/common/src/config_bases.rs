//! Base configuration types used across the codebase
//!
//! This module provides common patterns for configuration structs,
//! enabling code reuse and consistency across different configuration types.
//!
//! # Design Pattern
//!
//! These base types use composition via `#[serde(flatten)]` to allow
//! configuration structs to embed common patterns while maintaining
//! their own specific fields.
//!
//! # Example
//!
//! ```rust
//! use serde::{Deserialize, Serialize};
//! use toadstool_common::config_bases::{TimeoutConfig, HealthCheckConfig};
//!
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! pub struct MyServiceConfig {
//!     pub service_name: String,
//!     #[serde(flatten)]
//!     pub timeouts: TimeoutConfig,
//!     #[serde(flatten)]
//!     pub health_check: HealthCheckConfig,
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::time::Duration;

// ============================================================================
// Timeout Configurations
// ============================================================================

/// Standard timeout configuration for network operations
///
/// This provides a common set of timeout values that can be embedded
/// in various configuration structs using `#[serde(flatten)]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    /// Connection establishment timeout
    #[serde(default = "default_connection_timeout", with = "humantime_serde")]
    pub connection_timeout: Duration,

    /// Request/response timeout
    #[serde(default = "default_request_timeout", with = "humantime_serde")]
    pub request_timeout: Duration,

    /// Socket read timeout
    #[serde(default = "default_read_timeout", with = "humantime_serde")]
    pub read_timeout: Duration,

    /// Socket write timeout
    #[serde(default = "default_write_timeout", with = "humantime_serde")]
    pub write_timeout: Duration,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            connection_timeout: default_connection_timeout(),
            request_timeout: default_request_timeout(),
            read_timeout: default_read_timeout(),
            write_timeout: default_write_timeout(),
        }
    }
}

const fn default_connection_timeout() -> Duration {
    Duration::from_secs(30)
}

const fn default_request_timeout() -> Duration {
    Duration::from_secs(60)
}

const fn default_read_timeout() -> Duration {
    Duration::from_secs(30)
}

const fn default_write_timeout() -> Duration {
    Duration::from_secs(30)
}

// ============================================================================
// Health Check Configurations
// ============================================================================

/// Base health check configuration
///
/// Provides standard health checking parameters that can be used
/// across HTTP, TCP, gRPC, or other health check implementations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Enable health checks
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Interval between health checks
    #[serde(default = "default_health_check_interval", with = "humantime_serde")]
    pub interval: Duration,

    /// Timeout for each health check
    #[serde(default = "default_health_check_timeout", with = "humantime_serde")]
    pub timeout: Duration,

    /// Number of consecutive successful checks to mark as healthy
    #[serde(default = "default_healthy_threshold")]
    pub healthy_threshold: u32,

    /// Number of consecutive failed checks to mark as unhealthy
    #[serde(default = "default_unhealthy_threshold")]
    pub unhealthy_threshold: u32,

    /// Number of retries on failure before marking unhealthy
    #[serde(default = "default_retry_count")]
    pub retry_count: u32,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval: default_health_check_interval(),
            timeout: default_health_check_timeout(),
            healthy_threshold: default_healthy_threshold(),
            unhealthy_threshold: default_unhealthy_threshold(),
            retry_count: default_retry_count(),
        }
    }
}

const fn default_true() -> bool {
    true
}

const fn default_health_check_interval() -> Duration {
    Duration::from_secs(30)
}

const fn default_health_check_timeout() -> Duration {
    Duration::from_secs(10)
}

const fn default_healthy_threshold() -> u32 {
    2
}

const fn default_unhealthy_threshold() -> u32 {
    3
}

const fn default_retry_count() -> u32 {
    3
}

/// HTTP-specific health check configuration
///
/// Extends the base health check with HTTP-specific parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpHealthCheckConfig {
    /// Base health check configuration
    #[serde(flatten)]
    pub base: HealthCheckConfig,

    /// HTTP path to check
    #[serde(default = "default_health_path")]
    pub path: String,

    /// Expected HTTP status code (default: 200)
    #[serde(default = "default_http_status")]
    pub expected_status: u16,

    /// Optional HTTP method (default: GET)
    #[serde(default = "default_http_method")]
    pub method: String,
}

impl Default for HttpHealthCheckConfig {
    fn default() -> Self {
        Self {
            base: HealthCheckConfig::default(),
            path: default_health_path(),
            expected_status: default_http_status(),
            method: default_http_method(),
        }
    }
}

fn default_health_path() -> String {
    "/health".to_string()
}

const fn default_http_status() -> u16 {
    200
}

fn default_http_method() -> String {
    "GET".to_string()
}

// ============================================================================
// Resource Configurations
// ============================================================================

/// Resource limit specification
///
/// Follows Kubernetes-style resource specification with requests and limits.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceLimit {
    /// Resource limit (maximum)
    pub limit: Option<String>,

    /// Resource request (minimum/guaranteed)
    pub request: Option<String>,
}

/// Base resource configuration
///
/// Provides common CPU, memory, and storage resource specifications.
/// Can be extended for domain-specific resource types.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BaseResourceConfig {
    /// CPU resource limits
    #[serde(default)]
    pub cpu: ResourceLimit,

    /// Memory resource limits
    #[serde(default)]
    pub memory: ResourceLimit,

    /// Storage resource limits (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<ResourceLimit>,
}

// ============================================================================
// Validation Configurations
// ============================================================================

/// Base validation configuration
///
/// Provides common validation parameters for tokens, certificates,
/// and other security-related validation operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Enable validation
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Validate expiration timestamps
    #[serde(default = "default_true")]
    pub validate_expiration: bool,

    /// Clock skew tolerance for time-based validation
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        with = "humantime_serde_optional"
    )]
    pub clock_skew: Option<Duration>,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            validate_expiration: true,
            clock_skew: Some(Duration::from_secs(60)),
        }
    }
}

// Serde helper for optional Duration
mod humantime_serde_optional {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    #[allow(clippy::ref_option)] // Required by serde derive macro
    pub fn serialize<S>(value: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(duration) => humantime_serde::serialize(duration, serializer),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<Duration>::deserialize(deserializer)
    }
}

// ============================================================================
// Backend/Endpoint Configurations
// ============================================================================

/// Base backend endpoint configuration
///
/// Represents a network endpoint (address, port) that can be used
/// for load balancers, discovery backends, policy engines, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendEndpoint {
    /// Endpoint name or identifier
    pub name: String,

    /// Network address (hostname or IP)
    pub address: String,

    /// Network port
    pub port: u16,

    /// Whether this endpoint is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl BackendEndpoint {
    /// Create a new backend endpoint
    #[must_use]
    pub fn new(name: impl Into<String>, address: impl Into<String>, port: u16) -> Self {
        Self {
            name: name.into(),
            address: address.into(),
            port,
            enabled: true,
        }
    }

    /// Get the full URL for this endpoint
    #[must_use]
    pub fn url(&self, scheme: &str) -> String {
        format!("{}://{}:{}", scheme, self.address, self.port)
    }
}

// ============================================================================
// Retry Configurations
// ============================================================================

/// Retry configuration with exponential backoff
///
/// Provides a common retry strategy that can be used across
/// different service integrations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retries
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Initial/base delay between retries
    #[serde(default = "default_base_delay", with = "humantime_serde")]
    pub base_delay: Duration,

    /// Maximum delay between retries
    #[serde(default = "default_max_delay", with = "humantime_serde")]
    pub max_delay: Duration,

    /// Backoff multiplier (e.g., 2.0 for exponential)
    #[serde(default = "default_backoff_multiplier")]
    pub backoff_multiplier: f64,

    /// Jitter percentage (0-100) to add randomness
    #[serde(default = "default_jitter_percent")]
    pub jitter_percent: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: default_max_retries(),
            base_delay: default_base_delay(),
            max_delay: default_max_delay(),
            backoff_multiplier: default_backoff_multiplier(),
            jitter_percent: default_jitter_percent(),
        }
    }
}

const fn default_max_retries() -> u32 {
    3
}

const fn default_base_delay() -> Duration {
    Duration::from_millis(100)
}

const fn default_max_delay() -> Duration {
    Duration::from_secs(30)
}

const fn default_backoff_multiplier() -> f64 {
    2.0
}

const fn default_jitter_percent() -> f64 {
    10.0
}

// ============================================================================
// Connection Pool Configurations
// ============================================================================

/// Connection pooling configuration
///
/// Provides standard connection pool settings for HTTP clients and services.
/// This is a common pattern across networking layers and service integrations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionPoolConfig {
    /// Enable connection pooling
    #[serde(default = "default_pool_enabled")]
    pub enabled: bool,

    /// Maximum connections per host
    #[serde(default = "default_max_connections_per_host")]
    pub max_connections_per_host: u32,

    /// Maximum idle connections
    #[serde(default = "default_max_idle_connections")]
    pub max_idle_connections: u32,

    /// Idle connection timeout
    #[serde(default = "default_idle_connection_timeout", with = "humantime_serde")]
    pub idle_timeout: Duration,

    /// Connection lifetime
    #[serde(default = "default_connection_lifetime", with = "humantime_serde")]
    pub connection_lifetime: Duration,
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_connections_per_host: default_max_connections_per_host(),
            max_idle_connections: default_max_idle_connections(),
            idle_timeout: default_idle_connection_timeout(),
            connection_lifetime: default_connection_lifetime(),
        }
    }
}

const fn default_pool_enabled() -> bool {
    true
}

const fn default_max_connections_per_host() -> u32 {
    100
}

const fn default_max_idle_connections() -> u32 {
    10
}

const fn default_idle_connection_timeout() -> Duration {
    Duration::from_secs(300)
}

const fn default_connection_lifetime() -> Duration {
    Duration::from_secs(3600)
}

// ============================================================================
// Cache Configurations
// ============================================================================

/// Base cache configuration
///
/// Provides standard caching parameters that can be used across different
/// caching implementations (DNS, HTTP, data caching, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Enable caching
    #[serde(default = "default_cache_enabled")]
    pub enabled: bool,

    /// Cache entry TTL (time-to-live)
    #[serde(default = "default_cache_ttl", with = "humantime_serde")]
    pub ttl: Duration,

    /// Maximum number of cache entries
    #[serde(default = "default_max_cache_entries")]
    pub max_entries: u32,

    /// Negative cache TTL (for failed lookups)
    #[serde(default = "default_negative_cache_ttl", with = "humantime_serde")]
    pub negative_ttl: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ttl: default_cache_ttl(),
            max_entries: default_max_cache_entries(),
            negative_ttl: default_negative_cache_ttl(),
        }
    }
}

const fn default_cache_enabled() -> bool {
    true
}

const fn default_cache_ttl() -> Duration {
    Duration::from_secs(300) // 5 minutes
}

const fn default_max_cache_entries() -> u32 {
    1000
}

const fn default_negative_cache_ttl() -> Duration {
    Duration::from_secs(60) // 1 minute
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeout_config_defaults() {
        let config = TimeoutConfig::default();
        assert_eq!(config.connection_timeout, Duration::from_secs(30));
        assert_eq!(config.request_timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_health_check_config_defaults() {
        let config = HealthCheckConfig::default();
        assert!(config.enabled);
        assert_eq!(config.interval, Duration::from_secs(30));
        assert_eq!(config.timeout, Duration::from_secs(10));
        assert_eq!(config.healthy_threshold, 2);
        assert_eq!(config.unhealthy_threshold, 3);
    }

    #[test]
    fn test_resource_config_defaults() {
        let config = BaseResourceConfig::default();
        assert!(config.cpu.limit.is_none());
        assert!(config.memory.limit.is_none());
        assert!(config.storage.is_none());
    }

    #[test]
    fn test_backend_endpoint_url() {
        let endpoint = BackendEndpoint::new("test", "localhost", 8080);
        assert_eq!(endpoint.url("http"), "http://localhost:8080");
        assert_eq!(endpoint.url("https"), "https://localhost:8080");
    }

    #[test]
    fn test_retry_config_defaults() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert!((config.backoff_multiplier - 2.0).abs() < f64::EPSILON);
        assert!((config.jitter_percent - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_http_health_check_defaults() {
        let config = HttpHealthCheckConfig::default();
        assert_eq!(config.path, "/health");
        assert_eq!(config.expected_status, 200);
        assert_eq!(config.method, "GET");
        assert!(config.base.enabled);
    }

    #[test]
    fn test_validation_config_defaults() {
        let config = ValidationConfig::default();
        assert!(config.enabled);
        assert!(config.validate_expiration);
        assert!(config.clock_skew.is_some());
        assert_eq!(config.clock_skew.unwrap(), Duration::from_secs(60));
    }

    #[test]
    fn test_connection_pool_config_defaults() {
        let config = ConnectionPoolConfig::default();
        assert!(config.enabled);
        assert_eq!(config.max_connections_per_host, 100);
        assert_eq!(config.max_idle_connections, 10);
        assert_eq!(config.idle_timeout, Duration::from_secs(300));
        assert_eq!(config.connection_lifetime, Duration::from_secs(3600));
    }

    #[test]
    fn test_cache_config_defaults() {
        let config = CacheConfig::default();
        assert!(config.enabled);
        assert_eq!(config.ttl, Duration::from_secs(300));
        assert_eq!(config.max_entries, 1000);
        assert_eq!(config.negative_ttl, Duration::from_secs(60));
    }
}

// ============================================================================
// Telemetry & Observability Configurations
// ============================================================================

/// Telemetry configuration for metrics, tracing, and logging
///
/// This provides a standard set of observability settings that can be used
/// across different services for consistent monitoring and debugging.
///
/// # Example
///
/// ```rust
/// use toadstool_common::config_bases::TelemetryConfig;
///
/// let config = TelemetryConfig {
///     metrics_enabled: true,
///     tracing_enabled: true,
///     access_logs: true,
///     metrics_port: 9090,
///     tracing_endpoint: Some("http://jaeger:14268".to_string()),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    /// Enable metrics collection
    pub metrics_enabled: bool,

    /// Enable distributed tracing
    pub tracing_enabled: bool,

    /// Enable access logs
    pub access_logs: bool,

    /// Metrics port
    pub metrics_port: u16,

    /// Tracing export endpoint (e.g., Jaeger)
    pub tracing_endpoint: Option<String>,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            metrics_enabled: true,
            tracing_enabled: true,
            access_logs: true,
            metrics_port: default_metrics_port(),
            tracing_endpoint: None,
        }
    }
}

const fn default_metrics_port() -> u16 {
    9090
}
