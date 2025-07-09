//! # BYOB (Bring Your Own Biome) Compute Execution
//!
//! Handles compute execution requests for team biome deployments.
//! Receives requests from Songbird and executes team services using Toadstool's
//! universal compute capabilities.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{
    ExecutionRequest, ExecutionResponse, ExecutionStatus, RuntimeEngine, ToadStoolError,
    ToadStoolResult, WorkloadSpec, WorkloadType,
};

/// BYOB deployment request from Songbird
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByobDeploymentRequest {
    /// Unique deployment ID
    pub deployment_id: Uuid,
    /// Team identifier
    pub team_id: String,
    /// Deployment name
    pub deployment_name: String,
    /// Services to deploy
    pub services: HashMap<String, ServiceSpec>,
    /// Resource quotas for the team
    pub resource_quotas: TeamResourceQuotas,
    /// Security configuration
    pub security_config: TeamSecurityConfig,
    /// Network configuration
    pub network_config: TeamNetworkConfig,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
}

/// Service specification within a team deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceSpec {
    /// Service name
    pub name: String,
    /// Service version
    pub version: String,
    /// Container image or executable
    pub image: Option<String>,
    /// Command to run
    pub command: Option<Vec<String>>,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// Resource requirements
    pub resources: ServiceResourceRequirements,
    /// Port mappings
    pub ports: Vec<PortMapping>,
    /// Volume mounts
    pub volumes: Vec<VolumeMount>,
    /// Dependencies on other services
    pub dependencies: Vec<String>,
    /// Health check configuration
    pub health_check: Option<HealthCheck>,
    /// Replica count
    pub replicas: u32,
}

/// Resource requirements for a service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceResourceRequirements {
    /// CPU cores required
    pub cpu_cores: Option<f64>,
    /// Memory in bytes
    pub memory_bytes: Option<u64>,
    /// Storage in bytes
    pub storage_bytes: Option<u64>,
    /// GPU count
    pub gpu_count: Option<u32>,
}

/// Team resource quotas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamResourceQuotas {
    /// Maximum CPU cores for the team
    pub max_cpu_cores: f64,
    /// Maximum memory in bytes
    pub max_memory_bytes: u64,
    /// Maximum storage in bytes
    pub max_storage_bytes: u64,
    /// Maximum GPU count
    pub max_gpu_count: u32,
    /// Maximum concurrent services
    pub max_concurrent_services: u32,
}

/// Team security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamSecurityConfig {
    /// Isolation level
    pub isolation_level: String,
    /// Network policies
    pub network_policies: Vec<String>,
    /// Volume access policies
    pub volume_policies: Vec<String>,
    /// Resource access policies
    pub resource_policies: Vec<String>,
}

/// Team network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamNetworkConfig {
    /// Network name
    pub network_name: String,
    /// Subnet CIDR
    pub subnet_cidr: String,
    /// DNS configuration
    pub dns_config: Option<DnsConfig>,
    /// Load balancer configuration
    pub load_balancer: Option<LoadBalancerConfig>,
}

/// Port mapping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    /// Container port
    pub container_port: u16,
    /// Host port (optional)
    pub host_port: Option<u16>,
    /// Protocol (tcp/udp)
    pub protocol: String,
}

/// Volume mount configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMount {
    /// Source path
    pub source: String,
    /// Target path in container
    pub target: String,
    /// Mount type
    pub mount_type: String,
    /// Read-only flag
    pub read_only: bool,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Health check command
    pub command: Vec<String>,
    /// Check interval in seconds
    pub interval: u64,
    /// Timeout in seconds
    pub timeout: u64,
    /// Number of retries
    pub retries: u32,
    /// Start period in seconds
    pub start_period: u64,
}

/// DNS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsConfig {
    /// DNS servers
    pub servers: Vec<String>,
    /// Search domains
    pub search_domains: Vec<String>,
}

/// Load balancer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerConfig {
    /// Load balancer type
    pub lb_type: String,
    /// Configuration options
    pub options: HashMap<String, String>,
}

/// BYOB deployment response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByobDeploymentResponse {
    /// Deployment ID
    pub deployment_id: Uuid,
    /// Deployment status
    pub status: DeploymentStatus,
    /// Service statuses
    pub service_statuses: HashMap<String, ServiceStatus>,
    /// Resource usage
    pub resource_usage: ResourceUsage,
    /// Network information
    pub network_info: NetworkInfo,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
}

/// Deployment status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeploymentStatus {
    /// Deployment is starting
    Starting,
    /// Deployment is running
    Running,
    /// Deployment is stopping
    Stopping,
    /// Deployment is stopped
    Stopped,
    /// Deployment failed
    Failed { error: String },
}

/// Service status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatus {
    /// Service name
    pub name: String,
    /// Service state
    pub state: String,
    /// Running replicas
    pub running_replicas: u32,
    /// Desired replicas
    pub desired_replicas: u32,
    /// Health status
    pub health: String,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
}

/// Resource usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// CPU usage
    pub cpu_usage: f64,
    /// Memory usage in bytes
    pub memory_usage: u64,
    /// Storage usage in bytes
    pub storage_usage: u64,
    /// GPU usage
    pub gpu_usage: u32,
    /// Network usage
    pub network_usage: NetworkUsage,
}

/// Network usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkUsage {
    /// Bytes sent
    pub bytes_sent: u64,
    /// Bytes received
    pub bytes_received: u64,
    /// Packets sent
    pub packets_sent: u64,
    /// Packets received
    pub packets_received: u64,
}

/// Network information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    /// Network name
    pub network_name: String,
    /// Subnet CIDR
    pub subnet_cidr: String,
    /// Gateway IP
    pub gateway_ip: String,
    /// Service endpoints
    pub service_endpoints: HashMap<String, ServiceEndpoint>,
}

/// Service endpoint information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    /// Service name
    pub name: String,
    /// Internal IP
    pub internal_ip: String,
    /// External IP (if exposed)
    pub external_ip: Option<String>,
    /// Port mappings
    pub ports: Vec<PortMapping>,
}

/// BYOB compute executor - handles team biome deployments
pub struct ByobComputeExecutor {
    /// Runtime engine for executing workloads
    runtime_engine: Arc<dyn RuntimeEngine>,
    /// Active deployments
    active_deployments: Arc<RwLock<HashMap<Uuid, ActiveDeployment>>>,
    /// Configuration
    config: ByobExecutorConfig,
}

/// Active deployment tracking
#[derive(Debug)]
struct ActiveDeployment {
    /// Deployment request
    request: ByobDeploymentRequest,
    /// Deployment status
    status: DeploymentStatus,
    /// Service execution IDs
    service_executions: HashMap<String, Uuid>,
    /// Resource usage tracking
    resource_usage: ResourceUsage,
    /// Network information
    network_info: NetworkInfo,
    /// Created timestamp
    created_at: Instant,
    /// Updated timestamp
    updated_at: Instant,
}

/// BYOB executor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByobExecutorConfig {
    /// Maximum concurrent deployments
    pub max_concurrent_deployments: u32,
    /// Default network subnet
    pub default_network_subnet: String,
    /// Resource monitoring interval
    pub resource_monitoring_interval: Duration,
    /// Health check interval
    pub health_check_interval: Duration,
    /// Deployment timeout
    pub deployment_timeout: Duration,
}

impl Default for ByobExecutorConfig {
    fn default() -> Self {
        Self {
            max_concurrent_deployments: 50,
            default_network_subnet: "10.0.0.0/24".to_string(),
            resource_monitoring_interval: Duration::from_secs(30),
            health_check_interval: Duration::from_secs(10),
            deployment_timeout: Duration::from_secs(600), // 10 minutes
        }
    }
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
    pub fn new(
        runtime_engine: Arc<dyn RuntimeEngine>,
        config: ByobExecutorConfig,
    ) -> Self {
        Self {
            runtime_engine,
            active_deployments: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Validate deployment request
    fn validate_deployment_request(&self, request: &ByobDeploymentRequest) -> ToadStoolResult<()> {
        // Check resource quotas
        let total_cpu: f64 = request.services.values()
            .map(|s| s.resources.cpu_cores.unwrap_or(0.0))
            .sum();
        let total_memory: u64 = request.services.values()
            .map(|s| s.resources.memory_bytes.unwrap_or(0))
            .sum();
        let total_storage: u64 = request.services.values()
            .map(|s| s.resources.storage_bytes.unwrap_or(0))
            .sum();
        let total_gpu: u32 = request.services.values()
            .map(|s| s.resources.gpu_count.unwrap_or(0))
            .sum();

        if total_cpu > request.resource_quotas.max_cpu_cores {
            return Err(ToadStoolError::resource(format!(
                "CPU requirement {} exceeds team quota {}",
                total_cpu, request.resource_quotas.max_cpu_cores
            )));
        }

        if total_memory > request.resource_quotas.max_memory_bytes {
            return Err(ToadStoolError::resource(format!(
                "Memory requirement {} exceeds team quota {}",
                total_memory, request.resource_quotas.max_memory_bytes
            )));
        }

        if total_storage > request.resource_quotas.max_storage_bytes {
            return Err(ToadStoolError::resource(format!(
                "Storage requirement {} exceeds team quota {}",
                total_storage, request.resource_quotas.max_storage_bytes
            )));
        }

        if total_gpu > request.resource_quotas.max_gpu_count {
            return Err(ToadStoolError::resource(format!(
                "GPU requirement {} exceeds team quota {}",
                total_gpu, request.resource_quotas.max_gpu_count
            )));
        }

        if request.services.len() > request.resource_quotas.max_concurrent_services as usize {
            return Err(ToadStoolError::resource(format!(
                "Service count {} exceeds team quota {}",
                request.services.len(), request.resource_quotas.max_concurrent_services
            )));
        }

        Ok(())
    }

    /// Create execution request for a service
    fn create_service_execution_request(
        &self,
        service: &ServiceSpec,
        deployment_id: Uuid,
    ) -> ToadStoolResult<ExecutionRequest> {
        let workload = if let Some(image) = &service.image {
            // Container workload
            WorkloadSpec::Container {
                image: image.clone(),
                command: service.command.clone(),
                args: None,
                working_dir: None,
                user: None,
                volumes: service.volumes.clone().into_iter().map(|v| {
                    crate::workload::VolumeMount {
                        source: v.source,
                        target: v.target,
                        mount_type: match v.mount_type.as_str() {
                            "bind" => crate::workload::VolumeMountType::Bind,
                            "volume" => crate::workload::VolumeMountType::Volume,
                            _ => crate::workload::VolumeMountType::Bind,
                        },
                        read_only: v.read_only,
                    }
                }).collect(),
                ports: service.ports.clone().into_iter().map(|p| {
                    crate::workload::PortMapping {
                        container_port: p.container_port,
                        host_port: p.host_port.unwrap_or(8080),
                        protocol: p.protocol,
                    }
                }).collect(),
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
            resources: crate::ResourceRequirements {
                cpu: crate::CpuRequirements {
                    min_cores: service.resources.cpu_cores.unwrap_or(1.0),
                    architecture: None,
                    min_frequency_mhz: None,
                    required_features: Vec::new(),
                    max_cores: service.resources.cpu_cores,
                },
                memory: crate::MemoryRequirements {
                    min_bytes: service.resources.memory_bytes.unwrap_or(1024 * 1024 * 1024),
                    memory_type: None,
                    allow_swap: true,
                    max_bytes: service.resources.memory_bytes,
                },
                storage: crate::StorageRequirements {
                    min_bytes: service.resources.storage_bytes.unwrap_or(10 * 1024 * 1024 * 1024),
                    min_bandwidth_mbps: None,
                    min_iops: None,
                    storage_type: None,
                    max_bytes: service.resources.storage_bytes,
                },
                gpu: service.resources.gpu_count.map(|count| crate::GpuRequirements {
                    min_memory_mb: 1024,
                    min_compute_capability: None,
                    min_gpu_count: count,
                    vendor_preference: None,
                    requires_cuda: false,
                    requires_opencl: false,
                }),
                network: crate::NetworkRequirements::default(),
                custom: std::collections::HashMap::new(),
            },
            security_context: crate::SecurityContext::default(),
            timeout: Some(Duration::from_secs(300)), // 5 minutes
            environment: service.environment.clone(),
            input_data: crate::ExecutionInput::default(),
            callback_config: None,
        };

        Ok(execution_request)
    }

    /// Execute services in a deployment
    async fn execute_services(
        &self,
        deployment: &mut ActiveDeployment,
    ) -> ToadStoolResult<()> {
        info!("Starting service execution for deployment {}", deployment.request.deployment_id);

        // Execute services in dependency order
        for (service_name, service_spec) in &deployment.request.services {
            debug!("Executing service: {}", service_name);

            let execution_request = self.create_service_execution_request(
                service_spec,
                deployment.request.deployment_id,
            )?;

            let execution_id = execution_request.execution_id;
            
            // Execute the service
            match self.runtime_engine.execute(execution_request).await {
                Ok(response) => {
                    if response.status == ExecutionStatus::Success {
                        deployment.service_executions.insert(service_name.clone(), execution_id);
                        info!("Service {} started successfully", service_name);
                    } else {
                        warn!("Service {} failed to start: {:?}", service_name, response);
                        deployment.status = DeploymentStatus::Failed {
                            error: format!("Service {} failed to start", service_name),
                        };
                        return Err(ToadStoolError::runtime(format!(
                            "Service {} failed to start",
                            service_name
                        )));
                    }
                }
                Err(e) => {
                    error!("Failed to execute service {}: {:?}", service_name, e);
                    deployment.status = DeploymentStatus::Failed {
                        error: format!("Failed to execute service {}: {}", service_name, e),
                    };
                    return Err(e);
                }
            }
        }

        deployment.status = DeploymentStatus::Running;
        deployment.updated_at = Instant::now();

        Ok(())
    }

    /// Create network for deployment
    fn create_deployment_network(&self, deployment: &ByobDeploymentRequest) -> NetworkInfo {
        let network_name = format!("byob-{}-{}", deployment.team_id, deployment.deployment_id);
        let subnet_cidr = deployment.network_config.subnet_cidr.clone();
        let gateway_ip = "10.0.0.1".to_string(); // Default gateway

        // Create service endpoints
        let mut service_endpoints = HashMap::new();
        for (service_name, service_spec) in &deployment.services {
            let endpoint = ServiceEndpoint {
                name: service_name.clone(),
                internal_ip: format!("10.0.0.{}", 10 + service_endpoints.len()), // Simple IP allocation
                external_ip: None, // TODO: Implement external IP allocation
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
    async fn monitor_deployment_health(&self, deployment_id: Uuid) -> ToadStoolResult<()> {
        // TODO: Implement health monitoring
        // This would check service health endpoints and update deployment status
        Ok(())
    }

    /// Update resource usage for deployment
    async fn update_resource_usage(&self, deployment_id: Uuid) -> ToadStoolResult<()> {
        // TODO: Implement resource usage monitoring
        // This would collect CPU, memory, storage, GPU, and network usage
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
            if deployments.len() >= self.config.max_concurrent_deployments as usize {
                return Err(ToadStoolError::resource(
                    "Maximum concurrent deployments reached".to_string(),
                ));
            }
        }

        // Create network for deployment
        let network_info = self.create_deployment_network(&request);

        // Create active deployment
        let mut active_deployment = ActiveDeployment {
            request: request.clone(),
            status: DeploymentStatus::Starting,
            service_executions: HashMap::new(),
            resource_usage: ResourceUsage {
                cpu_usage: 0.0,
                memory_usage: 0,
                storage_usage: 0,
                gpu_usage: 0,
                network_usage: NetworkUsage {
                    bytes_sent: 0,
                    bytes_received: 0,
                    packets_sent: 0,
                    packets_received: 0,
                },
            },
            network_info: network_info.clone(),
            created_at: Instant::now(),
            updated_at: Instant::now(),
        };

        // Execute services
        self.execute_services(&mut active_deployment).await?;

        // Store active deployment
        {
            let mut deployments = self.active_deployments.write().await;
            deployments.insert(request.deployment_id, active_deployment);
        }

        // Create response
        let response = ByobDeploymentResponse {
            deployment_id: request.deployment_id,
            status: DeploymentStatus::Running,
            service_statuses: request.services.keys().map(|name| {
                (name.clone(), ServiceStatus {
                    name: name.clone(),
                    state: "running".to_string(),
                    running_replicas: 1,
                    desired_replicas: 1,
                    health: "healthy".to_string(),
                    updated_at: Utc::now(),
                })
            }).collect(),
            resource_usage: ResourceUsage {
                cpu_usage: 0.0,
                memory_usage: 0,
                storage_usage: 0,
                gpu_usage: 0,
                network_usage: NetworkUsage {
                    bytes_sent: 0,
                    bytes_received: 0,
                    packets_sent: 0,
                    packets_received: 0,
                },
            },
            network_info,
            created_at: request.created_at,
            updated_at: Utc::now(),
        };

        info!("BYOB deployment {} completed successfully", request.deployment_id);
        Ok(response)
    }

    async fn get_deployment_status(
        &self,
        deployment_id: Uuid,
    ) -> ToadStoolResult<ByobDeploymentResponse> {
        let deployments = self.active_deployments.read().await;
        
        if let Some(deployment) = deployments.get(&deployment_id) {
            let response = ByobDeploymentResponse {
                deployment_id,
                status: deployment.status.clone(),
                service_statuses: deployment.request.services.keys().map(|name| {
                    (name.clone(), ServiceStatus {
                        name: name.clone(),
                        state: "running".to_string(),
                        running_replicas: 1,
                        desired_replicas: 1,
                        health: "healthy".to_string(),
                        updated_at: Utc::now(),
                    })
                }).collect(),
                resource_usage: deployment.resource_usage.clone(),
                network_info: deployment.network_info.clone(),
                created_at: deployment.request.created_at,
                updated_at: Utc::now(),
            };

            Ok(response)
        } else {
            Err(ToadStoolError::not_found(format!(
                "Deployment {} not found",
                deployment_id
            )))
        }
    }

    async fn stop_deployment(&self, deployment_id: Uuid) -> ToadStoolResult<()> {
        info!("Stopping BYOB deployment: {}", deployment_id);

        let mut deployments = self.active_deployments.write().await;
        
        if let Some(mut deployment) = deployments.remove(&deployment_id) {
            deployment.status = DeploymentStatus::Stopping;
            
            // TODO: Stop all running services
            // This would terminate all execution sessions for the deployment
            
            info!("BYOB deployment {} stopped", deployment_id);
            Ok(())
        } else {
            Err(ToadStoolError::not_found(format!(
                "Deployment {} not found",
                deployment_id
            )))
        }
    }

    async fn list_deployments(&self) -> ToadStoolResult<Vec<ByobDeploymentResponse>> {
        let deployments = self.active_deployments.read().await;
        
        let mut responses = Vec::new();
        for (deployment_id, deployment) in deployments.iter() {
            let response = ByobDeploymentResponse {
                deployment_id: *deployment_id,
                status: deployment.status.clone(),
                service_statuses: deployment.request.services.keys().map(|name| {
                    (name.clone(), ServiceStatus {
                        name: name.clone(),
                        state: "running".to_string(),
                        running_replicas: 1,
                        desired_replicas: 1,
                        health: "healthy".to_string(),
                        updated_at: Utc::now(),
                    })
                }).collect(),
                resource_usage: deployment.resource_usage.clone(),
                network_info: deployment.network_info.clone(),
                created_at: deployment.request.created_at,
                updated_at: Utc::now(),
            };
            responses.push(response);
        }

        Ok(responses)
    }

    async fn get_resource_usage(&self, deployment_id: Uuid) -> ToadStoolResult<ResourceUsage> {
        let deployments = self.active_deployments.read().await;
        
        if let Some(deployment) = deployments.get(&deployment_id) {
            Ok(deployment.resource_usage.clone())
        } else {
            Err(ToadStoolError::not_found(format!(
                "Deployment {} not found",
                deployment_id
            )))
        }
    }
}

/// Create a default BYOB compute executor
pub fn create_byob_executor(
    runtime_engine: Arc<dyn RuntimeEngine>,
) -> Arc<dyn ByobExecutor> {
    Arc::new(ByobComputeExecutor::new(
        runtime_engine,
        ByobExecutorConfig::default(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio;

    #[tokio::test]
    async fn test_validate_deployment_request() {
        // TODO: Implement tests
    }

    #[tokio::test]
    async fn test_deploy_biome() {
        // TODO: Implement tests
    }

    #[tokio::test]
    async fn test_get_deployment_status() {
        // TODO: Implement tests
    }
} 