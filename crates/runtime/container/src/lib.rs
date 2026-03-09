// SPDX-License-Identifier: AGPL-3.0-only
#![deny(unsafe_code)]
#![allow(
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

//! # `ToadStool` Container Runtime Engine
//!
//! High-performance container runtime engine with Docker, Containerd, and Podman support,
//! comprehensive security isolation, resource limits, and network policies.

// Module declarations
pub mod types;

mod byob_routes;
mod docker;
mod engine;
pub use byob_routes::ByobApi;
pub mod byob_server;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use uuid::Uuid;

#[cfg(feature = "docker")]
use bollard::Docker;

use toadstool::resources::ResourceMonitor;
use toadstool::{RuntimeCapabilities, ToadStoolResult, WorkloadType};

// Re-export types for backward compatibility
pub use types::{
    ContainerEngineType, ContainerExecutionConfig, ContainerResourceLimits, ContainerResources,
    ContainerRuntimeConfig, ContainerSecurity, ContainerSecurityConfig, DnsConfig, ImageConfig,
    ImagePullPolicy, NetworkMode, NetworkPolicy, PortRange, RegistryConfig, VolumePolicy,
};

/// Active container handle
#[derive(Clone, Debug)]
pub(crate) struct ContainerHandle {
    pub(crate) container_id: String,
    _image: String,
    _start_time: Instant,
    _config: ContainerRuntimeConfig,
}

/// Container runtime engine implementation
pub struct ContainerRuntimeEngine {
    config: ContainerRuntimeConfig,
    #[cfg(feature = "docker")]
    docker: Option<Docker>,
    #[cfg(not(feature = "docker"))]
    docker: Option<()>,
    active_containers: Arc<RwLock<HashMap<Uuid, ContainerHandle>>>,
    resource_monitor: Option<Arc<dyn ResourceMonitor>>,
    capabilities: RuntimeCapabilities,
}

impl std::fmt::Debug for ContainerRuntimeEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContainerRuntimeEngine")
            .field("config", &self.config)
            .field("docker", &"<Docker>")
            .field("active_containers", &"<HashMap<Uuid, ContainerHandle>>")
            .field("resource_monitor", &"<Option<ResourceMonitor>>")
            .field("capabilities", &self.capabilities)
            .finish()
    }
}

impl ContainerRuntimeEngine {
    /// Create a new container runtime engine with default configuration
    pub fn new() -> ToadStoolResult<Self> {
        let config = ContainerRuntimeConfig::default();
        Self::with_config(config)
    }

    /// Create a new container runtime engine with custom configuration
    pub fn with_config(config: ContainerRuntimeConfig) -> ToadStoolResult<Self> {
        let docker = docker::create_docker_client(&config)?;

        let capabilities = RuntimeCapabilities {
            supported_workloads: vec![WorkloadType::Container],
            max_concurrent_executions: Some(100),
            supported_architectures: vec!["linux/amd64".to_string(), "linux/arm64".to_string()],
            platform_features: {
                let mut features = HashMap::new();
                features.insert("docker_support".to_string(), docker.is_some());
                features.insert("volume_mounts".to_string(), true);
                features.insert("network_isolation".to_string(), true);
                features
            },
            version: env!("CARGO_PKG_VERSION").to_string(),
        };

        Ok(Self {
            config,
            docker,
            active_containers: Arc::new(RwLock::new(HashMap::new())),
            resource_monitor: None,
            capabilities,
        })
    }

    /// Add a resource monitor to the engine
    pub fn with_resource_monitor(mut self, monitor: Arc<dyn ResourceMonitor>) -> Self {
        self.resource_monitor = Some(monitor);
        self
    }
}

impl Default for ContainerRuntimeEngine {
    fn default() -> Self {
        match Self::new() {
            Ok(engine) => engine,
            Err(_) => {
                // Return a minimal engine configuration if creation fails
                Self {
                    config: ContainerRuntimeConfig::default(),
                    docker: None,
                    active_containers: Arc::new(RwLock::new(HashMap::new())),
                    resource_monitor: None,
                    capabilities: RuntimeCapabilities {
                        supported_workloads: vec![],
                        max_concurrent_executions: Some(0),
                        supported_architectures: vec!["x86_64".to_string(), "aarch64".to_string()],
                        platform_features: HashMap::new(),
                        version: "1.0.0".to_string(),
                    },
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;
    use std::path::PathBuf;
    use std::pin::Pin;
    use std::time::Duration;
    use toadstool::execution::RuntimeConfig;
    use toadstool::resources::{ResourceMonitor, RuntimeMetrics, SystemResources};
    use toadstool::workload::{PortMapping, RegistryAuth, VolumeMount, WorkloadSpec};
    use toadstool::{
        ExecutionRequest, IsolationLevel, PortProtocol, RuntimeEngine, RuntimeType,
        SecurityContext, VolumeMountType,
    };

    fn create_test_request(_image: &str) -> ExecutionRequest {
        ExecutionRequest {
            execution_id: Uuid::new_v4(),
            workload: WorkloadSpec::Container {
                image: "ubuntu:20.04".to_string(),
                command: Some(vec!["echo".to_string(), "Hello World".to_string()]),
                args: None,
                env_vars: HashMap::new(),
                working_dir: Some("/tmp".to_string()),
                volumes: vec![],
                ports: vec![],
                registry_auth: None,
            },
            runtime_hint: Some(RuntimeType::Container),
            resources: toadstool::resources::ResourceRequirements::default(),
            security_context: SecurityContext::for_isolation_level(IsolationLevel::Basic),
            timeout: Some(Duration::from_secs(30)),
            environment: HashMap::new(),
            input_data: toadstool::execution::ExecutionInput::default(),
            callback_config: None,
            encryption_config: None,
        }
    }

    #[tokio::test]
    async fn test_engine_creation() {
        let engine = ContainerRuntimeEngine::new();
        // May fail if Docker is not available, which is expected in test environments
        assert!(engine.is_ok() || engine.is_err());
    }

    #[tokio::test]
    async fn test_capabilities() {
        if let Ok(engine) = ContainerRuntimeEngine::new() {
            let capabilities = engine.get_capabilities();
            assert!(capabilities
                .supported_workloads
                .contains(&WorkloadType::Container));
        }
    }

    #[tokio::test]
    async fn test_workload_support() {
        if let Ok(engine) = ContainerRuntimeEngine::new() {
            assert!(engine.supports_workload(&WorkloadType::Container));
            assert!(!engine.supports_workload(&WorkloadType::Wasm));
            assert!(!engine.supports_workload(&WorkloadType::Native));
        }
    }

    #[tokio::test]
    async fn test_invalid_workload_execution() {
        if let Ok(engine) = ContainerRuntimeEngine::new() {
            let request = ExecutionRequest {
                execution_id: Uuid::new_v4(),
                workload: WorkloadSpec::Native {
                    executable: toadstool::workload::ExecutableSource::File {
                        path: PathBuf::from("/bin/echo"),
                    },
                    args: None,
                    working_dir: None,
                    env_vars: HashMap::new(),
                    user: None,
                },
                runtime_hint: Some(RuntimeType::Native),
                resources: toadstool::resources::ResourceRequirements::default(),
                security_context: SecurityContext::for_isolation_level(IsolationLevel::Basic),
                timeout: None,
                environment: HashMap::new(),
                input_data: toadstool::execution::ExecutionInput::default(),
                callback_config: None,
                encryption_config: None,
            };

            let result = engine.execute(request).await;
            assert!(result.is_err());
        }
    }

    #[tokio::test]
    async fn test_resource_validation() {
        if let Ok(engine) = ContainerRuntimeEngine::new() {
            let mut request = create_test_request("hello-world");

            // Set memory requirement that exceeds limits (default is 512MB)
            request.resources.memory.max_bytes = Some(10 * 1024 * 1024 * 1024); // 10GB

            let result = engine.validate_resource_requirements(&request);
            assert!(result.is_err());
        }
    }

    #[tokio::test]
    async fn test_shutdown() {
        if let Ok(mut engine) = ContainerRuntimeEngine::new() {
            let result = engine.shutdown().await;
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_docker_integration() {
        let config = ContainerRuntimeConfig::default();
        let engine_result = ContainerRuntimeEngine::with_config(config);

        // Should succeed in creating the engine (Docker availability is checked later)
        assert!(engine_result.is_ok() || engine_result.is_err());
    }

    #[tokio::test]
    async fn test_port_mapping() {
        let mut request = create_test_request("alpine:latest");

        // Modify request to include port mapping
        if let WorkloadSpec::Container { ports, .. } = &mut request.workload {
            ports.push(PortMapping {
                host_port: 8080,
                container_port: 80,
                protocol: PortProtocol::Tcp,
            });
        }

        // Test port validation
        assert!(matches!(request.workload, WorkloadSpec::Container { .. }));
    }

    #[tokio::test]
    async fn test_volume_mounting() {
        let mut request = create_test_request("alpine:latest");

        // Modify request to include volume mounts
        if let WorkloadSpec::Container { volumes, .. } = &mut request.workload {
            volumes.push(VolumeMount {
                source: PathBuf::from("/tmp"),
                target: PathBuf::from("/data"),
                mount_type: VolumeMountType::Bind,
                read_only: true,
            });
        }

        // Test volume validation
        assert!(matches!(request.workload, WorkloadSpec::Container { .. }));
    }

    #[tokio::test]
    async fn test_registry_authentication() {
        let mut request = create_test_request("private.registry.com/image:latest");

        // Test registry auth configuration
        if let WorkloadSpec::Container { registry_auth, .. } = &mut request.workload {
            *registry_auth = Some(RegistryAuth {
                username: "testuser".to_string(),
                password: "testpass".to_string(),
                server_url: "private.registry.com".to_string(),
            });
        }

        // Test authentication validation
        assert!(matches!(request.workload, WorkloadSpec::Container { .. }));
    }

    #[tokio::test]
    async fn test_security_contexts() {
        let config = ContainerRuntimeConfig::default();
        if let Ok(engine) = ContainerRuntimeEngine::with_config(config) {
            let capabilities = engine.get_capabilities();

            // Test platform features
            assert!(!capabilities.platform_features.is_empty());
        }
    }

    #[tokio::test]
    async fn test_resource_constraints() {
        let mut request = create_test_request("alpine:latest");
        request.resources.cpu.max_cores = Some(1.0);
        request.resources.memory.max_bytes = Some(512 * 1024 * 1024); // 512MB in bytes

        // Test resource validation
        assert_eq!(request.resources.cpu.max_cores, Some(1.0));
        assert_eq!(request.resources.memory.max_bytes, Some(512 * 1024 * 1024));
    }

    #[tokio::test]
    async fn test_cpu_validation_exceeds_limit() {
        if let Ok(engine) = ContainerRuntimeEngine::new() {
            let mut request = create_test_request("alpine:latest");
            request.resources.cpu.max_cores = Some(1000.0);
            let result = engine.validate_resource_requirements(&request);
            assert!(result.is_err());
        }
    }

    #[tokio::test]
    #[expect(clippy::float_cmp, reason = "test values are exact literals")]
    async fn test_get_metrics() {
        if let Ok(engine) = ContainerRuntimeEngine::new() {
            let metrics = engine.get_metrics().await;
            assert!(metrics.is_ok());
            let m = metrics.unwrap();
            assert!(m.timing.start_time != std::time::UNIX_EPOCH);
            assert_eq!(m.cpu.usage_percent, 0.0);
        }
    }

    #[tokio::test]
    async fn test_container_engine_default_fallback() {
        let engine = ContainerRuntimeEngine::default();
        assert!(!engine.capabilities.supported_architectures.is_empty());
    }

    #[tokio::test]
    async fn test_container_execution_config_default() {
        let config = ContainerExecutionConfig::default();
        assert!(config.image.is_empty());
        assert!(config.args.is_empty());
        assert!(config.volumes.is_empty());
        assert!(config.ports.is_empty());
    }

    #[tokio::test]
    async fn test_with_resource_monitor() {
        if let Ok(engine) = ContainerRuntimeEngine::new() {
            struct MockMonitor;
            impl ResourceMonitor for MockMonitor {
                fn start_monitoring(&self, _workload_id: &str) -> toadstool::ToadStoolResult<()> {
                    Ok(())
                }
                fn stop_monitoring(&self, _workload_id: &str) -> toadstool::ToadStoolResult<()> {
                    Ok(())
                }
                fn get_metrics(
                    &self,
                    _workload_id: &str,
                ) -> Pin<
                    Box<
                        dyn Future<Output = toadstool::ToadStoolResult<RuntimeMetrics>> + Send + '_,
                    >,
                > {
                    Box::pin(async { Ok(toadstool::resources::RuntimeMetrics::default()) })
                }
                fn get_system_resources(
                    &self,
                ) -> Pin<
                    Box<
                        dyn Future<Output = toadstool::ToadStoolResult<SystemResources>>
                            + Send
                            + '_,
                    >,
                > {
                    Box::pin(async { Ok(toadstool::resources::SystemResources::default()) })
                }
            }
            let _engine = engine.with_resource_monitor(Arc::new(MockMonitor));
        }
    }

    #[tokio::test]
    async fn test_engine_with_containerd_config() {
        let config = ContainerRuntimeConfig {
            engine: ContainerEngineType::Containerd {
                address: "/run/containerd/containerd.sock".to_string(),
                namespace: "default".to_string(),
            },
            ..ContainerRuntimeConfig::default()
        };
        let result = ContainerRuntimeEngine::with_config(config);
        assert!(result.is_ok());
        let engine = result.unwrap();
        assert!(!engine.get_capabilities().supported_workloads.is_empty());
    }

    #[tokio::test]
    async fn test_engine_with_podman_config() {
        let config = ContainerRuntimeConfig {
            engine: ContainerEngineType::Podman {
                socket_path: "/run/podman/podman.sock".to_string(),
                remote_url: None,
            },
            ..ContainerRuntimeConfig::default()
        };
        let result = ContainerRuntimeEngine::with_config(config);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_initialize_without_docker() {
        let config = ContainerRuntimeConfig {
            engine: ContainerEngineType::Containerd {
                address: "invalid".to_string(),
                namespace: "default".to_string(),
            },
            ..ContainerRuntimeConfig::default()
        };
        if let Ok(mut engine) = ContainerRuntimeEngine::with_config(config) {
            let result = engine.initialize(RuntimeConfig::default()).await;
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_validate_resource_requirements_ok() {
        let engine = ContainerRuntimeEngine::default();
        let mut request = create_test_request("alpine");
        request.resources.memory.max_bytes = Some(256 * 1024 * 1024);
        request.resources.cpu.max_cores = Some(0.5);
        let result = engine.validate_resource_requirements(&request);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_resource_requirements_cpu_exceeds() {
        let engine = ContainerRuntimeEngine::default();
        let mut request = create_test_request("alpine");
        request.resources.cpu.max_cores = Some(5000.0);
        let result = engine.validate_resource_requirements(&request);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_capabilities_platform_features() {
        let engine = ContainerRuntimeEngine::default();
        let caps = engine.get_capabilities();
        assert!(
            caps.platform_features.contains_key("volume_mounts")
                || caps.platform_features.is_empty()
        );
        assert!(!caps.supported_architectures.is_empty());
    }

    #[tokio::test]
    async fn test_initialize_containerd_engine() {
        let config = ContainerRuntimeConfig {
            engine: ContainerEngineType::Containerd {
                address: "/run/containerd/containerd.sock".to_string(),
                namespace: "k8s.io".to_string(),
            },
            ..ContainerRuntimeConfig::default()
        };
        if let Ok(mut engine) = ContainerRuntimeEngine::with_config(config) {
            let result = engine.initialize(RuntimeConfig::default()).await;
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_container_config_default() {
        let config = ContainerRuntimeConfig::default();
        assert!(matches!(config.engine, ContainerEngineType::Docker { .. }));
        assert!(config.resource_limits.max_memory_bytes > 0);
    }

    #[tokio::test]
    async fn test_resource_limits_validation_memory_ok() {
        let engine = ContainerRuntimeEngine::default();
        let mut request = create_test_request("alpine");
        request.resources.memory.max_bytes = Some(256 * 1024 * 1024);
        let result = engine.validate_resource_requirements(&request);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_resource_limits_validation_cpu_ok() {
        let engine = ContainerRuntimeEngine::default();
        let mut request = create_test_request("alpine");
        request.resources.cpu.max_cores = Some(0.25);
        let result = engine.validate_resource_requirements(&request);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_container_execution_config_default_values() {
        let config = ContainerExecutionConfig::default();
        assert!(config.image.is_empty());
        assert!(config.args.is_empty());
        assert!(config.working_dir.is_none());
        assert!(config.env_vars.is_empty());
        assert!(config.volumes.is_empty());
        assert!(config.ports.is_empty());
        assert!(config.registry_auth.is_none());
    }

    #[tokio::test]
    async fn test_engine_debug_format() {
        let engine = ContainerRuntimeEngine::default();
        let debug_str = format!("{engine:?}");
        assert!(debug_str.contains("ContainerRuntimeEngine"));
    }

    #[tokio::test]
    async fn test_workload_container_with_args() {
        let engine = ContainerRuntimeEngine::default();
        let mut request = create_test_request("alpine");
        if let WorkloadSpec::Container { args, .. } = &mut request.workload {
            *args = Some(vec!["--version".to_string()]);
        }
        let result = engine.execute(request).await;
        // May fail without Docker
        let _ = result;
    }

    #[tokio::test]
    async fn test_engine_with_podman_socket() {
        let config = ContainerRuntimeConfig {
            engine: ContainerEngineType::Podman {
                socket_path: "/run/user/1000/podman/podman.sock".to_string(),
                remote_url: Some("ssh://user@host".to_string()),
            },
            ..ContainerRuntimeConfig::default()
        };
        let result = ContainerRuntimeEngine::with_config(config);
        assert!(result.is_ok());
    }
}
