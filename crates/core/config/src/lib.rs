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

//! # ToadStool Configuration Management
//!
//! This module provides comprehensive configuration management for ToadStool,
//! integrating with Songbird's port orchestration and providing environment-aware
//! configuration loading with dynamic updates.

use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use notify::{watcher, RecursiveMode, Watcher};
use validator::{Validate, ValidationError};
use tokio::sync::{broadcast, RwLock};

/// Configuration validation trait for custom validation logic
pub trait ConfigValidation {
    fn validate_custom(&self) -> Result<(), ConfigError>;
}

/// Enhanced validation for network endpoints
fn validate_endpoint(endpoint: &str) -> Result<(), ValidationError> {
    if endpoint.is_empty() {
        return Err(ValidationError::new("endpoint_empty"));
    }
    
    // Basic URL validation
    if !endpoint.starts_with("http://") && !endpoint.starts_with("https://") {
        return Err(ValidationError::new("invalid_endpoint_scheme"));
    }
    
    Ok(())
}

/// Validate port ranges
fn validate_port_range(port: u16) -> Result<(), ValidationError> {
    if port < 1024 || port > 65535 {
        return Err(ValidationError::new("port_out_of_range"));
    }
    Ok(())
}

/// Validate timeout values
fn validate_timeout(timeout: u64) -> Result<(), ValidationError> {
    if timeout == 0 || timeout > 3600 {
        return Err(ValidationError::new("timeout_out_of_range"));
    }
    Ok(())
}

/// Constants module for configuration values
pub mod constants {
    /// Network configuration constants
    pub mod network {
        /// Default ToadStool port
        pub const DEFAULT_TOADSTOOL_PORT: u16 = 8081;
        
        /// Default Songbird port
        pub const DEFAULT_SONGBIRD_PORT: u16 = 8080;
        
        /// Default BearDog port
        pub const DEFAULT_BEARDOG_PORT: u16 = 8082;
        
        /// Default NestGate port
        pub const DEFAULT_NESTGATE_PORT: u16 = 8083;
        
        /// Default localhost address
        pub const DEFAULT_LOCALHOST: &str = "127.0.0.1";
        
        /// Default bind address
        pub const DEFAULT_BIND_ADDRESS: &str = "0.0.0.0";
        
        /// Default maximum connections
        pub const DEFAULT_MAX_CONNECTIONS: usize = 1000;
    }
}

/// Enhanced ToadStool configuration with environment variable support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToadStoolConfig {
    /// Server configuration
    pub server: ServerConfig,
    /// Runtime configurations
    pub runtimes: RuntimesConfig,
    /// Security configuration
    pub security: SecurityConfig,
    /// Monitoring configuration
    pub monitoring: MonitoringConfig,
    /// Federation configuration
    pub federation: FederationConfig,
    /// Ecosystem integration configuration
    pub ecosystem: EcosystemConfig,
    /// Performance optimization configuration
    pub performance: PerformanceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server bind address
    pub bind_address: String,
    /// Server port
    pub port: u16,
    /// Maximum concurrent connections
    pub max_connections: usize,
    /// Request timeout in seconds
    pub request_timeout_seconds: u64,
    /// Enable TLS
    pub tls_enabled: bool,
    /// TLS certificate path
    pub tls_cert_path: Option<PathBuf>,
    /// TLS private key path
    pub tls_key_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimesConfig {
    /// Native runtime configuration
    pub native: NativeRuntimeConfig,
    /// Container runtime configuration
    pub container: ContainerRuntimeConfig,
    /// WASM runtime configuration
    pub wasm: WasmRuntimeConfig,
    /// GPU runtime configuration
    pub gpu: GpuRuntimeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeRuntimeConfig {
    /// Enable native runtime
    pub enabled: bool,
    /// Maximum concurrent executions
    pub max_concurrent: usize,
    /// Execution timeout in seconds
    pub timeout_seconds: u64,
    /// Resource limits
    pub resource_limits: ResourceLimitsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerRuntimeConfig {
    /// Enable container runtime
    pub enabled: bool,
    /// Container runtime (docker, podman, containerd)
    pub runtime: String,
    /// Maximum concurrent containers
    pub max_concurrent: usize,
    /// Container timeout in seconds
    pub timeout_seconds: u64,
    /// Default container image
    pub default_image: String,
    /// Resource limits
    pub resource_limits: ResourceLimitsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmRuntimeConfig {
    /// Enable WASM runtime
    pub enabled: bool,
    /// Maximum concurrent WASM instances
    pub max_concurrent: usize,
    /// WASM execution timeout in seconds
    pub timeout_seconds: u64,
    /// Memory limit per instance in MB
    pub memory_limit_mb: usize,
    /// Enable WASI support
    pub wasi_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuRuntimeConfig {
    /// Enable GPU runtime
    pub enabled: bool,
    /// GPU frameworks to enable (cuda, opencl, vulkan)
    pub frameworks: Vec<String>,
    /// Maximum concurrent GPU jobs
    pub max_concurrent: usize,
    /// GPU memory limit per job in MB
    pub memory_limit_mb: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimitsConfig {
    /// Maximum CPU cores per execution
    pub max_cpu_cores: f64,
    /// Maximum memory in MB
    pub max_memory_mb: usize,
    /// Maximum storage in MB
    pub max_storage_mb: usize,
    /// Maximum network bandwidth in Mbps
    pub max_network_mbps: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable authentication
    pub auth_enabled: bool,
    /// Authentication method (bearer, jwt, api_key)
    pub auth_method: String,
    /// JWT secret for token validation
    pub jwt_secret: Option<String>,
    /// API key for simple authentication
    pub api_key: Option<String>,
    /// Enable sandboxing
    pub sandbox_enabled: bool,
    /// Sandbox type (chroot, docker, firejail)
    pub sandbox_type: String,
    /// Enable network isolation
    pub network_isolation: bool,
    /// Allowed network destinations
    pub allowed_destinations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Enable metrics collection
    pub metrics_enabled: bool,
    /// Metrics collection interval in seconds
    pub metrics_interval_seconds: u64,
    /// Enable real-time monitoring
    pub realtime_enabled: bool,
    /// Metrics retention period in hours
    pub retention_hours: u64,
    /// Enable performance profiling
    pub profiling_enabled: bool,
    /// Export metrics to external systems
    pub export_enabled: bool,
    /// Export endpoints
    pub export_endpoints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationConfig {
    /// Enable federation
    pub enabled: bool,
    /// Federation discovery method (dns, static, consul)
    pub discovery_method: String,
    /// Static federation peers
    pub static_peers: Vec<String>,
    /// Federation port
    pub port: u16,
    /// Enable encryption for federation traffic
    pub encryption_enabled: bool,
    /// Federation authentication key
    pub auth_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemConfig {
    /// Songbird integration configuration
    pub songbird: SongbirdConfig,
    /// BearDog integration configuration
    pub beardog: BearDogConfig,
    /// NestGate integration configuration
    pub nestgate: NestGateConfig,
    /// BiomeOS integration configuration
    pub biomeos: BiomeOSConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongbirdConfig {
    /// Enable Songbird integration
    pub enabled: bool,
    /// Songbird endpoint URL
    pub endpoint: String,
    /// Connection timeout in seconds
    pub timeout_seconds: u64,
    /// Enable load balancing
    pub load_balancing: bool,
    /// Authentication token
    pub auth_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BearDogConfig {
    /// Enable BearDog integration
    pub enabled: bool,
    /// BearDog endpoint URL
    pub endpoint: String,
    /// Security level (low, medium, high, maximum)
    pub security_level: String,
    /// Enable crypto lock
    pub crypto_lock_enabled: bool,
    /// Authentication key
    pub auth_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NestGateConfig {
    /// Enable NestGate integration
    pub enabled: bool,
    /// NestGate endpoint URL
    pub endpoint: String,
    /// Storage tier (hot, warm, cold)
    pub storage_tier: String,
    /// Enable distributed storage
    pub distributed_enabled: bool,
    /// Authentication token
    pub auth_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeOSConfig {
    /// Enable BiomeOS integration
    pub enabled: bool,
    /// BiomeOS API endpoint
    pub api_endpoint: String,
    /// Enable BYOB (Bring Your Own Biome)
    pub byob_enabled: bool,
    /// Default team ID
    pub default_team_id: Option<String>,
    /// Authentication token
    pub auth_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Enable performance optimization
    pub optimization_enabled: bool,
    /// Performance monitoring interval in seconds
    pub monitoring_interval_seconds: u64,
    /// Enable runtime selection optimization
    pub runtime_selection_enabled: bool,
    /// Enable resource prediction
    pub resource_prediction_enabled: bool,
    /// Performance threshold percentile
    pub threshold_percentile: f64,
    /// Target resource utilization percentage
    pub target_utilization_percent: f64,
}

/// Configuration profile types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConfigProfile {
    Development,
    Staging,
    Production,
    Testing,
    Custom(String),
}

/// Enhanced configuration manager with hot-reload support
#[derive(Clone)]
pub struct ConfigManager {
    config: Arc<RwLock<ToadStoolConfig>>,
    env_prefix: String,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(ToadStoolConfig::default())),
            env_prefix: "TOADSTOOL".to_string(),
        }
    }

    /// Create configuration manager with custom environment prefix
    pub fn with_prefix(prefix: &str) -> Self {
        Self {
            config: Arc::new(RwLock::new(ToadStoolConfig::default())),
            env_prefix: prefix.to_string(),
        }
    }

    /// Load configuration from environment variables
    pub async fn load_from_env(&self) -> Result<(), ConfigError> {
        let mut config = self.config.write().await;
        
        // Server configuration
        if let Ok(bind_address) = env::var(format!("{}_BIND_ADDRESS", self.env_prefix)) {
            config.server.bind_address = bind_address;
        }
        
        if let Ok(port) = env::var(format!("{}_PORT", self.env_prefix)) {
            config.server.port = port.parse().map_err(|_| ConfigError::InvalidValue("port".to_string()))?;
        }
        
        if let Ok(max_connections) = env::var(format!("{}_MAX_CONNECTIONS", self.env_prefix)) {
            config.server.max_connections = max_connections.parse().map_err(|_| ConfigError::InvalidValue("max_connections".to_string()))?;
        }

        // Runtime configurations
        if let Ok(native_enabled) = env::var(format!("{}_NATIVE_ENABLED", self.env_prefix)) {
            config.runtimes.native.enabled = native_enabled.parse().unwrap_or(true);
        }

        if let Ok(container_enabled) = env::var(format!("{}_CONTAINER_ENABLED", self.env_prefix)) {
            config.runtimes.container.enabled = container_enabled.parse().unwrap_or(true);
        }

        if let Ok(wasm_enabled) = env::var(format!("{}_WASM_ENABLED", self.env_prefix)) {
            config.runtimes.wasm.enabled = wasm_enabled.parse().unwrap_or(true);
        }

        if let Ok(gpu_enabled) = env::var(format!("{}_GPU_ENABLED", self.env_prefix)) {
            config.runtimes.gpu.enabled = gpu_enabled.parse().unwrap_or(false);
        }

        // Security configuration
        if let Ok(auth_enabled) = env::var(format!("{}_AUTH_ENABLED", self.env_prefix)) {
            config.security.auth_enabled = auth_enabled.parse().unwrap_or(false);
        }

        if let Ok(jwt_secret) = env::var(format!("{}_JWT_SECRET", self.env_prefix)) {
            config.security.jwt_secret = Some(jwt_secret);
        }

        // Ecosystem configuration
        if let Ok(songbird_endpoint) = env::var(format!("{}_SONGBIRD_ENDPOINT", self.env_prefix)) {
            config.ecosystem.songbird.endpoint = songbird_endpoint;
        }

        if let Ok(beardog_endpoint) = env::var(format!("{}_BEARDOG_ENDPOINT", self.env_prefix)) {
            config.ecosystem.beardog.endpoint = beardog_endpoint;
        }

        if let Ok(nestgate_endpoint) = env::var(format!("{}_NESTGATE_ENDPOINT", self.env_prefix)) {
            config.ecosystem.nestgate.endpoint = nestgate_endpoint;
        }

        if let Ok(biomeos_endpoint) = env::var(format!("{}_BIOMEOS_ENDPOINT", self.env_prefix)) {
            config.ecosystem.biomeos.api_endpoint = biomeos_endpoint;
        }

        Ok(())
    }

    /// Load configuration from file
    pub async fn load_from_file(&self, path: &PathBuf) -> Result<(), ConfigError> {
        let content = tokio::fs::read_to_string(path).await
            .map_err(|e| ConfigError::FileError(e.to_string()))?;
        
        let loaded_config: ToadStoolConfig = match path.extension().and_then(|s| s.to_str()) {
            Some("yaml") | Some("yml") => {
                serde_yaml::from_str(&content)
                    .map_err(|e| ConfigError::ParseError(e.to_string()))?
            }
            Some("json") => {
                serde_json::from_str(&content)
                    .map_err(|e| ConfigError::ParseError(e.to_string()))?
            }
            Some("toml") => {
                toml::from_str(&content)
                    .map_err(|e| ConfigError::ParseError(e.to_string()))?
            }
            _ => return Err(ConfigError::UnsupportedFormat),
        };

        *self.config.write().await = loaded_config;
        Ok(())
    }

    /// Get current configuration
    pub async fn get_config(&self) -> ToadStoolConfig {
        self.config.read().await.clone()
    }

    /// Update configuration at runtime
    pub async fn update_config<F>(&self, updater: F) -> Result<(), ConfigError>
    where
        F: FnOnce(&mut ToadStoolConfig) -> Result<(), ConfigError>,
    {
        let mut config = self.config.write().await;
        updater(&mut config)?;
        self.validate_config(&config)?;
        Ok(())
    }

    /// Validate configuration
    fn validate_config(&self, config: &ToadStoolConfig) -> Result<(), ConfigError> {
        // Validate server configuration
        if config.server.port == 0 {
            return Err(ConfigError::ValidationError("Server port cannot be 0".to_string()));
        }

        if config.server.max_connections == 0 {
            return Err(ConfigError::ValidationError("Max connections cannot be 0".to_string()));
        }

        // Validate runtime configurations
        if !config.runtimes.native.enabled && !config.runtimes.container.enabled && 
           !config.runtimes.wasm.enabled && !config.runtimes.gpu.enabled {
            return Err(ConfigError::ValidationError("At least one runtime must be enabled".to_string()));
        }

        // Validate resource limits
        if config.runtimes.native.resource_limits.max_cpu_cores <= 0.0 {
            return Err(ConfigError::ValidationError("CPU cores limit must be positive".to_string()));
        }

        if config.runtimes.native.resource_limits.max_memory_mb == 0 {
            return Err(ConfigError::ValidationError("Memory limit must be positive".to_string()));
        }

        Ok(())
    }

    /// Get configuration value by path
    pub async fn get_value(&self, path: &str) -> Option<String> {
        let config = self.config.read().await;
        
        match path {
            "server.bind_address" => Some(config.server.bind_address.clone()),
            "server.port" => Some(config.server.port.to_string()),
            "runtimes.native.enabled" => Some(config.runtimes.native.enabled.to_string()),
            "runtimes.container.enabled" => Some(config.runtimes.container.enabled.to_string()),
            "runtimes.wasm.enabled" => Some(config.runtimes.wasm.enabled.to_string()),
            "runtimes.gpu.enabled" => Some(config.runtimes.gpu.enabled.to_string()),
            "security.auth_enabled" => Some(config.security.auth_enabled.to_string()),
            "monitoring.metrics_enabled" => Some(config.monitoring.metrics_enabled.to_string()),
            "federation.enabled" => Some(config.federation.enabled.to_string()),
            _ => None,
        }
    }
}

impl Default for ToadStoolConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            runtimes: RuntimesConfig::default(),
            security: SecurityConfig::default(),
            monitoring: MonitoringConfig::default(),
            federation: FederationConfig::default(),
            ecosystem: EcosystemConfig::default(),
            performance: PerformanceConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1".to_string(),
            port: 8080,
            max_connections: 1000,
            request_timeout_seconds: 30,
            tls_enabled: false,
            tls_cert_path: None,
            tls_key_path: None,
        }
    }
}

impl Default for RuntimesConfig {
    fn default() -> Self {
        Self {
            native: NativeRuntimeConfig::default(),
            container: ContainerRuntimeConfig::default(),
            wasm: WasmRuntimeConfig::default(),
            gpu: GpuRuntimeConfig::default(),
        }
    }
}

impl Default for NativeRuntimeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_concurrent: 10,
            timeout_seconds: 300,
            resource_limits: ResourceLimitsConfig::default(),
        }
    }
}

impl Default for ContainerRuntimeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            runtime: "docker".to_string(),
            max_concurrent: 5,
            timeout_seconds: 600,
            default_image: "ubuntu:latest".to_string(),
            resource_limits: ResourceLimitsConfig::default(),
        }
    }
}

impl Default for WasmRuntimeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_concurrent: 20,
            timeout_seconds: 60,
            memory_limit_mb: 128,
            wasi_enabled: true,
        }
    }
}

impl Default for GpuRuntimeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            frameworks: vec!["cuda".to_string()],
            max_concurrent: 2,
            memory_limit_mb: 1024,
        }
    }
}

impl Default for ResourceLimitsConfig {
    fn default() -> Self {
        Self {
            max_cpu_cores: 4.0,
            max_memory_mb: 2048,
            max_storage_mb: 10240,
            max_network_mbps: Some(100),
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            auth_enabled: false,
            auth_method: "bearer".to_string(),
            jwt_secret: None,
            api_key: None,
            sandbox_enabled: true,
            sandbox_type: "chroot".to_string(),
            network_isolation: false,
            allowed_destinations: vec!["127.0.0.1".to_string()],
        }
    }
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            metrics_enabled: true,
            metrics_interval_seconds: 60,
            realtime_enabled: false,
            retention_hours: 24,
            profiling_enabled: false,
            export_enabled: false,
            export_endpoints: vec![],
        }
    }
}

impl Default for FederationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            discovery_method: "static".to_string(),
            static_peers: vec![],
            port: 8081,
            encryption_enabled: true,
            auth_key: None,
        }
    }
}

impl Default for EcosystemConfig {
    fn default() -> Self {
        Self {
            songbird: SongbirdConfig::default(),
            beardog: BearDogConfig::default(),
            nestgate: NestGateConfig::default(),
            biomeos: BiomeOSConfig::default(),
        }
    }
}

impl Default for SongbirdConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: "http://localhost:8090".to_string(),
            timeout_seconds: 30,
            load_balancing: false,
            auth_token: None,
        }
    }
}

impl Default for BearDogConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: "http://localhost:8091".to_string(),
            security_level: "medium".to_string(),
            crypto_lock_enabled: false,
            auth_key: None,
        }
    }
}

impl Default for NestGateConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: "http://localhost:8092".to_string(),
            storage_tier: "hot".to_string(),
            distributed_enabled: false,
            auth_token: None,
        }
    }
}

impl Default for BiomeOSConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_endpoint: "http://localhost:8093".to_string(),
            byob_enabled: false,
            default_team_id: None,
            auth_token: None,
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            optimization_enabled: true,
            monitoring_interval_seconds: 30,
            runtime_selection_enabled: true,
            resource_prediction_enabled: false,
            threshold_percentile: 95.0,
            target_utilization_percent: 80.0,
        }
    }
}

/// Configuration errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("File error: {0}")]
    FileError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Invalid value for {0}")]
    InvalidValue(String),
    #[error("Unsupported configuration format")]
    UnsupportedFormat,
    #[error("Network error: {0}")]
    Network(String),
    #[error("Other error: {0}")]
    Other(String),
}

/// Configuration change notification
#[derive(Debug, Clone)]
pub struct ConfigChangeNotification {
    pub config_path: PathBuf,
    pub change_type: ConfigChangeType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub enum ConfigChangeType {
    Modified,
    Created,
    Deleted,
    Renamed,
}

/// Hot-reload configuration manager
pub struct HotReloadConfigManager {
    config_manager: ConfigManager,
    config_path: Option<PathBuf>,
    watcher: Option<notify::RecommendedWatcher>,
    change_notifier: Option<tokio::sync::broadcast::Sender<ConfigChangeNotification>>,
}

impl HotReloadConfigManager {
    pub fn new() -> Self {
        Self {
            config_manager: ConfigManager::new(),
            config_path: None,
            watcher: None,
            change_notifier: None,
        }
    }

    pub async fn enable_hot_reload(&mut self, config_path: PathBuf) -> Result<(), ConfigError> {
        let (tx, _rx) = tokio::sync::broadcast::channel(100);
        let (watch_tx, watch_rx) = mpsc::channel();
        
        let mut watcher = watcher(watch_tx, Duration::from_secs(1))
            .map_err(|e| ConfigError::FileError(format!("Failed to create file watcher: {}", e)))?;
        
        watcher.watch(&config_path, RecursiveMode::NonRecursive)
            .map_err(|e| ConfigError::FileError(format!("Failed to watch config file: {}", e)))?;
        
        let tx_clone = tx.clone();
        let config_path_clone = config_path.clone();
        let config_manager_clone = self.config_manager.clone();
        
        tokio::spawn(async move {
            while let Ok(event) = watch_rx.recv() {
                match event {
                    notify::DebouncedEvent::Write(_) | notify::DebouncedEvent::Create(_) => {
                        // Reload configuration
                        if let Err(e) = config_manager_clone.load_from_file(&config_path_clone).await {
                            tracing::error!("Failed to reload configuration: {}", e);
                        } else {
                            tracing::info!("Configuration reloaded successfully");
                            
                            let notification = ConfigChangeNotification {
                                config_path: config_path_clone.clone(),
                                change_type: ConfigChangeType::Modified,
                                timestamp: chrono::Utc::now(),
                            };
                            
                            let _ = tx_clone.send(notification);
                        }
                    }
                    _ => {}
                }
            }
        });
        
        self.config_path = Some(config_path);
        self.watcher = Some(watcher);
        self.change_notifier = Some(tx);
        
        Ok(())
    }
    
    pub fn subscribe_to_changes(&self) -> Option<tokio::sync::broadcast::Receiver<ConfigChangeNotification>> {
        self.change_notifier.as_ref().map(|tx| tx.subscribe())
    }
    
    pub async fn get_config(&self) -> ToadStoolConfig {
        self.config_manager.get_config().await
    }
    
    pub async fn update_config<F>(&self, updater: F) -> Result<(), ConfigError>
    where
        F: FnOnce(&mut ToadStoolConfig) -> Result<(), ConfigError>,
    {
        self.config_manager.update_config(updater).await
    }
}

/// Global configuration instance
static CONFIG_MANAGER: once_cell::sync::Lazy<ConfigManager> = once_cell::sync::Lazy::new(|| {
    ConfigManager::new()
});

/// Get global configuration manager
pub fn get_config_manager() -> &'static ConfigManager {
    &CONFIG_MANAGER
}

/// Initialize configuration from environment and file
pub async fn initialize_config(config_file: Option<PathBuf>) -> Result<(), ConfigError> {
    let manager = get_config_manager();
    
    // Load from file if provided
    if let Some(path) = config_file {
        manager.load_from_file(&path).await?;
    }
    
    // Override with environment variables
    manager.load_from_env().await?;
    
    Ok(())
}

/// Configuration with profile support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfiledConfig {
    /// Active profile
    pub profile: ConfigProfile,
    /// Base configuration
    pub base: ToadStoolConfig,
    /// Profile-specific overrides
    pub overrides: HashMap<ConfigProfile, serde_json::Value>,
}

impl ProfiledConfig {
    pub fn new(profile: ConfigProfile) -> Self {
        Self {
            profile,
            base: ToadStoolConfig::default(),
            overrides: HashMap::new(),
        }
    }
    
    /// Get the effective configuration for the current profile
    pub fn get_effective_config(&self) -> Result<ToadStoolConfig, ConfigError> {
        let mut config = self.base.clone();
        
        // Apply profile-specific overrides
        if let Some(overrides) = self.overrides.get(&self.profile) {
            self.apply_overrides(&mut config, overrides)?;
        }
        
        // Apply profile defaults
        self.apply_profile_defaults(&mut config)?;
        
        Ok(config)
    }
    
    fn apply_overrides(&self, config: &mut ToadStoolConfig, overrides: &serde_json::Value) -> Result<(), ConfigError> {
        // Use serde_json to merge the overrides into the config
        let config_value = serde_json::to_value(&*config)
            .map_err(|e| ConfigError::ParseError(format!("Failed to serialize config: {}", e)))?;
        
        let merged = merge_json_values(config_value, overrides.clone());
        
        *config = serde_json::from_value(merged)
            .map_err(|e| ConfigError::ParseError(format!("Failed to deserialize merged config: {}", e)))?;
        
        Ok(())
    }
    
    fn apply_profile_defaults(&self, config: &mut ToadStoolConfig) -> Result<(), ConfigError> {
        match self.profile {
            ConfigProfile::Development => {
                // Development profile defaults
                config.security.auth_enabled = false;
                config.security.sandbox_enabled = false;
                config.monitoring.metrics_enabled = true;
                config.monitoring.profiling_enabled = true;
                config.performance.optimization_enabled = false;
            }
            ConfigProfile::Staging => {
                // Staging profile defaults
                config.security.auth_enabled = true;
                config.security.sandbox_enabled = true;
                config.monitoring.metrics_enabled = true;
                config.monitoring.profiling_enabled = true;
                config.performance.optimization_enabled = true;
            }
            ConfigProfile::Production => {
                // Production profile defaults
                config.security.auth_enabled = true;
                config.security.sandbox_enabled = true;
                config.monitoring.metrics_enabled = true;
                config.monitoring.profiling_enabled = false;
                config.performance.optimization_enabled = true;
                config.server.tls_enabled = true;
            }
            ConfigProfile::Testing => {
                // Testing profile defaults
                config.security.auth_enabled = false;
                config.security.sandbox_enabled = false;
                config.monitoring.metrics_enabled = false;
                config.monitoring.profiling_enabled = false;
                config.performance.optimization_enabled = false;
            }
            ConfigProfile::Custom(_) => {
                // Custom profiles inherit from development by default
                config.security.auth_enabled = false;
                config.security.sandbox_enabled = false;
            }
        }
        
        Ok(())
    }
}

/// Merge two JSON values, with the second taking precedence
fn merge_json_values(mut base: serde_json::Value, override_val: serde_json::Value) -> serde_json::Value {
    match (&mut base, override_val) {
        (serde_json::Value::Object(base_map), serde_json::Value::Object(override_map)) => {
            for (key, value) in override_map {
                match base_map.get_mut(&key) {
                    Some(base_value) => {
                        *base_value = merge_json_values(base_value.clone(), value);
                    }
                    None => {
                        base_map.insert(key, value);
                    }
                }
            }
            base
        }
        (_, override_val) => override_val,
    }
}

/// Enhanced environment variable configuration loader
pub struct EnvConfigLoader {
    prefix: String,
    separator: String,
}

impl EnvConfigLoader {
    pub fn new(prefix: &str) -> Self {
        Self {
            prefix: prefix.to_string(),
            separator: "_".to_string(),
        }
    }
    
    pub fn with_separator(mut self, separator: &str) -> Self {
        self.separator = separator.to_string();
        self
    }
    
    /// Load configuration from environment variables with nested structure support
    pub fn load_from_env(&self) -> Result<ToadStoolConfig, ConfigError> {
        let mut config = ToadStoolConfig::default();
        let env_vars = self.collect_env_vars();
        
        // Parse environment variables into nested structure
        let nested_config = self.parse_nested_env_vars(&env_vars)?;
        
        // Merge with default config
        let config_value = serde_json::to_value(&config)
            .map_err(|e| ConfigError::ParseError(format!("Failed to serialize default config: {}", e)))?;
        
        let merged = merge_json_values(config_value, nested_config);
        
        config = serde_json::from_value(merged)
            .map_err(|e| ConfigError::ParseError(format!("Failed to deserialize env config: {}", e)))?;
        
        Ok(config)
    }
    
    fn collect_env_vars(&self) -> HashMap<String, String> {
        env::vars()
            .filter_map(|(key, value)| {
                if key.starts_with(&self.prefix) {
                    let stripped_key = key.strip_prefix(&self.prefix)
                        .unwrap_or(&key)
                        .strip_prefix(&self.separator)
                        .unwrap_or(&key);
                    Some((stripped_key.to_lowercase(), value))
                } else {
                    None
                }
            })
            .collect()
    }
    
    fn parse_nested_env_vars(&self, env_vars: &HashMap<String, String>) -> Result<serde_json::Value, ConfigError> {
        let mut result = serde_json::Map::new();
        
        for (key, value) in env_vars {
            let path_parts: Vec<&str> = key.split(&self.separator).collect();
            self.set_nested_value(&mut result, &path_parts, value)?;
        }
        
        Ok(serde_json::Value::Object(result))
    }
    
    fn set_nested_value(
        &self,
        object: &mut serde_json::Map<String, serde_json::Value>,
        path: &[&str],
        value: &str,
    ) -> Result<(), ConfigError> {
        if path.is_empty() {
            return Ok(());
        }
        
        if path.len() == 1 {
            // Leaf node - parse the value
            let parsed_value = self.parse_env_value(value)?;
            object.insert(path[0].to_string(), parsed_value);
        } else {
            // Intermediate node - create nested object
            let current_key = path[0];
            let remaining_path = &path[1..];
            
            let nested_object = object
                .entry(current_key.to_string())
                .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
            
            if let serde_json::Value::Object(nested_map) = nested_object {
                self.set_nested_value(nested_map, remaining_path, value)?;
            }
        }
        
        Ok(())
    }
    
    fn parse_env_value(&self, value: &str) -> Result<serde_json::Value, ConfigError> {
        // Try to parse as different types
        
        // Boolean
        if let Ok(bool_val) = value.parse::<bool>() {
            return Ok(serde_json::Value::Bool(bool_val));
        }
        
        // Integer
        if let Ok(int_val) = value.parse::<i64>() {
            return Ok(serde_json::Value::Number(serde_json::Number::from(int_val)));
        }
        
        // Float
        if let Ok(float_val) = value.parse::<f64>() {
            if let Some(number) = serde_json::Number::from_f64(float_val) {
                return Ok(serde_json::Value::Number(number));
            }
        }
        
        // Array (comma-separated)
        if value.contains(',') {
            let array_items: Vec<serde_json::Value> = value
                .split(',')
                .map(|item| self.parse_env_value(item.trim()).unwrap_or_else(|_| serde_json::Value::String(item.trim().to_string())))
                .collect();
            return Ok(serde_json::Value::Array(array_items));
        }
        
        // JSON (if starts with { or [)
        if value.starts_with('{') || value.starts_with('[') {
            if let Ok(json_val) = serde_json::from_str(value) {
                return Ok(json_val);
            }
        }
        
        // Default to string
        Ok(serde_json::Value::String(value.to_string()))
    }
}

/// Environment variable examples and documentation
pub mod env_examples {
    //! # Environment Variable Configuration Examples
    //! 
    //! ## Basic Usage
    //! ```bash
    //! export TOADSTOOL_SERVER_PORT=8081
    //! export TOADSTOOL_SERVER_BIND_ADDRESS="0.0.0.0"
    //! export TOADSTOOL_SECURITY_AUTH_ENABLED=true
    //! ```
    //! 
    //! ## Nested Configuration
    //! ```bash
    //! export TOADSTOOL_ECOSYSTEM_SONGBIRD_ENABLED=true
    //! export TOADSTOOL_ECOSYSTEM_SONGBIRD_ENDPOINT="http://songbird:8080"
    //! export TOADSTOOL_RUNTIMES_NATIVE_MAX_CONCURRENT=10
    //! ```
    //! 
    //! ## Array Values
    //! ```bash
    //! export TOADSTOOL_GPU_FRAMEWORKS="cuda,opencl,vulkan"
    //! export TOADSTOOL_SECURITY_ALLOWED_DESTINATIONS="localhost,127.0.0.1,10.0.0.0/8"
    //! ```
    //! 
    //! ## JSON Values
    //! ```bash
    //! export TOADSTOOL_CUSTOM_CONFIG='{"key": "value", "nested": {"setting": true}}'
    //! ```
}

/// Secrets management for sensitive configuration values
pub mod secrets {
    use super::*;
    use ring::rand::{SecureRandom, SystemRandom};
    use base64::Engine;
    use std::future::Future;
    use std::pin::Pin;
    
    /// External secret provider interface (object-safe)
    pub trait SecretProvider: Send + Sync {
        fn get_secret<'a>(&'a self, key: &'a str) -> Pin<Box<dyn Future<Output = Result<String, ConfigError>> + Send + 'a>>;
        fn set_secret<'a>(&'a self, key: &'a str, value: &'a str) -> Pin<Box<dyn Future<Output = Result<(), ConfigError>> + Send + 'a>>;
        fn delete_secret<'a>(&'a self, key: &'a str) -> Pin<Box<dyn Future<Output = Result<(), ConfigError>> + Send + 'a>>;
    }
    
    /// HashiCorp Vault secret provider
    pub struct VaultSecretProvider {
        client: reqwest::Client,
        vault_url: String,
        token: String,
        mount_path: String,
    }
    
    impl VaultSecretProvider {
        pub fn new(vault_url: String, token: String, mount_path: Option<String>) -> Self {
            Self {
                client: reqwest::Client::new(),
                vault_url,
                token,
                mount_path: mount_path.unwrap_or_else(|| "secret".to_string()),
            }
        }
        
        async fn get_secret_impl(&self, key: &str) -> Result<String, ConfigError> {
            let url = format!("{}/v1/{}/data/{}", self.vault_url, self.mount_path, key);
            
            let response = self.client
                .get(&url)
                .header("X-Vault-Token", &self.token)
                .send()
                .await
                .map_err(|e| ConfigError::Network(format!("Vault request failed: {}", e)))?;
            
            if !response.status().is_success() {
                return Err(ConfigError::Other(format!("Vault returned status: {}", response.status())));
            }
            
            let vault_response: serde_json::Value = response.json().await
                .map_err(|e| ConfigError::ParseError(format!("Failed to parse Vault response: {}", e)))?;
            
            vault_response["data"]["data"]["value"]
                .as_str()
                .ok_or_else(|| ConfigError::Other("Secret not found in Vault response".to_string()))
                .map(|s| s.to_string())
        }
    }
    
    impl SecretProvider for VaultSecretProvider {
        fn get_secret<'a>(&'a self, key: &'a str) -> Pin<Box<dyn Future<Output = Result<String, ConfigError>> + Send + 'a>> {
            Box::pin(self.get_secret_impl(key))
        }
        
        fn set_secret<'a>(&'a self, key: &'a str, value: &'a str) -> Pin<Box<dyn Future<Output = Result<(), ConfigError>> + Send + 'a>> {
            Box::pin(async move {
                let url = format!("{}/v1/{}/data/{}", self.vault_url, self.mount_path, key);
                let payload = serde_json::json!({
                    "data": {
                        "value": value
                    }
                });
                
                let response = self.client
                    .post(&url)
                    .header("X-Vault-Token", &self.token)
                    .json(&payload)
                    .send()
                    .await
                    .map_err(|e| ConfigError::Network(format!("Vault request failed: {}", e)))?;
                
                if !response.status().is_success() {
                    return Err(ConfigError::Other(format!("Vault returned status: {}", response.status())));
                }
                
                Ok(())
            })
        }
        
        fn delete_secret<'a>(&'a self, key: &'a str) -> Pin<Box<dyn Future<Output = Result<(), ConfigError>> + Send + 'a>> {
            Box::pin(async move {
                let url = format!("{}/v1/{}/metadata/{}", self.vault_url, self.mount_path, key);
                
                let response = self.client
                    .delete(&url)
                    .header("X-Vault-Token", &self.token)
                    .send()
                    .await
                    .map_err(|e| ConfigError::Network(format!("Vault request failed: {}", e)))?;
                
                if !response.status().is_success() {
                    return Err(ConfigError::Other(format!("Vault returned status: {}", response.status())));
                }
                
                Ok(())
            })
        }
    }
}

/// Configuration migration system for handling version upgrades
pub mod migration {
    use super::*;
    use semver::Version;
    
    /// Configuration version information
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct ConfigVersion {
        pub major: u64,
        pub minor: u64,
        pub patch: u64,
    }
    
    impl ConfigVersion {
        pub fn new(major: u64, minor: u64, patch: u64) -> Self {
            Self { major, minor, patch }
        }
        
        pub fn current() -> Self {
            Self::new(1, 0, 0)
        }
        
        pub fn to_semver(&self) -> Version {
            Version::new(self.major, self.minor, self.patch)
        }
        
        pub fn from_semver(version: &Version) -> Self {
            Self {
                major: version.major,
                minor: version.minor,
                patch: version.patch,
            }
        }
    }
    
    impl Default for ConfigVersion {
        fn default() -> Self {
            Self::current()
        }
    }
    
    impl std::fmt::Display for ConfigVersion {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
        }
    }
    
    /// Versioned configuration wrapper
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct VersionedConfig {
        pub version: ConfigVersion,
        pub config: serde_json::Value,
        pub metadata: ConfigMetadata,
    }
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ConfigMetadata {
        pub created_at: chrono::DateTime<chrono::Utc>,
        pub updated_at: chrono::DateTime<chrono::Utc>,
        pub migration_history: Vec<MigrationRecord>,
        pub checksum: Option<String>,
    }
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct MigrationRecord {
        pub from_version: ConfigVersion,
        pub to_version: ConfigVersion,
        pub migrated_at: chrono::DateTime<chrono::Utc>,
        pub migration_id: String,
    }
    
    /// Migration trait for implementing version-specific migrations
    pub trait ConfigMigration: Send + Sync {
        fn id(&self) -> &str;
        fn from_version(&self) -> ConfigVersion;
        fn to_version(&self) -> ConfigVersion;
        fn migrate(&self, config: serde_json::Value) -> Result<serde_json::Value, ConfigError>;
        fn rollback(&self, config: serde_json::Value) -> Result<serde_json::Value, ConfigError>;
    }
    
    /// Migration manager
    pub struct MigrationManager {
        migrations: Vec<Box<dyn ConfigMigration>>,
    }
    
    impl MigrationManager {
        pub fn new() -> Self {
            Self {
                migrations: Vec::new(),
            }
        }
        
        pub fn add_migration(&mut self, migration: Box<dyn ConfigMigration>) {
            self.migrations.push(migration);
        }
        
        /// Migrate configuration to the latest version
        pub fn migrate_to_latest(&self, versioned_config: VersionedConfig) -> Result<VersionedConfig, ConfigError> {
            let target_version = ConfigVersion::current();
            self.migrate_to_version(versioned_config, target_version)
        }
        
        /// Migrate configuration to a specific version
        pub fn migrate_to_version(&self, mut versioned_config: VersionedConfig, target_version: ConfigVersion) -> Result<VersionedConfig, ConfigError> {
            let current_version = versioned_config.version.clone();
            
            if current_version == target_version {
                return Ok(versioned_config);
            }
            
            // Find migration path
            let migration_path = self.find_migration_path(&current_version, &target_version)?;
            
            // Apply migrations in sequence
            for migration in migration_path {
                let old_version = versioned_config.version.clone();
                versioned_config.config = migration.migrate(versioned_config.config)?;
                versioned_config.version = migration.to_version();
                
                // Record migration
                let migration_record = MigrationRecord {
                    from_version: old_version,
                    to_version: migration.to_version(),
                    migrated_at: chrono::Utc::now(),
                    migration_id: migration.id().to_string(),
                };
                
                versioned_config.metadata.migration_history.push(migration_record);
                versioned_config.metadata.updated_at = chrono::Utc::now();
            }
            
            Ok(versioned_config)
        }
        
        fn find_migration_path(&self, from: &ConfigVersion, to: &ConfigVersion) -> Result<Vec<&dyn ConfigMigration>, ConfigError> {
            // Simple linear migration path for now
            // In a more complex system, this could use graph algorithms
            
            let mut path = Vec::new();
            let mut current_version = from.clone();
            
            while current_version != *to {
                let next_migration = self.migrations.iter()
                    .find(|m| m.from_version() == current_version)
                    .ok_or_else(|| ConfigError::Other(format!("No migration found from version {}", current_version)))?;
                
                path.push(next_migration.as_ref());
                current_version = next_migration.to_version();
                
                // Prevent infinite loops
                if path.len() > 100 {
                    return Err(ConfigError::Other("Migration path too long".to_string()));
                }
            }
            
            Ok(path)
        }
        
        /// Rollback configuration to a previous version
        pub fn rollback_to_version(&self, mut versioned_config: VersionedConfig, target_version: ConfigVersion) -> Result<VersionedConfig, ConfigError> {
            // Find rollback path from migration history
            let rollback_migrations: Vec<_> = versioned_config.metadata.migration_history.iter()
                .rev()
                .take_while(|record| record.to_version != target_version)
                .collect();
            
            for migration_record in rollback_migrations {
                let migration = self.migrations.iter()
                    .find(|m| m.id() == migration_record.migration_id)
                    .ok_or_else(|| ConfigError::Other(format!("Migration {} not found for rollback", migration_record.migration_id)))?;
                
                versioned_config.config = migration.rollback(versioned_config.config)?;
                versioned_config.version = migration.from_version();
            }
            
            // Remove rolled back migrations from history
            versioned_config.metadata.migration_history.retain(|record| {
                record.to_version.to_semver() <= target_version.to_semver()
            });
            
            versioned_config.metadata.updated_at = chrono::Utc::now();
            
            Ok(versioned_config)
        }
    }
    
    /// Example migration: v0.1.0 to v1.0.0
    pub struct V0ToV1Migration;
    
    impl ConfigMigration for V0ToV1Migration {
        fn id(&self) -> &str {
            "v0_to_v1_initial"
        }
        
        fn from_version(&self) -> ConfigVersion {
            ConfigVersion::new(0, 1, 0)
        }
        
        fn to_version(&self) -> ConfigVersion {
            ConfigVersion::new(1, 0, 0)
        }
        
        fn migrate(&self, mut config: serde_json::Value) -> Result<serde_json::Value, ConfigError> {
            // Example migration: rename old fields, add new defaults
            if let Some(obj) = config.as_object_mut() {
                // Rename old security field
                if let Some(old_security) = obj.remove("auth") {
                    obj.insert("security".to_string(), old_security);
                }
                
                // Add new performance section if missing
                if !obj.contains_key("performance") {
                    obj.insert("performance".to_string(), serde_json::json!({
                        "optimization_enabled": false,
                        "monitoring_interval_seconds": 60,
                        "runtime_selection_enabled": true,
                        "resource_prediction_enabled": false,
                        "threshold_percentile": 95.0,
                        "target_utilization_percent": 80.0
                    }));
                }
            }
            
            Ok(config)
        }
        
        fn rollback(&self, mut config: serde_json::Value) -> Result<serde_json::Value, ConfigError> {
            if let Some(obj) = config.as_object_mut() {
                // Reverse the migration
                if let Some(security) = obj.remove("security") {
                    obj.insert("auth".to_string(), security);
                }
                
                obj.remove("performance");
            }
            
            Ok(config)
        }
    }
}

/// Configuration documentation and examples
pub mod documentation {
    use super::*;
    
    /// Generate comprehensive configuration documentation
    pub struct ConfigDocumentationGenerator;
    
    impl ConfigDocumentationGenerator {
        /// Generate markdown documentation for all configuration options
        pub fn generate_markdown() -> String {
            let mut doc = String::new();
            
            doc.push_str("# ToadStool Configuration Reference\n\n");
            doc.push_str("This document provides a comprehensive reference for all ToadStool configuration options.\n\n");
            
            doc.push_str("## Table of Contents\n\n");
            doc.push_str("- [Server Configuration](#server-configuration)\n");
            doc.push_str("- [Runtime Configuration](#runtime-configuration)\n");
            doc.push_str("- [Security Configuration](#security-configuration)\n");
            doc.push_str("- [Monitoring Configuration](#monitoring-configuration)\n");
            doc.push_str("- [Federation Configuration](#federation-configuration)\n");
            doc.push_str("- [Ecosystem Configuration](#ecosystem-configuration)\n");
            doc.push_str("- [Performance Configuration](#performance-configuration)\n");
            doc.push_str("- [Environment Variables](#environment-variables)\n");
            doc.push_str("- [Configuration Profiles](#configuration-profiles)\n");
            doc.push_str("- [Secrets Management](#secrets-management)\n\n");
            
            doc.push_str(&Self::generate_server_docs());
            doc.push_str(&Self::generate_runtime_docs());
            doc.push_str(&Self::generate_security_docs());
            doc.push_str(&Self::generate_monitoring_docs());
            doc.push_str(&Self::generate_federation_docs());
            doc.push_str(&Self::generate_ecosystem_docs());
            doc.push_str(&Self::generate_performance_docs());
            doc.push_str(&Self::generate_env_vars_docs());
            doc.push_str(&Self::generate_profiles_docs());
            doc.push_str(&Self::generate_secrets_docs());
            
            doc
        }
        
        fn generate_server_docs() -> String {
            format!(r#"## Server Configuration

The server configuration controls how ToadStool's API server operates.

### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `bind_address` | String | `"{}"` | IP address to bind the server to |
| `port` | u16 | `{}` | Port number for the API server |
| `max_connections` | usize | `{}` | Maximum concurrent connections |
| `request_timeout_seconds` | u64 | `30` | Request timeout in seconds |
| `tls_enabled` | bool | `false` | Enable TLS/HTTPS |
| `tls_cert_path` | Option<PathBuf> | `None` | Path to TLS certificate |
| `tls_key_path` | Option<PathBuf> | `None` | Path to TLS private key |

### Example Configuration

```yaml
server:
  bind_address: "0.0.0.0"
  port: 8081
  max_connections: 1000
  request_timeout_seconds: 30
  tls_enabled: true
  tls_cert_path: "/etc/toadstool/cert.pem"
  tls_key_path: "/etc/toadstool/key.pem"
```

### Environment Variables

```bash
export TOADSTOOL_SERVER_BIND_ADDRESS="0.0.0.0"
export TOADSTOOL_SERVER_PORT=8081
export TOADSTOOL_SERVER_TLS_ENABLED=true
```

"#,
                constants::network::DEFAULT_BIND_ADDRESS,
                constants::network::DEFAULT_TOADSTOOL_PORT,
                constants::network::DEFAULT_MAX_CONNECTIONS
            )
        }
        
        fn generate_runtime_docs() -> String {
            r#"## Runtime Configuration

Configure the various runtime engines available in ToadStool.

### Native Runtime

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | bool | `true` | Enable native runtime |
| `max_concurrent` | usize | `10` | Maximum concurrent executions |
| `timeout_seconds` | u64 | `300` | Execution timeout |

### Container Runtime

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | bool | `true` | Enable container runtime |
| `runtime` | String | `"docker"` | Container runtime (docker, podman) |
| `max_concurrent` | usize | `5` | Maximum concurrent containers |
| `default_image` | String | `"alpine:latest"` | Default container image |

### WASM Runtime

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | bool | `true` | Enable WASM runtime |
| `max_concurrent` | usize | `20` | Maximum concurrent instances |
| `memory_limit_mb` | usize | `128` | Memory limit per instance |
| `wasi_enabled` | bool | `true` | Enable WASI support |

### GPU Runtime

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | bool | `false` | Enable GPU runtime |
| `frameworks` | Vec<String> | `["cuda"]` | GPU frameworks |
| `max_concurrent` | usize | `2` | Maximum concurrent jobs |
| `memory_limit_mb` | usize | `1024` | GPU memory limit |

### Example Configuration

```yaml
runtimes:
  native:
    enabled: true
    max_concurrent: 10
    timeout_seconds: 300
  container:
    enabled: true
    runtime: "docker"
    max_concurrent: 5
    default_image: "alpine:latest"
  wasm:
    enabled: true
    max_concurrent: 20
    memory_limit_mb: 128
    wasi_enabled: true
  gpu:
    enabled: false
    frameworks: ["cuda", "opencl"]
    max_concurrent: 2
    memory_limit_mb: 2048
```

"#.to_string()
        }
        
        fn generate_security_docs() -> String {
            r#"## Security Configuration

Configure authentication, authorization, and sandboxing.

### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `auth_enabled` | bool | `false` | Enable authentication |
| `auth_method` | String | `"bearer"` | Authentication method |
| `jwt_secret` | Option<String> | `None` | JWT signing secret |
| `api_key` | Option<String> | `None` | API key for simple auth |
| `sandbox_enabled` | bool | `false` | Enable sandboxing |
| `sandbox_type` | String | `"chroot"` | Sandbox type |
| `network_isolation` | bool | `false` | Enable network isolation |
| `allowed_destinations` | Vec<String> | `[]` | Allowed network destinations |

### Example Configuration

```yaml
security:
  auth_enabled: true
  auth_method: "jwt"
  jwt_secret: "${TOADSTOOL_JWT_SECRET}"
  sandbox_enabled: true
  sandbox_type: "docker"
  network_isolation: true
  allowed_destinations:
    - "localhost"
    - "127.0.0.1"
    - "10.0.0.0/8"
```

### Security Best Practices

1. **Always enable authentication in production**
2. **Use strong, randomly generated secrets**
3. **Enable sandboxing for untrusted code**
4. **Restrict network access with allow-lists**
5. **Use TLS for all external communications**

"#.to_string()
        }
        
        fn generate_monitoring_docs() -> String {
            r#"## Monitoring Configuration

Configure metrics collection and monitoring.

### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `metrics_enabled` | bool | `true` | Enable metrics collection |
| `metrics_interval_seconds` | u64 | `60` | Metrics collection interval |
| `realtime_enabled` | bool | `false` | Enable real-time monitoring |
| `retention_hours` | u64 | `24` | Metrics retention period |
| `profiling_enabled` | bool | `false` | Enable performance profiling |
| `export_enabled` | bool | `false` | Export metrics externally |
| `export_endpoints` | Vec<String> | `[]` | Export endpoints |

### Example Configuration

```yaml
monitoring:
  metrics_enabled: true
  metrics_interval_seconds: 30
  realtime_enabled: true
  retention_hours: 72
  profiling_enabled: true
  export_enabled: true
  export_endpoints:
    - "http://prometheus:9090/api/v1/write"
    - "http://grafana-agent:3100/loki/api/v1/push"
```

"#.to_string()
        }
        
        fn generate_federation_docs() -> String {
            r#"## Federation Configuration

Configure federation with other ToadStool instances.

### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | bool | `false` | Enable federation |
| `discovery_method` | String | `"dns"` | Discovery method |
| `static_peers` | Vec<String> | `[]` | Static federation peers |
| `port` | u16 | `8084` | Federation port |
| `encryption_enabled` | bool | `true` | Enable encryption |
| `auth_key` | Option<String> | `None` | Federation auth key |

### Example Configuration

```yaml
federation:
  enabled: true
  discovery_method: "static"
  static_peers:
    - "toadstool-1.example.com:8084"
    - "toadstool-2.example.com:8084"
  port: 8084
  encryption_enabled: true
  auth_key: "${TOADSTOOL_FEDERATION_KEY}"
```

"#.to_string()
        }
        
        fn generate_ecosystem_docs() -> String {
            format!(r#"## Ecosystem Configuration

Configure integration with the broader ecosystem.

### Songbird Configuration

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | bool | `false` | Enable Songbird integration |
| `endpoint` | String | `"http://localhost:{}"` | Songbird endpoint |
| `timeout_seconds` | u64 | `30` | Connection timeout |
| `load_balancing` | bool | `false` | Enable load balancing |
| `auth_token` | Option<String> | `None` | Authentication token |

### BearDog Configuration

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | bool | `false` | Enable BearDog integration |
| `endpoint` | String | `"http://localhost:{}"` | BearDog endpoint |
| `security_level` | String | `"medium"` | Security level |
| `crypto_lock_enabled` | bool | `false` | Enable crypto lock |
| `auth_key` | Option<String> | `None` | Authentication key |

### NestGate Configuration

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | bool | `false` | Enable NestGate integration |
| `endpoint` | String | `"http://localhost:{}"` | NestGate endpoint |
| `storage_tier` | String | `"hot"` | Storage tier |
| `distributed_enabled` | bool | `false` | Enable distributed storage |
| `auth_token` | Option<String> | `None` | Authentication token |

### Example Configuration

```yaml
ecosystem:
  songbird:
    enabled: true
    endpoint: "http://songbird.local:8080"
    load_balancing: true
    auth_token: "${{SONGBIRD_TOKEN}}"
  beardog:
    enabled: true
    endpoint: "http://beardog.local:8082"
    security_level: "high"
    crypto_lock_enabled: true
  nestgate:
    enabled: true
    endpoint: "http://nestgate.local:8083"
    storage_tier: "warm"
    distributed_enabled: true
```

"#,
                constants::network::DEFAULT_SONGBIRD_PORT,
                constants::network::DEFAULT_BEARDOG_PORT,
                constants::network::DEFAULT_NESTGATE_PORT
            )
        }
        
        fn generate_performance_docs() -> String {
            r#"## Performance Configuration

Configure performance optimization settings.

### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `optimization_enabled` | bool | `true` | Enable performance optimization |
| `monitoring_interval_seconds` | u64 | `60` | Performance monitoring interval |
| `runtime_selection_enabled` | bool | `true` | Enable intelligent runtime selection |
| `resource_prediction_enabled` | bool | `false` | Enable resource prediction |
| `threshold_percentile` | f64 | `95.0` | Performance threshold percentile |
| `target_utilization_percent` | f64 | `80.0` | Target resource utilization |

### Example Configuration

```yaml
performance:
  optimization_enabled: true
  monitoring_interval_seconds: 30
  runtime_selection_enabled: true
  resource_prediction_enabled: true
  threshold_percentile: 99.0
  target_utilization_percent: 85.0
```

"#.to_string()
        }
        
        fn generate_env_vars_docs() -> String {
            r#"## Environment Variables

All configuration options can be overridden using environment variables with the `TOADSTOOL_` prefix.

### Naming Convention

Environment variables follow the pattern: `TOADSTOOL_<SECTION>_<OPTION>`

Examples:
- `TOADSTOOL_SERVER_PORT=8081`
- `TOADSTOOL_SECURITY_AUTH_ENABLED=true`
- `TOADSTOOL_RUNTIMES_NATIVE_MAX_CONCURRENT=20`

### Nested Configuration

For nested configuration, use underscores to separate levels:
- `TOADSTOOL_ECOSYSTEM_SONGBIRD_ENABLED=true`
- `TOADSTOOL_ECOSYSTEM_SONGBIRD_ENDPOINT="http://songbird:8080"`

### Array Values

Arrays can be specified as comma-separated values:
- `TOADSTOOL_GPU_FRAMEWORKS="cuda,opencl,vulkan"`
- `TOADSTOOL_SECURITY_ALLOWED_DESTINATIONS="localhost,127.0.0.1"`

### JSON Values

Complex values can be specified as JSON:
- `TOADSTOOL_CUSTOM_CONFIG='{"key": "value", "nested": {"setting": true}}'`

"#.to_string()
        }
        
        fn generate_profiles_docs() -> String {
            r#"## Configuration Profiles

ToadStool supports configuration profiles for different environments.

### Available Profiles

- **Development**: Optimized for development with debugging enabled
- **Staging**: Production-like environment for testing
- **Production**: Optimized for production with security enabled
- **Testing**: Minimal configuration for automated testing

### Profile-Specific Defaults

#### Development Profile
```yaml
security:
  auth_enabled: false
  sandbox_enabled: false
monitoring:
  metrics_enabled: true
  profiling_enabled: true
performance:
  optimization_enabled: false
```

#### Production Profile
```yaml
security:
  auth_enabled: true
  sandbox_enabled: true
server:
  tls_enabled: true
monitoring:
  metrics_enabled: true
  profiling_enabled: false
performance:
  optimization_enabled: true
```

### Using Profiles

Set the profile using the `TOADSTOOL_PROFILE` environment variable:
```bash
export TOADSTOOL_PROFILE=production
```

Or specify in configuration:
```yaml
profile: production
```

"#.to_string()
        }
        
        fn generate_secrets_docs() -> String {
            r#"## Secrets Management

ToadStool provides secure secrets management for sensitive configuration values.

### Secret Providers

#### HashiCorp Vault
```yaml
secrets:
  vault:
    url: "https://vault.example.com"
    token: "${VAULT_TOKEN}"
    mount_path: "secret"
```

#### Environment Variables
```bash
export TOADSTOOL_JWT_SECRET="your-secret-here"
export TOADSTOOL_API_KEY="your-api-key"
```

### Encrypted Secrets

Secrets can be encrypted in configuration files:
```yaml
security:
  jwt_secret: "encrypted:base64encodedvalue"
  api_key: "encrypted:anotherencryptedvalue"
```

### Best Practices

1. **Never commit secrets to version control**
2. **Use external secret providers in production**
3. **Rotate secrets regularly**
4. **Use different secrets for different environments**
5. **Monitor secret access and usage**

### Secret Rotation

ToadStool supports hot-reload of secrets without restart:
```bash
# Update secret in Vault
vault kv put secret/toadstool jwt_secret="new-secret"

# ToadStool will automatically reload the secret
```

"#.to_string()
        }
        
        /// Generate JSON schema for configuration validation
        pub fn generate_json_schema() -> serde_json::Value {
            // This would generate a JSON schema for the ToadStoolConfig
            // In a real implementation, you might use the `schemars` crate
            serde_json::json!({
                "$schema": "http://json-schema.org/draft-07/schema#",
                "title": "ToadStool Configuration",
                "type": "object",
                "properties": {
                    "server": {
                        "type": "object",
                        "properties": {
                            "bind_address": {"type": "string", "format": "ipv4"},
                            "port": {"type": "integer", "minimum": 1024, "maximum": 65535},
                            "max_connections": {"type": "integer", "minimum": 1},
                            "request_timeout_seconds": {"type": "integer", "minimum": 1},
                            "tls_enabled": {"type": "boolean"},
                            "tls_cert_path": {"type": ["string", "null"]},
                            "tls_key_path": {"type": ["string", "null"]}
                        },
                        "required": ["bind_address", "port"]
                    }
                    // ... more schema definitions
                }
            })
        }
        
        /// Generate example configurations for different use cases
        pub fn generate_examples() -> HashMap<String, String> {
            let mut examples = HashMap::new();
            
            examples.insert("minimal".to_string(), r#"# Minimal ToadStool Configuration
server:
  bind_address: "127.0.0.1"
  port: 8081

runtimes:
  native:
    enabled: true
"#.to_string());
            
            examples.insert("production".to_string(), r#"# Production ToadStool Configuration
profile: production

server:
  bind_address: "0.0.0.0"
  port: 8081
  max_connections: 1000
  tls_enabled: true
  tls_cert_path: "/etc/toadstool/cert.pem"
  tls_key_path: "/etc/toadstool/key.pem"

security:
  auth_enabled: true
  auth_method: "jwt"
  jwt_secret: "${TOADSTOOL_JWT_SECRET}"
  sandbox_enabled: true
  network_isolation: true

monitoring:
  metrics_enabled: true
  export_enabled: true
  export_endpoints:
    - "http://prometheus:9090/api/v1/write"

ecosystem:
  songbird:
    enabled: true
    endpoint: "https://songbird.example.com"
  beardog:
    enabled: true
    endpoint: "https://beardog.example.com"
    security_level: "high"
"#.to_string());
            
            examples.insert("development".to_string(), r#"# Development ToadStool Configuration
profile: development

server:
  bind_address: "127.0.0.1"
  port: 8081

security:
  auth_enabled: false
  sandbox_enabled: false

monitoring:
  metrics_enabled: true
  profiling_enabled: true

performance:
  optimization_enabled: false
"#.to_string());
            
            examples
        }
    }
}
