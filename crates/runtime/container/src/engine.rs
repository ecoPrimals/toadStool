// SPDX-License-Identifier: AGPL-3.0-only
//! `RuntimeEngine` trait implementation for the container runtime.
//!
//! Execution, metrics, shutdown, and resource validation.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
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

use crate::docker;
use crate::types::{ContainerExecutionConfig, ContainerResourceLimits, ContainerSecurityConfig};
use crate::ContainerRuntimeEngine;

impl ContainerRuntimeEngine {
    /// Validate resource requirements against configured limits.
    pub fn validate_resource_requirements(
        &self,
        request: &ExecutionRequest,
    ) -> ToadStoolResult<()> {
        if let Some(memory_req) = request.resources.memory.max_bytes {
            if memory_req > self.config.resource_limits.max_memory_bytes {
                return Err(ToadStoolError::resource(format!(
                    "Memory requirement {} exceeds limit {}",
                    memory_req, self.config.resource_limits.max_memory_bytes
                )));
            }
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
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async {
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
        })
    }

    fn execute(
        &self,
        request: ExecutionRequest,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_>> {
        Box::pin(async move {
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
        })
    }

    fn get_capabilities(&self) -> RuntimeCapabilities {
        self.capabilities.clone()
    }

    fn supports_workload(&self, workload_type: &WorkloadType) -> bool {
        matches!(workload_type, WorkloadType::Container)
    }

    fn get_metrics(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<RuntimeMetrics>> + Send + '_>> {
        Box::pin(async {
            let start_time = SystemTime::now();

            let mut custom_metrics = HashMap::new();
            custom_metrics.insert(
                "active_containers".to_string(),
                serde_json::Value::Number(serde_json::Number::from(0)),
            );
            custom_metrics.insert(
                "available_engines".to_string(),
                serde_json::Value::Number(serde_json::Number::from(1)),
            );
            custom_metrics.insert(
                "runtime_health".to_string(),
                serde_json::Value::Number(serde_json::Number::from(1)),
            );

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
        })
    }

    fn shutdown(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async {
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
        })
    }
}
