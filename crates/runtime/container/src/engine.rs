// SPDX-License-Identifier: AGPL-3.0-or-later
//! `RuntimeEngine` trait implementation for the container runtime.
//!
//! Execution, metrics, shutdown, and resource validation.

use std::collections::HashMap;
use std::future::Future;
use std::time::SystemTime;

use tracing::{debug, info};
use uuid::Uuid;

use toadstool::execution::RuntimeConfig;
use toadstool::resources::{
    CpuMetrics, MemoryMetrics, NetworkMetrics, RuntimeMetrics, StorageMetrics, TimingMetrics,
};
use toadstool::{
    ExecutionRequest, ExecutionResponse, RuntimeCapabilities, RuntimeEngine, ToadStoolError,
    ToadStoolResult, WorkloadType,
};

use crate::ContainerRuntimeEngine;
use crate::docker;
use crate::types::{ContainerExecutionConfig, ContainerResourceLimits, ContainerSecurityConfig};

impl ContainerRuntimeEngine {
    /// Validate resource requirements against configured limits.
    pub fn validate_resource_requirements(
        &self,
        request: &ExecutionRequest,
    ) -> ToadStoolResult<()> {
        if let Some(memory_req) = request.resources.memory.max_bytes
            && memory_req > self.config.resource_limits.max_memory_bytes
        {
            return Err(ToadStoolError::resource(format!(
                "Memory requirement {} exceeds limit {}",
                memory_req, self.config.resource_limits.max_memory_bytes
            )));
        }

        if let Some(cpu_req) = request.resources.cpu.max_cores {
            let cpu_millicores = (cpu_req * 1000.0) as u32;
            if cpu_millicores > self.config.resource_limits.max_cpu_millicores {
                return Err(ToadStoolError::resource(format!(
                    "CPU requirement {} exceeds limit {}",
                    cpu_millicores, self.config.resource_limits.max_cpu_millicores
                )));
            }
        }

        Ok(())
    }
}

impl RuntimeEngine for ContainerRuntimeEngine {
    fn initialize(
        &mut self,
        _config: RuntimeConfig,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async {
            debug!("Initializing container runtime engine");

            #[cfg(feature = "docker")]
            if let Some(docker) = &self.docker {
                match docker.ping().await {
                    Ok(_) => {
                        info!("Docker connection established successfully");
                    }
                    Err(e) => {
                        return Err(ToadStoolError::configuration(format!(
                            "Docker connection test failed: {e}"
                        )));
                    }
                }
            }

            info!("Container runtime engine initialized successfully");
            Ok(())
        }
    }

    fn execute(
        &self,
        request: ExecutionRequest,
    ) -> impl std::future::Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_ {
        async move {
            debug!("Executing container workload: {}", request.execution_id);

            self.validate_resource_requirements(&request)?;

            if let toadstool::workload::WorkloadSpec::Container {
                image,
                args,
                working_dir,
                volumes,
                ports,
                registry_auth,
                ..
            } = &request.workload
            {
                let exec_config = ContainerExecutionConfig {
                    image: image.clone(),
                    args: args
                        .clone()
                        .unwrap_or_else(|| vec!["echo".to_string(), "test".to_string()]),
                    working_dir: working_dir.clone(),
                    env_vars: HashMap::new(),
                    volumes: volumes.clone(),
                    ports: ports.clone(),
                    resources: ContainerResourceLimits::default(),
                    security: ContainerSecurityConfig::default(),
                    registry_auth: registry_auth.clone(),
                };

                #[cfg(feature = "docker")]
                {
                    let docker = self.docker.as_ref().ok_or_else(|| {
                        ToadStoolError::configuration("Docker client not available")
                    })?;
                    docker::execute_container(docker, &self.config, &request, &exec_config).await
                }

                #[cfg(not(feature = "docker"))]
                {
                    let _ = (request, exec_config);
                    Err(ToadStoolError::not_supported("Docker feature not enabled"))
                }
            } else {
                Err(ToadStoolError::validation(
                    "Invalid workload type for container runtime",
                ))
            }
        }
    }

    fn get_capabilities(&self) -> RuntimeCapabilities {
        self.capabilities.clone()
    }

    fn supports_workload(&self, workload_type: &WorkloadType) -> bool {
        matches!(workload_type, WorkloadType::Container)
    }

    fn get_metrics(
        &self,
    ) -> impl std::future::Future<Output = ToadStoolResult<RuntimeMetrics>> + Send + '_ {
        async {
            let start_time = SystemTime::now();

            let cpu_metrics = CpuMetrics {
                usage_percent: 0.0,
                cores_used: 0.0,
                cpu_time_seconds: 0.0,
            };

            let memory_metrics = MemoryMetrics {
                usage_percent: 0.0,
                used_bytes: 0,
                peak_bytes: 0,
            };

            let network_metrics = NetworkMetrics {
                bytes_sent: 0,
                bytes_received: 0,
                packets_sent: 0,
                packets_received: 0,
            };

            let storage_metrics = StorageMetrics {
                usage_percent: 0.0,
                used_bytes: 0,
                bytes_read: 0,
                bytes_written: 0,
            };

            let timing_metrics = TimingMetrics {
                start_time,
                end_time: Some(SystemTime::now()),
                duration: start_time.elapsed().unwrap_or_default(),
            };

            Ok(RuntimeMetrics {
                cpu: cpu_metrics,
                memory: memory_metrics,
                storage: storage_metrics,
                network: network_metrics,
                gpu: None,
                timing: timing_metrics,
            })
        }
    }

    fn shutdown(&mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async {
            info!("Shutting down container runtime engine");

            let container_ids: Vec<Uuid> = {
                let containers = self.active_containers.read().await;
                containers.keys().copied().collect()
            };

            #[cfg(feature = "docker")]
            if let Some(docker) = &self.docker {
                let ids: Vec<String> = {
                    let containers = self.active_containers.read().await;
                    container_ids
                        .iter()
                        .filter_map(|id| containers.get(id).map(|h| h.container_id.clone()))
                        .collect()
                };
                docker::cleanup_containers(docker, &ids).await;
            }

            {
                let mut containers = self.active_containers.write().await;
                containers.clear();
            }

            info!("Container runtime engine shut down successfully");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use tokio::sync::RwLock;
    use uuid::Uuid;

    use toadstool::execution::RuntimeEngine;
    use toadstool::resources::{CpuRequirements, MemoryRequirements, ResourceRequirements};
    use toadstool::{ExecutionRequest, RuntimeCapabilities, WorkloadType};

    use crate::ContainerRuntimeEngine;
    use crate::types::{ContainerResourceLimits, ContainerRuntimeConfig};

    fn test_engine() -> ContainerRuntimeEngine {
        let mut platform_features = HashMap::new();
        platform_features.insert("docker_support".to_string(), false);
        platform_features.insert("volume_mounts".to_string(), true);
        platform_features.insert("network_isolation".to_string(), true);

        ContainerRuntimeEngine {
            config: ContainerRuntimeConfig::default(),
            docker: None,
            active_containers: Arc::new(RwLock::new(HashMap::new())),
            resource_monitor: None,
            capabilities: RuntimeCapabilities {
                supported_workloads: vec![WorkloadType::Container],
                max_concurrent_executions: Some(100),
                supported_architectures: vec!["linux/amd64".to_string(), "linux/arm64".to_string()],
                platform_features,
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        }
    }

    fn engine_with_resource_limits(limits: ContainerResourceLimits) -> ContainerRuntimeEngine {
        let config = ContainerRuntimeConfig {
            resource_limits: limits,
            ..Default::default()
        };
        let mut engine = test_engine();
        engine.config = config;
        engine
    }

    fn execution_request_with_resources(resources: ResourceRequirements) -> ExecutionRequest {
        ExecutionRequest {
            execution_id: Uuid::nil(),
            resources,
            ..ExecutionRequest::default()
        }
    }

    #[test]
    fn validate_resource_requirements_memory_exceeds_limit_returns_error() {
        let engine = engine_with_resource_limits(ContainerResourceLimits {
            max_memory_bytes: 1024,
            max_cpu_millicores: 10_000,
            ..ContainerResourceLimits::default()
        });
        let request = execution_request_with_resources(ResourceRequirements {
            memory: MemoryRequirements {
                min_bytes: 512,
                max_bytes: Some(2048),
            },
            ..ResourceRequirements::default()
        });
        let err = engine
            .validate_resource_requirements(&request)
            .expect_err("memory above limit should error");
        let msg = err.to_string();
        assert!(
            msg.contains("Memory requirement") && msg.contains("exceeds limit"),
            "unexpected error message: {msg}"
        );
    }

    #[test]
    fn validate_resource_requirements_cpu_exceeds_limit_returns_error() {
        let engine = engine_with_resource_limits(ContainerResourceLimits {
            max_memory_bytes: 1024 * 1024 * 1024,
            max_cpu_millicores: 500,
            ..ContainerResourceLimits::default()
        });
        let request = execution_request_with_resources(ResourceRequirements {
            cpu: CpuRequirements {
                min_cores: 0.1,
                max_cores: Some(1.0),
                architecture: None,
            },
            ..ResourceRequirements::default()
        });
        let err = engine
            .validate_resource_requirements(&request)
            .expect_err("cpu above limit should error");
        let msg = err.to_string();
        assert!(
            msg.contains("CPU requirement") && msg.contains("exceeds limit"),
            "unexpected error message: {msg}"
        );
    }

    #[test]
    fn validate_resource_requirements_within_limits_ok() {
        let engine = test_engine();
        let request = execution_request_with_resources(ResourceRequirements {
            memory: MemoryRequirements {
                min_bytes: 128 * 1024 * 1024,
                max_bytes: Some(256 * 1024 * 1024),
            },
            cpu: CpuRequirements {
                min_cores: 0.25,
                max_cores: Some(0.5),
                architecture: None,
            },
            ..ResourceRequirements::default()
        });
        engine
            .validate_resource_requirements(&request)
            .expect("within limits should succeed");
    }

    #[test]
    fn validate_resource_requirements_none_specified_ok() {
        let engine = test_engine();
        let request = execution_request_with_resources(ResourceRequirements {
            memory: MemoryRequirements {
                min_bytes: 1024,
                max_bytes: None,
            },
            cpu: CpuRequirements {
                min_cores: 0.1,
                max_cores: None,
                architecture: None,
            },
            ..ResourceRequirements::default()
        });
        engine
            .validate_resource_requirements(&request)
            .expect("no max requirements should succeed");
    }

    #[test]
    fn supports_workload_container_true() {
        let engine = test_engine();
        assert!(engine.supports_workload(&WorkloadType::Container));
    }

    #[test]
    fn supports_workload_other_types_false() {
        let engine = test_engine();
        assert!(!engine.supports_workload(&WorkloadType::Gpu));
        assert!(!engine.supports_workload(&WorkloadType::Native));
        assert!(!engine.supports_workload(&WorkloadType::Wasm));
        assert!(!engine.supports_workload(&WorkloadType::Python));
    }

    #[test]
    fn get_capabilities_returns_expected_runtime_capabilities() {
        let engine = test_engine();
        let caps = engine.get_capabilities();
        assert_eq!(caps.supported_workloads, vec![WorkloadType::Container]);
        assert_eq!(caps.max_concurrent_executions, Some(100));
        assert_eq!(
            caps.supported_architectures,
            vec!["linux/amd64".to_string(), "linux/arm64".to_string()]
        );
        assert!(caps.version == env!("CARGO_PKG_VERSION"));
        assert_eq!(caps.platform_features.get("volume_mounts"), Some(&true));
        assert_eq!(caps.platform_features.get("network_isolation"), Some(&true));
        assert_eq!(caps.platform_features.get("docker_support"), Some(&false));
    }
}
