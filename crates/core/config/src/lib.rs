#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(clippy::mixed_attributes_style)]
#![allow(clippy::only_used_in_recursion)]
#![allow(clippy::wrong_self_convention)]
// ToadStool - Universal Compute Platform
// Copyright (C) 2025 ToadStool Development Team
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! # `ToadStool` Configuration System
//!
//! Centralized configuration management for eliminating hardcoded values
//! and providing a unified configuration interface across the entire platform.

pub mod config_utils;
pub mod constants;
pub mod defaults;
pub mod discovery_defaults;
pub mod discovery_integration;
pub mod env_config;
pub mod mdns_discovery; // ✅ Phase 4: mDNS service discovery
pub mod network_config;
pub mod ports;
pub mod primal_capabilities; // ✅ NEW: Universal capability-based discovery
pub mod runtime_defaults;
pub mod services;

/// Network configuration utilities
///
/// **Migration Note**: For runtime configuration, use `EnvironmentConfig::from_env()`
/// or the `ConfigUtils` helper methods. These provide environment variable support
/// and better defaults.
///
/// **Recommended**:
/// - `EnvironmentConfig::from_env()` - Full configuration with env var support
/// - `ConfigUtils::get_songbird_port()` - Individual port getters with env var fallback
pub mod network {
    // Note: Old hardcoded constants were removed in 0.6.0 to encourage use of
    // EnvironmentConfig and ConfigUtils which provide:
    // - Environment variable override support
    // - Consistent defaults from toadstool_config::defaults
    // - Better testability and configuration management

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

    /// Generate default Songbird endpoint (fallback only)
    ///
    /// ⚠️ **DEPRECATED**: Use capability-based discovery instead.
    /// This function is kept only for backward compatibility and test fixtures.
    #[deprecated(
        since = "0.7.0",
        note = "Use ServiceDiscovery::find_by_capability(Capability::Coordination) instead"
    )]
    #[must_use]
    #[allow(deprecated)] // Using deprecated field during migration
    pub fn default_songbird_endpoint() -> String {
        let config = crate::env_config::EnvironmentConfig::from_env();
        format!(
            "http://{}:{}",
            config.network.bind_address, config.network.songbird_port
        )
    }

    /// Generate default `BearDog` endpoint (fallback only)
    ///
    /// ⚠️ **DEPRECATED**: Use capability-based discovery instead.
    #[deprecated(
        since = "0.7.0",
        note = "Use ServiceDiscovery::find_by_capability(Capability::Crypto) instead"
    )]
    #[must_use]
    #[allow(deprecated)] // Using deprecated field during migration
    pub fn default_beardog_endpoint() -> String {
        let config = crate::env_config::EnvironmentConfig::from_env();
        format!(
            "http://{}:{}",
            config.network.bind_address, config.network.beardog_port
        )
    }

    /// Generate default `NestGate` endpoint (fallback only)
    ///
    /// ⚠️ **DEPRECATED**: Use capability-based discovery instead.
    #[deprecated(
        since = "0.7.0",
        note = "Use ServiceDiscovery::find_by_capability(Capability::Storage) instead"
    )]
    #[must_use]
    #[allow(deprecated)] // Using deprecated field during migration
    pub fn default_nestgate_endpoint() -> String {
        let config = crate::env_config::EnvironmentConfig::from_env();
        format!(
            "http://{}:{}",
            config.network.bind_address, config.network.nestgate_port
        )
    }

    /// Generate default Squirrel MCP endpoint (fallback only)
    ///
    /// ⚠️ **DEPRECATED**: Use capability-based discovery instead.
    #[deprecated(
        since = "0.7.0",
        note = "Use ServiceDiscovery::find_by_capability(Capability::AI) instead"
    )]
    #[must_use]
    #[allow(deprecated)] // Using deprecated field during migration
    pub fn default_squirrel_endpoint() -> String {
        let config = crate::env_config::EnvironmentConfig::from_env();
        format!(
            "http://{}:{}",
            config.network.bind_address, config.network.squirrel_port
        )
    }

    /// Generate default `ToadStool` API endpoint (self-knowledge)
    ///
    /// ⚠️ **DEPRECATED**: Use PrimalIdentity for self-knowledge instead.
    #[deprecated(
        since = "0.7.0",
        note = "Use PrimalIdentity to get own endpoint instead"
    )]
    #[must_use]
    pub fn default_toadstool_endpoint() -> String {
        let config = crate::env_config::EnvironmentConfig::from_env();
        format!(
            "http://{}:{}",
            config.network.bind_address, config.network.toadstool_port
        )
    }

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

    // ===== LEGACY PORT FUNCTIONS (DEPRECATED) =====
    // These functions are kept for backward compatibility but should not be used in new code.
    // Use capability-based discovery instead via `toadstool_common::runtime_discovery`.
    //
    // **Migration Path**:
    // 1. Old: `network::get_songbird_endpoint()` - Hardcoded service name
    // 2. New: `ServiceDiscovery::find_by_capability(Capability::Coordination)` - Capability-based
    //
    // See `docs/guides/SELF_KNOWLEDGE_MIGRATION.md` for migration guide.

    /// Get Songbird port from environment or default
    ///
    /// ⚠️ **DEPRECATED**: Use capability-based discovery instead.
    /// This function will be removed in a future version.
    #[deprecated(
        since = "0.7.0",
        note = "Use ServiceDiscovery::find_by_capability(Capability::Coordination) instead"
    )]
    #[must_use]
    #[allow(deprecated)] // Using deprecated constant during migration
    pub fn get_songbird_port() -> u16 {
        std::env::var("TOADSTOOL_SONGBIRD_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(crate::defaults::network::SONGBIRD_PORT)
    }

    /// Get `BearDog` port from environment or default
    ///
    /// ⚠️ **DEPRECATED**: Use capability-based discovery instead.
    #[deprecated(
        since = "0.7.0",
        note = "Use ServiceDiscovery::find_by_capability(Capability::Crypto) instead"
    )]
    #[must_use]
    #[allow(deprecated)] // Using deprecated constant during migration
    pub fn get_beardog_port() -> u16 {
        std::env::var("TOADSTOOL_BEARDOG_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(crate::defaults::network::BEARDOG_PORT)
    }

    /// Get `NestGate` port from environment or default
    ///
    /// # ⚠️ DEPRECATED - Use Capability-Based Discovery
    ///
    /// Modern pattern: Discover storage services by capability.
    ///
    /// ```rust,ignore
    /// let discovery = RuntimeDiscovery::new(discovery_client);
    /// let storage_services = discovery
    ///     .discover_capability(&Capability::Storage(StorageCapability::ObjectStorage))
    ///     .await?;
    /// ```
    #[deprecated(
        since = "0.7.0",
        note = "Use RuntimeDiscovery::discover_capability(Capability::Storage) for service discovery"
    )]
    #[must_use]
    #[allow(deprecated)] // Using deprecated constant during migration
    pub fn get_nestgate_port() -> u16 {
        std::env::var("TOADSTOOL_NESTGATE_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(crate::defaults::network::NESTGATE_PORT)
    }

    /// Get Squirrel MCP port from environment or default
    ///
    /// # ⚠️ DEPRECATED - Use Capability-Based Discovery
    ///
    /// Modern pattern: Discover AI services by capability.
    ///
    /// ```rust,ignore
    /// let discovery = RuntimeDiscovery::new(discovery_client);
    /// let ai_services = discovery
    ///     .discover_capability(&Capability::AI)
    ///     .await?;
    /// ```
    #[deprecated(
        since = "0.7.0",
        note = "Use RuntimeDiscovery::discover_capability(Capability::AI) for service discovery"
    )]
    #[must_use]
    #[allow(deprecated)] // Using deprecated constant during migration
    pub fn get_squirrel_port() -> u16 {
        std::env::var("TOADSTOOL_SQUIRREL_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(crate::defaults::network::SQUIRREL_PORT)
    }

    /// Get `ToadStool` API port from environment or default
    ///
    /// ⚠️ **DEPRECATED**: Use self-knowledge via `PrimalIdentity` instead.
    #[deprecated(
        since = "0.7.0",
        note = "Use PrimalIdentity to get own endpoint instead"
    )]
    #[must_use]
    pub fn get_toadstool_port() -> u16 {
        std::env::var("TOADSTOOL_API_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(crate::defaults::network::API_PORT)
    }

    /// Get bind host from environment or default
    ///
    /// This function is still valid for self-knowledge purposes.
    #[must_use]
    pub fn get_bind_host() -> String {
        std::env::var("TOADSTOOL_BIND_HOST").unwrap_or_else(|_| "127.0.0.1".to_string())
    }

    /// Generate Songbird endpoint from environment configuration
    ///
    /// ⚠️ **DEPRECATED**: Use capability-based discovery instead.
    #[deprecated(
        since = "0.7.0",
        note = "Use ServiceDiscovery::find_by_capability(Capability::Coordination) instead"
    )]
    #[must_use]
    #[allow(deprecated)]
    pub fn get_songbird_endpoint() -> String {
        format!("http://{}:{}", get_bind_host(), get_songbird_port())
    }

    /// Generate `BearDog` endpoint from environment configuration
    ///
    /// ⚠️ **DEPRECATED**: Use capability-based discovery instead.
    #[deprecated(
        since = "0.7.0",
        note = "Use ServiceDiscovery::find_by_capability(Capability::Crypto) instead"
    )]
    #[must_use]
    #[allow(deprecated)]
    pub fn get_beardog_endpoint() -> String {
        format!("http://{}:{}", get_bind_host(), get_beardog_port())
    }

    /// Generate `NestGate` endpoint from environment configuration
    ///
    /// ⚠️ **DEPRECATED**: Use capability-based discovery instead.
    #[deprecated(
        since = "0.7.0",
        note = "Use ServiceDiscovery::find_by_capability(Capability::Storage) instead"
    )]
    #[must_use]
    #[allow(deprecated)]
    pub fn get_nestgate_endpoint() -> String {
        format!("http://{}:{}", get_bind_host(), get_nestgate_port())
    }

    /// Generate Squirrel MCP endpoint from environment configuration
    ///
    /// ⚠️ **DEPRECATED**: Use capability-based discovery instead.
    #[deprecated(
        since = "0.7.0",
        note = "Use ServiceDiscovery::find_by_capability(Capability::AI) instead"
    )]
    #[must_use]
    #[allow(deprecated)]
    pub fn get_squirrel_endpoint() -> String {
        format!("http://{}:{}", get_bind_host(), get_squirrel_port())
    }

    /// Generate `ToadStool` API endpoint from environment configuration
    ///
    /// ⚠️ **DEPRECATED**: Use self-knowledge via `PrimalIdentity` instead.
    #[deprecated(
        since = "0.7.0",
        note = "Use PrimalIdentity to get own endpoint instead"
    )]
    #[must_use]
    #[allow(deprecated)]
    pub fn get_toadstool_endpoint() -> String {
        format!("http://{}:{}", get_bind_host(), get_toadstool_port())
    }
}

/// Application configuration constants
pub mod app {
    /// Default application name
    pub const DEFAULT_APP_NAME: &str = "toadstool";

    /// Default environment
    pub const DEFAULT_ENVIRONMENT: &str = "development";

    /// Default log level
    pub const DEFAULT_LOG_LEVEL: &str = "info";

    /// Default config file name
    pub const DEFAULT_CONFIG_FILE: &str = "toadstool.toml";

    /// Default data directory
    pub const DEFAULT_DATA_DIR: &str = "./data";

    /// Default cache directory
    pub const DEFAULT_CACHE_DIR: &str = "./cache";

    /// Default logs directory
    pub const DEFAULT_LOGS_DIR: &str = "./logs";

    /// Default temp directory
    pub const DEFAULT_TEMP_DIR: &str = "/tmp";

    /// Default max file size in bytes (100MB)
    pub const DEFAULT_MAX_FILE_SIZE: u64 = 100 * 1024 * 1024;

    /// Default max log file size in bytes (10MB)
    pub const DEFAULT_MAX_LOG_SIZE: u64 = 10 * 1024 * 1024;

    /// Default max log files to keep
    pub const DEFAULT_MAX_LOG_FILES: u32 = 10;

    /// Default worker thread count
    pub const DEFAULT_WORKER_THREADS: usize = 4;

    /// Default queue size
    pub const DEFAULT_QUEUE_SIZE: usize = 1000;

    /// Default batch size
    pub const DEFAULT_BATCH_SIZE: usize = 100;

    /// Default polling interval in milliseconds
    pub const DEFAULT_POLLING_INTERVAL_MS: u64 = 500;

    /// Default heartbeat interval in seconds
    pub const DEFAULT_HEARTBEAT_INTERVAL_SECS: u64 = 30;

    /// Default health check interval in seconds
    pub const DEFAULT_HEALTH_CHECK_INTERVAL_SECS: u64 = 60;

    /// Default metrics collection interval in seconds
    pub const DEFAULT_METRICS_INTERVAL_SECS: u64 = 30;

    /// Default cleanup interval in seconds
    pub const DEFAULT_CLEANUP_INTERVAL_SECS: u64 = 300;

    /// Default session timeout in seconds
    pub const DEFAULT_SESSION_TIMEOUT_SECS: u64 = 3600;

    /// Default execution timeout in seconds
    pub const DEFAULT_EXECUTION_TIMEOUT_SECS: u64 = 1800;

    /// Default max concurrent executions
    pub const DEFAULT_MAX_CONCURRENT_EXECUTIONS: u32 = 10;

    /// Default max execution history
    pub const DEFAULT_MAX_EXECUTION_HISTORY: u32 = 1000;

    /// Default resource check interval in seconds
    pub const DEFAULT_RESOURCE_CHECK_INTERVAL_SECS: u64 = 30;

    /// Default max CPU usage percentage
    pub const DEFAULT_MAX_CPU_USAGE: f64 = 80.0;

    /// Default max memory usage percentage
    pub const DEFAULT_MAX_MEMORY_USAGE: f64 = 85.0;

    /// Default max disk usage percentage
    pub const DEFAULT_MAX_DISK_USAGE: f64 = 90.0;

    /// Default buffer size for I/O operations
    pub const DEFAULT_BUFFER_SIZE: usize = 8192;

    /// Default chunk size for large data processing
    pub const DEFAULT_CHUNK_SIZE: usize = 1024 * 1024; // 1MB

    /// Default compression level
    pub const DEFAULT_COMPRESSION_LEVEL: u32 = 6;

    /// Default encryption key length
    pub const DEFAULT_ENCRYPTION_KEY_LENGTH: usize = 32;

    /// Default hash algorithm
    pub const DEFAULT_HASH_ALGORITHM: &str = "sha256";

    /// Default signature algorithm
    pub const DEFAULT_SIGNATURE_ALGORITHM: &str = "ed25519";

    /// Default cache TTL in seconds
    pub const DEFAULT_CACHE_TTL_SECS: u64 = 3600;

    /// Default cache max size in bytes (100MB)
    pub const DEFAULT_CACHE_MAX_SIZE: u64 = 100 * 1024 * 1024;

    /// Default rate limit per second
    pub const DEFAULT_RATE_LIMIT_PER_SEC: u32 = 100;

    /// Default burst limit
    pub const DEFAULT_BURST_LIMIT: u32 = 200;

    /// Default grace period in seconds
    pub const DEFAULT_GRACE_PERIOD_SECS: u64 = 30;

    /// Default shutdown timeout in seconds
    pub const DEFAULT_SHUTDOWN_TIMEOUT_SECS: u64 = 60;
}

/// Testing configuration constants
pub mod testing {
    /// Default test timeout in seconds
    pub const DEFAULT_TEST_TIMEOUT_SECS: u64 = 30;

    /// Default test server port
    pub const DEFAULT_TEST_PORT: u16 = 9999;

    /// Default test data directory
    pub const DEFAULT_TEST_DATA_DIR: &str = "./test_data";

    /// Default test cache directory
    pub const DEFAULT_TEST_CACHE_DIR: &str = "./test_cache";

    /// Default test temp directory
    pub const DEFAULT_TEST_TEMP_DIR: &str = "./test_temp";

    /// Default test database URL
    pub const DEFAULT_TEST_DATABASE_URL: &str = "sqlite::memory:";

    /// Default test environment
    pub const DEFAULT_TEST_ENVIRONMENT: &str = "test";

    /// Default test log level
    pub const DEFAULT_TEST_LOG_LEVEL: &str = "debug";

    /// Default test concurrent connections
    pub const DEFAULT_TEST_CONCURRENT_CONNECTIONS: u32 = 10;

    /// Default test execution timeout in seconds
    pub const DEFAULT_TEST_EXECUTION_TIMEOUT_SECS: u64 = 60;

    /// Default test retry attempts
    pub const DEFAULT_TEST_RETRY_ATTEMPTS: u32 = 3;

    /// Default test retry delay in milliseconds
    pub const DEFAULT_TEST_RETRY_DELAY_MS: u64 = 100;

    /// Default test batch size
    pub const DEFAULT_TEST_BATCH_SIZE: usize = 10;

    /// Default test queue size
    pub const DEFAULT_TEST_QUEUE_SIZE: usize = 100;

    /// Default test polling interval in milliseconds
    pub const DEFAULT_TEST_POLLING_INTERVAL_MS: u64 = 100;

    /// Default test heartbeat interval in seconds
    pub const DEFAULT_TEST_HEARTBEAT_INTERVAL_SECS: u64 = 5;

    /// Default test health check interval in seconds
    pub const DEFAULT_TEST_HEALTH_CHECK_INTERVAL_SECS: u64 = 10;

    /// Default test metrics interval in seconds
    pub const DEFAULT_TEST_METRICS_INTERVAL_SECS: u64 = 5;

    /// Default test cleanup interval in seconds
    pub const DEFAULT_TEST_CLEANUP_INTERVAL_SECS: u64 = 30;

    /// Default test session timeout in seconds
    pub const DEFAULT_TEST_SESSION_TIMEOUT_SECS: u64 = 300;

    /// Default test buffer size
    pub const DEFAULT_TEST_BUFFER_SIZE: usize = 1024;

    /// Default test chunk size
    pub const DEFAULT_TEST_CHUNK_SIZE: usize = 4096;

    /// Default test cache TTL in seconds
    pub const DEFAULT_TEST_CACHE_TTL_SECS: u64 = 60;

    /// Default test rate limit per second
    pub const DEFAULT_TEST_RATE_LIMIT_PER_SEC: u32 = 50;

    /// Default test burst limit
    pub const DEFAULT_TEST_BURST_LIMIT: u32 = 100;

    /// Default test grace period in seconds
    pub const DEFAULT_TEST_GRACE_PERIOD_SECS: u64 = 10;

    /// Default test shutdown timeout in seconds
    pub const DEFAULT_TEST_SHUTDOWN_TIMEOUT_SECS: u64 = 30;
}

/// Development configuration constants
pub mod development {
    /// Default development environment
    pub const DEFAULT_DEV_ENVIRONMENT: &str = "development";

    /// Default development log level
    pub const DEFAULT_DEV_LOG_LEVEL: &str = "debug";

    /// Default development hot reload
    pub const DEFAULT_DEV_HOT_RELOAD: bool = true;

    /// Default development auto restart
    pub const DEFAULT_DEV_AUTO_RESTART: bool = true;

    /// Default development debug mode
    pub const DEFAULT_DEV_DEBUG_MODE: bool = true;

    /// Default development verbose logging
    pub const DEFAULT_DEV_VERBOSE_LOGGING: bool = true;

    /// Default development enable profiling
    pub const DEFAULT_DEV_ENABLE_PROFILING: bool = true;

    /// Default development enable metrics
    pub const DEFAULT_DEV_ENABLE_METRICS: bool = true;

    /// Default development enable tracing
    pub const DEFAULT_DEV_ENABLE_TRACING: bool = true;

    /// Default development watch file changes
    pub const DEFAULT_DEV_WATCH_FILES: bool = true;

    /// Default development mock external services
    pub const DEFAULT_DEV_MOCK_EXTERNAL: bool = true;

    /// Default development disable auth
    pub const DEFAULT_DEV_DISABLE_AUTH: bool = false;

    /// Default development enable cors
    pub const DEFAULT_DEV_ENABLE_CORS: bool = true;

    /// Default development pretty print logs
    pub const DEFAULT_DEV_PRETTY_LOGS: bool = true;

    /// Default development show stack traces
    pub const DEFAULT_DEV_SHOW_TRACES: bool = true;

    /// Default development validation level
    pub const DEFAULT_DEV_VALIDATION_LEVEL: &str = "strict";

    /// Default development performance mode
    pub const DEFAULT_DEV_PERFORMANCE_MODE: &str = "debug";

    /// Default development security mode
    pub const DEFAULT_DEV_SECURITY_MODE: &str = "permissive";

    /// Default development cache mode
    pub const DEFAULT_DEV_CACHE_MODE: &str = "memory";

    /// Default development database mode
    pub const DEFAULT_DEV_DATABASE_MODE: &str = "embedded";
}

/// Production configuration constants
pub mod production {
    /// Default production environment
    pub const DEFAULT_PROD_ENVIRONMENT: &str = "production";

    /// Default production log level
    pub const DEFAULT_PROD_LOG_LEVEL: &str = "info";

    /// Default production hot reload
    pub const DEFAULT_PROD_HOT_RELOAD: bool = false;

    /// Default production auto restart
    pub const DEFAULT_PROD_AUTO_RESTART: bool = true;

    /// Default production debug mode
    pub const DEFAULT_PROD_DEBUG_MODE: bool = false;

    /// Default production verbose logging
    pub const DEFAULT_PROD_VERBOSE_LOGGING: bool = false;

    /// Default production enable profiling
    pub const DEFAULT_PROD_ENABLE_PROFILING: bool = false;

    /// Default production enable metrics
    pub const DEFAULT_PROD_ENABLE_METRICS: bool = true;

    /// Default production enable tracing
    pub const DEFAULT_PROD_ENABLE_TRACING: bool = true;

    /// Default production watch file changes
    pub const DEFAULT_PROD_WATCH_FILES: bool = false;

    /// Default production mock external services
    pub const DEFAULT_PROD_MOCK_EXTERNAL: bool = false;

    /// Default production disable auth
    pub const DEFAULT_PROD_DISABLE_AUTH: bool = false;

    /// Default production enable cors
    pub const DEFAULT_PROD_ENABLE_CORS: bool = false;

    /// Default production pretty print logs
    pub const DEFAULT_PROD_PRETTY_LOGS: bool = false;

    /// Default production show stack traces
    pub const DEFAULT_PROD_SHOW_TRACES: bool = false;

    /// Default production validation level
    pub const DEFAULT_PROD_VALIDATION_LEVEL: &str = "strict";

    /// Default production performance mode
    pub const DEFAULT_PROD_PERFORMANCE_MODE: &str = "optimized";

    /// Default production security mode
    pub const DEFAULT_PROD_SECURITY_MODE: &str = "strict";

    /// Default production cache mode
    pub const DEFAULT_PROD_CACHE_MODE: &str = "distributed";

    /// Default production database mode
    pub const DEFAULT_PROD_DATABASE_MODE: &str = "clustered";
}

// ===== Configuration Type Definitions =====
// Refactored into domain-specific modules under types/
// See types/ directory for organized configuration types
pub mod types;

// Re-export all types for backward compatibility and convenience
// This allows both:
//   - `use toadstool_config::ToadStoolConfig;` (old style)
//   - `use toadstool_config::types::ToadStoolConfig;` (new style)
pub use types::{
    // Individual configuration types
    ApplicationConfig,
    AuditConfig,
    AuthConfig,
    AuthzConfig,
    BackendCacheConfig,
    ConnectionConfig,
    ContainerConfig,
    DatabaseConfig,
    EncryptionConfig,
    EndpointConfig,
    FeatureFlags,
    GpuConfig,
    LoggingConfig,
    MetricsConfig,
    NetworkConfig,
    PythonConfig,
    ResourceLimits,
    RuntimeConfig,
    SandboxConfig,
    SecurityConfig,
    TlsConfig,
    // Main config orchestrator
    ToadStoolConfig,
    WasmConfig,
};
