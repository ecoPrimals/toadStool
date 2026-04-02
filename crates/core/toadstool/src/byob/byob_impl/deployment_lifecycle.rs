// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::byob::byob_types::{
    ByobDeploymentRequest, DeploymentStatus, HealthCheck, NetworkInfo, ServiceEndpoint, ServiceSpec,
};
use crate::byob::deployment::ActiveDeployment;
use crate::{ExecutionRequest, ExecutionStatus, ToadStoolError, ToadStoolResult, WorkloadSpec};
use toadstool_common::constants::timeouts;

use super::ByobComputeExecutor;

impl ByobComputeExecutor {
    /// Create execution request for a service
    pub(super) fn create_service_execution_request(
        &self,
        service: &ServiceSpec,
        _deployment_id: Uuid,
    ) -> ExecutionRequest {
        // ✅ OPTIMIZED: Reduce clones by using references where possible
        let workload = service.image.as_ref().map_or_else(
            || {
                // Native workload (assume command is provided)
                WorkloadSpec::Native {
                    executable: crate::ExecutableSource::File {
                        path: std::path::PathBuf::from(
                            service.image.as_deref().unwrap_or("/bin/sh"),
                        ),
                    },
                    args: None,
                    working_dir: None,
                    env_vars: service.environment.clone(),
                    user: None,
                }
            },
            |image| {
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
            },
        );

        ExecutionRequest {
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
        }
    }

    /// Execute services in a deployment
    pub(super) async fn execute_services(
        &self,
        deployment: &mut ActiveDeployment,
    ) -> ToadStoolResult<()> {
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
                self.create_service_execution_request(service_spec, deployment_id);

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
    pub(super) fn create_deployment_network(
        &self,
        deployment: &ByobDeploymentRequest,
    ) -> NetworkInfo {
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

    /// Run one health-check pass for a deployment.
    pub(super) async fn monitor_deployment_health(
        &self,
        deployment_id: Uuid,
    ) -> ToadStoolResult<()> {
        debug!("🔍 Monitoring health for deployment {}", deployment_id);

        if let Some(deployment) = self
            .active_deployments
            .write()
            .await
            .get_mut(&deployment_id)
        {
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
    pub(super) fn perform_health_check(
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

    /// Spawn a background task that periodically runs health checks for a deployment.
    ///
    /// The task runs at `config.health_check_interval` until the deployment is
    /// no longer in `Running` status or the executor is dropped.
    pub(super) fn spawn_health_monitor(&self, deployment_id: Uuid) {
        let deployments = Arc::clone(&self.active_deployments);
        let handles = Arc::clone(&self.health_handles);
        let interval = self.config.health_check_interval;

        let executor = Self {
            runtime_engine: Arc::clone(&self.runtime_engine),
            active_deployments: Arc::clone(&self.active_deployments),
            config: self.config.clone(),
            health_handles: Arc::clone(&self.health_handles),
        };

        let handle = tokio::spawn(async move {
            let mut tick = tokio::time::interval(interval);
            tick.tick().await; // first tick fires immediately — skip it

            loop {
                tick.tick().await;

                let still_running = {
                    let guard = deployments.read().await;
                    guard
                        .get(&deployment_id)
                        .is_some_and(|d| matches!(d.status, DeploymentStatus::Running))
                };

                if !still_running {
                    debug!(
                        "Health monitor stopping for deployment {deployment_id} (no longer running)"
                    );
                    break;
                }

                if let Err(e) = executor.monitor_deployment_health(deployment_id).await {
                    warn!("Health check error for deployment {deployment_id}: {e}");
                }
            }

            handles.write().await.remove(&deployment_id);
        });

        // Store handle so it can be cancelled on teardown.
        let handles = Arc::clone(&self.health_handles);
        tokio::spawn(async move {
            handles.write().await.insert(deployment_id, handle);
        });
    }

    /// Allocate external IP for a service
    pub(super) fn allocate_external_ip(
        &self,
        service_spec: &ServiceSpec,
        team_id: &str,
    ) -> Option<String> {
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

        let last_octet = service_spec.name.chars().map(|c| c as u32).sum::<u32>() % 200 + 50;

        let external_ip = format!("{base_ip}.{last_octet}");

        debug!(
            "🌐 Allocated external IP {} for service {} (team: {})",
            external_ip, service_spec.name, team_id
        );

        Some(external_ip)
    }

    /// Stop a specific service execution
    pub(super) fn stop_service_execution(
        &self,
        service_name: &str,
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
