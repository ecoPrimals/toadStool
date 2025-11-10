//! # Default Configuration Constants
//!
//! **Centralized** default values for all ToadStool configuration.
//! All hardcoded values should be defined here and accessed through environment
//! configuration or these constants.
//!
//! **Philosophy**: Every default can be overridden via environment variables.
//! These are **fallback values** only, not hardcoded limitations.
//!
//! # Organization
//!
//! Constants are organized into logical modules:
//! - `network`: Network ports and addresses
//! - `ports`: Port ranges for dynamic allocation
//! - `timeouts`: Timeout durations for various operations
//! - `retries`: Retry and resilience settings
//! - `storage`: Storage backend configuration
//! - `resources`: CPU, memory, and resource limits
//! - `endpoints`: Service endpoint URLs
//! - `logging`: Logging configuration
//! - `validation`: Min/max thresholds for configuration validation
//! - `durations`: Helper functions returning Duration values
//!
//! # Usage Examples
//!
//! ```rust
//! use toadstool_config::defaults;
//!
//! // Use network defaults
//! let api_port = defaults::network::API_PORT;
//! let bind_addr = format!("{}:{}", defaults::network::LOCALHOST, api_port);
//!
//! // Use timeout defaults
//! use std::time::Duration;
//! let timeout = Duration::from_millis(defaults::timeouts::REQUEST_MS);
//!
//! // Use resource defaults
//! let workers = defaults::resources::WORKER_THREADS;
//! let max_connections = defaults::resources::MAX_CONNECTIONS;
//! ```
//!
//! # Environment Variable Override Pattern
//!
//! All defaults can be overridden via environment variables:
//! ```rust
//! use std::env;
//! use toadstool_config::defaults;
//!
//! // Get port with environment override
//! let api_port = env::var("TOADSTOOL_API_PORT")
//!     .ok()
//!     .and_then(|s| s.parse().ok())
//!     .unwrap_or(defaults::network::API_PORT);
//! ```
//!
//! For complete environment configuration, use `EnvironmentConfig::from_env()`.

/// Network-related default values
///
/// # Example
///
/// ```rust
/// use toadstool_config::defaults::network;
///
/// // Build service endpoint URLs
/// let songbird_url = format!("http://{}:{}", network::LOCALHOST, network::SONGBIRD_PORT);
/// let beardog_url = format!("http://{}:{}", network::LOCALHOST, network::BEARDOG_PORT);
///
/// // Validate network defaults
/// assert_eq!(network::LOCALHOST, "127.0.0.1");
/// assert!(network::API_PORT > 0);
/// assert!(network::METRICS_PORT > 0);
/// ```
pub mod network {
    /// Default localhost address for binding
    pub const LOCALHOST: &str = "127.0.0.1";

    /// Default Songbird service port
    pub const SONGBIRD_PORT: u16 = 8080;

    /// Default BearDog authentication service port
    pub const BEARDOG_PORT: u16 = 8081;

    /// Default NestGate orchestration service port
    pub const NESTGATE_PORT: u16 = 8082;

    /// Default Squirrel MCP service port
    pub const SQUIRREL_PORT: u16 = 8083;

    /// Default ToadStool API port
    pub const API_PORT: u16 = 8084;

    /// Default metrics/telemetry port
    pub const METRICS_PORT: u16 = 9090;

    /// Default discovery service port
    pub const DISCOVERY_PORT: u16 = 8084;

    /// Default federation port for cross-primal communication
    pub const FEDERATION_PORT: u16 = 7777;
}

/// Port range defaults for dynamic allocation
///
/// # Example
///
/// ```rust
/// use toadstool_config::defaults::ports;
///
/// // Allocate port range for containers
/// let start_port = ports::CONTAINER_START;
/// let end_port = ports::CONTAINER_END;
/// for port in start_port..=end_port {
///     // Use port...
/// }
///
/// // Service mesh sidecar ports
/// let proxy_port = ports::SIDECAR_LISTEN;
/// let admin_port = ports::SIDECAR_ADMIN;
/// ```
pub mod ports {
    /// Default starting port for container allocations
    pub const CONTAINER_START: u16 = 3000;

    /// Default ending port for container allocations
    pub const CONTAINER_END: u16 = 3999;

    /// Default starting port for general port range
    pub const RANGE_START: u16 = 8080;

    /// Default ending port for general port range
    pub const RANGE_END: u16 = 8999;

    /// Default service mesh sidecar listen port
    pub const SIDECAR_LISTEN: u16 = 15001;

    /// Default service mesh sidecar admin port
    pub const SIDECAR_ADMIN: u16 = 15000;
}

/// Timeout defaults (in milliseconds)
///
/// # Example
///
/// ```rust
/// use toadstool_config::defaults::timeouts;
/// use std::time::Duration;
///
/// // Create timeout durations
/// let exec_timeout = Duration::from_millis(timeouts::EXECUTION_MS);
/// let conn_timeout = Duration::from_millis(timeouts::CONNECTION_MS);
/// let req_timeout = Duration::from_millis(timeouts::REQUEST_MS);
///
/// // Validate timeout values
/// assert!(exec_timeout.as_secs() > 0);
/// assert!(conn_timeout.as_secs() > 0);
/// assert!(req_timeout.as_secs() > 0);
/// ```
///
/// See also: `durations` module for helper functions that return `Duration` values directly.
pub mod timeouts {
    /// Default execution timeout for tasks
    pub const EXECUTION_MS: u64 = 30_000;

    /// Default health check interval
    pub const HEALTH_CHECK_MS: u64 = 5_000;

    /// Default connection timeout
    pub const CONNECTION_MS: u64 = 5_000;

    /// Default request timeout
    pub const REQUEST_MS: u64 = 30_000;

    /// Default idle timeout
    pub const IDLE_MS: u64 = 60_000;

    /// Default discovery timeout
    pub const DISCOVERY_MS: u64 = 5_000;

    /// Default discovery interval
    pub const DISCOVERY_INTERVAL_MS: u64 = 30_000;

    /// Default keepalive timeout (in seconds)
    pub const KEEPALIVE_SEC: u64 = 60;
}

/// Retry and resilience defaults
///
/// # Example
///
/// ```rust
/// use toadstool_config::defaults::retries;
/// use std::time::Duration;
///
/// // Configure retry logic
/// let max_attempts = retries::MAX_ATTEMPTS;
/// let initial_backoff = Duration::from_millis(retries::BACKOFF_MS);
/// let max_backoff = Duration::from_millis(retries::MAX_BACKOFF_MS);
/// let multiplier = retries::BACKOFF_MULTIPLIER;
///
/// // Validate retry configuration
/// assert!(max_attempts > 0);
/// assert!(initial_backoff.as_millis() > 0);
/// assert!(max_backoff >= initial_backoff);
/// assert!(multiplier > 1.0);
/// ```
pub mod retries {
    /// Default maximum retry attempts
    pub const MAX_ATTEMPTS: u32 = 3;

    /// Default retry backoff duration (in milliseconds)
    pub const BACKOFF_MS: u64 = 1_000;

    /// Default exponential backoff multiplier
    pub const BACKOFF_MULTIPLIER: f64 = 2.0;

    /// Default maximum backoff duration (in milliseconds)
    pub const MAX_BACKOFF_MS: u64 = 30_000;
}

/// Storage and database defaults
///
/// # Example
///
/// ```rust
/// use toadstool_config::defaults::storage;
///
/// // Build storage backend URLs
/// let minio_url = format!("localhost:{}", storage::MINIO_PORT);
/// let redis_url = format!("redis://localhost:{}", storage::REDIS_PORT);
/// let postgres_url = format!("postgres://localhost:{}/toadstool", storage::POSTGRES_PORT);
///
/// // Validate storage defaults
/// assert!(!storage::DISTRIBUTED_URL.is_empty());
/// assert!(storage::MINIO_PORT > 0);
/// ```
pub mod storage {
    /// Default distributed storage URL (MinIO/S3 compatible)
    pub const DISTRIBUTED_URL: &str = "s3://localhost:9000";

    /// Default MinIO/S3 port
    pub const MINIO_PORT: u16 = 9000;

    /// Default Redis port
    pub const REDIS_PORT: u16 = 6379;

    /// Default PostgreSQL port
    pub const POSTGRES_PORT: u16 = 5432;
}

/// Resource limits
///
/// # Example
///
/// ```rust
/// use toadstool_config::defaults::resources;
///
/// // Get resource defaults
/// let workers = resources::WORKER_THREADS;
/// let max_conns = resources::MAX_CONNECTIONS;
///
/// // Use Kubernetes-style resource specifications
/// let cpu_limit = resources::SIDECAR_CPU_LIMIT;      // "200m" = 200 millicores
/// let mem_limit = resources::SIDECAR_MEMORY_LIMIT;  // "256Mi" = 256 mebibytes
///
/// // Validate resource values
/// assert!(workers > 0);
/// assert!(max_conns > 0);
/// assert!(!cpu_limit.is_empty());
/// assert!(!mem_limit.is_empty());
/// ```
pub mod resources {
    /// Default worker thread count
    pub const WORKER_THREADS: usize = 4;

    /// Default max connections
    pub const MAX_CONNECTIONS: usize = 1000;

    /// Default retry count
    pub const RETRY_COUNT: u32 = 3;

    /// Default sidecar CPU limit
    pub const SIDECAR_CPU_LIMIT: &str = "200m";

    /// Default sidecar memory limit
    pub const SIDECAR_MEMORY_LIMIT: &str = "256Mi";

    /// Default sidecar CPU request
    pub const SIDECAR_CPU_REQUEST: &str = "100m";

    /// Default sidecar memory request
    pub const SIDECAR_MEMORY_REQUEST: &str = "128Mi";
}

/// Endpoint defaults
pub mod endpoints {
    /// Default Songbird endpoint
    pub fn songbird() -> String {
        format!("http://localhost:{}", super::network::SONGBIRD_PORT)
    }

    /// Default BearDog endpoint
    pub fn beardog() -> String {
        format!("http://localhost:{}", super::network::BEARDOG_PORT)
    }

    /// Default NestGate endpoint
    pub fn nestgate() -> String {
        format!("http://localhost:{}", super::network::NESTGATE_PORT)
    }

    /// Default Squirrel endpoint
    pub fn squirrel() -> String {
        format!("http://localhost:{}", super::network::SQUIRREL_PORT)
    }

    /// Default API endpoint
    pub fn api() -> String {
        format!("http://localhost:{}", super::network::API_PORT)
    }

    /// Default cloud endpoint
    pub fn cloud() -> String {
        "http://localhost:8080".to_string()
    }
}

/// Logging defaults
pub mod logging {
    /// Default log level
    pub const LEVEL: &str = "info";

    /// Default log format
    pub const FORMAT: &str = "json";
}

/// Validation threshold constants
///
/// These constants define minimum and maximum values for various configuration
/// parameters to ensure safe and reasonable operation.
///
/// # Example
///
/// ```rust
/// use toadstool_config::defaults::validation;
///
/// // Validate cache configuration
/// fn validate_cache_size(size: usize) -> Result<(), String> {
///     if size < validation::MIN_CACHE_SIZE {
///         return Err(format!("Cache size {} below minimum {}", size, validation::MIN_CACHE_SIZE));
///     }
///     if size > validation::MAX_CACHE_SIZE {
///         return Err(format!("Cache size {} exceeds maximum {}", size, validation::MAX_CACHE_SIZE));
///     }
///     Ok(())
/// }
///
/// // Validate worker thread count
/// fn validate_workers(count: usize) -> Result<(), String> {
///     if count < validation::MIN_WORKER_THREADS {
///         return Err(format!("Worker count {} below minimum {}", count, validation::MIN_WORKER_THREADS));
///     }
///     if count > validation::MAX_WORKER_THREADS {
///         return Err(format!("Worker count {} exceeds maximum {}", count, validation::MAX_WORKER_THREADS));
///     }
///     Ok(())
/// }
/// ```
pub mod validation {
    /// Minimum cache size (entries)
    pub const MIN_CACHE_SIZE: usize = 100;

    /// Maximum cache size (entries)
    pub const MAX_CACHE_SIZE: usize = 100_000;

    /// Minimum cache TTL (seconds)
    pub const MIN_CACHE_TTL_SECS: u64 = 60;

    /// Maximum cache TTL (seconds)
    pub const MAX_CACHE_TTL_SECS: u64 = 86_400; // 24 hours

    /// Minimum flush interval (seconds)
    pub const MIN_FLUSH_INTERVAL_SECS: u64 = 10;

    /// Maximum flush interval (seconds)
    pub const MAX_FLUSH_INTERVAL_SECS: u64 = 3600; // 1 hour

    /// Minimum worker thread count
    pub const MIN_WORKER_THREADS: usize = 1;

    /// Maximum worker thread count
    pub const MAX_WORKER_THREADS: usize = 128;

    /// Minimum connection pool size
    pub const MIN_POOL_SIZE: usize = 1;

    /// Maximum connection pool size
    pub const MAX_POOL_SIZE: usize = 10_000;

    /// Minimum timeout value (milliseconds)
    pub const MIN_TIMEOUT_MS: u64 = 100;

    /// Maximum timeout value (milliseconds)
    pub const MAX_TIMEOUT_MS: u64 = 3_600_000; // 1 hour

    /// Minimum retry attempts
    pub const MIN_RETRY_ATTEMPTS: u32 = 0;

    /// Maximum retry attempts
    pub const MAX_RETRY_ATTEMPTS: u32 = 10;

    /// Minimum port number
    pub const MIN_PORT: u16 = 1024; // Avoid privileged ports

    /// Maximum port number
    pub const MAX_PORT: u16 = 65535;
}

/// Helper functions to get Duration values
pub mod durations {
    use super::timeouts;
    use std::time::Duration;

    /// Default execution timeout as Duration
    pub fn execution() -> Duration {
        Duration::from_millis(timeouts::EXECUTION_MS)
    }

    /// Default health check interval as Duration
    pub fn health_check() -> Duration {
        Duration::from_millis(timeouts::HEALTH_CHECK_MS)
    }

    /// Default connection timeout as Duration
    pub fn connection() -> Duration {
        Duration::from_millis(timeouts::CONNECTION_MS)
    }

    /// Default request timeout as Duration
    pub fn request() -> Duration {
        Duration::from_millis(timeouts::REQUEST_MS)
    }

    /// Default idle timeout as Duration
    pub fn idle() -> Duration {
        Duration::from_millis(timeouts::IDLE_MS)
    }

    /// Default discovery timeout as Duration
    pub fn discovery() -> Duration {
        Duration::from_millis(timeouts::DISCOVERY_MS)
    }

    /// Default discovery interval as Duration
    pub fn discovery_interval() -> Duration {
        Duration::from_millis(timeouts::DISCOVERY_INTERVAL_MS)
    }

    /// Default keepalive timeout as Duration
    pub fn keepalive() -> Duration {
        Duration::from_secs(timeouts::KEEPALIVE_SEC)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_ports_are_distinct() {
        let ports = [
            network::SONGBIRD_PORT,
            network::BEARDOG_PORT,
            network::NESTGATE_PORT,
            network::SQUIRREL_PORT,
            network::API_PORT,
        ];

        // Verify all ports are unique
        for (i, &port1) in ports.iter().enumerate() {
            for &port2 in ports.iter().skip(i + 1) {
                assert_ne!(
                    port1, port2,
                    "Ports must be distinct: {} vs {}",
                    port1, port2
                );
            }
        }
    }

    #[test]
    fn test_port_ranges_are_valid() {
        // Use runtime assertions in tests instead of const assertions
        // These will be evaluated at test time, not compile time
        if ports::CONTAINER_START >= ports::CONTAINER_END {
            panic!(
                "Container port range must be valid: START={} END={}",
                ports::CONTAINER_START,
                ports::CONTAINER_END
            );
        }
        if ports::RANGE_START >= ports::RANGE_END {
            panic!(
                "General port range must be valid: START={} END={}",
                ports::RANGE_START,
                ports::RANGE_END
            );
        }
    }

    #[test]
    fn test_timeouts_are_positive() {
        // Use runtime checks in tests instead of const assertions
        if timeouts::EXECUTION_MS == 0 {
            panic!("EXECUTION_MS must be positive");
        }
        if timeouts::HEALTH_CHECK_MS == 0 {
            panic!("HEALTH_CHECK_MS must be positive");
        }
        if timeouts::CONNECTION_MS == 0 {
            panic!("CONNECTION_MS must be positive");
        }
        if timeouts::REQUEST_MS == 0 {
            panic!("REQUEST_MS must be positive");
        }
    }

    #[test]
    fn test_endpoints_are_valid() {
        let songbird = endpoints::songbird();
        assert!(songbird.starts_with("http://"));
        assert!(songbird.contains("8080"));

        let beardog = endpoints::beardog();
        assert!(beardog.starts_with("http://"));
        assert!(beardog.contains("8081"));
    }

    #[test]
    fn test_durations_conversion() {
        let exec_duration = durations::execution();
        assert_eq!(exec_duration.as_millis(), timeouts::EXECUTION_MS as u128);

        let health_duration = durations::health_check();
        assert_eq!(
            health_duration.as_millis(),
            timeouts::HEALTH_CHECK_MS as u128
        );
    }

    #[test]
    fn test_resource_limits_format() {
        assert!(resources::SIDECAR_CPU_LIMIT.ends_with('m'));
        assert!(resources::SIDECAR_MEMORY_LIMIT.ends_with("Mi"));
    }

    #[test]
    fn test_validation_thresholds_are_valid() {
        // Cache validation
        assert!(validation::MIN_CACHE_SIZE > 0);
        assert!(validation::MAX_CACHE_SIZE > validation::MIN_CACHE_SIZE);
        assert!(validation::MIN_CACHE_TTL_SECS > 0);
        assert!(validation::MAX_CACHE_TTL_SECS > validation::MIN_CACHE_TTL_SECS);

        // Flush interval validation
        assert!(validation::MIN_FLUSH_INTERVAL_SECS > 0);
        assert!(validation::MAX_FLUSH_INTERVAL_SECS > validation::MIN_FLUSH_INTERVAL_SECS);

        // Worker thread validation
        assert!(validation::MIN_WORKER_THREADS > 0);
        assert!(validation::MAX_WORKER_THREADS > validation::MIN_WORKER_THREADS);

        // Pool size validation
        assert!(validation::MIN_POOL_SIZE > 0);
        assert!(validation::MAX_POOL_SIZE > validation::MIN_POOL_SIZE);

        // Timeout validation
        assert!(validation::MIN_TIMEOUT_MS > 0);
        assert!(validation::MAX_TIMEOUT_MS > validation::MIN_TIMEOUT_MS);

        // Retry validation
        assert!(validation::MAX_RETRY_ATTEMPTS > validation::MIN_RETRY_ATTEMPTS);

        // Port validation
        assert!(validation::MIN_PORT >= 1024, "MIN_PORT should avoid privileged ports");
        assert!(validation::MAX_PORT == 65535, "MAX_PORT should be max valid port");
        assert!(validation::MAX_PORT > validation::MIN_PORT);
    }

    #[test]
    fn test_validation_practical_values() {
        // Test that current resource defaults are within validation ranges
        assert!(resources::WORKER_THREADS >= validation::MIN_WORKER_THREADS);
        assert!(resources::WORKER_THREADS <= validation::MAX_WORKER_THREADS);

        assert!(resources::MAX_CONNECTIONS >= validation::MIN_POOL_SIZE);
        assert!(resources::MAX_CONNECTIONS <= validation::MAX_POOL_SIZE);

        // Test that timeout defaults are within validation ranges
        assert!(timeouts::EXECUTION_MS >= validation::MIN_TIMEOUT_MS);
        assert!(timeouts::EXECUTION_MS <= validation::MAX_TIMEOUT_MS);
        assert!(timeouts::CONNECTION_MS >= validation::MIN_TIMEOUT_MS);
        assert!(timeouts::CONNECTION_MS <= validation::MAX_TIMEOUT_MS);
        assert!(timeouts::REQUEST_MS >= validation::MIN_TIMEOUT_MS);
        assert!(timeouts::REQUEST_MS <= validation::MAX_TIMEOUT_MS);

        // Test that retry defaults are within validation ranges
        assert!(retries::MAX_ATTEMPTS >= validation::MIN_RETRY_ATTEMPTS);
        assert!(retries::MAX_ATTEMPTS <= validation::MAX_RETRY_ATTEMPTS);

        // Test that port defaults are within validation ranges
        assert!(network::API_PORT >= validation::MIN_PORT);
        assert!(network::API_PORT <= validation::MAX_PORT);
        assert!(network::METRICS_PORT >= validation::MIN_PORT);
        assert!(network::METRICS_PORT <= validation::MAX_PORT);
    }

    #[test]
    fn test_validation_ranges_make_sense() {
        // Cache TTL: should allow from 1 minute to 24 hours
        assert_eq!(validation::MIN_CACHE_TTL_SECS, 60);
        assert_eq!(validation::MAX_CACHE_TTL_SECS, 86_400);

        // Flush interval: should allow from 10 seconds to 1 hour
        assert_eq!(validation::MIN_FLUSH_INTERVAL_SECS, 10);
        assert_eq!(validation::MAX_FLUSH_INTERVAL_SECS, 3600);

        // Worker threads: should allow from 1 to 128
        assert_eq!(validation::MIN_WORKER_THREADS, 1);
        assert_eq!(validation::MAX_WORKER_THREADS, 128);

        // Timeout: should allow from 100ms to 1 hour
        assert_eq!(validation::MIN_TIMEOUT_MS, 100);
        assert_eq!(validation::MAX_TIMEOUT_MS, 3_600_000);

        // Retries: should allow from 0 to 10
        assert_eq!(validation::MIN_RETRY_ATTEMPTS, 0);
        assert_eq!(validation::MAX_RETRY_ATTEMPTS, 10);
    }
}
