// SPDX-License-Identifier: AGPL-3.0-or-later
//! BYOB compute executor implementation

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::sync::RwLock;
use tracing::{debug, error, info};
use uuid::Uuid;

#[cfg(test)]
pub(crate) use super::byob_types::{
    ByobDeploymentRequest, ByobDeploymentResponse, DeploymentStatus, HealthCheck, ResourceUsage,
    ServiceSpec,
};
#[cfg(not(test))]
use super::byob_types::{
    ByobDeploymentRequest, ByobDeploymentResponse, DeploymentStatus, ResourceUsage,
};
use super::config::ByobExecutorConfig;
use super::deployment::ActiveDeployment;
use super::resource_metrics::{ResourceMetricsReader, merge_sample_with_gpu};
use super::validation::DeploymentValidator;

#[cfg(test)]
pub(crate) use crate::{
    ExecutionRequest, ExecutionStatus, RuntimeEngine, StubRuntimeEngine, ToadStoolError,
    ToadStoolResult,
};
#[cfg(not(test))]
use crate::{RuntimeEngine, StubRuntimeEngine, ToadStoolError, ToadStoolResult};

mod deployment_lifecycle;

/// BYOB compute executor for team biome deployment and lifecycle management.
pub struct ByobComputeExecutor<E: RuntimeEngine + 'static> {
    /// Runtime engine for executing workloads
    runtime_engine: Arc<E>,
    /// Active deployments
    active_deployments: Arc<RwLock<HashMap<Uuid, ActiveDeployment>>>,
    /// Configuration
    config: ByobExecutorConfig,
    /// Background health monitor handles (per deployment).
    health_handles: Arc<RwLock<HashMap<Uuid, tokio::task::JoinHandle<()>>>>,
}

/// BYOB executor trait
pub trait ByobExecutor: Send + Sync {
    /// Deploy a team biome
    fn deploy_biome(
        &self,
        request: ByobDeploymentRequest,
    ) -> impl Future<Output = ToadStoolResult<ByobDeploymentResponse>> + Send + '_;

    /// Get deployment status
    fn get_deployment_status(
        &self,
        deployment_id: Uuid,
    ) -> impl Future<Output = ToadStoolResult<ByobDeploymentResponse>> + Send + '_;

    /// Stop a deployment
    fn stop_deployment(
        &self,
        deployment_id: Uuid,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + '_;

    /// List active deployments
    fn list_deployments(
        &self,
    ) -> impl Future<Output = ToadStoolResult<Vec<ByobDeploymentResponse>>> + Send + '_;

    /// Get resource usage for a deployment
    fn get_resource_usage(
        &self,
        deployment_id: Uuid,
    ) -> impl Future<Output = ToadStoolResult<ResourceUsage>> + Send + '_;
}

/// Dispatches [`ByobExecutor`] to concrete implementations (enum dispatch).
pub enum ByobExecutorDispatch<E: RuntimeEngine + 'static = StubRuntimeEngine> {
    /// Production compute executor.
    Compute(ByobComputeExecutor<E>),
}

impl<E: RuntimeEngine + 'static> ByobExecutor for ByobExecutorDispatch<E> {
    fn deploy_biome(
        &self,
        request: ByobDeploymentRequest,
    ) -> impl Future<Output = ToadStoolResult<ByobDeploymentResponse>> + Send + '_ {
        match self {
            Self::Compute(executor) => executor.deploy_biome(request),
        }
    }

    fn get_deployment_status(
        &self,
        deployment_id: Uuid,
    ) -> impl Future<Output = ToadStoolResult<ByobDeploymentResponse>> + Send + '_ {
        match self {
            Self::Compute(executor) => executor.get_deployment_status(deployment_id),
        }
    }

    fn stop_deployment(
        &self,
        deployment_id: Uuid,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        match self {
            Self::Compute(executor) => executor.stop_deployment(deployment_id),
        }
    }

    fn list_deployments(
        &self,
    ) -> impl Future<Output = ToadStoolResult<Vec<ByobDeploymentResponse>>> + Send + '_ {
        match self {
            Self::Compute(executor) => executor.list_deployments(),
        }
    }

    fn get_resource_usage(
        &self,
        deployment_id: Uuid,
    ) -> impl Future<Output = ToadStoolResult<ResourceUsage>> + Send + '_ {
        match self {
            Self::Compute(executor) => executor.get_resource_usage(deployment_id),
        }
    }
}

impl<E: RuntimeEngine + 'static> ByobComputeExecutor<E> {
    /// Create a new BYOB compute executor
    pub fn new(runtime_engine: Arc<E>, config: ByobExecutorConfig) -> Self {
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
            .write().unwrap_or_else(|e| e.into_inner())
            .get_mut(&deployment_id)
        {
            let pid = std::process::id();
            let reader = ResourceMetricsReader::new();
            let prev = deployment.resource_poll_state.as_ref();
            let (sample, new_state) = reader.sample(pid, prev);
            deployment.resource_poll_state = Some(new_state);

            let usage = merge_sample_with_gpu(sample, &deployment.request.services);
            let cpu_total = usage.cpu_usage;
            let total_memory = usage.memory_usage;
            let total_storage = usage.storage_usage;

            deployment.update_resource_usage(usage);

            for (service_name, execution_id) in &deployment.service_executions {
                debug!(
                    "📊 Collected metrics for service {} (execution: {})",
                    service_name, execution_id
                );
            }

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

impl<E: RuntimeEngine + 'static> ByobExecutor for ByobComputeExecutor<E> {
    fn deploy_biome(
        &self,
        request: ByobDeploymentRequest,
    ) -> impl Future<Output = ToadStoolResult<ByobDeploymentResponse>> + Send + '_ {
        async move {
            info!("Starting BYOB deployment: {}", request.deployment_id);

            // Validate deployment request
            DeploymentValidator::validate_deployment(&request)?;

            // Check concurrent deployment limit
            {
                let deployments = self.active_deployments.read().unwrap_or_else(|e| e.into_inner());
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
                let mut deployments = self.active_deployments.write().unwrap_or_else(|e| e.into_inner());
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
    }

    fn get_deployment_status(
        &self,
        deployment_id: Uuid,
    ) -> impl Future<Output = ToadStoolResult<ByobDeploymentResponse>> + Send + '_ {
        async move {
            self.active_deployments
                .read().unwrap_or_else(|e| e.into_inner())
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
    }

    fn stop_deployment(
        &self,
        deployment_id: Uuid,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async move {
            info!("🛑 Stopping deployment: {}", deployment_id);

            // Cancel background health monitor for this deployment
            let removed = self.health_handles.write().unwrap_or_else(|e| e.into_inner()).remove(&deployment_id);
            if let Some(handle) = removed {
                handle.abort();
            }

            if let Some(deployment) = self
                .active_deployments
                .write().unwrap_or_else(|e| e.into_inner())
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
    }

    fn list_deployments(
        &self,
    ) -> impl Future<Output = ToadStoolResult<Vec<ByobDeploymentResponse>>> + Send + '_ {
        async move {
            let responses = self
                .active_deployments
                .read().unwrap_or_else(|e| e.into_inner())
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
    }

    fn get_resource_usage(
        &self,
        deployment_id: Uuid,
    ) -> impl Future<Output = ToadStoolResult<ResourceUsage>> + Send + '_ {
        async move {
            // Refresh usage metrics before returning so callers always see current stats.
            self.update_resource_usage(deployment_id).await?;

            self.active_deployments
                .read().unwrap_or_else(|e| e.into_inner())
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
}

/// Create a default BYOB compute executor
pub fn create_byob_executor<E: RuntimeEngine + 'static>(
    runtime_engine: Arc<E>,
) -> Arc<ByobExecutorDispatch<E>> {
    Arc::new(ByobExecutorDispatch::Compute(ByobComputeExecutor::new(
        runtime_engine,
        ByobExecutorConfig::default(),
    )))
}

#[cfg(test)]
mod byob_impl_tests;
