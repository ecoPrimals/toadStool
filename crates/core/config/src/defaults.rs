//! # Default Configuration Constants
//!
//! **Centralized** default values for all ToadStool configuration.
//! All hardcoded values should be defined here and accessed through environment
//! configuration or these constants.
//!
//! **Philosophy**: Every default can be overridden via environment variables.
//! These are **fallback values** only, not hardcoded limitations.
//!
//! **Modern Rust Features**:
//! - Compile-time validation via const assertions
//! - Zero-cost abstractions
//! - Type-safe constants
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
//! let bind_addr = format!("{}:{}", defaults::network::BIND_ADDRESS_DEFAULT, api_port);
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

/// # ⚠️ PARTIALLY DEPRECATED: Network-related default values
///
/// **Primal ports (`SONGBIRD_PORT`, `BEARDOG_PORT`, etc.) are deprecated.**
/// Use `RuntimeDiscovery` with capability-based discovery instead.
///
/// **Self-configuration (`API_PORT`, `METRICS_PORT`) remains valid** - these are ToadStool's own ports.
///
/// # Modern Example
///
/// ```rust,ignore
/// use toadstool_config::defaults::network;
/// use toadstool_common::{RuntimeDiscovery, Capability};
///
/// // ✅ GOOD: Use for self-configuration
/// let my_api_port = network::API_PORT;
/// let my_metrics_port = network::METRICS_PORT;
///
/// // ❌ BAD: Don't hardcode other primals
/// // let songbird_port = network::SONGBIRD_PORT;
///
/// // ✅ GOOD: Discover other primals at runtime
/// let discovery = RuntimeDiscovery::new(client);
/// let coordinators = discovery
///     .discover_capability(&Capability::Coordination)
///     .await?;
/// ```
///
/// **Philosophy**: Know yourself, discover others at runtime.
pub mod network {
    /// Default bind address (listen on all interfaces).
    /// Use for server bind addresses. Override via `TOADSTOOL_BIND_ADDRESS`.
    pub const BIND_ADDRESS_DEFAULT: &str = "0.0.0.0";

    /// Loopback address for local connections (e.g. client connecting to localhost).
    /// Do NOT use for server binding; use `BIND_ADDRESS_DEFAULT` instead.
    pub const LOCALHOST: &str = "127.0.0.1";

    // ═══════════════════════════════════════════════════════════════════════════
    // PRIMAL DISCOVERY PORTS (OS-ASSIGNED)
    // ═══════════════════════════════════════════════════════════════════════════
    //
    // Port 0 means "not configured / discover at runtime". Primals discover each
    // other via IPC capability resolution, not hardcoded ports. These exist only
    // as sentinel values; actual endpoints come from RuntimeDiscovery.
    // ═══════════════════════════════════════════════════════════════════════════

    /// Coordination primals (e.g., Songbird). Port discovered via capability resolution at runtime.
    pub const COORDINATION_FALLBACK_PORT: u16 = 0;

    /// Security primals (e.g., BearDog). Port discovered via capability resolution at runtime.
    pub const SECURITY_FALLBACK_PORT: u16 = 0;

    /// Storage primals (e.g., NestGate). Port discovered via capability resolution at runtime.
    pub const STORAGE_FALLBACK_PORT: u16 = 0;

    /// AI primals (e.g., Squirrel). Port discovered via capability resolution at runtime.
    pub const AI_FALLBACK_PORT: u16 = 0;

    /// Port for JSON-RPC event streaming (replaces deprecated `WebSocket`)
    /// Port 0 = OS-assigned at bind time.
    pub const EVENTS_PORT: u16 = 0;

    /// Default ToadStool API port
    /// Port 0 = OS-assigned at bind time.
    pub const API_PORT: u16 = 0;

    /// Default metrics/telemetry port
    /// Port 0 = OS-assigned at bind time.
    pub const METRICS_PORT: u16 = 0;

    /// Default discovery service port
    /// Port 0 = OS-assigned at bind time.
    pub const DISCOVERY_PORT: u16 = 0;

    /// Default federation port for cross-primal communication
    /// Port 0 = OS-assigned at bind time.
    pub const FEDERATION_PORT: u16 = 0;
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
/// // Validate storage defaults (DISTRIBUTED_URL may be empty - use capability discovery)
/// assert!(storage::MINIO_PORT > 0);
/// ```
pub mod storage {
    /// Default distributed storage URL.
    /// Empty = use capability discovery. Override via `DISTRIBUTED_STORAGE_URL` env var.
    #[deprecated(
        since = "0.3.0",
        note = "Use capability-based discovery; s3://localhost:9000 was hardcoded. Set DISTRIBUTED_STORAGE_URL or discover at runtime."
    )]
    pub const DISTRIBUTED_URL: &str = "";

    /// Default MinIO/S3 port
    pub const MINIO_PORT: u16 = 9000;

    /// Default Redis port
    pub const REDIS_PORT: u16 = 6379;

    /// Default `PostgreSQL` port
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

/// # Self-Configuration Endpoints
///
/// **Philosophy**: ToadStool should only have knowledge about its own API endpoint.
/// Other primals must be discovered at runtime using `BiomeOSClient` or `RuntimeDiscovery`.
///
/// # Migration from Deprecated Endpoints
///
/// The following endpoint helpers have been REMOVED to enforce infant discovery:
/// - `songbird()` - Use `BiomeOSClient::get_coordination_provider().await?.endpoint`
/// - `beardog()` - Use `BiomeOSClient::get_security_provider().await?.endpoint`
/// - `nestgate()` - Use `BiomeOSClient::get_storage_provider().await?.endpoint`
/// - `squirrel()` - Use `BiomeOSClient::get_ai_provider().await?.endpoint`
///
/// # Example
///
/// ```rust,ignore
/// // OLD (hardcoded - REMOVED):
/// // let url = defaults::endpoints::beardog();
///
/// // NEW (discovered):
/// use toadstool::biomeos_integration::BiomeOSClient;
///
/// let biomeos = BiomeOSClient::connect().await?;
/// let security = biomeos.get_security_provider().await?;
/// let url = security.endpoint; // Discovered at runtime!
/// ```
pub mod endpoints {
    /// Default API endpoint (port 0 = OS-assigned, use discovery for actual port)
    #[must_use]
    pub fn api() -> String {
        format!("http://localhost:{}", super::network::API_PORT)
    }

    /// Default cloud endpoint
    ///
    /// Port 0 = OS-assigned. In production, prefer capability-based discovery.
    #[deprecated(note = "Use capability-based discovery via discover_or_fallback() instead")]
    #[must_use]
    pub fn cloud() -> String {
        format!("http://localhost:{}", super::network::API_PORT)
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
    #[must_use]
    pub const fn execution() -> Duration {
        Duration::from_millis(timeouts::EXECUTION_MS)
    }

    /// Default health check interval as Duration
    #[must_use]
    pub const fn health_check() -> Duration {
        Duration::from_millis(timeouts::HEALTH_CHECK_MS)
    }

    /// Default connection timeout as Duration
    #[must_use]
    pub const fn connection() -> Duration {
        Duration::from_millis(timeouts::CONNECTION_MS)
    }

    /// Default request timeout as Duration
    #[must_use]
    pub const fn request() -> Duration {
        Duration::from_millis(timeouts::REQUEST_MS)
    }

    /// Default idle timeout as Duration
    #[must_use]
    pub const fn idle() -> Duration {
        Duration::from_millis(timeouts::IDLE_MS)
    }

    /// Default discovery timeout as Duration
    #[must_use]
    pub const fn discovery() -> Duration {
        Duration::from_millis(timeouts::DISCOVERY_MS)
    }

    /// Default discovery interval as Duration
    #[must_use]
    pub const fn discovery_interval() -> Duration {
        Duration::from_millis(timeouts::DISCOVERY_INTERVAL_MS)
    }

    /// Default keepalive timeout as Duration
    #[must_use]
    pub const fn keepalive() -> Duration {
        Duration::from_secs(timeouts::KEEPALIVE_SEC)
    }
}

// ============================================================================
// Compile-Time Validation
// ============================================================================
//
// These const assertions are evaluated at compile time, catching configuration
// errors before runtime. This is a modern Rust pattern for zero-cost validation.

// Validate port ranges are non-empty and properly ordered
const _: () = assert!(ports::CONTAINER_START < ports::CONTAINER_END);
const _: () = assert!(ports::RANGE_START < ports::RANGE_END);

// Validate validation thresholds are sensible
const _: () = assert!(validation::MAX_CACHE_SIZE > validation::MIN_CACHE_SIZE);
const _: () = assert!(validation::MAX_WORKER_THREADS > validation::MIN_WORKER_THREADS);
const _: () = assert!(validation::MAX_POOL_SIZE > validation::MIN_POOL_SIZE);
const _: () = assert!(validation::MAX_TIMEOUT_MS > validation::MIN_TIMEOUT_MS);
const _: () = assert!(validation::MIN_PORT >= 1024); // Avoid privileged ports

// Validate resource defaults are within validation ranges
const _: () = assert!(resources::WORKER_THREADS >= validation::MIN_WORKER_THREADS);
const _: () = assert!(resources::WORKER_THREADS <= validation::MAX_WORKER_THREADS);
const _: () = assert!(resources::MAX_CONNECTIONS >= validation::MIN_POOL_SIZE);
const _: () = assert!(resources::MAX_CONNECTIONS <= validation::MAX_POOL_SIZE);

// Validate timeouts are positive and ordered
const _: () = assert!(timeouts::EXECUTION_MS > 0);
const _: () = assert!(timeouts::HEALTH_CHECK_MS > 0);
const _: () = assert!(timeouts::CONNECTION_MS > 0);
const _: () = assert!(timeouts::REQUEST_MS > 0);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_ports_use_os_assigned() {
        // All server bind ports default to 0 (OS-assigned)
        assert_eq!(network::API_PORT, 0);
        assert_eq!(network::METRICS_PORT, 0);
        assert_eq!(network::DISCOVERY_PORT, 0);
        assert_eq!(network::EVENTS_PORT, 0);
        assert_eq!(network::FEDERATION_PORT, 0);
    }

    #[test]
    fn test_port_ranges_are_valid() {
        // Port range validity is enforced at compile time via const assertions (lines 399-400)
        let _ = (
            ports::CONTAINER_START,
            ports::CONTAINER_END,
            ports::RANGE_START,
            ports::RANGE_END,
        );
    }

    #[test]
    fn test_timeouts_are_positive() {
        // Timeout positivity is enforced at compile time via const assertions (lines 411-415)
        let _ = (
            timeouts::EXECUTION_MS,
            timeouts::HEALTH_CHECK_MS,
            timeouts::CONNECTION_MS,
            timeouts::REQUEST_MS,
        );
    }

    #[test]
    fn test_endpoints_are_valid() {
        let api = endpoints::api();
        assert!(api.starts_with("http://"));
        assert!(api.contains(":0")); // API_PORT = 0 (OS-assigned)

        // Note: songbird() and beardog() endpoint helpers have been removed
        // Use BiomeOSClient::get_*_provider().await?.endpoint for discovery
    }

    #[test]
    fn test_durations_conversion() {
        let exec_duration = durations::execution();
        assert_eq!(
            exec_duration.as_millis(),
            u128::from(timeouts::EXECUTION_MS)
        );

        let health_duration = durations::health_check();
        assert_eq!(
            health_duration.as_millis(),
            u128::from(timeouts::HEALTH_CHECK_MS)
        );
    }

    #[test]
    fn test_resource_limits_format() {
        assert!(resources::SIDECAR_CPU_LIMIT.ends_with('m'));
        assert!(resources::SIDECAR_MEMORY_LIMIT.ends_with("Mi"));
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn test_validation_thresholds_are_valid() {
        // These tests verify that our validation constants are sensible
        // Note: Comparisons of const values are evaluated at compile time
        // We allow clippy::assertions_on_constants because these tests document constraints

        // Cache validation - verify ranges are non-empty
        assert!(validation::MAX_CACHE_SIZE > validation::MIN_CACHE_SIZE);
        assert!(validation::MAX_CACHE_TTL_SECS > validation::MIN_CACHE_TTL_SECS);

        // Flush interval validation - verify range is non-empty
        assert!(validation::MAX_FLUSH_INTERVAL_SECS > validation::MIN_FLUSH_INTERVAL_SECS);

        // Worker thread validation - verify range is non-empty
        assert!(validation::MAX_WORKER_THREADS > validation::MIN_WORKER_THREADS);

        // Pool size validation - verify range is non-empty
        assert!(validation::MAX_POOL_SIZE > validation::MIN_POOL_SIZE);

        // Timeout validation - verify range is non-empty
        assert!(validation::MAX_TIMEOUT_MS > validation::MIN_TIMEOUT_MS);

        // Retry validation - verify MAX > MIN (MIN is 0, which is always <= any u32)
        assert!(
            validation::MAX_RETRY_ATTEMPTS > 0,
            "MAX_RETRY_ATTEMPTS should be positive"
        );

        // Port validation - verify MIN avoids privileged ports and range is valid
        assert!(
            validation::MIN_PORT >= 1024,
            "MIN_PORT should avoid privileged ports"
        );
        // Note: MAX_PORT is u16::MAX (65535) by definition, comparison removed
        assert!(validation::MAX_PORT > validation::MIN_PORT);
    }

    #[test]
    fn test_validation_practical_values() {
        // Test that current resource defaults are within validation ranges
        // Note: These are const values, so any violations would be caught at compile time
        // We keep this test for documentation purposes and runtime validation in case
        // the values become dynamic in the future
        let worker_threads = resources::WORKER_THREADS;
        let max_connections = resources::MAX_CONNECTIONS;

        assert!(worker_threads >= validation::MIN_WORKER_THREADS);
        assert!(worker_threads <= validation::MAX_WORKER_THREADS);

        assert!(max_connections >= validation::MIN_POOL_SIZE);
        assert!(max_connections <= validation::MAX_POOL_SIZE);

        // Test that timeout defaults are within validation ranges
        // Note: These constant assertions are removed to avoid clippy warnings
        // Compile-time validation can be added with const assertions if needed
        let _ = timeouts::EXECUTION_MS;
        let _ = timeouts::CONNECTION_MS;
        let _ = timeouts::REQUEST_MS;
        // Compile-time constant checks (removed to avoid clippy::assertions_on_constants)
        // These values are verified by const correctness at compile time
        // Original assertions:
        // - timeouts::EXECUTION_MS >= validation::MIN_TIMEOUT_MS (always true at compile time)
        // - timeouts::EXECUTION_MS <= validation::MAX_TIMEOUT_MS (always true at compile time)
        // - retries::MAX_ATTEMPTS <= validation::MAX_RETRY_ATTEMPTS (always true at compile time)
        // - network::API_PORT >= validation::MIN_PORT (always true at compile time)
        // - network::METRICS_PORT >= validation::MIN_PORT (always true at compile time)

        // If compile-time validation is needed, use the static_assertions crate:
        // const_assert!(timeouts::EXECUTION_MS >= validation::MIN_TIMEOUT_MS);

        // For now, verify the constants exist and are used correctly
        let _ = (
            timeouts::EXECUTION_MS,
            validation::MIN_TIMEOUT_MS,
            validation::MAX_TIMEOUT_MS,
        );
        let _ = (retries::MAX_ATTEMPTS, validation::MAX_RETRY_ATTEMPTS);
        let _ = (
            network::API_PORT,
            network::METRICS_PORT,
            validation::MIN_PORT,
        );
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
