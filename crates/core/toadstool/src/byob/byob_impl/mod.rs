// SPDX-License-Identifier: AGPL-3.0-only
//! BYOB compute executor implementation

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};
use uuid::Uuid;

#[cfg(test)]
pub(crate) use super::byob_types::{
    ByobDeploymentRequest, ByobDeploymentResponse, DeploymentStatus, HealthCheck, NetworkUsage,
    ResourceUsage, ServiceSpec,
};
#[cfg(not(test))]
use super::byob_types::{
    ByobDeploymentRequest, ByobDeploymentResponse, DeploymentStatus, NetworkUsage, ResourceUsage,
};
use super::config::ByobExecutorConfig;
use super::deployment::ActiveDeployment;
use super::validation::DeploymentValidator;

#[cfg(test)]
pub(crate) use crate::{
    ExecutionRequest, ExecutionStatus, RuntimeEngine, ToadStoolError, ToadStoolResult,
};
#[cfg(not(test))]
use crate::{RuntimeEngine, ToadStoolError, ToadStoolResult};

mod deployment_lifecycle;

/// BYOB compute executor for team biome deployment and lifecycle management.
pub struct ByobComputeExecutor {
    /// Runtime engine for executing workloads
    runtime_engine: Arc<dyn RuntimeEngine>,
    /// Active deployments
    active_deployments: Arc<RwLock<HashMap<Uuid, ActiveDeployment>>>,
    /// Configuration
    config: ByobExecutorConfig,
    /// Background health monitor handles (per deployment).
    health_handles: Arc<RwLock<HashMap<Uuid, tokio::task::JoinHandle<()>>>>,
}

/// BYOB executor trait
// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
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
            health_handles: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Update resource usage for deployment by polling service metrics
    async fn update_resource_usage(&self, deployment_id: Uuid) -> ToadStoolResult<()> {
        debug!(
            "📊 Updating resource usage for deployment {}",
            deployment_id
        );

        if let Some(deployment) = self
            .active_deployments
            .write()
            .await
            .get_mut(&deployment_id)
        {
            // Collect resource usage from all services
            let mut cpu_total = 0.0;
            let mut total_memory = 0;
            let mut total_storage = 0;
            let mut gpu_total = 0;
            let mut total_network_sent = 0;
            let mut total_network_received = 0;

            for (service_name, execution_id) in &deployment.service_executions {
                // NOTE: Resource usage simulation - estimates based on service specifications.
                // Full implementation would query runtime engine via RuntimeEngine trait.
                if let Some(service_spec) = deployment.request.services.get(service_name) {
                    // Simulate CPU usage (50-80% of allocated)
                    if let Some(cpu_cores) = service_spec.resources.cpu_cores {
                        cpu_total += cpu_cores * 0.65; // Simulate 65% usage
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
                        gpu_total += gpu_count;
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
                cpu_usage: cpu_total,
                memory_usage: total_memory,
                storage_usage: total_storage,
                gpu_usage: gpu_total,
                network_usage: NetworkUsage {
                    bytes_sent: total_network_sent,
                    bytes_received: total_network_received,
                    packets_sent: total_network_sent / 1024, // Rough estimate
                    packets_received: total_network_received / 1024,
                },
            });

            debug!(
                "📊 Updated resource usage for deployment {}: CPU: {:.2}, Memory: {}MB, Storage: {}MB",
                deployment_id,
                cpu_total,
                total_memory / (1024 * 1024),
                total_storage / (1024 * 1024)
            );
        }

        Ok(())
    }
}

// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
impl ByobExecutor for ByobComputeExecutor {
    async fn deploy_biome(
        &self,
        request: ByobDeploymentRequest,
    ) -> ToadStoolResult<ByobDeploymentResponse> {
        info!("Starting BYOB deployment: {}", request.deployment_id);

        // Validate deployment request
        DeploymentValidator::validate_deployment(&request)?;

        // Check concurrent deployment limit
        {
            let deployments = self.active_deployments.read().await;
            if deployments.len() >= self.config.max_concurrent_deployments as usize {
                return Err(ToadStoolError::resource(
                    "Maximum concurrent deployments reached".to_string(),
                ));
            }
        }

        // Create network for deployment
        let network_info = self.create_deployment_network(&request);

        // Create active deployment (move request/network_info — no clones)
        let mut active_deployment = ActiveDeployment::new(request, network_info);

        // Execute services (updates status to Running)
        self.execute_services(&mut active_deployment).await?;

        // Create response before storing
        let response = active_deployment.to_response();

        let dep_id = active_deployment.request.deployment_id;

        // Store active deployment
        {
            let mut deployments = self.active_deployments.write().await;
            deployments.insert(dep_id, active_deployment);
        }

        // Spawn background health monitor for this deployment
        self.spawn_health_monitor(dep_id);

        info!(
            "BYOB deployment {} completed successfully",
            response.deployment_id
        );
        Ok(response)
    }

    async fn get_deployment_status(
        &self,
        deployment_id: Uuid,
    ) -> ToadStoolResult<ByobDeploymentResponse> {
        self.active_deployments
            .read()
            .await
            .get(&deployment_id)
            .map_or_else(
                || {
                    Err(ToadStoolError::not_found(format!(
                        "Deployment {deployment_id} not found"
                    )))
                },
                |deployment| Ok(deployment.to_response()),
            )
    }

    async fn stop_deployment(&self, deployment_id: Uuid) -> ToadStoolResult<()> {
        info!("🛑 Stopping deployment: {}", deployment_id);

        // Cancel background health monitor for this deployment
        let removed = self.health_handles.write().await.remove(&deployment_id);
        if let Some(handle) = removed {
            handle.abort();
        }

        if let Some(deployment) = self
            .active_deployments
            .write()
            .await
            .get_mut(&deployment_id)
        {
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
                    match self.stop_service_execution(&service_name, *execution_id) {
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
        let responses = self
            .active_deployments
            .read()
            .await
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
            .map(ActiveDeployment::to_response)
            .collect();

        Ok(responses)
    }

    async fn get_resource_usage(&self, deployment_id: Uuid) -> ToadStoolResult<ResourceUsage> {
        // Refresh usage metrics before returning so callers always see current stats.
        self.update_resource_usage(deployment_id).await?;

        self.active_deployments
            .read()
            .await
            .get(&deployment_id)
            .map_or_else(
                || {
                    Err(ToadStoolError::not_found(format!(
                        "Deployment {deployment_id} not found"
                    )))
                },
                |deployment| Ok(deployment.resource_usage.clone()),
            )
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
