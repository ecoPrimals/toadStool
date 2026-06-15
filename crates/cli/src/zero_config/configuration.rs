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
    #[expect(
        clippy::unused_async,
        reason = "async signature required by trait/interface"
    )] // Config generation; async for API consistency
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
    #[expect(
        clippy::unused_async,
        reason = "async signature required by trait/interface"
    )] // Config generation; async for API consistency
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
    #[expect(
        clippy::unused_async,
        reason = "async signature required by trait/interface"
    )] // Config generation; async for API consistency
    pub(crate) async fn generate_security_config(&self) -> Result<SecurityConfig> {
        debug!("Generating security configuration");

        // ✅ DEEP DEBT: Check for security CAPABILITY, not specific primal
        let security_provider_available = self.ecosystem_services.security.is_some()
            || std::env::var(toadstool_common::interned_strings::socket_env::TOADSTOOL_SECURITY_PROVIDER).is_ok();

        Ok(SecurityConfig {
            level: "standard".to_string(),
            isolation: "process".to_string(),
            security_provider_enabled: security_provider_available,
            crypto_policies: vec!["default".to_string()],
        })
    }

    /// Generate network configuration
    #[expect(
        clippy::unused_async,
        reason = "async signature required by trait/interface"
    )] // Config generation; async for API consistency
    pub(crate) async fn generate_network_config(&self) -> Result<NetworkConfig> {
        debug!("Generating network configuration");

        // ✅ DEEP DEBT: Check for coordination CAPABILITY, not specific primal
        let coordination_provider_available = self.ecosystem_services.coordination.is_some()
            || std::env::var(toadstool_common::interned_strings::socket_env::TOADSTOOL_COORDINATION_PROVIDER).is_ok();

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
            dns_servers: std::env::var(toadstool_common::interned_strings::socket_env::TOADSTOOL_DNS_SERVERS)
                .ok()
                .map(|v| v.split(',').map(str::trim).map(String::from).collect())
                .unwrap_or_default(),
            coordination_enabled: coordination_provider_available,
        })
    }

    /// Generate storage configuration
    #[expect(
        clippy::unused_async,
        reason = "async signature required by trait/interface"
    )] // Config generation; async for API consistency
    pub(crate) async fn generate_storage_config(&self) -> Result<StorageConfig> {
        debug!("Generating storage configuration");

        // ✅ DEEP DEBT: Check for storage CAPABILITY, not specific primal
        let storage_provider_available = self.ecosystem_services.storage.is_some()
            || std::env::var(toadstool_common::interned_strings::socket_env::TOADSTOOL_STORAGE_PROVIDER).is_ok();

        Ok(StorageConfig {
            backend: "local".to_string(),
            storage_provider_enabled: storage_provider_available,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn test_deployment() -> ZeroConfigDeployment {
        ZeroConfigDeployment {
            system_info: SystemInfo {
                cpu: CpuInfo {
                    cores: 4,
                    architecture: "x86_64".to_string(),
                    model: "Test CPU".to_string(),
                    frequency: 3000,
                    vendor: "TestVendor".to_string(),
                },
                memory: MemoryInfo {
                    total_bytes: 8 * 1024 * 1024 * 1024,
                    available_bytes: 4 * 1024 * 1024 * 1024,
                    memory_type: "DDR4".to_string(),
                },
                storage: StorageInfo {
                    total_bytes: 500 * 1024 * 1024 * 1024,
                    available_bytes: 250 * 1024 * 1024 * 1024,
                    storage_type: "SSD".to_string(),
                    filesystem: "ext4".to_string(),
                },
                network: NetworkInfo::default(),
                os: OsInfo {
                    name: "Linux".to_string(),
                    version: "6.0".to_string(),
                    kernel: "6.0.0".to_string(),
                    arch: "x86_64".to_string(),
                },
                container_runtime: ContainerRuntimeInfo::default(),
                gpu: GpuInfo {
                    count: 0,
                    vendor: "none".to_string(),
                    model: "none".to_string(),
                    memory_bytes: 0,
                    cuda: false,
                },
            },
            ecosystem_services: EcosystemServices::default(),
            config: AutoGeneratedConfig::default(),
            start_time: Instant::now(),
        }
    }

    #[tokio::test]
    async fn biome_config_respects_cpu_cores() {
        let deploy = test_deployment();
        let biome = deploy.generate_biome_config().await.unwrap();
        let expected_cpu = (4.0_f64 * 0.8).max(1.0);
        assert!((biome.resources.cpu_limit - expected_cpu).abs() < f64::EPSILON);
        assert!(biome.resources.gpu_limit.is_none());
    }

    #[tokio::test]
    async fn runtime_config_no_docker_no_cuda() {
        let deploy = test_deployment();
        let rt = deploy.generate_runtime_config().await.unwrap();
        assert_eq!(rt.container_runtime, "none");
        assert!(rt.gpu_runtime.is_none());
        assert!(rt.preferred_runtimes.contains(&"native".to_string()));
        assert!(rt.preferred_runtimes.contains(&"wasm".to_string()));
        assert!(!rt.preferred_runtimes.contains(&"container".to_string()));
    }

    #[tokio::test]
    async fn runtime_config_with_docker_and_cuda() {
        let mut deploy = test_deployment();
        deploy.system_info.container_runtime.docker = true;
        deploy.system_info.gpu.cuda = true;
        let rt = deploy.generate_runtime_config().await.unwrap();
        assert_eq!(rt.container_runtime, "docker");
        assert_eq!(rt.gpu_runtime, Some("cuda".to_string()));
        assert!(rt.preferred_runtimes.contains(&"container".to_string()));
        assert!(rt.preferred_runtimes.contains(&"gpu".to_string()));
    }

    #[tokio::test]
    async fn security_config_no_provider() {
        temp_env::async_with_vars([("TOADSTOOL_SECURITY_PROVIDER", None::<&str>)], async {
            let deploy = test_deployment();
            let sec = deploy.generate_security_config().await.unwrap();
            assert!(!sec.security_provider_enabled);
        })
        .await;
    }

    #[tokio::test]
    async fn security_config_with_env_provider() {
        temp_env::async_with_vars([("TOADSTOOL_SECURITY_PROVIDER", Some("test"))], async {
            let deploy = test_deployment();
            let sec = deploy.generate_security_config().await.unwrap();
            assert!(sec.security_provider_enabled);
        })
        .await;
    }

    #[tokio::test]
    async fn network_config_dns_from_env() {
        temp_env::async_with_vars(
            [
                ("TOADSTOOL_DNS_SERVERS", Some("1.1.1.1, 8.8.8.8")),
                ("TOADSTOOL_COORDINATION_PROVIDER", None::<&str>),
            ],
            async {
                let deploy = test_deployment();
                let net = deploy.generate_network_config().await.unwrap();
                assert_eq!(net.dns_servers, vec!["1.1.1.1", "8.8.8.8"]);
                assert!(!net.coordination_enabled);
            },
        )
        .await;
    }

    #[tokio::test]
    async fn storage_config_no_provider() {
        temp_env::async_with_vars([("TOADSTOOL_STORAGE_PROVIDER", None::<&str>)], async {
            let deploy = test_deployment();
            let storage = deploy.generate_storage_config().await.unwrap();
            assert!(!storage.storage_provider_enabled);
            assert!(!storage.backup_enabled);
        })
        .await;
    }

    #[tokio::test]
    async fn storage_config_with_env_provider() {
        temp_env::async_with_vars([("TOADSTOOL_STORAGE_PROVIDER", Some("test"))], async {
            let deploy = test_deployment();
            let storage = deploy.generate_storage_config().await.unwrap();
            assert!(storage.storage_provider_enabled);
            assert!(storage.backup_enabled);
        })
        .await;
    }

    #[tokio::test]
    async fn full_generate_configuration() {
        temp_env::async_with_vars(
            [
                ("TOADSTOOL_SECURITY_PROVIDER", None::<&str>),
                ("TOADSTOOL_COORDINATION_PROVIDER", None::<&str>),
                ("TOADSTOOL_STORAGE_PROVIDER", None::<&str>),
                ("TOADSTOOL_DNS_SERVERS", None::<&str>),
            ],
            async {
                let mut deploy = test_deployment();
                ConfigurationExt::generate_configuration(&mut deploy)
                    .await
                    .unwrap();
                assert!(!deploy.config.biome.name.is_empty());
                assert!(!deploy.config.runtime.preferred_runtimes.is_empty());
            },
        )
        .await;
    }
}
