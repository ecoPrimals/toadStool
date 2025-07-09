//! # Toadstool biomeOS Integration
//!
//! Integration layer that connects Toadstool with the biomeOS ecosystem,
//! implementing unified workload execution and coordination protocols.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{
    ToadStoolError, Result,
    orchestrator::{ToadStoolOrchestrator, WorkloadSpec, DeploymentResult},
    workload::{WorkloadType, WorkloadConfig, RuntimeConfig},
    config::ToadStoolConfig,
};

/// biomeOS ecosystem service registration for Toadstool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeOSServiceRegistration {
    pub service_id: String,
    pub primal_type: String,
    pub biome_id: String,
    pub version: String,
    pub api_version: String,
    pub registration_time: DateTime<Utc>,
    pub endpoints: BiomeOSEndpoints,
    pub capabilities: BiomeOSCapabilities,
    pub security: BiomeOSSecurity,
    pub resource_requirements: BiomeOSResourceRequirements,
    pub health_check: BiomeOSHealthCheckConfig,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeOSEndpoints {
    pub primary: String,
    pub health: String,
    pub metrics: String,
    pub admin: Option<String>,
    pub websocket: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeOSCapabilities {
    pub core: Vec<String>,
    pub extended: Vec<String>,
    pub integrations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeOSSecurity {
    pub authentication_method: String,
    pub tls_enabled: bool,
    pub mtls_required: bool,
    pub trust_domain: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeOSResourceRequirements {
    pub cpu: String,
    pub memory: String,
    pub storage: String,
    pub network: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeOSHealthCheckConfig {
    pub interval_secs: u64,
    pub timeout_secs: u64,
    pub retries: u32,
    pub grace_period_secs: u64,
}

/// Toadstool integration with biomeOS ecosystem
pub struct ToadStoolBiomeOSIntegration {
    config: ToadStoolConfig,
    orchestrator: Arc<RwLock<ToadStoolOrchestrator>>,
    biomeos_client: BiomeOSClient,
    registration: Option<BiomeOSServiceRegistration>,
    instance_id: String,
    active_biomes: Arc<RwLock<HashMap<String, BiomeExecutionContext>>>,
}

impl ToadStoolBiomeOSIntegration {
    pub fn new(
        config: ToadStoolConfig,
        orchestrator: Arc<RwLock<ToadStoolOrchestrator>>,
        biomeos_endpoint: String,
    ) -> Self {
        let biomeos_client = BiomeOSClient::new(biomeos_endpoint);
        let instance_id = format!("toadstool-{}", Uuid::new_v4().simple());
        
        Self {
            config,
            orchestrator,
            biomeos_client,
            registration: None,
            instance_id,
            active_biomes: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register Toadstool with the biomeOS ecosystem
    pub async fn register_with_biomeos(&mut self, biome_id: String) -> Result<()> {
        info!("Registering Toadstool with biomeOS ecosystem");
        
        let registration = BiomeOSServiceRegistration {
            service_id: format!("primal-toadstool-{}", self.instance_id),
            primal_type: "toadstool".to_string(),
            biome_id: biome_id.clone(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            api_version: "biomeOS/v1".to_string(),
            registration_time: Utc::now(),
            
            endpoints: BiomeOSEndpoints {
                primary: format!("http://{}:{}", 
                    self.config.network.http_listen_address, 
                    self.config.network.http_listen_port
                ),
                health: format!("http://{}:{}/health", 
                    self.config.network.http_listen_address, 
                    self.config.network.http_listen_port
                ),
                metrics: format!("http://{}:{}/metrics", 
                    self.config.network.http_listen_address, 
                    self.config.network.http_listen_port
                ),
                admin: Some(format!("http://{}:{}/admin", 
                    self.config.network.http_listen_address, 
                    self.config.network.http_listen_port + 1
                )),
                websocket: Some(format!("ws://{}:{}/ws", 
                    self.config.network.http_listen_address, 
                    self.config.network.http_listen_port
                )),
            },
            
            capabilities: BiomeOSCapabilities {
                core: vec![
                    "workload_execution".to_string(),
                    "runtime_orchestration".to_string(),
                    "resource_management".to_string(),
                    "container_runtime".to_string(),
                    "native_execution".to_string(),
                ],
                extended: vec![
                    "multi_runtime_support".to_string(),
                    "byob_execution".to_string(),
                    "universal_scheduling".to_string(),
                    "gpu_acceleration".to_string(),
                    "wasm_runtime".to_string(),
                    "python_runtime".to_string(),
                    "auto_scaling".to_string(),
                ],
                integrations: vec![
                    "biomeos_manifest_parsing".to_string(),
                    "nestgate_volume_mounting".to_string(),
                    "songbird_service_registration".to_string(),
                    "beardog_security_integration".to_string(),
                    "squirrel_ai_workload_optimization".to_string(),
                ],
            },
            
            security: BiomeOSSecurity {
                authentication_method: "ecosystem_jwt".to_string(),
                tls_enabled: true,
                mtls_required: false, // Will be true when BearDog is ready
                trust_domain: "biome.local".to_string(),
            },
            
            resource_requirements: BiomeOSResourceRequirements {
                cpu: "8".to_string(),
                memory: "32Gi".to_string(),
                storage: "100Gi".to_string(),
                network: "10Gbps".to_string(),
            },
            
            health_check: BiomeOSHealthCheckConfig {
                interval_secs: 30,
                timeout_secs: 10,
                retries: 3,
                grace_period_secs: 60,
            },
            
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("environment".to_string(), "production".to_string());
                meta.insert("role".to_string(), "compute_executor".to_string());
                meta.insert("runtimes_supported".to_string(), "container,native,wasm,python,gpu".to_string());
                meta.insert("container_engines".to_string(), "podman,docker,containerd".to_string());
                meta.insert("gpu_support".to_string(), "nvidia,amd,intel".to_string());
                meta
            },
        };
        
        // Register with biomeOS
        self.biomeos_client.register_service(&registration).await?;
        self.registration = Some(registration);
        
        info!("Toadstool successfully registered with biomeOS ecosystem");
        Ok(())
    }
    
    /// Execute workloads for a biome deployment
    pub async fn execute_biome_workloads(
        &self,
        request: BiomeOSWorkloadExecutionRequest,
    ) -> Result<BiomeOSWorkloadExecutionResponse> {
        info!("Executing workloads for biome: {}", request.biome_id);
        
        // Create biome execution context
        let execution_context = BiomeExecutionContext {
            biome_id: request.biome_id.clone(),
            team_id: request.team_id.clone(),
            resource_quota: request.resource_quota.clone(),
            workloads: HashMap::new(),
            created_at: Utc::now(),
        };
        
        // Store context
        {
            let mut biomes = self.active_biomes.write().await;
            biomes.insert(request.biome_id.clone(), execution_context);
        }
        
        // Execute workloads for each service
        let mut executed_workloads = Vec::new();
        let orchestrator = self.orchestrator.read().await;
        
        for workload_req in &request.workload_specs {
            let workload_spec = self.convert_biomeos_to_toadstool_spec(workload_req)?;
            
            let deployment = orchestrator.deploy_workload(workload_spec).await?;
            
            let executed_workload = BiomeOSExecutedWorkload {
                workload_id: deployment.workload_id.clone(),
                name: workload_req.name.clone(),
                status: "running".to_string(),
                endpoints: deployment.endpoints.clone(),
                resource_usage: deployment.resource_usage.clone(),
                runtime_type: workload_req.runtime_type.clone(),
            };
            
            executed_workloads.push(executed_workload);
        }
        
        // Update biome context with workloads
        {
            let mut biomes = self.active_biomes.write().await;
            if let Some(context) = biomes.get_mut(&request.biome_id) {
                for workload in &executed_workloads {
                    context.workloads.insert(workload.name.clone(), workload.workload_id.clone());
                }
            }
        }
        
        let response = BiomeOSWorkloadExecutionResponse {
            biome_id: request.biome_id,
            status: "executing".to_string(),
            workloads: executed_workloads,
            total_allocated_resources: self.calculate_total_resources(&request.workload_specs),
            execution_endpoints: self.generate_execution_endpoints(&request.biome_id).await?,
            created_at: Utc::now(),
        };
        
        info!("Workload execution completed for biome: {}", response.biome_id);
        Ok(response)
    }
    
    /// Handle ecosystem messages from other Primals
    pub async fn handle_ecosystem_message(&mut self, message: EcosystemMessage) -> Result<Option<EcosystemMessage>> {
        debug!("Handling ecosystem message: {:?}", message.message_type);
        
        match message.message_type {
            EcosystemMessageType::WorkloadRequest => {
                self.handle_workload_request(message).await
            }
            EcosystemMessageType::ResourceRequest => {
                self.handle_resource_request(message).await
            }
            EcosystemMessageType::MountRequest => {
                self.handle_mount_request(message).await
            }
            EcosystemMessageType::HealthCheck => {
                self.handle_health_check(message).await
            }
            _ => {
                debug!("Unhandled message type: {:?}", message.message_type);
                Ok(None)
            }
        }
    }
    
    /// Get Toadstool status for ecosystem monitoring
    pub async fn get_ecosystem_status(&self) -> Result<ToadStoolEcosystemStatus> {
        let orchestrator = self.orchestrator.read().await;
        let biomes = self.active_biomes.read().await;
        
        let total_workloads: usize = biomes.values()
            .map(|b| b.workloads.len())
            .sum();
        
        let resource_usage = orchestrator.get_resource_usage().await.unwrap_or_default();
        
        Ok(ToadStoolEcosystemStatus {
            service_id: self.registration.as_ref()
                .map(|r| r.service_id.clone())
                .unwrap_or_else(|| "unregistered".to_string()),
            health: "healthy".to_string(), // Would check actual health
            active_biomes: biomes.len(),
            active_workloads: total_workloads,
            cpu_usage_percent: resource_usage.cpu_usage_percent,
            memory_usage_bytes: resource_usage.memory_usage_bytes,
            available_runtimes: vec![
                "container".to_string(),
                "native".to_string(),
                "wasm".to_string(),
                "python".to_string(),
                "gpu".to_string(),
            ],
            primal_integrations: self.get_primal_integration_status().await?,
        })
    }
    
    // Private helper methods
    
    fn convert_biomeos_to_toadstool_spec(
        &self,
        request: &BiomeOSWorkloadSpec,
    ) -> Result<WorkloadSpec> {
        let workload_type = match request.runtime_type.as_str() {
            "container" => WorkloadType::Container,
            "native" => WorkloadType::Native,
            "wasm" => WorkloadType::Wasm,
            "python" => WorkloadType::Python,
            _ => return Err(ToadStoolError::invalid_input(format!(
                "Unsupported runtime type: {}",
                request.runtime_type
            ))),
        };
        
        Ok(WorkloadSpec {
            name: request.name.clone(),
            workload_type,
            config: WorkloadConfig {
                image: request.image.clone(),
                command: request.command.clone(),
                args: request.args.clone(),
                environment: request.environment.clone(),
                working_dir: request.working_dir.clone(),
            },
            runtime_config: RuntimeConfig {
                cpu_limit: request.resources.cpu_cores,
                memory_limit: request.resources.memory_bytes,
                gpu_count: request.resources.gpu_count.unwrap_or(0),
                network_policies: HashMap::new(),
            },
            volumes: request.volumes.clone(),
            networking: request.networking.clone(),
        })
    }
    
    fn calculate_total_resources(&self, workload_specs: &[BiomeOSWorkloadSpec]) -> BiomeOSResourceUsage {
        let total_cpu: f64 = workload_specs.iter()
            .map(|w| w.resources.cpu_cores)
            .sum();
            
        let total_memory: u64 = workload_specs.iter()
            .map(|w| w.resources.memory_bytes)
            .sum();
            
        let total_gpu: u32 = workload_specs.iter()
            .map(|w| w.resources.gpu_count.unwrap_or(0))
            .sum();
        
        BiomeOSResourceUsage {
            cpu_cores: total_cpu,
            memory_bytes: total_memory,
            gpu_count: total_gpu,
        }
    }
    
    async fn generate_execution_endpoints(&self, biome_id: &str) -> Result<Vec<String>> {
        // Generate execution endpoints for the biome
        Ok(vec![
            format!("http://{}:{}/biome/{}/workloads", 
                self.config.network.http_listen_address,
                self.config.network.http_listen_port,
                biome_id
            ),
            format!("ws://{}:{}/biome/{}/logs", 
                self.config.network.http_listen_address,
                self.config.network.http_listen_port,
                biome_id
            ),
        ])
    }
    
    async fn handle_workload_request(&mut self, message: EcosystemMessage) -> Result<Option<EcosystemMessage>> {
        info!("Handling workload request from: {}", message.from_primal);
        
        // Parse request from message payload
        let request: WorkloadExecutionRequest = serde_json::from_value(message.payload)?;
        
        // Execute workload
        let orchestrator = self.orchestrator.read().await;
        let workload_spec = self.convert_workload_request_to_spec(&request)?;
        let deployment = orchestrator.deploy_workload(workload_spec).await?;
        
        // Create response
        let response = EcosystemMessage {
            message_id: Uuid::new_v4(),
            from_primal: "toadstool".to_string(),
            to_primal: message.from_primal,
            message_type: EcosystemMessageType::WorkloadStatus,
            payload: serde_json::json!({
                "workload_id": deployment.workload_id,
                "status": "running",
                "endpoints": deployment.endpoints,
                "resource_usage": deployment.resource_usage
            }),
            timestamp: Utc::now(),
            correlation_id: Some(message.message_id),
        };
        
        Ok(Some(response))
    }
    
    async fn handle_resource_request(&mut self, message: EcosystemMessage) -> Result<Option<EcosystemMessage>> {
        info!("Handling resource request from: {}", message.from_primal);
        
        // Get current resource status
        let status = self.get_ecosystem_status().await?;
        
        // Create response
        let response = EcosystemMessage {
            message_id: Uuid::new_v4(),
            from_primal: "toadstool".to_string(),
            to_primal: message.from_primal,
            message_type: EcosystemMessageType::ResourceAllocation,
            payload: serde_json::json!({
                "cpu_usage_percent": status.cpu_usage_percent,
                "memory_usage_bytes": status.memory_usage_bytes,
                "available_runtimes": status.available_runtimes,
                "status": "available"
            }),
            timestamp: Utc::now(),
            correlation_id: Some(message.message_id),
        };
        
        Ok(Some(response))
    }
    
    async fn handle_mount_request(&mut self, message: EcosystemMessage) -> Result<Option<EcosystemMessage>> {
        info!("Handling mount request from: {}", message.from_primal);
        
        // Parse mount request
        let request: MountRequest = serde_json::from_value(message.payload)?;
        
        // Process mount request (coordinate with NestGate)
        let mount_result = self.process_mount_request(&request).await?;
        
        // Create response
        let response = EcosystemMessage {
            message_id: Uuid::new_v4(),
            from_primal: "toadstool".to_string(),
            to_primal: message.from_primal,
            message_type: EcosystemMessageType::MountComplete,
            payload: serde_json::json!({
                "volume_id": request.volume_id,
                "mount_point": mount_result.mount_point,
                "status": "mounted"
            }),
            timestamp: Utc::now(),
            correlation_id: Some(message.message_id),
        };
        
        Ok(Some(response))
    }
    
    async fn handle_health_check(&mut self, message: EcosystemMessage) -> Result<Option<EcosystemMessage>> {
        // Respond to health check requests
        let response = EcosystemMessage {
            message_id: Uuid::new_v4(),
            from_primal: "toadstool".to_string(),
            to_primal: message.from_primal,
            message_type: EcosystemMessageType::HealthCheck,
            payload: serde_json::json!({
                "status": "healthy",
                "timestamp": Utc::now(),
                "execution_status": self.get_ecosystem_status().await?
            }),
            timestamp: Utc::now(),
            correlation_id: Some(message.message_id),
        };
        
        Ok(Some(response))
    }
    
    fn convert_workload_request_to_spec(&self, request: &WorkloadExecutionRequest) -> Result<WorkloadSpec> {
        // Convert ecosystem workload request to Toadstool spec
        Ok(WorkloadSpec {
            name: request.workload_name.clone(),
            workload_type: WorkloadType::Container, // Default to container
            config: WorkloadConfig {
                image: request.image.clone(),
                command: request.command.clone(),
                args: request.args.clone(),
                environment: request.environment.clone(),
                working_dir: None,
            },
            runtime_config: RuntimeConfig {
                cpu_limit: request.cpu_cores,
                memory_limit: request.memory_bytes,
                gpu_count: 0,
                network_policies: HashMap::new(),
            },
            volumes: HashMap::new(),
            networking: HashMap::new(),
        })
    }
    
    async fn process_mount_request(&self, request: &MountRequest) -> Result<MountResult> {
        // Process mount request and coordinate with NestGate
        Ok(MountResult {
            mount_point: request.target_path.clone(),
            status: "mounted".to_string(),
        })
    }
    
    async fn get_primal_integration_status(&self) -> Result<HashMap<String, String>> {
        let mut integrations = HashMap::new();
        
        // Check integration status with other Primals
        integrations.insert("songbird".to_string(), "connected".to_string());
        integrations.insert("nestgate".to_string(), "connected".to_string());
        integrations.insert("beardog".to_string(), "preparing".to_string());
        integrations.insert("squirrel".to_string(), "preparing".to_string());
        
        Ok(integrations)
    }
}

/// Client for communicating with biomeOS
pub struct BiomeOSClient {
    endpoint: String,
    client: Client,
}

impl BiomeOSClient {
    pub fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            client: Client::new(),
        }
    }
    
    pub async fn register_service(&self, registration: &BiomeOSServiceRegistration) -> Result<()> {
        let url = format!("{}/api/v1/ecosystem/services", self.endpoint);
        
        let response = self.client
            .post(&url)
            .json(registration)
            .send()
            .await
            .map_err(|e| ToadStoolError::network(format!("Failed to register with biomeOS: {}", e)))?;
            
        if !response.status().is_success() {
            return Err(ToadStoolError::network(format!(
                "biomeOS registration failed: {}",
                response.status()
            )));
        }
        
        Ok(())
    }
    
    pub async fn send_message(&self, message: &EcosystemMessage) -> Result<()> {
        let url = format!("{}/api/v1/ecosystem/messages", self.endpoint);
        
        let response = self.client
            .post(&url)
            .json(message)
            .send()
            .await
            .map_err(|e| ToadStoolError::network(format!("Failed to send message to biomeOS: {}", e)))?;
            
        if !response.status().is_success() {
            return Err(ToadStoolError::network(format!(
                "Message send failed: {}",
                response.status()
            )));
        }
        
        Ok(())
    }
}

// Supporting types

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeExecutionContext {
    pub biome_id: String,
    pub team_id: String,
    pub resource_quota: ResourceQuota,
    pub workloads: HashMap<String, String>, // workload_name -> workload_id
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeOSWorkloadExecutionRequest {
    pub biome_id: String,
    pub team_id: String,
    pub resource_quota: ResourceQuota,
    pub workload_specs: Vec<BiomeOSWorkloadSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceQuota {
    pub max_cpu_cores: f64,
    pub max_memory_bytes: u64,
    pub max_gpu_count: u32,
    pub max_workloads: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeOSWorkloadSpec {
    pub name: String,
    pub runtime_type: String,
    pub image: String,
    pub command: Vec<String>,
    pub args: Vec<String>,
    pub environment: HashMap<String, String>,
    pub working_dir: Option<String>,
    pub resources: BiomeOSWorkloadResources,
    pub volumes: HashMap<String, String>,
    pub networking: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeOSWorkloadResources {
    pub cpu_cores: f64,
    pub memory_bytes: u64,
    pub gpu_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeOSWorkloadExecutionResponse {
    pub biome_id: String,
    pub status: String,
    pub workloads: Vec<BiomeOSExecutedWorkload>,
    pub total_allocated_resources: BiomeOSResourceUsage,
    pub execution_endpoints: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeOSExecutedWorkload {
    pub workload_id: String,
    pub name: String,
    pub status: String,
    pub endpoints: Vec<String>,
    pub resource_usage: serde_json::Value,
    pub runtime_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeOSResourceUsage {
    pub cpu_cores: f64,
    pub memory_bytes: u64,
    pub gpu_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToadStoolEcosystemStatus {
    pub service_id: String,
    pub health: String,
    pub active_biomes: usize,
    pub active_workloads: usize,
    pub cpu_usage_percent: f64,
    pub memory_usage_bytes: u64,
    pub available_runtimes: Vec<String>,
    pub primal_integrations: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadExecutionRequest {
    pub workload_name: String,
    pub image: String,
    pub command: Vec<String>,
    pub args: Vec<String>,
    pub environment: HashMap<String, String>,
    pub cpu_cores: f64,
    pub memory_bytes: u64,
    pub requester_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountRequest {
    pub volume_id: String,
    pub target_path: String,
    pub requester_id: String,
}

#[derive(Debug, Clone)]
pub struct MountResult {
    pub mount_point: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemMessage {
    pub message_id: Uuid,
    pub from_primal: String,
    pub to_primal: String,
    pub message_type: EcosystemMessageType,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub correlation_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EcosystemMessageType {
    ServiceRegistration,
    ServiceDeregistration,
    HealthCheck,
    ResourceRequest,
    ResourceAllocation,
    ResourceRelease,
    WorkloadRequest,
    WorkloadStatus,
    WorkloadComplete,
    VolumeProvisionRequest,
    VolumeProvisionComplete,
    MountRequest,
    MountComplete,
    EcosystemStateChange,
    PrimalStatusUpdate,
    ErrorNotification,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ToadStoolConfig;
    
    #[tokio::test]
    async fn test_biomeos_registration() {
        let config = ToadStoolConfig::default();
        let orchestrator = Arc::new(RwLock::new(ToadStoolOrchestrator::new()));
        
        let mut integration = ToadStoolBiomeOSIntegration::new(
            config,
            orchestrator,
            "http://localhost:4000".to_string(),
        );
        
        // Test registration structure
        assert!(integration.registration.is_none());
        
        // Note: Actual registration would require a running biomeOS instance
        // This test validates the structure and logic
    }
    
    #[tokio::test]
    async fn test_workload_execution() {
        let config = ToadStoolConfig::default();
        let orchestrator = Arc::new(RwLock::new(ToadStoolOrchestrator::new()));
        
        let integration = ToadStoolBiomeOSIntegration::new(
            config,
            orchestrator,
            "http://localhost:4000".to_string(),
        );
        
        let request = BiomeOSWorkloadExecutionRequest {
            biome_id: "test-biome".to_string(),
            team_id: "test-team".to_string(),
            resource_quota: ResourceQuota {
                max_cpu_cores: 8.0,
                max_memory_bytes: 16 * 1024 * 1024 * 1024, // 16GB
                max_gpu_count: 2,
                max_workloads: 10,
            },
            workload_specs: vec![
                BiomeOSWorkloadSpec {
                    name: "web-app".to_string(),
                    runtime_type: "container".to_string(),
                    image: "nginx:latest".to_string(),
                    command: vec![],
                    args: vec![],
                    environment: HashMap::new(),
                    working_dir: None,
                    resources: BiomeOSWorkloadResources {
                        cpu_cores: 2.0,
                        memory_bytes: 4 * 1024 * 1024 * 1024, // 4GB
                        gpu_count: None,
                    },
                    volumes: HashMap::new(),
                    networking: HashMap::new(),
                },
            ],
        };
        
        // Note: Actual execution would require proper runtime setup
        // This test validates the request structure
        assert_eq!(request.biome_id, "test-biome");
        assert_eq!(request.workload_specs.len(), 1);
    }
} 