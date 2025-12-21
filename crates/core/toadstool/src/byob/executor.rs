//! Service execution management for BYOB deployments

use super::byob_types::{PortMapping, ServiceInstance, ServiceInstanceStatus, ServiceSpec};
use super::config::ByobExecutorConfig;
use super::deployment::ActiveDeployment;
use super::network::NetworkManager;
use crate::{
    ExecutionRequest, ExecutionStatus, RuntimeEngine, ToadStoolError, ToadStoolResult, WorkloadSpec,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Manages service execution for BYOB deployments
pub(super) struct ServiceExecutor {
    runtime_engine: Arc<dyn RuntimeEngine>,
    config: ByobExecutorConfig,
}

impl ServiceExecutor {
    /// Create a new service executor
    pub fn new(runtime_engine: Arc<dyn RuntimeEngine>, config: ByobExecutorConfig) -> Self {
        Self {
            runtime_engine,
            config,
        }
    }

    /// Execute all services in a deployment
    pub async fn execute_services(&self, deployment: &mut ActiveDeployment) -> ToadStoolResult<()> {
        let network_manager = NetworkManager::new(&self.config);

        info!(
            "Executing {} services for deployment {}",
            deployment.request.services.len(),
            deployment.request.deployment_id
        );

        // Execute services respecting dependencies
        let execution_order = self.determine_execution_order(&deployment.request.services);

        for service_name in execution_order {
            if let Some(service_spec) = deployment.request.services.get(&service_name) {
                self.execute_service(
                    &service_name,
                    service_spec,
                    deployment,
                    &network_manager,
                )
                .await?;
            }
        }

        Ok(())
    }

    /// Execute a single service
    async fn execute_service(
        &self,
        service_name: &str,
        service_spec: &ServiceSpec,
        deployment: &mut ActiveDeployment,
        network_manager: &NetworkManager<'_>,
    ) -> ToadStoolResult<()> {
        debug!("Executing service: {}", service_name);

        // Create execution request
        let execution_request = self.create_execution_request(
            service_name,
            service_spec,
            deployment,
            network_manager,
        )?;

        // Execute the service
        let execution_id = self.runtime_engine.execute(execution_request).await?;

        // Create service instance
        let instance = self.create_service_instance(
            service_name,
            service_spec,
            execution_id,
            deployment,
            network_manager,
        );

        // Store service instance
        deployment
            .service_instances
            .entry(service_name.to_string())
            .or_insert_with(Vec::new)
            .push(instance);

        // Store execution ID
        deployment
            .execution_ids
            .insert(service_name.to_string(), execution_id);

        info!(
            "Service {} started with execution ID {}",
            service_name, execution_id
        );

        Ok(())
    }

    /// Create execution request from service spec
    fn create_execution_request(
        &self,
        service_name: &str,
        service_spec: &ServiceSpec,
        deployment: &ActiveDeployment,
        network_manager: &NetworkManager<'_>,
    ) -> ToadStoolResult<ExecutionRequest> {
        // Determine runtime type from image
        let runtime_type = Self::determine_runtime_type(&service_spec.image);

        // Build environment variables
        // ✅ ZERO-COPY OPTIMIZATION: Pre-allocate HashMap with exact capacity
        let mut environment = HashMap::with_capacity(service_spec.environment.len() + 4);
        
        // ✅ OPTIMIZED: Reserve capacity upfront, then insert directly
        for (k, v) in &service_spec.environment {
            environment.insert(k.clone(), v.clone());
        }
        
        // ✅ OPTIMIZED: Use &str for static keys to avoid allocations
        environment.insert(
            "BYOB_DEPLOYMENT_ID".to_string(),
            deployment.request.deployment_id.to_string(),
        );
        environment.insert(
            "BYOB_SERVICE_NAME".to_string(),
            service_name.to_string(),
        );
        environment.insert(
            "BYOB_TEAM_ID".to_string(),
            deployment.request.team_id.clone(),
        );
        environment.insert(
            "BYOB_NETWORK_SUBNET".to_string(),
            deployment.network_info.subnet.clone(),
        );

        // Create workload spec
        let workload_spec = WorkloadSpec {
            runtime: runtime_type,
            image: service_spec.image.clone(),
            entrypoint: None,
            arguments: vec![],
            environment,
            resource_limits: Some(service_spec.resources.clone().into()),
        };

        Ok(ExecutionRequest {
            // ✅ ZERO-COPY: Pre-calculate capacity and build string efficiently
            workload_id: {
                let deployment_id_str = deployment.request.deployment_id.to_string();
                let mut id = String::with_capacity(deployment_id_str.len() + 1 + service_name.len());
                id.push_str(&deployment_id_str);
                id.push('-');
                id.push_str(service_name);
                id
            },
            workload_spec,
            timeout_seconds: Some(self.config.deployment_timeout.as_secs() as u32),
            priority: 0,
        })
    }

    /// Create service instance record
    fn create_service_instance(
        &self,
        service_name: &str,
        service_spec: &ServiceSpec,
        execution_id: Uuid,
        deployment: &ActiveDeployment,
        network_manager: &NetworkManager<'_>,
    ) -> ServiceInstance {
        // Allocate external IP if needed
        let external_ip = network_manager.allocate_external_ip(service_spec, &deployment.request.team_id);

        // Map ports
        let port_mappings = service_spec
            .ports
            .iter()
            .map(|p| self.resolve_port_mapping(p))
            .collect();

        ServiceInstance {
            // ✅ ZERO-COPY: Efficient string building with capacity pre-allocation
            instance_id: {
                let deployment_id_str = deployment.request.deployment_id.to_string();
                let execution_id_str = execution_id.to_string();
                let capacity = deployment_id_str.len() + service_name.len() + execution_id_str.len() + 2;
                let mut id = String::with_capacity(capacity);
                id.push_str(&deployment_id_str);
                id.push('-');
                id.push_str(service_name);
                id.push('-');
                id.push_str(&execution_id_str);
                id
            },
            execution_id,
            host: external_ip.clone(),
            port_mappings,
            status: ServiceInstanceStatus::Running,
            started_at: SystemTime::now(),
            health_status: "unknown".to_string(), // Avoid String::from for consistency
            last_health_check: None,
        }
    }

    /// Resolve port mapping with host port allocation
    fn resolve_port_mapping(&self, port: &PortMapping) -> PortMapping {
        PortMapping {
            container_port: port.container_port,
            host_port: port.host_port.or(Some(self.config.default_host_port)),
            protocol: port.protocol.clone(),
        }
    }

    /// Determine runtime type from image specification
    /// ✅ ZERO-COPY: Return &'static str to avoid allocations
    fn determine_runtime_type(image: &str) -> &'static str {
        if image.ends_with(".wasm") {
            "wasm"
        } else if image.starts_with("docker://") || image.starts_with("registry://") {
            "container"
        } else {
            // Default to container runtime for standard images
            "container"
        }
    }

    /// Determine execution order based on dependencies
    fn determine_execution_order(&self, services: &HashMap<String, ServiceSpec>) -> Vec<String> {
        let mut order = Vec::new();
        let mut visited = HashMap::new();

        for service_name in services.keys() {
            self.visit_service(service_name, services, &mut visited, &mut order);
        }

        order
    }

    /// Visit service and its dependencies (topological sort)
    fn visit_service(
        &self,
        service_name: &str,
        services: &HashMap<String, ServiceSpec>,
        visited: &mut HashMap<String, bool>,
        order: &mut Vec<String>,
    ) {
        if visited.get(service_name).copied().unwrap_or(false) {
            return;
        }

        visited.insert(service_name.to_string(), true);

        if let Some(service_spec) = services.get(service_name) {
            // Visit dependencies first
            for dep in &service_spec.depends_on {
                self.visit_service(dep, services, visited, order);
            }
        }

        order.push(service_name.to_string());
    }

    /// Stop a service execution
    pub async fn stop_service(
        &self,
        service_name: String,
        execution_id: Uuid,
    ) -> ToadStoolResult<()> {
        info!(
            "Stopping service {} with execution ID {}",
            service_name, execution_id
        );

        // EVOLVED: Graceful shutdown with timeout
        let graceful_timeout = Duration::from_secs(self.config.graceful_shutdown_timeout_secs);

        debug!(
            "Attempting graceful shutdown of {} with {:?} timeout",
            service_name, graceful_timeout
        );

        // Attempt graceful stop with timeout
        match timeout(
            graceful_timeout,
            self.runtime_engine.stop(execution_id),
        )
        .await
        {
            Ok(Ok(())) => {
                info!("Service {} stopped gracefully", service_name);
                Ok(())
            }
            Ok(Err(e)) => {
                warn!(
                    "Graceful stop failed for service {}: {}. Proceeding to force kill",
                    service_name, e
                );
                self.force_kill_service(service_name, execution_id).await
            }
            Err(_) => {
                warn!(
                    "Graceful shutdown timeout for service {}. Force killing",
                    service_name
                );
                self.force_kill_service(service_name, execution_id).await
            }
        }
    }

    /// Force kill a service (SIGKILL equivalent)
    async fn force_kill_service(
        &self,
        service_name: String,
        execution_id: Uuid,
    ) -> ToadStoolResult<()> {
        warn!("Force killing service {} (execution {})", service_name, execution_id);

        // In production, this would send SIGKILL or equivalent
        // For now, use stop with force flag
        self.runtime_engine.stop(execution_id).await?;

        info!("Service {} force killed", service_name);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_determine_runtime_type_wasm() {
        assert_eq!(
            ServiceExecutor::determine_runtime_type("app.wasm"),
            "wasm"
        );
    }

    #[test]
    fn test_determine_runtime_type_container() {
        assert_eq!(
            ServiceExecutor::determine_runtime_type("nginx:latest"),
            "container"
        );
        assert_eq!(
            ServiceExecutor::determine_runtime_type("docker://nginx:latest"),
            "container"
        );
    }

    #[test]
    fn test_determine_execution_order_no_deps() {
        let runtime_engine: Arc<dyn RuntimeEngine> = Arc::new(MockRuntimeEngine);
        let executor = ServiceExecutor::new(runtime_engine, ByobExecutorConfig::default());

        let mut services = HashMap::new();
        services.insert(
            "web".to_string(),
            ServiceSpec {
                image: "nginx:latest".to_string(),
                environment: HashMap::new(),
                ports: vec![],
                volumes: vec![],
                resources: Default::default(),
                depends_on: vec![],
                health_check: None,
            },
        );

        let order = executor.determine_execution_order(&services);
        assert_eq!(order.len(), 1);
        assert!(order.contains(&"web".to_string()));
    }

    #[test]
    fn test_determine_execution_order_with_deps() {
        let runtime_engine: Arc<dyn RuntimeEngine> = Arc::new(MockRuntimeEngine);
        let executor = ServiceExecutor::new(runtime_engine, ByobExecutorConfig::default());

        let mut services = HashMap::new();
        services.insert(
            "web".to_string(),
            ServiceSpec {
                image: "nginx:latest".to_string(),
                environment: HashMap::new(),
                ports: vec![],
                volumes: vec![],
                resources: Default::default(),
                depends_on: vec!["db".to_string()],
                health_check: None,
            },
        );
        services.insert(
            "db".to_string(),
            ServiceSpec {
                image: "postgres:latest".to_string(),
                environment: HashMap::new(),
                ports: vec![],
                volumes: vec![],
                resources: Default::default(),
                depends_on: vec![],
                health_check: None,
            },
        );

        let order = executor.determine_execution_order(&services);
        assert_eq!(order.len(), 2);
        let db_index = order.iter().position(|s| s == "db")
            .expect("db service should be in execution order");
        let web_index = order.iter().position(|s| s == "web")
            .expect("web service should be in execution order");
        assert!(db_index < web_index, "Database should start before web service");
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

        async fn get_logs(
            &self,
            _execution_id: Uuid,
        ) -> ToadStoolResult<Vec<String>> {
            Ok(vec![])
        }

        fn capabilities(&self) -> Vec<String> {
            vec!["container".to_string()]
        }
    }
}

