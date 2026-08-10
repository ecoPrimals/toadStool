// SPDX-License-Identifier: AGPL-3.0-or-later
//! Configuration manager for applying ToadStool configuration to the system

use std::path::PathBuf;

use std::fs;
use tracing::info;

use toadstool_config::ToadStoolConfig;

use super::paths::config_path_for_platform;
use crate::ToadStoolError;
use toadstool_common::platform_paths::Platform;

/// Configuration manager for applying configurations
#[derive(Debug, Clone)]
pub struct ConfigManager {
    config_path: PathBuf,
}

impl ConfigManager {
    /// Creates a new config manager with platform-specific config path.
    #[must_use]
    pub fn new() -> Self {
        let platform = Platform::detect();
        let config_path = config_path_for_platform(platform);
        Self { config_path }
    }

    /// Create a `ConfigManager` with a custom path (for testing).
    #[must_use]
    pub const fn with_path(config_path: PathBuf) -> Self {
        Self { config_path }
    }

    /// Apply configuration to the system
    pub async fn apply_configuration(
        &self,
        config: &ToadStoolConfig,
    ) -> Result<(), ToadStoolError> {
        info!("⚙️ Applying ToadStool configuration...");

        if !self.config_path.exists() {
            fs::create_dir_all(&self.config_path)?;
        }

        // Write main configuration file
        let config_json = serde_json::to_string_pretty(config)?;
        fs::write(self.config_path.join("toadstool.json"), config_json)?;

        // Write runtime-specific configurations
        self.write_runtime_configs(config)?;

        // Write security configuration
        self.write_security_config(config)?;

        // Write logging/observability configuration
        self.write_observability_config(config)?;

        info!("✅ Configuration applied successfully");
        Ok(())
    }

    fn write_runtime_configs(&self, config: &ToadStoolConfig) -> Result<(), ToadStoolError> {
        let runtime_dir = self.config_path.join("runtimes");
        if !runtime_dir.exists() {
            fs::create_dir_all(&runtime_dir)?;
        }

        // Native runtime config (always enabled)
        let native_config = serde_json::json!({
            "enabled": true,
            "max_concurrent": config.runtime.max_concurrent_executions,
            "timeout_seconds": config.runtime.execution_timeout.as_secs(),
            "memory_limit_mb": (config.runtime.resource_limits.max_memory_usage / 1024.0) as u64
        });
        fs::write(
            runtime_dir.join("native.json"),
            serde_json::to_string_pretty(&native_config)?,
        )?;

        // Container runtime config
        let container_config = serde_json::json!({
            "enabled": true,
            "engine": config.runtime.container.runtime,
            "memory_limit_mb": (config.runtime.resource_limits.max_memory_usage / 1024.0) as u64,
            "cpu_limit": config.runtime.resource_limits.max_cpu_usage
        });
        fs::write(
            runtime_dir.join("container.json"),
            serde_json::to_string_pretty(&container_config)?,
        )?;

        // WASM runtime config
        let wasm_config = serde_json::json!({
            "enabled": true,
            "memory_limit_mb": 128,
            "enable_wasi": true
        });
        fs::write(
            runtime_dir.join("wasm.json"),
            serde_json::to_string_pretty(&wasm_config)?,
        )?;

        // GPU runtime config (if present)
        if config.runtime.gpu.is_some() {
            let gpu_config = serde_json::json!({
                "enabled": true,
                "memory_fraction": 0.8,
                "compute_mode": "default"
            });
            fs::write(
                runtime_dir.join("gpu.json"),
                serde_json::to_string_pretty(&gpu_config)?,
            )?;
        }

        Ok(())
    }

    fn write_security_config(&self, config: &ToadStoolConfig) -> Result<(), ToadStoolError> {
        let security_config = serde_json::json!({
            "auth": {
                "enabled": config.security.auth.enabled,
                "provider": config.security.auth.provider
            },
            "sandboxing": {
                "enabled": config.security.sandbox.enabled,
                "sandbox_type": config.security.sandbox.sandbox_type
            },
            "resource_limits": {
                "max_cpu_usage": config.runtime.resource_limits.max_cpu_usage,
                "max_memory_usage": config.runtime.resource_limits.max_memory_usage,
                "max_disk_usage": config.runtime.resource_limits.max_disk_usage
            }
        });

        fs::write(
            self.config_path.join("security.json"),
            serde_json::to_string_pretty(&security_config)?,
        )?;

        Ok(())
    }

    fn write_observability_config(
        &self,
        config: &ToadStoolConfig,
    ) -> Result<(), ToadStoolError> {
        let observability_config = serde_json::json!({
            "logging": {
                "level": config.logging.level,
                "format": config.logging.format,
                "log_to_file": config.logging.log_to_file
            },
            "metrics": config.metrics.as_ref().map(|m| serde_json::json!({
                "enabled": m.enabled,
                "endpoint": m.endpoint,
                "format": m.format
            }))
        });

        fs::write(
            self.config_path.join("observability.json"),
            serde_json::to_string_pretty(&observability_config)?,
        )?;

        Ok(())
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use toadstool_config::ToadStoolConfig;

    #[tokio::test]
    async fn test_apply_configuration_creates_directory_and_files() {
        let temp_dir = tempdir().expect("create temp dir");
        let config_path = temp_dir.path().to_path_buf();
        let manager = ConfigManager::with_path(config_path.clone());

        let config = ToadStoolConfig::default();
        manager.apply_configuration(&config).await.unwrap();

        assert!(config_path.exists());
        assert!(config_path.is_dir());

        let toadstool_json = config_path.join("toadstool.json");
        assert!(toadstool_json.exists(), "toadstool.json should exist");
        let content = tokio::fs::read_to_string(&toadstool_json).await.unwrap();
        assert!(
            content.contains("toadstool") || content.contains("app") || content.contains("runtime")
        );

        let runtime_dir = config_path.join("runtimes");
        assert!(runtime_dir.exists());
        assert!(runtime_dir.join("native.json").exists());
        assert!(runtime_dir.join("container.json").exists());
        assert!(runtime_dir.join("wasm.json").exists());

        let security_json = config_path.join("security.json");
        assert!(security_json.exists());

        let observability_json = config_path.join("observability.json");
        assert!(observability_json.exists());
    }

    #[tokio::test]
    async fn test_apply_configuration_writes_valid_json() {
        let temp_dir = tempdir().expect("create temp dir");
        let config_path = temp_dir.path().to_path_buf();
        let manager = ConfigManager::with_path(config_path.clone());

        let config = ToadStoolConfig::default();
        manager.apply_configuration(&config).await.unwrap();

        let toadstool_content = tokio::fs::read_to_string(config_path.join("toadstool.json"))
            .await
            .unwrap();
        let _parsed: serde_json::Value =
            serde_json::from_str(&toadstool_content).expect("toadstool.json should be valid JSON");

        let native_content =
            tokio::fs::read_to_string(config_path.join("runtimes").join("native.json"))
                .await
                .unwrap();
        let native_json: serde_json::Value =
            serde_json::from_str(&native_content).expect("native.json should be valid JSON");
        assert_eq!(native_json["enabled"], true);
    }

    #[tokio::test]
    async fn test_apply_configuration_idempotent() {
        let temp_dir = tempdir().expect("create temp dir");
        let config_path = temp_dir.path().to_path_buf();
        let manager = ConfigManager::with_path(config_path.clone());

        let config = ToadStoolConfig::default();
        manager.apply_configuration(&config).await.unwrap();
        manager.apply_configuration(&config).await.unwrap();

        assert!(config_path.join("toadstool.json").exists());
    }

    #[tokio::test]
    async fn test_apply_configuration_with_gpu_writes_gpu_config() {
        let temp_dir = tempdir().expect("create temp dir");
        let config_path = temp_dir.path().to_path_buf();
        let manager = ConfigManager::with_path(config_path.clone());

        let mut config = ToadStoolConfig::default();
        config.runtime.gpu = Some(toadstool_config::GpuConfig::default());

        manager.apply_configuration(&config).await.unwrap();

        let gpu_json = config_path.join("runtimes").join("gpu.json");
        assert!(gpu_json.exists());
    }
}
