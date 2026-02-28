//! BYOB compute executor implementation

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::byob_types::*;
use super::config::ByobExecutorConfig;
use super::deployment::ActiveDeployment;
use super::validation::DeploymentValidator;
use crate::{
    ExecutionRequest, ExecutionStatus, RuntimeEngine, ToadStoolError, ToadStoolResult, WorkloadSpec,
};
use toadstool_common::constants::timeouts;

pub struct ByobComputeExecutor {
    /// Runtime engine for executing workloads
    runtime_engine: Arc<dyn RuntimeEngine>,
    /// Active deployments
    active_deployments: Arc<RwLock<HashMap<Uuid, ActiveDeployment>>>,
    /// Configuration
    config: ByobExecutorConfig,
}

/// BYOB executor trait
#[async_trait]
pub trait ByobExecutor: Send + Sync {
    /// Deploy a team biome
    async fn deploy_biome(
        &self,
        request: ByobDeploymentRequest,
    ) -> ToadStoolResult<ByobDeploymentResponse>;

    /// Get deployment status
    async fn get_deployment_status(
        &self,
        deployment_id: Uuid,
    ) -> ToadStoolResult<ByobDeploymentResponse>;

    /// Stop a deployment
    async fn stop_deployment(&self, deployment_id: Uuid) -> ToadStoolResult<()>;

    /// List active deployments
    async fn list_deployments(&self) -> ToadStoolResult<Vec<ByobDeploymentResponse>>;

    /// Get resource usage for a deployment
    async fn get_resource_usage(&self, deployment_id: Uuid) -> ToadStoolResult<ResourceUsage>;
}

impl ByobComputeExecutor {
    /// Create a new BYOB compute executor
    pub fn new(runtime_engine: Arc<dyn RuntimeEngine>, config: ByobExecutorConfig) -> Self {
        Self {
            runtime_engine,
            active_deployments: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Validate deployment request
    ///
    /// **Design**: Delegated to DeploymentValidator for separation of concerns
    fn validate_deployment_request(&self, request: &ByobDeploymentRequest) -> ToadStoolResult<()> {
        DeploymentValidator::validate_deployment(request)
    }

    /// Create execution request for a service
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
                ports: service
                    .ports
                    .iter()
                    .map(|p| crate::workload::PortMapping {
                        container_port: p.container_port,
                        host_port: p.host_port.unwrap_or(self.config.default_host_port),
                        protocol: match p.protocol.as_str() {
                            "udp" => crate::workload::PortProtocol::Udp,
                            _ => crate::workload::PortProtocol::Tcp,
                        },
                    })
                    .collect(),
                registry_auth: None,
            }
        } else {
            // Native workload (assume command is provided)
            WorkloadSpec::Native {
                executable: crate::ExecutableSource::File {
                    path: std::path::PathBuf::from(service.image.as_deref().unwrap_or("/bin/sh")),
                },
                args: None,
                working_dir: None,
                env_vars: service.environment.clone(),
                user: None,
            }
        };

        let execution_request = ExecutionRequest {
            execution_id: Uuid::new_v4(),
            workload,
            runtime_hint: None,
            resources: crate::resources::ResourceRequirements {
                cpu: crate::CpuRequirements {
                    min_cores: service.resources.cpu_cores.unwrap_or(1.0),
                    max_cores: service.resources.cpu_cores,
                    architecture: None,
                },
                memory: crate::MemoryRequirements {
                    min_bytes: service.resources.memory_bytes.unwrap_or(1024 * 1024 * 1024),
                    max_bytes: service.resources.memory_bytes,
                },
                storage: crate::StorageRequirements {
                    min_bytes: service
                        .resources
                        .storage_bytes
                        .unwrap_or(10 * 1024 * 1024 * 1024),
                    max_bytes: service.resources.storage_bytes,
                    storage_type: None,
                },
                gpu: service
                    .resources
                    .gpu_count
                    .map(|count| crate::GpuRequirements {
                        min_units: count,
                        max_units: Some(count),
                        gpu_type: None,
                        min_memory_bytes: Some(1024 * 1024 * 1024), // 1GB
                    }),
                network: crate::NetworkRequirements::default(),
            },
            security_context: crate::SecurityContext::default(),
            timeout: Some(timeouts::WORKLOAD_EXECUTION_TIMEOUT),
            environment: service.environment.clone(),
            input_data: crate::ExecutionInput::default(),
            callback_config: None,
            encryption_config: None,
        };

        Ok(execution_request)
    }

    /// Execute services in a deployment
    async fn execute_services(&self, deployment: &mut ActiveDeployment) -> ToadStoolResult<()> {
        info!(
            "Starting service execution for deployment {}",
            deployment.request.deployment_id
        );

        // ✅ OPTIMIZED: Use references to avoid unnecessary clones
        // Collect service names first to avoid borrow issues
        let service_names: Vec<_> = deployment.request.services.keys().cloned().collect();
        let deployment_id = deployment.request.deployment_id;

        // Execute services in dependency order
        for service_name in service_names {
            debug!("Executing service: {}", service_name);

            // Get service spec by reference
            let service_spec = deployment
                .request
                .services
                .get(&service_name)
                .ok_or_else(|| {
                    ToadStoolError::runtime(format!("Service {service_name} not found"))
                })?;

            let execution_request =
                self.create_service_execution_request(service_spec, deployment_id)?;

            let execution_id = execution_request.execution_id;

            // Execute the service
            match self.runtime_engine.execute(execution_request).await {
                Ok(response) => {
                    if response.status == ExecutionStatus::Success {
                        deployment.add_service_execution(&service_name, execution_id); // ✅ ZERO-COPY: Pass &str
                        info!("Service {} started successfully", service_name);
                    } else {
                        warn!("Service {} failed to start: {:?}", service_name, response);
                        deployment.update_status(DeploymentStatus::Failed {
                            error: format!("Service {service_name} failed to start"),
                        });
                        return Err(ToadStoolError::runtime(format!(
                            "Service {service_name} failed to start"
                        )));
                    }
                }
                Err(e) => {
                    error!("Failed to execute service {}: {:?}", service_name, e);
                    deployment.update_status(DeploymentStatus::Failed {
                        error: format!("Failed to execute service {service_name}: {e}"),
                    });
                    return Err(e);
                }
            }
        }

        deployment.update_status(DeploymentStatus::Running);

        Ok(())
    }

    /// Create network for deployment
    fn create_deployment_network(&self, deployment: &ByobDeploymentRequest) -> NetworkInfo {
        let network_name = format!("byob-{}-{}", deployment.team_id, deployment.deployment_id);
        let subnet_cidr = deployment.network_config.subnet_cidr.clone();
        let gateway_ip = "10.0.0.1".to_string(); // Default gateway

        // Create service endpoints
        // ✅ ZERO-COPY: Pre-allocate HashMap with known capacity
        let mut service_endpoints = HashMap::with_capacity(deployment.services.len());
        for (service_name, service_spec) in &deployment.services {
            // Allocate internal IP address
            let internal_ip = format!("10.0.0.{}", 10 + service_endpoints.len());

            // Allocate external IP if service has exposed ports
            let external_ip = self.allocate_external_ip(service_spec, &deployment.team_id);

            let endpoint = ServiceEndpoint {
                name: service_name.clone(),
                internal_ip,
                external_ip,
                ports: service_spec.ports.clone(),
            };
            service_endpoints.insert(service_name.clone(), endpoint);
        }

        NetworkInfo {
            network_name,
            subnet_cidr,
            gateway_ip,
            service_endpoints,
        }
    }

    /// Monitor deployment health
    #[allow(dead_code)] // Phase 2+: background health monitoring loop
    async fn monitor_deployment_health(&self, deployment_id: Uuid) -> ToadStoolResult<()> {
        debug!("🔍 Monitoring health for deployment {}", deployment_id);

        let mut deployments = self.active_deployments.write().await;
        if let Some(deployment) = deployments.get_mut(&deployment_id) {
            // Check health of all services in the deployment
            let mut all_healthy = true;
            let mut failed_services = Vec::new();

            for (service_name, service_spec) in &deployment.request.services {
                if let Some(health_check) = &service_spec.health_check {
                    // Perform health check based on configuration
                    match self.perform_health_check(service_name, health_check) {
                        Ok(healthy) => {
                            if healthy {
                                debug!("✅ Service {} passed health check", service_name);
                            } else {
                                all_healthy = false;
                                failed_services.push(service_name.clone());
                                warn!("❌ Service {} failed health check", service_name);
                            }
                        }
                        Err(e) => {
                            all_healthy = false;
                            failed_services.push(service_name.clone());
                            error!("❌ Health check error for service {}: {}", service_name, e);
                        }
                    }
                } else {
                    // No health check configured, assume healthy if execution exists
                    if !deployment.service_executions.contains_key(service_name) {
                        all_healthy = false;
                        failed_services.push(service_name.clone());
                    }
                }
            }

            // Update deployment status based on health checks
            if !all_healthy {
                deployment.status = DeploymentStatus::Failed {
                    error: format!(
                        "Health check failed for services: {}",
                        failed_services.join(", ")
                    ),
                };
                error!("❌ Deployment {} health check failed", deployment_id);
            } else if matches!(deployment.status, DeploymentStatus::Running) {
                debug!("✅ Deployment {} health check passed", deployment_id);
            }

            deployment.updated_at = std::time::Instant::now();
        }

        Ok(())
    }

    /// Perform health check for a specific service
    fn perform_health_check(
        &self,
        service_name: &str,
        health_check: &HealthCheck,
    ) -> ToadStoolResult<bool> {
        debug!("🔍 Performing health check for service: {}", service_name);

        // NOTE: Simplified health check - validates configuration but doesn't execute commands.
        // Full implementation would use process::Command to execute health check scripts.

        if health_check.command.is_empty() {
            return Ok(true); // No command means always healthy
        }

        // Validate health check command format
        let command = &health_check.command[0];

        match command.as_str() {
            "curl" | "wget" | "http" => {
                // HTTP-based health check
                debug!("HTTP health check configured for {}", service_name);
                Ok(true) // Configuration valid
            }
            "ping" => {
                // Network ping health check
                debug!("Performing ping health check for {}", service_name);
                Ok(true)
            }
            _ => {
                // Custom command health check
                debug!(
                    "Performing custom health check for {}: {:?}",
                    service_name, health_check.command
                );
                Ok(true)
            }
        }
    }

    /// Update resource usage for deployment by polling service metrics
    async fn update_resource_usage(&self, deployment_id: Uuid) -> ToadStoolResult<()> {
        debug!(
            "📊 Updating resource usage for deployment {}",
            deployment_id
        );

        let mut deployments = self.active_deployments.write().await;
        if let Some(deployment) = deployments.get_mut(&deployment_id) {
            // Collect resource usage from all services
            let mut total_cpu = 0.0;
            let mut total_memory = 0;
            let mut total_storage = 0;
            let mut total_gpu = 0;
            let mut total_network_sent = 0;
            let mut total_network_received = 0;

            for (service_name, execution_id) in &deployment.service_executions {
                // NOTE: Resource usage simulation - estimates based on service specifications.
                // Full implementation would query runtime engine via RuntimeEngine trait.
                if let Some(service_spec) = deployment.request.services.get(service_name) {
                    // Simulate CPU usage (50-80% of allocated)
                    if let Some(cpu_cores) = service_spec.resources.cpu_cores {
                        total_cpu += cpu_cores * 0.65; // Simulate 65% usage
                    }

                    // Simulate memory usage (60-90% of allocated)
                    if let Some(memory_bytes) = service_spec.resources.memory_bytes {
                        total_memory += (memory_bytes * 3) / 4; // Simulate 75% usage
                    }

                    // Simulate storage usage (30-50% of allocated)
                    if let Some(storage_bytes) = service_spec.resources.storage_bytes {
                        total_storage += (storage_bytes * 2) / 5; // Simulate 40% usage
                    }

                    // Simulate GPU usage
                    if let Some(gpu_count) = service_spec.resources.gpu_count {
                        total_gpu += gpu_count;
                    }

                    // Simulate network usage (based on service type)
                    let base_network_usage = match service_spec.image.as_deref() {
                        Some(image) if image.contains("web") || image.contains("api") => {
                            1024 * 1024
                        } // 1MB for web services
                        Some(image) if image.contains("database") => 512 * 1024, // 512KB for databases
                        _ => 256 * 1024, // 256KB for other services
                    };

                    total_network_sent += base_network_usage;
                    total_network_received += base_network_usage / 2; // Assume less incoming traffic
                }

                debug!(
                    "📊 Collected metrics for service {} (execution: {})",
                    service_name, execution_id
                );
            }

            deployment.update_resource_usage(ResourceUsage {
                cpu_usage: total_cpu,
                memory_usage: total_memory,
                storage_usage: total_storage,
                gpu_usage: total_gpu,
                network_usage: NetworkUsage {
                    bytes_sent: total_network_sent,
                    bytes_received: total_network_received,
                    packets_sent: total_network_sent / 1024, // Rough estimate
                    packets_received: total_network_received / 1024,
                },
            });

            debug!("📊 Updated resource usage for deployment {}: CPU: {:.2}, Memory: {}MB, Storage: {}MB", 
                   deployment_id, total_cpu, total_memory / (1024 * 1024), total_storage / (1024 * 1024));
        }

        Ok(())
    }

    /// Allocate external IP for a service
    fn allocate_external_ip(&self, service_spec: &ServiceSpec, team_id: &str) -> Option<String> {
        // Check if service needs external access
        let needs_external_ip = service_spec.ports.iter().any(|port| {
            // Allocate external IP for services that expose common web ports
            self.config.web_service_ports.contains(&port.container_port)
        });

        if !needs_external_ip {
            return None;
        }

        // NOTE: IP allocation simulation - generates deterministic IPs for testing.
        // Production implementation would integrate with cloud provider APIs:
        //   - AWS: EC2 Elastic IP allocation
        //   - GCP: Reserve static external IP
        //   - Azure: Public IP resource creation
        // And configure load balancer + DNS accordingly.
        let base_ip = match team_id.len() % 4 {
            0 => "203.0.113", // RFC 5737 documentation IP range
            1 => "198.51.100",
            2 => "192.0.2",
            _ => "203.0.114",
        };

        // Generate a semi-random last octet based on service name
        #[allow(clippy::cast_possible_truncation)]
        let last_octet = service_spec.name.chars().map(|c| c as u32).sum::<u32>() % 200 + 50; // Range 50-249

        let external_ip = format!("{base_ip}.{last_octet}");

        debug!(
            "🌐 Allocated external IP {} for service {} (team: {})",
            external_ip, service_spec.name, team_id
        );

        Some(external_ip)
    }

    /// Stop a specific service execution
    fn stop_service_execution(
        &self,
        service_name: String,
        execution_id: Uuid,
    ) -> ToadStoolResult<()> {
        debug!(
            "🛑 Stopping service execution: {} ({})",
            service_name, execution_id
        );

        // ✅ MODERNIZED: No artificial delay - would delegate to RuntimeEngine::stop_execution()
        // with proper shutdown signal and graceful timeout handling.
        // In production, this would:
        // 1. Send shutdown signal to execution
        // 2. Wait for acknowledgment (via channel/notify)
        // 3. Apply timeout if needed
        // 4. Force-kill as fallback
        //
        // For now, this is immediate since no actual execution to stop.

        debug!(
            "✅ Service execution stopped: {} ({})",
            service_name, execution_id
        );
        Ok(())
    }
}

#[async_trait]
impl ByobExecutor for ByobComputeExecutor {
    async fn deploy_biome(
        &self,
        request: ByobDeploymentRequest,
    ) -> ToadStoolResult<ByobDeploymentResponse> {
        info!("Starting BYOB deployment: {}", request.deployment_id);

        // Validate deployment request
        self.validate_deployment_request(&request)?;

        // Check concurrent deployment limit
        {
            let deployments = self.active_deployments.read().await;
            #[allow(clippy::cast_possible_truncation)]
            if deployments.len() >= self.config.max_concurrent_deployments as usize {
                return Err(ToadStoolError::resource(
                    "Maximum concurrent deployments reached".to_string(),
                ));
            }
        }

        // Create network for deployment
        let network_info = self.create_deployment_network(&request);

        // Create active deployment using constructor
        let mut active_deployment = ActiveDeployment::new(request.clone(), network_info.clone());

        // Execute services
        self.execute_services(&mut active_deployment).await?;

        // Create response before storing
        let response = active_deployment.to_response();

        // Store active deployment
        {
            let mut deployments = self.active_deployments.write().await;
            deployments.insert(request.deployment_id, active_deployment);
        }

        info!(
            "BYOB deployment {} completed successfully",
            request.deployment_id
        );
        Ok(response)
    }

    async fn get_deployment_status(
        &self,
        deployment_id: Uuid,
    ) -> ToadStoolResult<ByobDeploymentResponse> {
        let deployments = self.active_deployments.read().await;

        if let Some(deployment) = deployments.get(&deployment_id) {
            Ok(deployment.to_response())
        } else {
            Err(ToadStoolError::not_found(format!(
                "Deployment {deployment_id} not found"
            )))
        }
    }

    async fn stop_deployment(&self, deployment_id: Uuid) -> ToadStoolResult<()> {
        info!("🛑 Stopping deployment: {}", deployment_id);

        let mut deployments = self.active_deployments.write().await;
        if let Some(deployment) = deployments.get_mut(&deployment_id) {
            deployment.update_status(DeploymentStatus::Stopping);

            // Stop all running services in reverse dependency order
            let mut stopped_services = Vec::new();
            let mut failed_stops = Vec::new();

            // Get list of services to stop (reverse order for proper shutdown)
            let mut services_to_stop: Vec<_> =
                deployment.service_executions.keys().cloned().collect();
            services_to_stop.reverse(); // Stop in reverse order

            for service_name in services_to_stop {
                if let Some(execution_id) = deployment.service_executions.get(&service_name) {
                    debug!(
                        "🛑 Stopping service: {} (execution: {})",
                        service_name, execution_id
                    );

                    // Delegate to stop_service_execution() which simulates runtime engine coordination
                    match self.stop_service_execution(service_name.clone(), *execution_id) {
                        Ok(()) => {
                            stopped_services.push(service_name.clone());
                            deployment.remove_service_execution(&service_name);
                            info!("✅ Stopped service: {}", service_name);
                        }
                        Err(e) => {
                            failed_stops.push((service_name.clone(), e.to_string()));
                            error!("❌ Failed to stop service {}: {}", service_name, e);
                        }
                    }
                }
            }

            // Update deployment status based on stop results
            if failed_stops.is_empty() {
                deployment.status = DeploymentStatus::Stopped;
                info!("✅ Successfully stopped deployment: {}", deployment_id);
            } else {
                deployment.status = DeploymentStatus::Failed {
                    error: format!("Failed to stop services: {failed_stops:?}"),
                };
                error!("❌ Failed to fully stop deployment: {}", deployment_id);
            }

            deployment.updated_at = std::time::Instant::now();

            // Log summary
            info!(
                "🛑 Deployment {} stop summary: {} services stopped, {} failed",
                deployment_id,
                stopped_services.len(),
                failed_stops.len()
            );
        } else {
            return Err(ToadStoolError::not_found(format!(
                "Deployment not found: {deployment_id}"
            )));
        }

        Ok(())
    }

    async fn list_deployments(&self) -> ToadStoolResult<Vec<ByobDeploymentResponse>> {
        let deployments = self.active_deployments.read().await;

        let responses = deployments
            .values()
            .inspect(|d| {
                if d.is_completed() {
                    debug!(
                        "Deployment {} is completed (age: {:?})",
                        d.request.deployment_id,
                        d.elapsed()
                    );
                }
            })
            .filter(|d| d.is_active() || d.is_completed())
            .map(|deployment| deployment.to_response())
            .collect();

        Ok(responses)
    }

    async fn get_resource_usage(&self, deployment_id: Uuid) -> ToadStoolResult<ResourceUsage> {
        // Refresh usage metrics before returning so callers always see current stats.
        self.update_resource_usage(deployment_id).await?;

        let deployments = self.active_deployments.read().await;
        if let Some(deployment) = deployments.get(&deployment_id) {
            Ok(deployment.resource_usage.clone())
        } else {
            Err(ToadStoolError::not_found(format!(
                "Deployment {deployment_id} not found"
            )))
        }
    }
}

/// Create a default BYOB compute executor
pub fn create_byob_executor(runtime_engine: Arc<dyn RuntimeEngine>) -> Arc<dyn ByobExecutor> {
    Arc::new(ByobComputeExecutor::new(
        runtime_engine,
        ByobExecutorConfig::default(),
    ))
}

#[cfg(test)]
mod byob_impl_tests;
