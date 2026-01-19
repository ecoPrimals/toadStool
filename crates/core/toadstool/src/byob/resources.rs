//! Resource usage monitoring for BYOB deployments

use super::byob_types::ResourceUsage;
use super::deployment::ActiveDeployment;
use crate::{RuntimeEngine, ToadStoolResult};
use std::sync::Arc;
use std::time::SystemTime;
use tracing::{debug, error};
use uuid::Uuid;

/// Resource monitoring manager
pub(super) struct ResourceMonitor {
    runtime_engine: Arc<dyn RuntimeEngine>,
}

impl ResourceMonitor {
    /// Create a new resource monitor
    pub fn new(runtime_engine: Arc<dyn RuntimeEngine>) -> Self {
        Self { runtime_engine }
    }

    /// Update resource usage for a deployment
    pub async fn update_usage(&self, deployment: &mut ActiveDeployment) -> ToadStoolResult<()> {
        debug!(
            "Updating resource usage for deployment {}",
            deployment.request.deployment_id
        );

        let mut total_cpu = 0.0;
        let mut total_memory = 0;
        let mut total_storage = 0;
        let mut total_gpu = 0;

        // Collect usage from all service instances
        for (service_name, execution_id) in &deployment.service_executions {
            match self.get_service_usage(*execution_id).await {
                Ok(usage) => {
                    total_cpu += usage.cpu_usage;
                    total_memory += usage.memory_usage;
                    total_storage += usage.storage_usage;
                    total_gpu += usage.gpu_usage;

                    debug!(
                        "Service {} using CPU: {:.2}%, Memory: {} bytes",
                        service_name, usage.cpu_usage_percent, usage.memory_bytes
                    );
                }
                Err(e) => {
                    error!("Failed to get usage for service {}: {}", service_name, e);
                }
            }
        }

        // Update deployment's resource usage
        deployment.resource_usage = ResourceUsage {
            cpu_usage: total_cpu,
            memory_usage: total_memory,
            storage_usage: total_storage,
            gpu_usage: total_gpu,
            network_usage: super::byob_types::NetworkUsage {
                bytes_sent: 0,
                bytes_received: 0,
                packets_sent: 0,
                packets_received: 0,
            },
        };

        Ok(())
    }

    /// Get resource usage for a specific service
    async fn get_service_usage(&self, execution_id: Uuid) -> ToadStoolResult<ResourceUsage> {
        // Query the runtime engine for actual metrics
        let status = self.runtime_engine.get_status(execution_id).await?;
        
        // Get runtime metrics if available
        let metrics = match &status {
            crate::ExecutionStatus::Running => {
                // For running executions, try to get real metrics
                // Note: Not all runtime engines may support metrics yet
                // This is a best-effort approach with graceful fallback
                self.estimate_resources_from_status(&status).await
            }
            crate::ExecutionStatus::Completed { .. } | 
            crate::ExecutionStatus::Failed { .. } => {
                // For completed/failed executions, return zero usage
                ResourceUsage {
                    cpu_usage: 0.0,
                    memory_usage: 0,
                    storage_usage: 0,
                    gpu_usage: 0,
                    network_usage: super::byob_types::NetworkUsage {
                        bytes_sent: 0,
                        bytes_received: 0,
                        packets_sent: 0,
                        packets_received: 0,
                    },
                }
            }
            _ => {
                // For other states (pending, etc.), return minimal usage
                ResourceUsage {
                    cpu_usage: 0.0,
                    memory_usage: 0,
                    storage_usage: 0,
                    gpu_usage: 0,
                    network_usage: super::byob_types::NetworkUsage {
                        bytes_sent: 0,
                        bytes_received: 0,
                        packets_sent: 0,
                        packets_received: 0,
                    },
                }
            }
        };
        
        Ok(metrics)
    }
    
    /// Estimate resources from execution status
    /// 
    /// This provides a best-effort resource estimation when detailed metrics
    /// are not available from the runtime engine. Different runtime engines
    /// have varying levels of metrics support:
    /// - Container runtime: Full metrics via cgroups/Docker stats
    /// - WASM runtime: Memory usage tracking
    /// - Native runtime: Process-level metrics via sysinfo
    /// - GPU runtime: GPU utilization via device APIs
    async fn estimate_resources_from_status(
        &self,
        status: &crate::ExecutionStatus,
    ) -> ResourceUsage {
        // For running executions, we provide conservative estimates
        // Real production deployment would integrate with:
        // 1. Container runtime: docker stats / podman stats
        // 2. Process monitor: sysinfo crate for process metrics
        // 3. cgroups: Direct cgroup v2 metrics reading
        // 4. GPU monitoring: nvidia-smi / rocm-smi integration
        
        match status {
            crate::ExecutionStatus::Running => {
                // Conservative running estimate
                // Real implementation would query the specific runtime engine
                ResourceUsage {
                    cpu_usage: 0.0, // Would query process CPU %
                    memory_usage: 0,         // Would query process RSS
                    storage_usage: 0,        // Would query container overlay size
                    gpu_usage: 0,            // Would query GPU device utilization
                    network_usage: super::byob_types::NetworkUsage {
                        bytes_sent: 0,
                        bytes_received: 0,
                        packets_sent: 0,
                        packets_received: 0,
                    },
                }
            }
            _ => {
                // Non-running states have no resource usage
                ResourceUsage {
                    cpu_usage: 0.0,
                    memory_usage: 0,
                    storage_usage: 0,
                    gpu_usage: 0,
                    network_usage: super::byob_types::NetworkUsage {
                        bytes_sent: 0,
                        bytes_received: 0,
                        packets_sent: 0,
                        packets_received: 0,
                    },
                }
            }
        }
    }

    /// Get current usage for a deployment
    pub fn get_usage(&self, deployment: &ActiveDeployment) -> ResourceUsage {
        deployment.resource_usage.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::byob::*;
    use crate::{ExecutionRequest, ExecutionStatus};
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_update_usage() {
        let runtime_engine: Arc<dyn RuntimeEngine> = Arc::new(MockRuntimeEngine);
        let monitor = ResourceMonitor::new(runtime_engine);

        let mut deployment = create_test_deployment();

        // Add a mock execution ID
        deployment
            .service_executions
            .insert("test-service".to_string(), Uuid::new_v4());

        let result = monitor.update_usage(&mut deployment).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_usage() {
        let runtime_engine: Arc<dyn RuntimeEngine> = Arc::new(MockRuntimeEngine);
        let monitor = ResourceMonitor::new(runtime_engine);

        let mut deployment = create_test_deployment();
        deployment.resource_usage = ResourceUsage {
            cpu_usage: 50.0,
            memory_usage: 1024 * 1024 * 1024,
            storage_usage: 0,
            gpu_usage: 0,
            network_usage: super::byob_types::NetworkUsage {
                bytes_sent: 0,
                bytes_received: 0,
                packets_sent: 0,
                packets_received: 0,
            },
        };

        let usage = monitor.get_usage(&deployment);
        assert_eq!(usage.cpu_usage, 50.0);
    }

    fn create_test_deployment() -> ActiveDeployment {
        let request = ByobDeploymentRequest {
            deployment_id: Uuid::new_v4(),
            team_id: "test-team".to_string(),
            services: HashMap::new(),
            network_config: None,
            resource_quotas: Default::default(),
        };

        let network_info = NetworkInfo {
            network_id: "test-network".to_string(),
            subnet: "10.0.0.0/24".to_string(),
            gateway: "10.0.0.1".to_string(),
            dns_servers: vec!["8.8.8.8".to_string()],
            isolation_enabled: true,
        };

        ActiveDeployment::new(request, network_info)
    }

    // Mock runtime engine for tests
    struct MockRuntimeEngine;

    #[async_trait::async_trait]
    impl RuntimeEngine for MockRuntimeEngine {
        async fn execute(&self, _request: ExecutionRequest) -> ToadStoolResult<Uuid> {
            Ok(Uuid::new_v4())
        }

        async fn stop(&self, _execution_id: Uuid) -> ToadStoolResult<()> {
            Ok(())
        }

        async fn get_status(&self, _execution_id: Uuid) -> ToadStoolResult<ExecutionStatus> {
            Ok(ExecutionStatus::Running)
        }

        async fn get_logs(&self, _execution_id: Uuid) -> ToadStoolResult<Vec<String>> {
            Ok(vec![])
        }

        fn capabilities(&self) -> Vec<String> {
            vec!["container".to_string()]
        }
    }
}

