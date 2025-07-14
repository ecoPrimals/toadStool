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

//! Runtime defaults and configuration loading for ToadStool
//!
//! This module provides runtime configuration loading that integrates with
//! the centralized configuration system and Songbird port orchestration.

use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::{ConfigError, ConfigResult, ToadStoolConfig};

/// Runtime configuration loader that integrates with centralized config
#[derive(Debug, Clone)]
pub struct RuntimeConfigLoader {
    config_paths: Vec<PathBuf>,
    _env_prefix: String,
    config_cache: std::sync::Arc<RwLock<Option<ToadStoolConfig>>>,
}

impl RuntimeConfigLoader {
    /// Create a new runtime config loader
    pub fn new() -> Self {
        Self {
            config_paths: get_standard_config_paths(),
            _env_prefix: "TOADSTOOL".to_string(),
            config_cache: std::sync::Arc::new(RwLock::new(None)),
        }
    }

    /// Load configuration from all sources
    pub async fn load_config(&self) -> ConfigResult<ToadStoolConfig> {
        // Check cache first
        if let Some(cached_config) = self.config_cache.read().await.as_ref() {
            return Ok(cached_config.clone());
        }

        let mut config = ToadStoolConfig::default();

        // Load from configuration files
        for config_path in &self.config_paths {
            if config_path.exists() {
                match self.load_config_file(config_path) {
                    Ok(file_config) => {
                        config = merge_configs(config, file_config);
                        info!("Loaded configuration from: {}", config_path.display());
                    }
                    Err(e) => {
                        warn!(
                            "Failed to load config from {}: {}",
                            config_path.display(),
                            e
                        );
                    }
                }
            }
        }

        // Load from environment variables
        config = self.load_from_environment(config)?;

        // Load from Songbird service discovery (if enabled)
        if config.network.songbird_orchestration.enabled {
            match self.load_from_songbird(&config).await {
                Ok(songbird_config) => {
                    config = merge_configs(config, songbird_config);
                    info!("Loaded configuration from Songbird service discovery");
                }
                Err(e) => {
                    warn!("Failed to load config from Songbird: {}", e);
                }
            }
        }

        // Validate configuration
        self.validate_config(&config)?;

        // Cache the configuration
        *self.config_cache.write().await = Some(config.clone());

        Ok(config)
    }

    /// Load configuration from a specific file
    fn load_config_file(&self, path: &PathBuf) -> ConfigResult<ToadStoolConfig> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::Loading(format!("Failed to read config file: {e}")))?;

        let config = match path.extension().and_then(|ext| ext.to_str()) {
            Some("toml") => toml::from_str(&content)
                .map_err(|e| ConfigError::Loading(format!("Failed to parse TOML: {e}")))?,
            Some("yaml") | Some("yml") => serde_yaml::from_str(&content)
                .map_err(|e| ConfigError::Loading(format!("Failed to parse YAML: {e}")))?,
            Some("json") => serde_json::from_str(&content)
                .map_err(|e| ConfigError::Loading(format!("Failed to parse JSON: {e}")))?,
            _ => {
                return Err(ConfigError::Loading(
                    "Unsupported config file format. Use .toml, .yaml, .yml, or .json".to_string(),
                ))
            }
        };

        Ok(config)
    }

    /// Load configuration from environment variables
    fn load_from_environment(&self, mut config: ToadStoolConfig) -> ConfigResult<ToadStoolConfig> {
        // Environment name
        if let Ok(env_name) = env::var("TOADSTOOL_ENV") {
            config.environment.name = env_name;
        }

        // Debug and verbose modes
        if let Ok(debug) = env::var("TOADSTOOL_DEBUG") {
            config.environment.debug = debug.to_lowercase() == "true";
        }

        if let Ok(verbose) = env::var("TOADSTOOL_VERBOSE") {
            config.environment.verbose = verbose.to_lowercase() == "true";
        }

        // Network configuration
        if let Ok(bind_address) = env::var("TOADSTOOL_BIND_ADDRESS") {
            config.network.bind_address = bind_address;
        }

        if let Ok(port) = env::var("TOADSTOOL_PORT") {
            config.network.port = port
                .parse()
                .map_err(|e| ConfigError::Environment(format!("Invalid port: {e}")))?;
        }

        // Songbird configuration
        if let Ok(songbird_endpoint) = env::var("TOADSTOOL_SONGBIRD_ENDPOINT") {
            config.network.songbird_orchestration.endpoint = songbird_endpoint;
        }

        if let Ok(songbird_enabled) = env::var("TOADSTOOL_SONGBIRD_ORCHESTRATION_ENABLED") {
            config.network.songbird_orchestration.enabled =
                songbird_enabled.to_lowercase() == "true";
        }

        // Resource limits
        if let Ok(max_cpu) = env::var("TOADSTOOL_MAX_CPU_PERCENT") {
            config.resources.limits.max_cpu_percent = max_cpu
                .parse()
                .map_err(|e| ConfigError::Environment(format!("Invalid CPU limit: {e}")))?;
        }

        if let Ok(max_memory) = env::var("TOADSTOOL_MAX_MEMORY_BYTES") {
            config.resources.limits.max_memory_bytes = max_memory
                .parse()
                .map_err(|e| ConfigError::Environment(format!("Invalid memory limit: {e}")))?;
        }

        // Runtime engines
        if let Ok(native_enabled) = env::var("TOADSTOOL_NATIVE_RUNTIME_ENABLED") {
            config.runtime.engines.native.enabled = native_enabled.to_lowercase() == "true";
        }

        if let Ok(container_enabled) = env::var("TOADSTOOL_CONTAINER_RUNTIME_ENABLED") {
            config.runtime.engines.container.enabled = container_enabled.to_lowercase() == "true";
        }

        if let Ok(wasm_enabled) = env::var("TOADSTOOL_WASM_RUNTIME_ENABLED") {
            config.runtime.engines.wasm.enabled = wasm_enabled.to_lowercase() == "true";
        }

        if let Ok(gpu_enabled) = env::var("TOADSTOOL_GPU_RUNTIME_ENABLED") {
            config.runtime.engines.gpu.enabled = gpu_enabled.to_lowercase() == "true";
        }

        debug!("Loaded configuration from environment variables");
        Ok(config)
    }

    /// Load configuration from Songbird service discovery
    async fn load_from_songbird(
        &self,
        base_config: &ToadStoolConfig,
    ) -> ConfigResult<ToadStoolConfig> {
        let client = reqwest::Client::new();
        let discovery_url = format!(
            "{}/api/v1/discovery",
            base_config.network.songbird_orchestration.endpoint
        );

        let response = client
            .get(&discovery_url)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| {
                ConfigError::SongbirdIntegration(format!("Failed to connect to Songbird: {e}"))
            })?;

        if !response.status().is_success() {
            return Err(ConfigError::SongbirdIntegration(format!(
                "Songbird returned error: {}",
                response.status()
            )));
        }

        let discovery_data: serde_json::Value = response.json().await.map_err(|e| {
            ConfigError::SongbirdIntegration(format!("Failed to parse Songbird response: {e}"))
        })?;

        // Extract configuration from Songbird response
        let mut config = base_config.clone();

        // Update service endpoints from discovery
        if let Some(services) = discovery_data.get("services").and_then(|s| s.as_array()) {
            for service in services {
                if let Some(service_name) = service.get("name").and_then(|n| n.as_str()) {
                    if let Some(endpoint) = service.get("endpoint").and_then(|e| e.as_str()) {
                        match service_name {
                            "songbird" => {
                                config.ecosystem.primals.songbird.endpoint = endpoint.to_string();
                            }
                            "beardog" => {
                                config.ecosystem.primals.beardog.endpoint = endpoint.to_string();
                            }
                            "nestgate" => {
                                config.ecosystem.primals.nestgate.endpoint = endpoint.to_string();
                            }
                            "squirrel" => {
                                config.ecosystem.primals.squirrel.endpoint = endpoint.to_string();
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        // Update port allocation if provided
        if let Some(port_allocation) = discovery_data.get("port_allocation") {
            if let Some(toadstool_port) = port_allocation.get("toadstool").and_then(|p| p.as_u64())
            {
                config.network.port = toadstool_port as u16;
            }
        }

        Ok(config)
    }

    /// Validate configuration
    fn validate_config(&self, config: &ToadStoolConfig) -> ConfigResult<()> {
        // Validate port ranges
        if config.network.port == 0 {
            return Err(ConfigError::Validation("Port cannot be 0".to_string()));
        }

        // Validate Songbird orchestration settings
        if config.network.songbird_orchestration.enabled {
            if config.network.songbird_orchestration.endpoint.is_empty() {
                return Err(ConfigError::Validation(
                    "Songbird endpoint cannot be empty when orchestration is enabled".to_string(),
                ));
            }

            let port_range = &config.network.songbird_orchestration.dynamic_port_range;
            if port_range.start >= port_range.end {
                return Err(ConfigError::Validation(
                    "Invalid port range: start must be less than end".to_string(),
                ));
            }
        }

        // Validate resource limits
        if config.resources.limits.max_cpu_percent <= 0.0
            || config.resources.limits.max_cpu_percent > 100.0
        {
            return Err(ConfigError::Validation(
                "CPU percentage must be between 0 and 100".to_string(),
            ));
        }

        if config.resources.limits.max_memory_bytes == 0 {
            return Err(ConfigError::Validation(
                "Memory limit cannot be 0".to_string(),
            ));
        }

        // Validate runtime engines
        let enabled_engines = [
            config.runtime.engines.native.enabled,
            config.runtime.engines.container.enabled,
            config.runtime.engines.wasm.enabled,
            config.runtime.engines.gpu.enabled,
        ];

        if !enabled_engines.iter().any(|&enabled| enabled) {
            return Err(ConfigError::Validation(
                "At least one runtime engine must be enabled".to_string(),
            ));
        }

        debug!("Configuration validation passed");
        Ok(())
    }

    /// Clear configuration cache
    pub async fn clear_cache(&self) {
        *self.config_cache.write().await = None;
    }

    /// Reload configuration (clears cache and loads fresh)
    pub async fn reload_config(&self) -> ConfigResult<ToadStoolConfig> {
        self.clear_cache().await;
        self.load_config().await
    }
}

impl Default for RuntimeConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Get standard configuration file paths
fn get_standard_config_paths() -> Vec<PathBuf> {
    let mut paths = vec![
        PathBuf::from("toadstool.toml"),
        PathBuf::from("toadstool.yaml"),
        PathBuf::from("toadstool.yml"),
        PathBuf::from("toadstool.json"),
    ];

    // Configuration directory
    if let Ok(config_dir) = env::var("TOADSTOOL_CONFIG_DIR") {
        let config_dir = PathBuf::from(config_dir);
        paths.push(config_dir.join("toadstool.toml"));
        paths.push(config_dir.join("toadstool.yaml"));
        paths.push(config_dir.join("toadstool.yml"));
        paths.push(config_dir.join("toadstool.json"));
    }

    // User configuration directory
    if let Ok(home_dir) = env::var("HOME") {
        let home_dir = PathBuf::from(home_dir);
        paths.push(home_dir.join(".config/toadstool/toadstool.toml"));
        paths.push(home_dir.join(".config/toadstool/toadstool.yaml"));
        paths.push(home_dir.join(".config/toadstool/toadstool.yml"));
        paths.push(home_dir.join(".config/toadstool/toadstool.json"));
    }

    // System configuration directory
    if cfg!(unix) {
        paths.push(PathBuf::from("/etc/toadstool/toadstool.toml"));
        paths.push(PathBuf::from("/etc/toadstool/toadstool.yaml"));
        paths.push(PathBuf::from("/etc/toadstool/toadstool.yml"));
        paths.push(PathBuf::from("/etc/toadstool/toadstool.json"));
    }

    paths
}

/// Merge two configurations, with the second taking precedence
fn merge_configs(_base: ToadStoolConfig, override_config: ToadStoolConfig) -> ToadStoolConfig {
    // For now, we'll use the override config entirely
    // In a more sophisticated implementation, we would merge field by field
    override_config
}

/// Environment variable mappings for configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentMappings {
    pub mappings: HashMap<String, String>,
}

impl EnvironmentMappings {
    /// Create default environment variable mappings
    pub fn default_mappings() -> Self {
        let mut mappings = HashMap::new();

        // Environment
        mappings.insert("TOADSTOOL_ENV".to_string(), "environment.name".to_string());
        mappings.insert(
            "TOADSTOOL_DEBUG".to_string(),
            "environment.debug".to_string(),
        );
        mappings.insert(
            "TOADSTOOL_VERBOSE".to_string(),
            "environment.verbose".to_string(),
        );

        // Network
        mappings.insert(
            "TOADSTOOL_BIND_ADDRESS".to_string(),
            "network.bind_address".to_string(),
        );
        mappings.insert("TOADSTOOL_PORT".to_string(), "network.port".to_string());

        // Songbird
        mappings.insert(
            "TOADSTOOL_SONGBIRD_ENDPOINT".to_string(),
            "network.songbird_orchestration.endpoint".to_string(),
        );
        mappings.insert(
            "TOADSTOOL_SONGBIRD_ORCHESTRATION_ENABLED".to_string(),
            "network.songbird_orchestration.enabled".to_string(),
        );

        // Resources
        mappings.insert(
            "TOADSTOOL_MAX_CPU_PERCENT".to_string(),
            "resources.limits.max_cpu_percent".to_string(),
        );
        mappings.insert(
            "TOADSTOOL_MAX_MEMORY_BYTES".to_string(),
            "resources.limits.max_memory_bytes".to_string(),
        );
        mappings.insert(
            "TOADSTOOL_MAX_STORAGE_BYTES".to_string(),
            "resources.limits.max_storage_bytes".to_string(),
        );

        // Runtime Engines
        mappings.insert(
            "TOADSTOOL_NATIVE_RUNTIME_ENABLED".to_string(),
            "runtime.engines.native.enabled".to_string(),
        );
        mappings.insert(
            "TOADSTOOL_CONTAINER_RUNTIME_ENABLED".to_string(),
            "runtime.engines.container.enabled".to_string(),
        );
        mappings.insert(
            "TOADSTOOL_WASM_RUNTIME_ENABLED".to_string(),
            "runtime.engines.wasm.enabled".to_string(),
        );
        mappings.insert(
            "TOADSTOOL_GPU_RUNTIME_ENABLED".to_string(),
            "runtime.engines.gpu.enabled".to_string(),
        );

        // Security
        mappings.insert(
            "TOADSTOOL_SECURITY_ISOLATION_LEVEL".to_string(),
            "security.isolation_level".to_string(),
        );
        mappings.insert(
            "TOADSTOOL_SECURITY_SANDBOXING_ENABLED".to_string(),
            "security.sandboxing.enabled".to_string(),
        );

        // Monitoring
        mappings.insert(
            "TOADSTOOL_MONITORING_ENABLED".to_string(),
            "monitoring.enabled".to_string(),
        );
        mappings.insert(
            "TOADSTOOL_METRICS_COLLECTION_ENABLED".to_string(),
            "monitoring.metrics_collection.enabled".to_string(),
        );

        Self { mappings }
    }
}

/// Global configuration instance
static CONFIG_LOADER: std::sync::OnceLock<RuntimeConfigLoader> = std::sync::OnceLock::new();

/// Get global configuration loader
pub fn get_config_loader() -> &'static RuntimeConfigLoader {
    CONFIG_LOADER.get_or_init(RuntimeConfigLoader::new)
}

/// Load global configuration
pub async fn load_global_config() -> ConfigResult<ToadStoolConfig> {
    get_config_loader().load_config().await
}

/// Reload global configuration
pub async fn reload_global_config() -> ConfigResult<ToadStoolConfig> {
    get_config_loader().reload_config().await
}

/// Port orchestration utilities
pub mod port_orchestration {
    use super::*;

    /// Request port allocation from Songbird
    pub async fn request_port_allocation(
        config: &ToadStoolConfig,
        service_name: &str,
        preferred_port: Option<u16>,
    ) -> ConfigResult<u16> {
        if !config.network.songbird_orchestration.enabled {
            return preferred_port.ok_or_else(|| {
                ConfigError::PortOrchestration(
                    "Songbird orchestration disabled and no preferred port".to_string(),
                )
            });
        }

        let client = reqwest::Client::new();
        let request_url = format!(
            "{}/api/v1/port-allocation",
            config.network.songbird_orchestration.endpoint
        );

        let request_body = serde_json::json!({
            "service_name": service_name,
            "preferred_port": preferred_port,
            "port_range": {
                "start": config.network.songbird_orchestration.dynamic_port_range.start,
                "end": config.network.songbird_orchestration.dynamic_port_range.end
            }
        });

        let response = client
            .post(&request_url)
            .json(&request_body)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| {
                ConfigError::PortOrchestration(format!("Failed to request port allocation: {e}"))
            })?;

        if !response.status().is_success() {
            return Err(ConfigError::PortOrchestration(format!(
                "Port allocation request failed: {}",
                response.status()
            )));
        }

        let response_data: serde_json::Value = response.json().await.map_err(|e| {
            ConfigError::PortOrchestration(format!("Failed to parse port allocation response: {e}"))
        })?;

        let allocated_port = response_data
            .get("allocated_port")
            .and_then(|p| p.as_u64())
            .ok_or_else(|| {
                ConfigError::PortOrchestration("No allocated port in response".to_string())
            })?;

        Ok(allocated_port as u16)
    }

    /// Release port allocation to Songbird
    pub async fn release_port_allocation(
        config: &ToadStoolConfig,
        service_name: &str,
        port: u16,
    ) -> ConfigResult<()> {
        if !config.network.songbird_orchestration.enabled {
            return Ok(());
        }

        let client = reqwest::Client::new();
        let request_url = format!(
            "{}/api/v1/port-allocation/{}",
            config.network.songbird_orchestration.endpoint, port
        );

        let request_body = serde_json::json!({
            "service_name": service_name,
            "port": port
        });

        let response = client
            .delete(&request_url)
            .json(&request_body)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| {
                ConfigError::PortOrchestration(format!("Failed to release port allocation: {e}"))
            })?;

        if !response.status().is_success() {
            warn!(
                "Failed to release port allocation for {}: {}",
                service_name,
                response.status()
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_config_loader_creation() {
        let loader = RuntimeConfigLoader::new();
        assert!(!loader.config_paths.is_empty());
        assert_eq!(loader._env_prefix, "TOADSTOOL");
    }

    #[tokio::test]
    async fn test_default_config_loading() {
        let loader = RuntimeConfigLoader::new();
        let config = loader.load_config().await.unwrap();

        assert!(!config.network.bind_address.is_empty());
        assert!(config.network.port > 0);
    }

    #[tokio::test]
    async fn test_config_validation() {
        let loader = RuntimeConfigLoader::new();
        let mut config = ToadStoolConfig::default();

        // Test valid configuration
        assert!(loader.validate_config(&config).is_ok());

        // Test invalid port
        config.network.port = 0;
        assert!(loader.validate_config(&config).is_err());

        // Test invalid CPU percentage
        config.network.port = 8080;
        config.resources.limits.max_cpu_percent = 150.0;
        assert!(loader.validate_config(&config).is_err());
    }

    #[tokio::test]
    async fn test_config_file_loading() {
        let temp_dir = tempdir().unwrap();
        let config_file = temp_dir.path().join("test.toml");

        let config_content = r#"
[environment]
name = "test"
debug = true

[network]
bind_address = "127.0.0.1"
port = 9999
"#;

        fs::write(&config_file, config_content).unwrap();

        let loader = RuntimeConfigLoader::new();
        let config = loader.load_config_file(&config_file).unwrap();

        assert_eq!(config.environment.name, "test");
        assert!(config.environment.debug);
        assert_eq!(config.network.bind_address, "127.0.0.1");
        assert_eq!(config.network.port, 9999);
    }

    #[test]
    fn test_environment_mappings() {
        let mappings = EnvironmentMappings::default_mappings();
        assert!(mappings.mappings.contains_key("TOADSTOOL_ENV"));
        assert!(mappings.mappings.contains_key("TOADSTOOL_BIND_ADDRESS"));
        assert!(mappings
            .mappings
            .contains_key("TOADSTOOL_SONGBIRD_ENDPOINT"));
    }

    #[test]
    fn test_standard_config_paths() {
        let paths = get_standard_config_paths();
        assert!(!paths.is_empty());
        assert!(paths.iter().any(|p| p
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("toadstool")));
    }
}
