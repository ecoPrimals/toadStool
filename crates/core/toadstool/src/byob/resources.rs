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
        for (service_name, execution_id) in &deployment.execution_ids {
            match self.get_service_usage(*execution_id).await {
                Ok(usage) => {
                    total_cpu += usage.cpu_usage_percent;
                    total_memory += usage.memory_bytes;
                    total_storage += usage.storage_bytes;
                    total_gpu += usage.gpu_count;

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
        deployment.resource_usage = Some(ResourceUsage {
            cpu_usage_percent: total_cpu,
            memory_bytes: total_memory,
            storage_bytes: total_storage,
            gpu_count: total_gpu,
            network_rx_bytes: 0, // Network monitoring available via sysinfo integration if needed
            network_tx_bytes: 0,
            timestamp: SystemTime::now(),
        });

        Ok(())
    }

    /// Get resource usage for a specific service
    async fn get_service_usage(&self, _execution_id: Uuid) -> ToadStoolResult<ResourceUsage> {
        // In production, this would query the runtime engine for actual metrics
        // For now, return mock data
        Ok(ResourceUsage {
            cpu_usage_percent: 0.0,
            memory_bytes: 0,
            storage_bytes: 0,
            gpu_count: 0,
            network_rx_bytes: 0,
            network_tx_bytes: 0,
            timestamp: SystemTime::now(),
        })
    }

    /// Get current usage for a deployment
    pub fn get_usage(&self, deployment: &ActiveDeployment) -> ToadStoolResult<ResourceUsage> {
        deployment
            .resource_usage
            .clone()
            .ok_or_else(|| crate::ToadStoolError::not_found("Resource usage not available"))
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
            .execution_ids
            .insert("test-service".to_string(), Uuid::new_v4());

        let result = monitor.update_usage(&mut deployment).await;
        assert!(result.is_ok());
        assert!(deployment.resource_usage.is_some());
    }

    #[tokio::test]
    async fn test_get_usage() {
        let runtime_engine: Arc<dyn RuntimeEngine> = Arc::new(MockRuntimeEngine);
        let monitor = ResourceMonitor::new(runtime_engine);

        let mut deployment = create_test_deployment();
        deployment.resource_usage = Some(ResourceUsage {
            cpu_usage_percent: 50.0,
            memory_bytes: 1024 * 1024 * 1024,
            storage_bytes: 0,
            gpu_count: 0,
            network_rx_bytes: 0,
            network_tx_bytes: 0,
            timestamp: SystemTime::now(),
        });

        let usage = monitor.get_usage(&deployment).map_err(|e| {
            ToadStoolError::runtime(format!("Failed to get resource usage: {}", e))
        })?;
        assert_eq!(usage.cpu_usage_percent, 50.0);
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

