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

use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;

use serde::{Deserialize, Serialize};

pub mod config_utils;
pub mod env_config;
pub mod runtime_defaults;

/// Network configuration constants
pub mod network {
    /// Default localhost address
    pub const DEFAULT_LOCALHOST: &str = "127.0.0.1";

    /// Default Songbird service port
    pub const DEFAULT_SONGBIRD_PORT: u16 = 8080;

    /// Default `BearDog` service port
    pub const DEFAULT_BEARDOG_PORT: u16 = 8081;

    /// Default `NestGate` service port
    pub const DEFAULT_NESTGATE_PORT: u16 = 8082;

    /// Default Squirrel MCP service port
    pub const DEFAULT_SQUIRREL_PORT: u16 = 8083;

    /// Default `ToadStool` API port
    pub const DEFAULT_TOADSTOOL_PORT: u16 = 8084;

    /// Default federation port
    pub const DEFAULT_FEDERATION_PORT: u16 = 7777;

    /// Default metrics port
    pub const DEFAULT_METRICS_PORT: u16 = 9090;

    /// Default health check port
    pub const DEFAULT_HEALTH_PORT: u16 = 8085;

    /// Default WebSocket port
    pub const DEFAULT_WEBSOCKET_PORT: u16 = 8086;

    /// Default container port range start
    pub const DEFAULT_CONTAINER_PORT_START: u16 = 3000;

    /// Default container port range end
    pub const DEFAULT_CONTAINER_PORT_END: u16 = 3999;

    /// Default port allocation range start
    pub const DEFAULT_PORT_RANGE_START: u16 = 8080;

    /// Default port allocation range end
    pub const DEFAULT_PORT_RANGE_END: u16 = 8999;

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

    /// Generate default Songbird endpoint
    #[must_use]
    pub fn default_songbird_endpoint() -> String {
        format!("http://{DEFAULT_LOCALHOST}:{DEFAULT_SONGBIRD_PORT}")
    }

    /// Generate default `BearDog` endpoint
    #[must_use]
    pub fn default_beardog_endpoint() -> String {
        format!("http://{DEFAULT_LOCALHOST}:{DEFAULT_BEARDOG_PORT}")
    }

    /// Generate default `NestGate` endpoint
    #[must_use]
    pub fn default_nestgate_endpoint() -> String {
        format!("http://{DEFAULT_LOCALHOST}:{DEFAULT_NESTGATE_PORT}")
    }

    /// Generate default Squirrel MCP endpoint
    #[must_use]
    pub fn default_squirrel_endpoint() -> String {
        format!("http://{DEFAULT_LOCALHOST}:{DEFAULT_SQUIRREL_PORT}")
    }

    /// Generate default `ToadStool` API endpoint
    #[must_use]
    pub fn default_toadstool_endpoint() -> String {
        format!("http://{DEFAULT_LOCALHOST}:{DEFAULT_TOADSTOOL_PORT}")
    }

    /// Generate default federation address
    #[must_use]
    pub fn default_federation_address() -> std::net::SocketAddr {
        format!("{DEFAULT_LOCALHOST}:{DEFAULT_FEDERATION_PORT}")
            .parse()
            .unwrap_or_else(|_| {
                tracing::error!("Invalid default federation address configuration");
                std::net::SocketAddr::from(([127, 0, 0, 1], DEFAULT_FEDERATION_PORT))
            })
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

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToadStoolConfig {
    /// Application configuration
    pub app: ApplicationConfig,
    /// Network configuration
    pub network: NetworkConfig,
    /// Runtime configuration
    pub runtime: RuntimeConfig,
    /// Security configuration
    pub security: SecurityConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Database configuration
    pub database: Option<DatabaseConfig>,
    /// Cache configuration
    pub cache: Option<CacheConfig>,
    /// Metrics configuration
    pub metrics: Option<MetricsConfig>,
    /// Feature flags
    pub features: FeatureFlags,
    /// Environment-specific overrides
    pub overrides: HashMap<String, serde_json::Value>,
}

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationConfig {
    /// Application name
    pub name: String,
    /// Application version
    pub version: String,
    /// Environment (development, staging, production)
    pub environment: String,
    /// Data directory
    pub data_dir: String,
    /// Cache directory
    pub cache_dir: String,
    /// Logs directory
    pub logs_dir: String,
    /// Temporary directory
    pub temp_dir: String,
    /// Worker thread count
    pub worker_threads: usize,
    /// Queue size
    pub queue_size: usize,
    /// Batch processing size
    pub batch_size: usize,
    /// Graceful shutdown timeout
    pub shutdown_timeout: Duration,
}

impl Default for ApplicationConfig {
    fn default() -> Self {
        Self {
            name: app::DEFAULT_APP_NAME.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            environment: app::DEFAULT_ENVIRONMENT.to_string(),
            data_dir: app::DEFAULT_DATA_DIR.to_string(),
            cache_dir: app::DEFAULT_CACHE_DIR.to_string(),
            logs_dir: app::DEFAULT_LOGS_DIR.to_string(),
            temp_dir: app::DEFAULT_TEMP_DIR.to_string(),
            worker_threads: app::DEFAULT_WORKER_THREADS,
            queue_size: app::DEFAULT_QUEUE_SIZE,
            batch_size: app::DEFAULT_BATCH_SIZE,
            shutdown_timeout: Duration::from_secs(app::DEFAULT_SHUTDOWN_TIMEOUT_SECS),
        }
    }
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Bind address
    pub bind_address: SocketAddr,
    /// External endpoints
    pub endpoints: EndpointConfig,
    /// Connection settings
    pub connection: ConnectionConfig,
    /// TLS configuration
    pub tls: Option<TlsConfig>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            bind_address: format!(
                "{}:{}",
                network::DEFAULT_LOCALHOST,
                network::DEFAULT_TOADSTOOL_PORT
            )
            .parse()
            .expect("Invalid default bind address"),
            endpoints: EndpointConfig::default(),
            connection: ConnectionConfig::default(),
            tls: None,
        }
    }
}

/// External service endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointConfig {
    /// Songbird service endpoint
    pub songbird: String,
    /// `BearDog` service endpoint
    pub beardog: String,
    /// `NestGate` service endpoint
    pub nestgate: String,
    /// Squirrel MCP service endpoint
    pub squirrel: String,
    /// Federation endpoint
    pub federation: String,
    /// Metrics endpoint
    pub metrics: String,
    /// Health check endpoint
    pub health: String,
}

impl Default for EndpointConfig {
    fn default() -> Self {
        Self {
            songbird: network::default_songbird_endpoint(),
            beardog: network::default_beardog_endpoint(),
            nestgate: network::default_nestgate_endpoint(),
            squirrel: network::default_squirrel_endpoint(),
            federation: format!(
                "http://{}:{}",
                network::DEFAULT_LOCALHOST,
                network::DEFAULT_FEDERATION_PORT
            ),
            metrics: format!(
                "http://{}:{}",
                network::DEFAULT_LOCALHOST,
                network::DEFAULT_METRICS_PORT
            ),
            health: format!(
                "http://{}:{}",
                network::DEFAULT_LOCALHOST,
                network::DEFAULT_HEALTH_PORT
            ),
        }
    }
}

/// Connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    /// Request timeout
    pub request_timeout: Duration,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Max retry attempts
    pub max_retries: u32,
    /// Keepalive interval
    pub keepalive_interval: Duration,
    /// Max connections per host
    pub max_connections_per_host: u32,
    /// Connection pool size
    pub pool_size: u32,
    /// Enable HTTP/2
    pub enable_http2: bool,
    /// Enable compression
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

/// TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Certificate file path
    pub cert_file: String,
    /// Private key file path
    pub key_file: String,
    /// CA certificate file path
    pub ca_file: Option<String>,
    /// Verify certificates
    pub verify_certs: bool,
    /// TLS version
    pub tls_version: String,
    /// Cipher suites
    pub cipher_suites: Vec<String>,
}

/// Runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// Execution timeout
    pub execution_timeout: Duration,
    /// Max concurrent executions
    pub max_concurrent_executions: u32,
    /// Resource limits
    pub resource_limits: ResourceLimits,
    /// Container settings
    pub container: ContainerConfig,
    /// WASM settings
    pub wasm: WasmConfig,
    /// Python settings
    pub python: PythonConfig,
    /// GPU settings
    pub gpu: Option<GpuConfig>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            execution_timeout: Duration::from_secs(app::DEFAULT_EXECUTION_TIMEOUT_SECS),
            max_concurrent_executions: app::DEFAULT_MAX_CONCURRENT_EXECUTIONS,
            resource_limits: ResourceLimits::default(),
            container: ContainerConfig::default(),
            wasm: WasmConfig::default(),
            python: PythonConfig::default(),
            gpu: None,
        }
    }
}

/// Resource limits configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Max CPU usage percentage
    pub max_cpu_usage: f64,
    /// Max memory usage percentage
    pub max_memory_usage: f64,
    /// Max disk usage percentage
    pub max_disk_usage: f64,
    /// Max network bandwidth in bytes per second
    pub max_network_bandwidth: u64,
    /// Max open file descriptors
    pub max_open_files: u64,
    /// Max process count
    pub max_processes: u32,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_cpu_usage: app::DEFAULT_MAX_CPU_USAGE,
            max_memory_usage: app::DEFAULT_MAX_MEMORY_USAGE,
            max_disk_usage: app::DEFAULT_MAX_DISK_USAGE,
            max_network_bandwidth: 1024 * 1024 * 1024, // 1 GB/s
            max_open_files: 1024,
            max_processes: 100,
        }
    }
}

/// Container runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerConfig {
    /// Container runtime (docker, podman, etc.)
    pub runtime: String,
    /// Default image registry
    pub default_registry: String,
    /// Port range for container ports
    pub port_range: (u16, u16),
    /// Network mode
    pub network_mode: String,
    /// Security options
    pub security_opts: Vec<String>,
    /// Volume mounts
    pub volume_mounts: Vec<String>,
    /// Environment variables
    pub environment: HashMap<String, String>,
}

impl Default for ContainerConfig {
    fn default() -> Self {
        Self {
            runtime: "docker".to_string(),
            default_registry: "docker.io".to_string(),
            port_range: (
                network::DEFAULT_CONTAINER_PORT_START,
                network::DEFAULT_CONTAINER_PORT_END,
            ),
            network_mode: "bridge".to_string(),
            security_opts: vec!["no-new-privileges".to_string()],
            volume_mounts: vec![],
            environment: HashMap::new(),
        }
    }
}

/// WASM runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmConfig {
    /// WASM runtime engine
    pub engine: String,
    /// Max memory size in bytes
    pub max_memory: u64,
    /// Max execution time in seconds
    pub max_execution_time: u64,
    /// Enable WASI
    pub enable_wasi: bool,
    /// WASI allowed directories
    pub wasi_allowed_dirs: Vec<String>,
    /// WASI environment variables
    pub wasi_env: HashMap<String, String>,
}

impl Default for WasmConfig {
    fn default() -> Self {
        Self {
            engine: "wasmtime".to_string(),
            max_memory: 64 * 1024 * 1024, // 64MB
            max_execution_time: 300,      // 5 minutes
            enable_wasi: true,
            wasi_allowed_dirs: vec!["/tmp".to_string()],
            wasi_env: HashMap::new(),
        }
    }
}

/// Python runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonConfig {
    /// Python executable path
    pub executable: String,
    /// Virtual environment path
    pub venv_path: Option<String>,
    /// Package index URL
    pub index_url: String,
    /// Max memory size in bytes
    pub max_memory: u64,
    /// Max execution time in seconds
    pub max_execution_time: u64,
    /// Allowed modules
    pub allowed_modules: Vec<String>,
    /// Restricted modules
    pub restricted_modules: Vec<String>,
}

impl Default for PythonConfig {
    fn default() -> Self {
        Self {
            executable: "python3".to_string(),
            venv_path: None,
            index_url: "https://pypi.org/simple".to_string(),
            max_memory: 128 * 1024 * 1024, // 128MB
            max_execution_time: 300,       // 5 minutes
            allowed_modules: vec!["numpy".to_string(), "pandas".to_string()],
            restricted_modules: vec!["os".to_string(), "subprocess".to_string()],
        }
    }
}

/// GPU runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuConfig {
    /// GPU runtime (cuda, opencl, etc.)
    pub runtime: String,
    /// GPU device IDs to use
    pub device_ids: Vec<u32>,
    /// Max memory usage per device
    pub max_memory_per_device: u64,
    /// Max execution time in seconds
    pub max_execution_time: u64,
    /// Enable profiling
    pub enable_profiling: bool,
}

impl Default for GpuConfig {
    fn default() -> Self {
        Self {
            runtime: "cuda".to_string(),
            device_ids: vec![0],
            max_memory_per_device: 2 * 1024 * 1024 * 1024, // 2GB
            max_execution_time: 300,                       // 5 minutes
            enable_profiling: false,
        }
    }
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecurityConfig {
    /// Authentication settings
    pub auth: AuthConfig,
    /// Authorization settings
    pub authz: AuthzConfig,
    /// Encryption settings
    pub encryption: EncryptionConfig,
    /// Audit settings
    pub audit: AuditConfig,
    /// Sandbox settings
    pub sandbox: SandboxConfig,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Enable authentication
    pub enabled: bool,
    /// Authentication provider
    pub provider: String,
    /// JWT secret key
    pub jwt_secret: Option<String>,
    /// Session timeout
    pub session_timeout: Duration,
    /// Max login attempts
    pub max_login_attempts: u32,
    /// Lockout duration
    pub lockout_duration: Duration,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: "local".to_string(),
            jwt_secret: None,
            session_timeout: Duration::from_secs(app::DEFAULT_SESSION_TIMEOUT_SECS),
            max_login_attempts: 5,
            lockout_duration: Duration::from_secs(300), // 5 minutes
        }
    }
}

/// Authorization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthzConfig {
    /// Enable authorization
    pub enabled: bool,
    /// Authorization provider
    pub provider: String,
    /// Default permissions
    pub default_permissions: Vec<String>,
    /// Admin permissions
    pub admin_permissions: Vec<String>,
}

impl Default for AuthzConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: "local".to_string(),
            default_permissions: vec!["read".to_string()],
            admin_permissions: vec!["read".to_string(), "write".to_string(), "admin".to_string()],
        }
    }
}

/// Encryption configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    /// Enable encryption
    pub enabled: bool,
    /// Encryption algorithm
    pub algorithm: String,
    /// Key derivation function
    pub key_derivation: String,
    /// Key length in bytes
    pub key_length: usize,
    /// Encryption at rest
    pub encrypt_at_rest: bool,
    /// Encryption in transit
    pub encrypt_in_transit: bool,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            algorithm: "aes-256-gcm".to_string(),
            key_derivation: "pbkdf2".to_string(),
            key_length: app::DEFAULT_ENCRYPTION_KEY_LENGTH,
            encrypt_at_rest: false,
            encrypt_in_transit: true,
        }
    }
}

/// Audit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Enable audit logging
    pub enabled: bool,
    /// Audit log file path
    pub log_file: String,
    /// Audit log level
    pub log_level: String,
    /// Audit log format
    pub log_format: String,
    /// Audit log rotation
    pub log_rotation: bool,
    /// Max audit log size
    pub max_log_size: u64,
    /// Max audit log files
    pub max_log_files: u32,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            log_file: "audit.log".to_string(),
            log_level: "info".to_string(),
            log_format: "json".to_string(),
            log_rotation: true,
            max_log_size: app::DEFAULT_MAX_LOG_SIZE,
            max_log_files: app::DEFAULT_MAX_LOG_FILES,
        }
    }
}

/// Sandbox configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Enable sandboxing
    pub enabled: bool,
    /// Sandbox type
    pub sandbox_type: String,
    /// Allowed system calls
    pub allowed_syscalls: Vec<String>,
    /// Blocked system calls
    pub blocked_syscalls: Vec<String>,
    /// Allowed network access
    pub allow_network: bool,
    /// Allowed file access
    pub allow_file_access: bool,
    /// Allowed directories
    pub allowed_dirs: Vec<String>,
    /// Blocked directories
    pub blocked_dirs: Vec<String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sandbox_type: "seccomp".to_string(),
            allowed_syscalls: vec![
                "read".to_string(),
                "write".to_string(),
                "open".to_string(),
                "close".to_string(),
            ],
            blocked_syscalls: vec![
                "execve".to_string(),
                "fork".to_string(),
                "clone".to_string(),
            ],
            allow_network: false,
            allow_file_access: true,
            allowed_dirs: vec!["/tmp".to_string()],
            blocked_dirs: vec!["/etc".to_string(), "/proc".to_string(), "/sys".to_string()],
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    pub level: String,
    /// Log format
    pub format: String,
    /// Log to file
    pub log_to_file: bool,
    /// Log file path
    pub log_file: String,
    /// Log rotation
    pub log_rotation: bool,
    /// Max log size
    pub max_log_size: u64,
    /// Max log files
    pub max_log_files: u32,
    /// Enable colors
    pub enable_colors: bool,
    /// Enable timestamps
    pub enable_timestamps: bool,
    /// Enable thread IDs
    pub enable_thread_ids: bool,
    /// Enable module paths
    pub enable_module_paths: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: app::DEFAULT_LOG_LEVEL.to_string(),
            format: "pretty".to_string(),
            log_to_file: false,
            log_file: "toadstool.log".to_string(),
            log_rotation: true,
            max_log_size: app::DEFAULT_MAX_LOG_SIZE,
            max_log_files: app::DEFAULT_MAX_LOG_FILES,
            enable_colors: true,
            enable_timestamps: true,
            enable_thread_ids: false,
            enable_module_paths: false,
        }
    }
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database URL
    pub url: String,
    /// Database type
    pub database_type: String,
    /// Max connections
    pub max_connections: u32,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Query timeout
    pub query_timeout: Duration,
    /// Enable migrations
    pub enable_migrations: bool,
    /// Migration directory
    pub migration_dir: String,
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Cache type
    pub cache_type: String,
    /// Cache URL
    pub url: Option<String>,
    /// Max size in bytes
    pub max_size: u64,
    /// TTL in seconds
    pub ttl: Duration,
    /// Enable compression
    pub enable_compression: bool,
    /// Compression algorithm
    pub compression_algorithm: String,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            cache_type: "memory".to_string(),
            url: None,
            max_size: app::DEFAULT_CACHE_MAX_SIZE,
            ttl: Duration::from_secs(app::DEFAULT_CACHE_TTL_SECS),
            enable_compression: false,
            compression_algorithm: "gzip".to_string(),
        }
    }
}

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Enable metrics
    pub enabled: bool,
    /// Metrics endpoint
    pub endpoint: String,
    /// Metrics format
    pub format: String,
    /// Collection interval
    pub collection_interval: Duration,
    /// Retention period
    pub retention_period: Duration,
    /// Enable histograms
    pub enable_histograms: bool,
    /// Enable counters
    pub enable_counters: bool,
    /// Enable gauges
    pub enable_gauges: bool,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            endpoint: format!(
                "http://{}:{}/metrics",
                network::DEFAULT_LOCALHOST,
                network::DEFAULT_METRICS_PORT
            ),
            format: "prometheus".to_string(),
            collection_interval: Duration::from_secs(app::DEFAULT_METRICS_INTERVAL_SECS),
            retention_period: Duration::from_secs(3600 * 24 * 7), // 7 days
            enable_histograms: true,
            enable_counters: true,
            enable_gauges: true,
        }
    }
}

/// Feature flags configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    /// Enable experimental features
    pub enable_experimental: bool,
    /// Enable beta features
    pub enable_beta: bool,
    /// Enable debug features
    pub enable_debug: bool,
    /// Enable profiling features
    pub enable_profiling: bool,
    /// Enable distributed mode
    pub enable_distributed: bool,
    /// Enable federation
    pub enable_federation: bool,
    /// Enable WebSocket
    pub enable_websocket: bool,
    /// Enable GraphQL
    pub enable_graphql: bool,
    /// Enable gRPC
    pub enable_grpc: bool,
    /// Enable `OpenAPI`
    pub enable_openapi: bool,
    /// Enable auto-configuration
    pub enable_auto_config: bool,
    /// Enable hot reload
    pub enable_hot_reload: bool,
    /// Enable live reload
    pub enable_live_reload: bool,
    /// Enable watch mode
    pub enable_watch_mode: bool,
    /// Custom feature flags
    pub custom: HashMap<String, bool>,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            enable_experimental: false,
            enable_beta: false,
            enable_debug: cfg!(debug_assertions),
            enable_profiling: false,
            enable_distributed: true,
            enable_federation: true,
            enable_websocket: true,
            enable_graphql: false,
            enable_grpc: false,
            enable_openapi: true,
            enable_auto_config: true,
            enable_hot_reload: cfg!(debug_assertions),
            enable_live_reload: cfg!(debug_assertions),
            enable_watch_mode: cfg!(debug_assertions),
            custom: HashMap::new(),
        }
    }
}

/// Configuration loading and validation
impl ToadStoolConfig {
    /// Load configuration from file
    ///
    /// # Errors
    /// Returns an error if the configuration file cannot be read or parsed
    pub fn load_from_file<P: AsRef<std::path::Path>>(
        path: P,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    /// Load configuration from environment variables
    ///
    /// # Errors
    /// Returns an error if the environment variables cannot be parsed
    pub fn load_from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let mut config = Self::default();

        // Override with environment variables
        if let Ok(env) = std::env::var("TOADSTOOL_ENVIRONMENT") {
            config.app.environment = env;
        }

        if let Ok(log_level) = std::env::var("TOADSTOOL_LOG_LEVEL") {
            config.logging.level = log_level;
        }

        if let Ok(bind_address) = std::env::var("TOADSTOOL_BIND_ADDRESS") {
            config.network.bind_address = bind_address.parse()?;
        }

        if let Ok(songbird_endpoint) = std::env::var("TOADSTOOL_SONGBIRD_ENDPOINT") {
            config.network.endpoints.songbird = songbird_endpoint;
        }

        config.validate()?;
        Ok(config)
    }

    /// Validate configuration
    ///
    /// # Errors
    /// Returns an error if the configuration is invalid
    pub fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Validate application configuration
        if self.app.name.is_empty() {
            return Err("Application name cannot be empty".into());
        }

        if self.app.worker_threads == 0 {
            return Err("Worker threads must be greater than 0".into());
        }

        // Validate network configuration
        if self.network.endpoints.songbird.is_empty() {
            return Err("Songbird endpoint cannot be empty".into());
        }

        // Validate runtime configuration
        if self.runtime.max_concurrent_executions == 0 {
            return Err("Max concurrent executions must be greater than 0".into());
        }

        // Validate resource limits
        if self.runtime.resource_limits.max_cpu_usage <= 0.0
            || self.runtime.resource_limits.max_cpu_usage > 100.0
        {
            return Err("Max CPU usage must be between 0 and 100".into());
        }

        if self.runtime.resource_limits.max_memory_usage <= 0.0
            || self.runtime.resource_limits.max_memory_usage > 100.0
        {
            return Err("Max memory usage must be between 0 and 100".into());
        }

        Ok(())
    }

    /// Get environment-specific configuration
    #[must_use]
    pub fn for_environment(mut self, environment: &str) -> Self {
        self.app.environment = environment.to_string();

        // Apply environment-specific defaults
        match environment {
            "development" => {
                self.logging.level = development::DEFAULT_DEV_LOG_LEVEL.to_string();
                self.logging.enable_colors = true;
                self.features.enable_debug = development::DEFAULT_DEV_DEBUG_MODE;
                self.features.enable_hot_reload = development::DEFAULT_DEV_HOT_RELOAD;
                self.security.auth.enabled = false;
            }
            "production" => {
                self.logging.level = production::DEFAULT_PROD_LOG_LEVEL.to_string();
                self.logging.enable_colors = production::DEFAULT_PROD_PRETTY_LOGS;
                self.features.enable_debug = production::DEFAULT_PROD_DEBUG_MODE;
                self.features.enable_hot_reload = production::DEFAULT_PROD_HOT_RELOAD;
                self.security.auth.enabled = true;
            }
            "test" => {
                self.logging.level = testing::DEFAULT_TEST_LOG_LEVEL.to_string();
                self.app.data_dir = testing::DEFAULT_TEST_DATA_DIR.to_string();
                self.app.cache_dir = testing::DEFAULT_TEST_CACHE_DIR.to_string();
                self.runtime.execution_timeout =
                    Duration::from_secs(testing::DEFAULT_TEST_EXECUTION_TIMEOUT_SECS);
                self.security.auth.enabled = false;
            }
            _ => {
                // Use default configuration
            }
        }

        self
    }

    /// Merge with override configuration
    #[must_use]
    pub fn merge(mut self, overrides: HashMap<String, serde_json::Value>) -> Self {
        self.overrides.extend(overrides);
        self
    }

    /// Get configuration value with override support
    #[must_use]
    pub fn get_override<T>(&self, key: &str, default: T) -> T
    where
        T: serde::de::DeserializeOwned + Clone,
    {
        self.overrides
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or(default)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_configuration() {
        let config = ToadStoolConfig::default();
        assert_eq!(config.app.name, app::DEFAULT_APP_NAME);
        assert_eq!(config.app.environment, app::DEFAULT_ENVIRONMENT);
        assert_eq!(config.logging.level, app::DEFAULT_LOG_LEVEL);
        assert!(config.features.enable_websocket);
        assert!(config.features.enable_federation);
    }

    #[test]
    fn test_network_constants() {
        assert_eq!(network::DEFAULT_LOCALHOST, "127.0.0.1");
        assert_eq!(network::DEFAULT_SONGBIRD_PORT, 8080);
        assert_eq!(network::DEFAULT_BEARDOG_PORT, 8081);
        assert_eq!(network::DEFAULT_NESTGATE_PORT, 8082);

        let songbird_endpoint = network::default_songbird_endpoint();
        assert!(songbird_endpoint.contains("8080"));
        assert!(songbird_endpoint.contains("127.0.0.1"));
    }

    #[test]
    fn test_configuration_validation() {
        let mut config = ToadStoolConfig::default();
        assert!(config.validate().is_ok());

        config.app.name = String::new();
        assert!(config.validate().is_err());

        config.app.name = "test".to_string();
        config.app.worker_threads = 0;
        assert!(config.validate().is_err());

        config.app.worker_threads = 4;
        config.runtime.resource_limits.max_cpu_usage = 150.0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_environment_specific_config() {
        let config = ToadStoolConfig::default().for_environment("development");
        assert_eq!(config.app.environment, "development");
        assert_eq!(config.logging.level, "debug");
        assert!(config.features.enable_debug);
        assert!(!config.security.auth.enabled);

        let config = ToadStoolConfig::default().for_environment("production");
        assert_eq!(config.app.environment, "production");
        assert_eq!(config.logging.level, "info");
        assert!(!config.features.enable_debug);
        assert!(config.security.auth.enabled);
    }

    #[test]
    fn test_configuration_overrides() {
        let mut overrides = HashMap::new();
        overrides.insert("custom_setting".to_string(), serde_json::Value::Bool(true));

        let config = ToadStoolConfig::default().merge(overrides);
        assert!(config.get_override("custom_setting", false));
        assert_eq!(config.get_override("non_existent", 42), 42);
    }
}
