// SPDX-License-Identifier: AGPL-3.0-or-later
//! Configuration generation functionality

use crate::Result;
use std::future::Future;
use tracing::{debug, info};

use super::types::*;

/// Configuration extension trait
pub trait ConfigurationExt {
    /// Generate optimal configuration
    fn generate_configuration(&mut self) -> impl Future<Output = Result<()>> + Send;
}

impl ConfigurationExt for ZeroConfigDeployment {
    async fn generate_configuration(&mut self) -> Result<()> {
        info!("⚙️ Generating optimal configuration");

        // Generate biome configuration
        self.config.biome = self.generate_biome_config().await?;

        // Generate runtime configuration
        self.config.runtime = self.generate_runtime_config().await?;

        // Generate security configuration
        self.config.security = self.generate_security_config().await?;

        // Generate network configuration
        self.config.network = self.generate_network_config().await?;

        // Generate storage configuration
        self.config.storage = self.generate_storage_config().await?;

        info!("✅ Configuration generation completed");
        Ok(())
    }
}

impl ZeroConfigDeployment {
    /// Generate biome configuration
    #[allow(clippy::unused_async)] // Config generation; async for API consistency
    pub(crate) async fn generate_biome_config(&self) -> Result<BiomeConfig> {
        debug!("Generating biome configuration");

        let resources = BiomeResources {
            cpu_limit: (self.system_info.cpu.cores as f64 * 0.8).max(1.0),
            memory_limit: format!(
                "{}MB",
                self.system_info.memory.total_bytes / 1024 / 1024 * 80 / 100
            ),
            storage_limit: format!(
                "{}GB",
                self.system_info.storage.total_bytes / 1024 / 1024 / 1024 * 50 / 100
            ),
            gpu_limit: if self.system_info.gpu.count > 0 {
                Some(self.system_info.gpu.count)
            } else {
                None
            },
        };

        Ok(BiomeConfig {
            name: "auto-generated-biome".to_string(),
            version: "1.0.0".to_string(),
            description: "Automatically generated biome configuration".to_string(),
            resources,
        })
    }

    /// Generate runtime configuration
    #[allow(clippy::unused_async)] // Config generation; async for API consistency
    pub(crate) async fn generate_runtime_config(&self) -> Result<RuntimeConfig> {
        debug!("Generating runtime configuration");

        let mut preferred_runtimes = vec!["native".to_string(), "wasm".to_string()];

        let container_runtime = if self.system_info.container_runtime.docker {
            preferred_runtimes.push("container".to_string());
            "docker".to_string()
        } else if self.system_info.container_runtime.podman {
            preferred_runtimes.push("container".to_string());
            "podman".to_string()
        } else {
            "none".to_string()
        };

        let gpu_runtime = if self.system_info.gpu.cuda {
            preferred_runtimes.push("gpu".to_string());
            Some("cuda".to_string())
        } else {
            None
        };

        Ok(RuntimeConfig {
            preferred_runtimes,
            container_runtime,
            wasm_runtime: "wasmtime".to_string(),
            gpu_runtime,
        })
    }

    /// Generate security configuration
    #[allow(clippy::unused_async)] // Config generation; async for API consistency
    pub(crate) async fn generate_security_config(&self) -> Result<SecurityConfig> {
        debug!("Generating security configuration");

        // ✅ DEEP DEBT: Check for security CAPABILITY, not specific primal
        let security_provider_available = self.ecosystem_services.beardog.is_some()
            || std::env::var("TOADSTOOL_SECURITY_PROVIDER").is_ok();

        Ok(SecurityConfig {
            level: "standard".to_string(),
            isolation: "process".to_string(),
            beardog_enabled: security_provider_available, // Renamed from beardog_enabled
            crypto_policies: vec!["default".to_string()],
        })
    }

    /// Generate network configuration
    #[allow(clippy::unused_async)] // Config generation; async for API consistency
    pub(crate) async fn generate_network_config(&self) -> Result<NetworkConfig> {
        debug!("Generating network configuration");

        // ✅ DEEP DEBT: Check for coordination CAPABILITY, not specific primal
        let coordination_provider_available = self.ecosystem_services.songbird.is_some()
            || std::env::var("TOADSTOOL_COORDINATION_PROVIDER").is_ok();

        let host_port = toadstool_config::config_utils::ConfigUtils::get_toadstool_port();

        Ok(NetworkConfig {
            mode: "bridge".to_string(),
            port_mappings: vec![PortMapping {
                host_port,
                container_port: host_port,
                protocol: "tcp".to_string(),
            }],
            // Discover DNS servers from TOADSTOOL_DNS_SERVERS env var or inherit
            // from the host. Never assume specific public resolvers.
            dns_servers: std::env::var("TOADSTOOL_DNS_SERVERS")
                .ok()
                .map(|v| v.split(',').map(str::trim).map(String::from).collect())
                .unwrap_or_default(),
            songbird_enabled: coordination_provider_available,
        })
    }

    /// Generate storage configuration
    #[allow(clippy::unused_async)] // Config generation; async for API consistency
    pub(crate) async fn generate_storage_config(&self) -> Result<StorageConfig> {
        debug!("Generating storage configuration");

        // ✅ DEEP DEBT: Check for storage CAPABILITY, not specific primal
        let storage_provider_available = self.ecosystem_services.nestgate.is_some()
            || std::env::var("TOADSTOOL_STORAGE_PROVIDER").is_ok();

        Ok(StorageConfig {
            backend: "local".to_string(),
            nestgate_enabled: storage_provider_available, // Capability-based check
            volumes: vec![VolumeConfig {
                name: "data".to_string(),
                size: "10GB".to_string(),
                mount_point: "/data".to_string(),
                read_only: false,
            }],
            backup_enabled: storage_provider_available,
        })
    }
}
