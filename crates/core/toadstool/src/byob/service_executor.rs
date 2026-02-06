//! Service Execution - Trait-Based BYOB Refactoring
//!
//! **Purpose**: Handle service execution lifecycle
//! **Deep Debt**: Smart refactoring (improve architecture, not just split files)

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};
use uuid::Uuid;

use super::byob_types::*;
use crate::{ExecutionRequest, RuntimeEngine, ToadStoolError, ToadStoolResult, WorkloadSpec};

/// Service execution lifecycle management
///
/// **Responsibilities**:
/// - Create execution requests from service specs
/// - Execute services via RuntimeEngine
/// - Stop service executions
///
/// **Design**: Single responsibility (service execution only)
#[async_trait]
pub trait ServiceExecutor: Send + Sync {
    /// Create execution request for a service
    ///
    /// **Transforms**: `ServiceSpec` → `ExecutionRequest`
    fn create_service_execution_request(
        &self,
        service: &ServiceSpec,
        deployment_id: Uuid,
    ) -> ToadStoolResult<ExecutionRequest>;

    /// Execute all services in a deployment
    ///
    /// **Side Effects**: Spawns services via RuntimeEngine
    async fn execute_services(
        &self,
        deployment_id: Uuid,
        services: Vec<ServiceSpec>,
        network_name: String,
        _resource_constraints: ResourceConstraints,
    ) -> ToadStoolResult<()>;

    /// Stop a service execution
    ///
    /// **Side Effects**: Terminates running service
    async fn stop_service_execution(&self, execution_id: Uuid) -> ToadStoolResult<()>;
}

/// Implementation for ByobComputeExecutor
///
/// **Context**: Needs RuntimeEngine and active deployments
pub struct ByobServiceExecutor {
    runtime_engine: Arc<dyn RuntimeEngine>,
    active_deployments: Arc<RwLock<std::collections::HashMap<Uuid, super::deployment::ActiveDeployment>>>,
}

impl ByobServiceExecutor {
    pub fn new(
        runtime_engine: Arc<dyn RuntimeEngine>,
        active_deployments: Arc<RwLock<std::collections::HashMap<Uuid, super::deployment::ActiveDeployment>>>,
    ) -> Self {
        Self {
            runtime_engine,
            active_deployments,
        }
    }
}

#[async_trait]
impl ServiceExecutor for ByobServiceExecutor {
    fn create_service_execution_request(
        &self,
        service: &ServiceSpec,
        _deployment_id: Uuid,
    ) -> ToadStoolResult<ExecutionRequest> {
        // ✅ OPTIMIZED: Reduce clones by using references where possible
        let workload = if let Some(image) = &service.image {
            // Container workload
            WorkloadSpec::Container {
                image: image.clone(),
                command: service.command.clone(),
                args: None,
                working_dir: None,
                env_vars: service.environment.clone(),
                volumes: service
                    .volumes
                    .iter()
                    .map(|v| crate::workload::VolumeMount {
                        source: v.source.as_str().into(),
                        target: v.target.as_str().into(),
                        mount_type: match v.mount_type.as_str() {
                            "volume" => crate::workload::VolumeMountType::Volume,
                            _ => crate::workload::VolumeMountType::Bind,
                        },
                        read_only: v.read_only,
                    })
                    .collect(),
            }
        } else if let Some(code) = &service.code {
            // Code workload
            WorkloadSpec::Code {
                code: code.to_string(),
                language: service
                    .language
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
                args: service.command.clone(),
            }
        } else {
            return Err(ToadStoolError::InvalidInput(
                "Service must have either 'image' or 'code'".to_string(),
            ));
        };

        Ok(ExecutionRequest {
            workload,
            resources: crate::resource::ResourceRequirements {
                cpu_cores: service.resources.cpu_cores.unwrap_or(1.0),
                memory_mb: service.resources.memory_mb.unwrap_or(512),
                gpu_required: service.resources.gpu_required,
                gpu_memory_mb: service.resources.gpu_memory_mb,
            },
            priority: crate::ExecutionPriority::Normal,
            timeout: std::time::Duration::from_secs(3600),
        })
    }

    async fn execute_services(
        &self,
        deployment_id: Uuid,
        services: Vec<ServiceSpec>,
        network_name: String,
        _resource_constraints: ResourceConstraints,
    ) -> ToadStoolResult<()> {
        debug!(
            "Executing {} services for deployment {}",
            services.len(),
            deployment_id
        );

        for service in services {
            // Create execution request
            let request = self.create_service_execution_request(&service, deployment_id)?;

            // Execute via runtime engine
            let execution_id = self.runtime_engine.execute(request).await?;

            debug!(
                "Service {} started with execution_id: {}",
                service.name, execution_id
            );

            // Update deployment with execution ID
            let mut deployments = self.active_deployments.write().await;
            if let Some(deployment) = deployments.get_mut(&deployment_id) {
                deployment.service_status.insert(
                    service.name.clone(),
                    crate::ExecutionStatus::Running {
                        started_at: std::time::SystemTime::now(),
                    },
                );
            }
        }

        info!(
            "Successfully executed all services for deployment {} on network {}",
            deployment_id, network_name
        );
        Ok(())
    }

    async fn stop_service_execution(&self, execution_id: Uuid) -> ToadStoolResult<()> {
        debug!("Stopping service execution {}", execution_id);
        self.runtime_engine.stop(execution_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::MockRuntimeEngine;

    #[tokio::test]
    async fn test_create_service_execution_request_container() {
        let mock_engine = Arc::new(MockRuntimeEngine::new());
        let deployments = Arc::new(RwLock::new(std::collections::HashMap::new()));
        let executor = ByobServiceExecutor::new(mock_engine, deployments);

        let service = ServiceSpec {
            name: "web".to_string(),
            image: Some("nginx:latest".to_string()),
            command: None,
            code: None,
            language: None,
            ports: vec![],
            environment: vec![],
            volumes: vec![],
            resources: ResourceConstraints {
                cpu_cores: Some(2.0),
                memory_mb: Some(1024),
                gpu_required: false,
                gpu_memory_mb: None,
            },
            health_check: None,
        };

        let request = executor
            .create_service_execution_request(&service, Uuid::new_v4())
            .expect("Should create request");

        match request.workload {
            WorkloadSpec::Container { image, .. } => {
                assert_eq!(image, "nginx:latest");
            }
            _ => panic!("Expected Container workload"),
        }

        assert_eq!(request.resources.cpu_cores, 2.0);
        assert_eq!(request.resources.memory_mb, 1024);
    }

    #[tokio::test]
    async fn test_create_service_execution_request_code() {
        let mock_engine = Arc::new(MockRuntimeEngine::new());
        let deployments = Arc::new(RwLock::new(std::collections::HashMap::new()));
        let executor = ByobServiceExecutor::new(mock_engine, deployments);

        let service = ServiceSpec {
            name: "function".to_string(),
            image: None,
            command: None,
            code: Some("print('hello')".to_string()),
            language: Some("python".to_string()),
            ports: vec![],
            environment: vec![],
            volumes: vec![],
            resources: ResourceConstraints {
                cpu_cores: Some(1.0),
                memory_mb: Some(256),
                gpu_required: false,
                gpu_memory_mb: None,
            },
            health_check: None,
        };

        let request = executor
            .create_service_execution_request(&service, Uuid::new_v4())
            .expect("Should create request");

        match request.workload {
            WorkloadSpec::Code { code, language, .. } => {
                assert_eq!(code, "print('hello')");
                assert_eq!(language, "python");
            }
            _ => panic!("Expected Code workload"),
        }
    }

    #[tokio::test]
    async fn test_create_service_execution_request_invalid() {
        let mock_engine = Arc::new(MockRuntimeEngine::new());
        let deployments = Arc::new(RwLock::new(std::collections::HashMap::new()));
        let executor = ByobServiceExecutor::new(mock_engine, deployments);

        let service = ServiceSpec {
            name: "invalid".to_string(),
            image: None, // Neither image nor code
            command: None,
            code: None,
            language: None,
            ports: vec![],
            environment: vec![],
            volumes: vec![],
            resources: ResourceConstraints {
                cpu_cores: Some(1.0),
                memory_mb: Some(256),
                gpu_required: false,
                gpu_memory_mb: None,
            },
            health_check: None,
        };

        let result = executor.create_service_execution_request(&service, Uuid::new_v4());
        assert!(result.is_err());
    }
}
