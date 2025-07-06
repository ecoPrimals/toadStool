//! Configuration Management for ToadStool
//!
//! This crate implements the zero-hardcoding hierarchical configuration system
//! that allows all ToadStool components to be configured via files and environment variables.

use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};

use config::{Config, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info, warn};

// Module declarations will be added in future iterations

/// Configuration error types
#[derive(Error, Debug)]
pub enum ToadStoolConfigError {
    /// Configuration file not found
    #[error("Configuration file not found: {path}")]
    FileNotFound { path: String },

    /// Invalid configuration format
    #[error("Invalid configuration format: {message}")]
    InvalidFormat { message: String },

    /// Configuration validation error
    #[error("Configuration validation error: {message}")]
    ValidationError { message: String },

    /// Environment variable error
    #[error("Environment variable error: {message}")]
    EnvironmentError { message: String },

    /// IO error
    #[error("Configuration IO error: {source}")]
    IoError { source: std::io::Error },

    /// Config library error
    #[error("Config error: {source}")]
    ConfigError { source: ConfigError },
}

impl From<std::io::Error> for ToadStoolConfigError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError { source: err }
    }
}

impl From<ConfigError> for ToadStoolConfigError {
    fn from(err: ConfigError) -> Self {
        Self::ConfigError { source: err }
    }
}

/// Result type for configuration operations
pub type ConfigResult<T> = Result<T, ToadStoolConfigError>;

/// Trust level for ecosystem services
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TrustLevel {
    /// Untrusted services (require verification)
    Untrusted,
    /// Low trust (basic verification)
    Low,
    /// Medium trust (standard verification)
    Medium,
    /// High trust (enhanced verification)  
    High,
    /// Complete trust (no verification)
    Trusted,
}

impl Default for TrustLevel {
    fn default() -> Self {
        Self::Medium
    }
}

/// Core ToadStool configuration structure
#[derive(Debug, Clone)]
pub struct ToadStoolConfig {
    /// Runtime configuration
    pub runtime: RuntimeConfig,
    /// Security configuration  
    pub security: SecurityConfig,
    /// Resource configuration
    pub resources: ResourceConfig,
    /// Custom configurations
    pub custom: HashMap<String, serde_json::Value>,
    /// Network configuration
    pub network: NetworkConfig,
    /// Ecosystem configuration
    pub ecosystem: EcosystemConfig,
    /// Monitoring configuration
    pub monitoring: MonitoringConfig,
    /// Configuration source (not serialized)
    pub config_source: Option<Config>,
}

/// Runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// Default runtime selection strategy
    pub default_strategy: String,
    /// Runtime preferences
    pub preferences: Vec<String>,
    /// Runtime-specific configurations
    pub engines: HashMap<String, serde_json::Value>,
    /// Execution timeouts
    pub timeouts: TimeoutConfig,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            default_strategy: "first_available".to_string(),
            preferences: vec![
                "wasm".to_string(),
                "container".to_string(),
                "native".to_string(),
            ],
            engines: HashMap::new(),
            timeouts: TimeoutConfig::default(),
        }
    }
}

/// Timeout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    /// Default execution timeout in seconds
    pub default_execution: u64,
    /// Initialization timeout in seconds
    pub initialization: u64,
    /// Shutdown timeout in seconds
    pub shutdown: u64,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            default_execution: 300, // 5 minutes
            initialization: 60,     // 1 minute
            shutdown: 30,           // 30 seconds
        }
    }
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Default isolation level
    pub default_isolation: String,
    /// Sandbox configuration
    pub sandbox: SandboxConfig,
    /// Policy enforcement settings
    pub policies: PolicySettings,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            default_isolation: "standard".to_string(),
            sandbox: SandboxConfig::default(),
            policies: PolicySettings::default(),
        }
    }
}

/// Sandbox configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Enable seccomp filtering
    pub enable_seccomp: bool,
    /// Enable capability dropping
    pub drop_capabilities: bool,
    /// Enable network isolation
    pub network_isolation: bool,
    /// Enable filesystem isolation
    pub filesystem_isolation: bool,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            enable_seccomp: true,
            drop_capabilities: true,
            network_isolation: true,
            filesystem_isolation: true,
        }
    }
}

/// Policy enforcement settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicySettings {
    /// Strict policy enforcement
    pub strict_enforcement: bool,
    /// Policy violation action
    pub violation_action: String,
    /// Custom policies directory
    pub policies_dir: Option<PathBuf>,
}

impl Default for PolicySettings {
    fn default() -> Self {
        Self {
            strict_enforcement: true,
            violation_action: "terminate".to_string(),
            policies_dir: None,
        }
    }
}

/// Resource management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConfig {
    /// Default resource limits
    pub default_limits: DefaultLimits,
    /// Monitoring configuration
    pub monitoring: MonitoringConfig,
    /// Resource allocation strategy
    pub allocation_strategy: String,
}

impl Default for ResourceConfig {
    fn default() -> Self {
        Self {
            default_limits: DefaultLimits::default(),
            monitoring: MonitoringConfig::default(),
            allocation_strategy: "balanced".to_string(),
        }
    }
}

/// Default resource limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultLimits {
    /// CPU cores (fractional allowed)
    pub cpu_cores: f64,
    /// Memory in megabytes
    pub memory_mb: u64,
    /// Storage in megabytes
    pub storage_mb: u64,
    /// Network bandwidth in Mbps
    pub network_mbps: Option<u32>,
}

impl Default for DefaultLimits {
    fn default() -> Self {
        Self {
            cpu_cores: 1.0,
            memory_mb: 512,
            storage_mb: 1024,
            network_mbps: None,
        }
    }
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Enable monitoring
    pub enabled: bool,
    /// Metrics collection interval in seconds
    pub metrics_interval: u64,
    /// Log level
    pub log_level: String,
    /// Enable telemetry
    pub enable_telemetry: bool,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: std::env::var("TOADSTOOL_MONITORING")
                .map(|s| s.to_lowercase() == "true")
                .unwrap_or(true),
            metrics_interval: std::env::var("TOADSTOOL_METRICS_INTERVAL")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
            log_level: std::env::var("TOADSTOOL_LOG_LEVEL")
                .unwrap_or_else(|_| "info".to_string()),
            enable_telemetry: std::env::var("TOADSTOOL_TELEMETRY")
                .map(|s| s.to_lowercase() == "true")
                .unwrap_or(false),
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
    /// Log output destination
    pub output: String,
    /// Log file path (if output is "file")
    pub file_path: Option<PathBuf>,
    /// Enable structured logging
    pub structured: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "pretty".to_string(),
            output: "stdout".to_string(),
            file_path: None,
            structured: false,
        }
    }
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server bind address
    pub bind_address: String,
    /// Server port
    pub port: u16,
    /// Enable TLS
    pub tls_enabled: bool,
    /// TLS certificate file path
    pub tls_cert_path: Option<PathBuf>,
    /// TLS private key file path
    pub tls_key_path: Option<PathBuf>,
    /// Maximum concurrent connections
    pub max_connections: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1".to_string(),
            port: 8080,
            tls_enabled: false,
            tls_cert_path: None,
            tls_key_path: None,
            max_connections: 1000,
        }
    }
}

/// Platform-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlatformConfig {
    /// Platform-specific runtime settings
    pub runtime_settings: HashMap<String, serde_json::Value>,
    /// Platform-specific security settings
    pub security_settings: HashMap<String, serde_json::Value>,
    /// Platform-specific optimizations
    pub optimizations: HashMap<String, bool>,
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Default bind address
    pub bind_address: String,
    /// Default discovery timeout
    pub timeout_seconds: u64,
    /// Enable TLS
    pub enable_tls: bool,
    /// Maximum concurrent connections
    pub max_connections: usize,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            bind_address: std::env::var("TOADSTOOL_BIND_ADDRESS")
                .unwrap_or_else(|_| "0.0.0.0:8080".to_string()),
            timeout_seconds: std::env::var("TOADSTOOL_TIMEOUT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),
            enable_tls: std::env::var("TOADSTOOL_ENABLE_TLS")
                .map(|s| s.to_lowercase() == "true")
                .unwrap_or(false),
            max_connections: std::env::var("TOADSTOOL_MAX_CONNECTIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1000),
        }
    }
}

/// Ecosystem configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemConfig {
    /// Auto discovery
    pub auto_discovery: bool,
    /// Discovery interval
    pub discovery_interval: u64,
    /// Trust level
    pub trust_level: TrustLevel,
    /// Required services
    pub required_services: Vec<String>,
}

impl Default for EcosystemConfig {
    fn default() -> Self {
        Self {
            auto_discovery: std::env::var("TOADSTOOL_AUTO_DISCOVERY")
                .map(|s| s.to_lowercase() == "true")
                .unwrap_or(true),
            discovery_interval: std::env::var("TOADSTOOL_DISCOVERY_INTERVAL")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(60),
            trust_level: TrustLevel::Medium,
            required_services: vec![], // No required services by default
        }
    }
}

/// Configuration builder for creating ToadStool configurations
#[derive(Debug)]
pub struct ConfigBuilder {
    config: Config,
    config_paths: Vec<PathBuf>,
    env_prefix: String,
}

impl ConfigBuilder {
    /// Create a new configuration builder
    pub fn new() -> Self {
        Self {
            config: Config::builder().build().unwrap_or_else(|_| {
                warn!("Failed to create default config, using empty config");
                Config::default()
            }),
            config_paths: Vec::new(),
            env_prefix: "TOADSTOOL".to_string(),
        }
    }

    /// Add a configuration file
    pub fn add_file<P: AsRef<Path>>(mut self, path: P) -> Self {
        let path = path.as_ref().to_path_buf();
        self.config_paths.push(path.clone());

        if path.exists() {
            info!("Loading configuration file: {}", path.display());
            let file = File::from(path.clone()).required(false);
            match Config::builder()
                .add_source(self.config.clone())
                .add_source(file)
                .build() {
                Ok(config) => self.config = config,
                Err(e) => {
                    warn!("Failed to load configuration file {}: {}", path.display(), e);
                    // Continue with existing config
                }
            }
        } else {
            warn!(
                "Configuration file not found (optional): {}",
                path.display()
            );
        }

        self
    }

    /// Add configuration files from a directory
    pub fn add_directory<P: AsRef<Path>>(mut self, dir: P) -> ConfigResult<Self> {
        let dir = dir.as_ref();

        if !dir.exists() {
            return Ok(self);
        }

        let entries = std::fs::read_dir(dir)?;
        let mut config_files = Vec::new();

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if let Some(extension) = path.extension() {
                if matches!(
                    extension.to_str(),
                    Some("toml") | Some("yaml") | Some("yml") | Some("json")
                ) {
                    config_files.push(path);
                }
            }
        }

        // Sort files for consistent loading order
        config_files.sort();

        for config_file in config_files {
            self = self.add_file(config_file);
        }

        Ok(self)
    }

    /// Set environment variable prefix
    pub fn env_prefix<S: Into<String>>(mut self, prefix: S) -> Self {
        self.env_prefix = prefix.into();
        self
    }

    /// Add environment variables
    pub fn add_env(mut self) -> Self {
        let env = Environment::with_prefix(&self.env_prefix)
            .prefix_separator("_")
            .separator("__");

        match Config::builder()
            .add_source(self.config.clone())
            .add_source(env)
            .build() {
            Ok(config) => self.config = config,
            Err(e) => {
                warn!("Failed to add environment variables: {}", e);
                // Continue with existing config
            }
        }

        self
    }

    /// Build the configuration
    pub fn build(self) -> ConfigResult<ToadStoolConfig> {
        debug!("Building ToadStool configuration");

        // Clone config before deserializing to preserve original
        let config_clone = self.config.clone();

        // Try to deserialize into our serializable structure first
        let serializable_config: SerializableConfig = config_clone.try_deserialize()
            .unwrap_or_else(|_| {
                warn!("Failed to deserialize full configuration, using defaults");
                SerializableConfig {
                    runtime: RuntimeConfig::default(),
                    security: SecurityConfig::default(),
                    resources: ResourceConfig::default(),
                    custom: HashMap::new(),
                    network: NetworkConfig::default(),
                    ecosystem: EcosystemConfig::default(),
                    monitoring: MonitoringConfig::default(),
                }
            });

        // Convert to main config structure
        let mut toadstool_config = ToadStoolConfig::from(serializable_config);
        toadstool_config.config_source = Some(self.config);

        info!("Configuration loaded successfully");
        debug!("Loaded configuration: {:#?}", toadstool_config);

        Ok(toadstool_config)
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Get standard configuration directory paths
pub fn get_config_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // System-wide configuration
    if cfg!(unix) {
        paths.push(PathBuf::from("/etc/toadstool"));
    }

    // User configuration
    if let Some(dirs) = directories::ProjectDirs::from("com", "toadstool", "toadstool") {
        paths.push(dirs.config_dir().to_path_buf());
    }

    // Current directory
    paths.push(PathBuf::from("."));

    // Environment variable override
    if let Ok(config_dir) = env::var("TOADSTOOL_CONFIG_DIR") {
        paths.push(PathBuf::from(config_dir));
    }

    paths
}

/// Load ToadStool configuration with default paths and environment variables
pub fn load_config() -> ConfigResult<ToadStoolConfig> {
    let mut builder = ConfigBuilder::new();

    // Add configuration files from standard paths
    for config_path in get_config_paths() {
        builder = builder.add_directory(&config_path)?;

        // Also try specific config files
        for filename in &[
            "toadstool.toml",
            "toadstool.yaml",
            "toadstool.yml",
            "toadstool.json",
        ] {
            let config_file = config_path.join(filename);
            builder = builder.add_file(config_file);
        }
    }

    // Add environment variables
    builder = builder.add_env();

    // Build final configuration
    builder.build()
}

/// Runtime-specific configurations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuntimeConfigurations {
    /// WebAssembly runtime configuration
    pub wasm: WasmRuntimeDefaults,
    /// Container runtime configuration
    pub container: ContainerRuntimeDefaults,
    /// GPU runtime configuration
    pub gpu: GpuRuntimeDefaults,
    /// Native runtime configuration
    pub native: NativeRuntimeDefaults,
}

/// WebAssembly runtime defaults
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmRuntimeDefaults {
    /// Maximum concurrent instances
    pub max_concurrent_instances: u32,
    /// Default memory limit in MB
    pub default_memory_limit_mb: u64,
    /// Cache configuration
    pub cache: WasmCacheDefaults,
    /// Timeout configuration
    pub timeouts: WasmTimeoutDefaults,
    /// Security configuration
    pub security: WasmSecurityDefaults,
}

impl Default for WasmRuntimeDefaults {
    fn default() -> Self {
        Self {
            max_concurrent_instances: 1000,
            default_memory_limit_mb: 128,
            cache: WasmCacheDefaults::default(),
            timeouts: WasmTimeoutDefaults::default(),
            security: WasmSecurityDefaults::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmCacheDefaults {
    /// Enable caching by default
    pub enabled: bool,
    /// Default cache size in MB
    pub max_size_mb: u64,
    /// Cache TTL in hours
    pub ttl_hours: u64,
    /// Cache cleanup interval in minutes
    pub cleanup_interval_minutes: u64,
}

impl Default for WasmCacheDefaults {
    fn default() -> Self {
        Self {
            enabled: true,
            max_size_mb: 512,
            ttl_hours: 24,
            cleanup_interval_minutes: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmTimeoutDefaults {
    /// Maximum execution timeout in seconds
    pub max_execution_seconds: u64,
    /// Module load timeout in seconds
    pub module_load_seconds: u64,
    /// Instance creation timeout in seconds
    pub instance_creation_seconds: u64,
}

impl Default for WasmTimeoutDefaults {
    fn default() -> Self {
        Self {
            max_execution_seconds: 3600,   // 1 hour
            module_load_seconds: 300,      // 5 minutes
            instance_creation_seconds: 60, // 1 minute
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmSecurityDefaults {
    /// Maximum open file descriptors
    pub max_open_fds: u32,
    /// Enable filesystem access by default
    pub filesystem_access_enabled: bool,
    /// Enable network access by default
    pub network_access_enabled: bool,
    /// Maximum memory pages
    pub max_memory_pages: u32,
}

impl Default for WasmSecurityDefaults {
    fn default() -> Self {
        Self {
            max_open_fds: 64,
            filesystem_access_enabled: false,
            network_access_enabled: false,
            max_memory_pages: 2048, // 128MB / 64KB per page
        }
    }
}

/// Container runtime defaults
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerRuntimeDefaults {
    /// Maximum concurrent containers
    pub max_concurrent_containers: u32,
    /// Default resource limits
    pub resources: ContainerResourceDefaults,
    /// Network configuration
    pub network: ContainerNetworkDefaults,
    /// Security configuration
    pub security: ContainerSecurityDefaults,
    /// Image management
    pub images: ContainerImageDefaults,
}

impl Default for ContainerRuntimeDefaults {
    fn default() -> Self {
        Self {
            max_concurrent_containers: 100,
            resources: ContainerResourceDefaults::default(),
            network: ContainerNetworkDefaults::default(),
            security: ContainerSecurityDefaults::default(),
            images: ContainerImageDefaults::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerResourceDefaults {
    /// Default memory limit in MB
    pub memory_limit_mb: u64,
    /// Default CPU limit in millicores
    pub cpu_limit_millicores: u32,
    /// Default execution timeout in seconds
    pub execution_timeout_seconds: u64,
    /// Default disk I/O limit in MB/s
    pub disk_io_limit_mbps: u64,
}

impl Default for ContainerResourceDefaults {
    fn default() -> Self {
        Self {
            memory_limit_mb: 512,
            cpu_limit_millicores: 1000,      // 1 CPU core
            execution_timeout_seconds: 3600, // 1 hour
            disk_io_limit_mbps: 100,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerNetworkDefaults {
    /// Default DNS servers
    pub dns_servers: Vec<String>,
    /// Allowed port ranges
    pub allowed_port_ranges: Vec<PortRangeConfig>,
    /// Enable custom networks
    pub allow_custom_networks: bool,
}

impl Default for ContainerNetworkDefaults {
    fn default() -> Self {
        Self {
            dns_servers: vec!["8.8.8.8".to_string(), "8.8.4.4".to_string()],
            allowed_port_ranges: vec![
                PortRangeConfig {
                    start: 8000,
                    end: 8999,
                },
                PortRangeConfig {
                    start: 3000,
                    end: 3999,
                },
            ],
            allow_custom_networks: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortRangeConfig {
    pub start: u16,
    pub end: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerSecurityDefaults {
    /// Require non-root user
    pub require_non_root: bool,
    /// Drop all capabilities by default
    pub drop_all_capabilities: bool,
    /// Enable seccomp by default
    pub enable_seccomp: bool,
    /// Read-only root filesystem
    pub read_only_root_fs: bool,
}

impl Default for ContainerSecurityDefaults {
    fn default() -> Self {
        Self {
            require_non_root: true,
            drop_all_capabilities: true,
            enable_seccomp: true,
            read_only_root_fs: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerImageDefaults {
    /// Enable image caching
    pub cache_enabled: bool,
    /// Cache size limit in MB
    pub cache_size_limit_mb: u64,
    /// Cache cleanup interval in hours
    pub cache_cleanup_interval_hours: u64,
    /// Image pull timeout in seconds
    pub pull_timeout_seconds: u64,
}

impl Default for ContainerImageDefaults {
    fn default() -> Self {
        Self {
            cache_enabled: true,
            cache_size_limit_mb: 5120, // 5 GB
            cache_cleanup_interval_hours: 1,
            pull_timeout_seconds: 300, // 5 minutes
        }
    }
}

/// GPU runtime defaults
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuRuntimeDefaults {
    /// Maximum concurrent kernels
    pub max_concurrent_kernels: u32,
    /// Default memory per kernel in MB
    pub default_memory_per_kernel_mb: u64,
    /// Maximum kernel execution time in seconds
    pub max_kernel_execution_seconds: u64,
    /// Device selection strategy
    pub device_selection_strategy: String,
    /// Memory management
    pub memory: GpuMemoryDefaults,
    /// Monitoring configuration
    pub monitoring: GpuMonitoringDefaults,
}

impl Default for GpuRuntimeDefaults {
    fn default() -> Self {
        Self {
            max_concurrent_kernels: 100,
            default_memory_per_kernel_mb: 2048, // 2 GB
            max_kernel_execution_seconds: 60,
            device_selection_strategy: "auto".to_string(),
            memory: GpuMemoryDefaults::default(),
            monitoring: GpuMonitoringDefaults::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuMemoryDefaults {
    /// Enable memory pooling
    pub pooling_enabled: bool,
    /// Pool size in MB
    pub pool_size_mb: u64,
    /// Memory allocation strategy
    pub allocation_strategy: String,
}

impl Default for GpuMemoryDefaults {
    fn default() -> Self {
        Self {
            pooling_enabled: true,
            pool_size_mb: 512,
            allocation_strategy: "on_demand".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuMonitoringDefaults {
    /// Enable performance profiling
    pub profiling_enabled: bool,
    /// Enable memory tracking
    pub memory_tracking_enabled: bool,
    /// Enable power monitoring
    pub power_monitoring_enabled: bool,
    /// Monitoring interval in seconds
    pub monitoring_interval_seconds: u64,
}

impl Default for GpuMonitoringDefaults {
    fn default() -> Self {
        Self {
            profiling_enabled: false,
            memory_tracking_enabled: true,
            power_monitoring_enabled: false,
            monitoring_interval_seconds: 1,
        }
    }
}

/// Native runtime defaults
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeRuntimeDefaults {
    /// Maximum concurrent processes
    pub max_concurrent_processes: u32,
    /// Default execution timeout in seconds
    pub default_execution_timeout_seconds: u64,
    /// Process limits
    pub process_limits: NativeProcessLimits,
    /// Security configuration
    pub security: NativeSecurityDefaults,
}

impl Default for NativeRuntimeDefaults {
    fn default() -> Self {
        Self {
            max_concurrent_processes: 100,
            default_execution_timeout_seconds: 300, // 5 minutes
            process_limits: NativeProcessLimits::default(),
            security: NativeSecurityDefaults::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeProcessLimits {
    /// Maximum CPU cores per process
    pub max_cpu_cores: f64,
    /// Maximum memory in MB per process
    pub max_memory_mb: u64,
    /// Maximum file descriptors per process
    pub max_file_descriptors: u32,
}

impl Default for NativeProcessLimits {
    fn default() -> Self {
        Self {
            max_cpu_cores: 32.0,
            max_memory_mb: 128 * 1024, // 128 GB
            max_file_descriptors: 1024,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeSecurityDefaults {
    /// Enable sandbox by default
    pub sandbox_enabled: bool,
    /// Drop privileges by default
    pub drop_privileges: bool,
    /// Enable resource limits by default
    pub resource_limits_enabled: bool,
}

impl Default for NativeSecurityDefaults {
    fn default() -> Self {
        Self {
            sandbox_enabled: true,
            drop_privileges: true,
            resource_limits_enabled: true,
        }
    }
}

/// Update the main ToadStoolConfig to include runtime configurations
impl ToadStoolConfig {
    /// Get runtime-specific configuration
    pub fn get_runtime_config(&self) -> RuntimeConfigurations {
        // This could be loaded from a separate config file or environment
        // For now, return defaults that can be overridden
        RuntimeConfigurations::default()
    }
}

/// Configuration for serialization (excludes non-serializable fields)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableConfig {
    /// Runtime configuration
    pub runtime: RuntimeConfig,
    /// Security configuration  
    pub security: SecurityConfig,
    /// Resource configuration
    pub resources: ResourceConfig,
    /// Custom configurations
    pub custom: HashMap<String, serde_json::Value>,
    /// Network configuration
    pub network: NetworkConfig,
    /// Ecosystem configuration
    pub ecosystem: EcosystemConfig,
    /// Monitoring configuration
    pub monitoring: MonitoringConfig,
}

impl From<ToadStoolConfig> for SerializableConfig {
    fn from(config: ToadStoolConfig) -> Self {
        Self {
            runtime: config.runtime,
            security: config.security,
            resources: config.resources,
            custom: config.custom,
            network: config.network,
            ecosystem: config.ecosystem,
            monitoring: config.monitoring,
        }
    }
}

impl From<SerializableConfig> for ToadStoolConfig {
    fn from(config: SerializableConfig) -> Self {
        Self {
            runtime: config.runtime,
            security: config.security,
            resources: config.resources,
            custom: config.custom,
            network: config.network,
            ecosystem: config.ecosystem,
            monitoring: config.monitoring,
            config_source: None,
        }
    }
}

impl Default for ToadStoolConfig {
    fn default() -> Self {
        Self {
            // Core configuration
            runtime: RuntimeConfig::default(),
            security: SecurityConfig::default(),
            resources: ResourceConfig::default(),
            
            // Network configuration with environment-aware defaults
            network: NetworkConfig::default(),
            
            // Ecosystem configuration
            ecosystem: EcosystemConfig::default(),
            
            // Monitoring configuration
            monitoring: MonitoringConfig::default(),

            // Custom configurations
            custom: HashMap::new(),

            // Configuration source (when available)
            config_source: Some(
                Config::builder()
                    .add_source(config::Environment::with_prefix("TOADSTOOL"))
                    .build()
                    .unwrap_or_else(|e| {
                        warn!("Failed to build configuration from environment: {}, using defaults", e);
                        Config::default()
                    })
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_default_config() {
        let config = ToadStoolConfig::default();
        assert_eq!(config.runtime.default_strategy, "first_available");
        assert_eq!(config.security.default_isolation, "standard");
        assert!(config.resources.monitoring.enabled);
        assert_eq!(config.network.bind_address, "0.0.0.0");
        assert_eq!(config.ecosystem.trust_level, TrustLevel::Medium);
    }

    #[test]
    fn test_config_builder() {
        let config = ConfigBuilder::new().build().unwrap();

        // Should build successfully with defaults
        assert_eq!(config.runtime.default_strategy, "first_available");
        assert_eq!(config.security.default_isolation, "standard");
        assert!(config.resources.monitoring.enabled);
    }

    #[test]
    fn test_config_from_file() {
        let temp_dir = tempdir().unwrap();
        let config_file = temp_dir.path().join("test.toml");

        let config_content = r#"
[runtime]
default_strategy = "preference_list"

[security]
default_isolation = "enhanced"

[network]
bind_address = "127.0.0.1"
timeout_seconds = 60
"#;

        fs::write(&config_file, config_content).unwrap();

        let config = ConfigBuilder::new().add_file(&config_file).build().unwrap();

        assert_eq!(config.runtime.default_strategy, "preference_list");
        assert_eq!(config.security.default_isolation, "enhanced");
        assert_eq!(config.network.bind_address, "127.0.0.1");
        assert_eq!(config.network.timeout_seconds, 60);
    }
}
