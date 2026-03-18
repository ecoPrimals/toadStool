// SPDX-License-Identifier: AGPL-3.0-or-later
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
//! Constants are organized into domain-specific modules:
//! - `network`: Network ports and addresses
//! - `ports`: Port ranges for dynamic allocation
//! - `timeouts`: Timeout durations for various operations
//! - `durations`: Helper functions returning Duration values
//! - `retries`: Retry and resilience settings
//! - `storage`: Storage backend configuration
//! - `resources`: CPU, memory, and resource limits
//! - `endpoints`: Service endpoint URLs
//! - `logging`: Logging configuration
//! - `validation`: Min/max thresholds for configuration validation
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

pub mod durations;
pub mod endpoints;
pub mod logging;
pub mod network;
pub mod ports;
pub mod resources;
pub mod retries;
pub mod storage;
pub mod timeouts;
pub mod validation;

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
        // Port range validity is enforced at compile time via const assertions
        let _ = (
            ports::CONTAINER_START,
            ports::CONTAINER_END,
            ports::RANGE_START,
            ports::RANGE_END,
        );
    }

    #[test]
    fn test_timeouts_are_positive() {
        // Timeout positivity is enforced at compile time via const assertions
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
        assert!(validation::MAX_PORT > validation::MIN_PORT);
    }

    #[test]
    fn test_validation_practical_values() {
        // Test that current resource defaults are within validation ranges
        let worker_threads = resources::WORKER_THREADS;
        let max_connections = resources::MAX_CONNECTIONS;

        assert!(worker_threads >= validation::MIN_WORKER_THREADS);
        assert!(worker_threads <= validation::MAX_WORKER_THREADS);

        assert!(max_connections >= validation::MIN_POOL_SIZE);
        assert!(max_connections <= validation::MAX_POOL_SIZE);

        let _ = (
            timeouts::EXECUTION_MS,
            timeouts::CONNECTION_MS,
            timeouts::REQUEST_MS,
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
