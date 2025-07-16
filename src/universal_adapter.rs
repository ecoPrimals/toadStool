//! Universal Primal Adapter for Toadstool
//!
//! This adapter enables Toadstool to coordinate with any Primal (standard, custom, or forked)
//! using a universal compute orchestration pattern. It manages workload execution coordination
//! and provides seamless integration for compute-related Primal interactions.

use crate::resources::UniversalResourceManager;
use crate::scheduler::UniversalScheduler;
use crate::security::UniversalSecurityManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use toadstool::ToadStoolResult;
use toadstool_config::network;
use toadstool_config::helpers;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use tokio::sync::RwLock;
use tracing::{info, warn, error};
use uuid::Uuid;

/// Universal coordination configuration for any Primal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalCoordination {
    /// Whether this Primal is enabled for coordination
    pub enabled: bool,
    
    /// Network endpoint for coordination
    pub endpoint: Option<String>,
    
    /// Coordination capabilities this Primal provides
    pub capabilities: Vec<String>,
    
    /// API version supported by this Primal
    pub api_version: String,
    
    /// Priority for workload routing (higher = preferred)
    pub priority: u32,
}

/// Universal adapter for Toadstool compute coordination
pub struct ToadstoolUniversalAdapter {
    /// HTTP client for making requests
    client: Client,
    
    /// Configuration for all available Primals
    primal_configs: Arc<RwLock<HashMap<String, PrimalCoordination>>>,
    
    /// Toadstool's compute identity
    compute_identity: ComputeIdentity,
    
    /// Active workload coordination sessions
    active_sessions: Arc<RwLock<HashMap<String, WorkloadSession>>>,
    
    /// Resource allocation tracker
    resource_tracker: Arc<RwLock<ResourceTracker>>,
}

/// Toadstool's compute identity for universal coordination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeIdentity {
    pub instance_id: String,
    pub capabilities: Vec<String>,
    pub endpoints: HashMap<String, String>,
    pub supported_runtimes: Vec<String>,
    pub compute_info: ComputeCapabilities,
}

/// Compute capabilities that Toadstool provides
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeCapabilities {
    pub cpu_cores: u32,
    pub memory_bytes: u64,
    pub gpu_count: u32,
    pub storage_bytes: u64,
    pub supported_architectures: Vec<String>,
    pub container_runtimes: Vec<String>,
    pub native_execution: bool,
    pub wasm_support: bool,
    pub python_runtime: bool,
}

/// Active workload coordination session
#[derive(Debug, Clone)]
pub struct WorkloadSession {
    pub session_id: String,
    pub workload_id: String,
    pub primal_name: String,
    pub started_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub status: SessionStatus,
    pub resource_allocation: ResourceAllocation,
}

/// Resource allocation for a workload
#[derive(Debug, Clone)]
pub struct ResourceAllocation {
    pub cpu_cores: f64,
    pub memory_bytes: u64,
    pub gpu_count: u32,
    pub storage_bytes: u64,
    pub network_bandwidth: Option<String>,
}

/// Resource usage tracker
#[derive(Debug, Clone)]
pub struct ResourceTracker {
    pub total_cpu_cores: f64,
    pub available_cpu_cores: f64,
    pub total_memory_bytes: u64,
    pub available_memory_bytes: u64,
    pub total_gpu_count: u32,
    pub available_gpu_count: u32,
    pub active_workloads: HashMap<String, ResourceAllocation>,
}

#[derive(Debug, Clone)]
pub enum SessionStatus {
    Initializing,
    Running,
    Paused,
    Completed,
    Failed,
    Terminated,
}

impl ToadstoolUniversalAdapter {
    /// Create a new universal adapter for Toadstool
    pub fn new() -> Result<Self, AdapterError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| AdapterError::ConfigurationError(format!("Failed to create HTTP client: {}", e)))?;
            
        let compute_identity = ComputeIdentity {
            instance_id: format!("toadstool-{}", Uuid::new_v4().simple()),
            capabilities: vec![
                "compute".to_string(),
                "execution".to_string(),
                "orchestration".to_string(),
                "runtime_management".to_string(),
                "workload_scheduling".to_string(),
                "resource_management".to_string(),
                "multi_runtime_support".to_string(),
                "container_execution".to_string(),
                "native_execution".to_string(),
                "wasm_execution".to_string(),
                "python_runtime".to_string(),
                "gpu_acceleration".to_string(),
            ],
            endpoints: HashMap::new(), // Will be populated during initialization
            supported_runtimes: vec![
                "container".to_string(),
                "native".to_string(),
                "wasm".to_string(),
                "python".to_string(),
                "gpu".to_string(),
            ],
            compute_info: ComputeCapabilities {
                cpu_cores: 0, // Will be detected from system
                memory_bytes: 0,
                gpu_count: 0,
                storage_bytes: 0,
                supported_architectures: vec![
                    "x86_64".to_string(),
                    "aarch64".to_string(),
                    "riscv64".to_string(),
                ],
                container_runtimes: vec![
                    "docker".to_string(),
                    "podman".to_string(),
                    "containerd".to_string(),
                ],
                native_execution: true,
                wasm_support: true,
                python_runtime: true,
            },
        };
        
        let resource_tracker = ResourceTracker {
            total_cpu_cores: 0.0,
            available_cpu_cores: 0.0,
            total_memory_bytes: 0,
            available_memory_bytes: 0,
            total_gpu_count: 0,
            available_gpu_count: 0,
            active_workloads: HashMap::new(),
        };
        
        Ok(Self {
            client,
            primal_configs: Arc::new(RwLock::new(HashMap::new())),
            compute_identity,
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            resource_tracker: Arc::new(RwLock::new(resource_tracker)),
        })
    }
    
    /// Initialize the universal adapter with default Primal configurations
    pub async fn initialize_with_defaults(&self) {
        let mut configs = self.primal_configs.write().await;
        
        // Songbird for orchestration coordination
        configs.insert("songbird".to_string(), PrimalCoordination {
            enabled: true,
            endpoint: Some(helpers::default_songbird_endpoint()),
            capabilities: vec![
                "orchestration".to_string(),
                "service_discovery".to_string(),
                "load_balancing".to_string(),
                "coordination".to_string(),
            ],
            api_version: "universal/v1".to_string(),
            priority: 100, // High priority for orchestration
        });
        
        // NestGate for storage coordination
        configs.insert("nestgate".to_string(), PrimalCoordination {
            enabled: true,
            endpoint: Some(helpers::default_nestgate_endpoint()),
            capabilities: vec![
                "storage".to_string(),
                "volume_management".to_string(),
                "data_provisioning".to_string(),
                "mount_coordination".to_string(),
            ],
            api_version: "universal/v1".to_string(),
            priority: 90, // High priority for storage
        });
        
        // BearDog for security coordination
        configs.insert("beardog".to_string(), PrimalCoordination {
            enabled: false, // Will be enabled when ready
            endpoint: Some(helpers::default_beardog_endpoint()),
            capabilities: vec![
                "security".to_string(),
                "authentication".to_string(),
                "encryption".to_string(),
                "workload_security".to_string(),
            ],
            api_version: "universal/v1".to_string(),
            priority: 95, // High priority for security
        });
        
        // Squirrel for AI coordination
        configs.insert("squirrel".to_string(), PrimalCoordination {
            enabled: true, // Squirrel MCP is now production ready!
            endpoint: Some("http://squirrel:5000".to_string()),
            capabilities: vec![
                "ai".to_string(),
                "ml".to_string(),
                "agents".to_string(),
                "mcp".to_string(),
                "ai_workloads".to_string(),
                "natural_language".to_string(),
                "context_management".to_string(),
                "plugin_execution".to_string(),
            ],
            api_version: "universal/v1".to_string(),
            priority: 90, // High priority for AI coordination
        });
        
        info!("Universal adapter initialized with {} Primal configurations", configs.len());
    }
    
    /// Universal coordination method that works with any Primal
    pub async fn coordinate_with_primal(&self, primal_name: &str, coordination_request: CoordinationRequest) -> Result<CoordinationResult, AdapterError> {
        let configs = self.primal_configs.read().await;
        let primal_config = configs.get(primal_name)
            .ok_or_else(|| AdapterError::PrimalNotFound(primal_name.to_string()))?;
            
        if !primal_config.enabled {
            info!("Primal {} coordination disabled - skipping", primal_name);
            return Ok(CoordinationResult::skipped(primal_name));
        }

        if let Some(endpoint) = &primal_config.endpoint {
            info!("Coordinating with {} at: {}", primal_name, endpoint);
            
            // Create coordination session if it's a workload request
            let session = if matches!(coordination_request.request_type, CoordinationRequestType::WorkloadExecution) {
                Some(self.create_workload_session(primal_name, &coordination_request).await)
            } else {
                None
            };
            
            // Use universal coordination based on capabilities
            let result = self.call_universal_primal_api(primal_name, endpoint, primal_config, coordination_request).await;
            
            // Update session status if applicable
            if let Some(ref session) = session {
                self.update_session_status(&session.session_id, &result).await;
            }
            
            return result;
        }

        warn!("{} coordination endpoint not available - continuing without", primal_name);
        Ok(CoordinationResult::unavailable(primal_name))
    }
    
    /// Coordinate workload execution with appropriate Primals
    pub async fn coordinate_workload_execution(&self, workload_spec: WorkloadSpec) -> Vec<CoordinationResult> {
        let mut results = Vec::new();
        let configs = self.primal_configs.read().await;
        
        // Create coordination requests based on workload requirements
        for (primal_name, config) in configs.iter() {
            if !config.enabled {
                continue;
            }
            
            // Determine if this Primal should be involved in workload execution
            let should_coordinate = self.should_coordinate_for_workload(primal_name, &config.capabilities, &workload_spec);
            
            if should_coordinate {
                let coordination_request = self.create_workload_coordination_request(primal_name, &workload_spec);
                
                match self.coordinate_with_primal(primal_name, coordination_request).await {
                    Ok(result) => results.push(result),
                    Err(e) => {
                        warn!("Coordination with {} failed: {}", primal_name, e);
                        results.push(CoordinationResult::failed(primal_name, e.to_string()));
                    }
                }
            }
        }
        
        results
    }
    
    /// Universal API call that adapts to any Primal's interface
    async fn call_universal_primal_api(
        &self,
        primal_name: &str,
        endpoint: &str,
        config: &PrimalCoordination,
        request: CoordinationRequest,
    ) -> Result<CoordinationResult, AdapterError> {
        // Determine the appropriate API path based on capabilities
        let api_path = self.determine_api_path(primal_name, &config.capabilities, &request.request_type);
        let full_url = format!("{}{}", endpoint, api_path);
        
        // Create universal coordination payload
        let coordination_payload = self.create_universal_payload(primal_name, &config.capabilities, request);
        
        info!("Universal coordination with {} at {}", primal_name, full_url);
        
        let response = self.client
            .post(&full_url)
            .json(&coordination_payload)
            .send()
            .await
            .map_err(|e| AdapterError::NetworkError(format!("Request failed: {}", e)))?;
        
        if response.status().is_success() {
            info!("Successfully coordinated with {} (universal adapter)", primal_name);
            
            // Parse response if available
            if let Ok(response_data) = response.json::<serde_json::Value>().await {
                return Ok(CoordinationResult::success(primal_name, Some(response_data)));
            }
            
            Ok(CoordinationResult::success(primal_name, None))
        } else {
            let error_msg = format!("{} coordination failed: {}", primal_name, response.status());
            warn!("{} (universal adapter)", error_msg);
            Ok(CoordinationResult::failed(primal_name, error_msg))
        }
    }
    
    /// Determine the appropriate API path based on Primal capabilities and request type
    fn determine_api_path(&self, primal_name: &str, capabilities: &[String], request_type: &CoordinationRequestType) -> String {
        match request_type {
            CoordinationRequestType::WorkloadExecution => {
                // Route based on primary capability
                for capability in capabilities {
                    match capability.as_str() {
                        "orchestration" | "coordination" => return "/api/v1/coordinate-workload".to_string(),
                        "storage" | "volume_management" => return "/api/v1/provision-storage".to_string(),
                        "security" | "authentication" => return "/api/v1/secure-workload".to_string(),
                        "ai" | "ml" => return "/api/v1/optimize-workload".to_string(),
                        _ => continue,
                    }
                }
            }
            CoordinationRequestType::ResourceRequest => return "/api/v1/resources".to_string(),
            CoordinationRequestType::HealthCheck => return "/api/v1/health".to_string(),
            CoordinationRequestType::Custom(_) => return "/api/v1/coordinate".to_string(),
        }
        
        // Fallback to standard coordination endpoint
        "/api/v1/coordinate".to_string()
    }
    
    /// Create universal payload that any Primal can understand
    fn create_universal_payload(&self, primal_name: &str, capabilities: &[String], request: CoordinationRequest) -> serde_json::Value {
        serde_json::json!({
            "coordination_request": {
                "from": "toadstool",
                "to": primal_name,
                "compute_identity": self.compute_identity,
                "capabilities_requested": capabilities,
                "api_version": "universal/v1",
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "request_type": request.request_type,
                "request_data": request.data
            },
            "compute_context": {
                "available_cpu_cores": self.compute_identity.compute_info.cpu_cores,
                "available_memory_bytes": self.compute_identity.compute_info.memory_bytes,
                "supported_runtimes": self.compute_identity.supported_runtimes,
                "gpu_acceleration": self.compute_identity.compute_info.gpu_count > 0,
                "native_execution": self.compute_identity.compute_info.native_execution,
                "wasm_support": self.compute_identity.compute_info.wasm_support
            }
        })
    }
    
    /// Determine if a Primal should be involved in workload execution
    fn should_coordinate_for_workload(&self, primal_name: &str, capabilities: &[String], workload_spec: &WorkloadSpec) -> bool {
        match primal_name {
            "songbird" => true, // Always coordinate with orchestrator
            "nestgate" => workload_spec.requires_storage(),
            "beardog" => workload_spec.requires_security(),
            "squirrel" => workload_spec.is_ai_workload(),
            _ => {
                // For custom Primals, check if capabilities match workload requirements
                capabilities.iter().any(|cap| workload_spec.requires_capability(cap))
            }
        }
    }
    
    /// Create workload-specific coordination request
    fn create_workload_coordination_request(&self, primal_name: &str, workload_spec: &WorkloadSpec) -> CoordinationRequest {
        CoordinationRequest {
            request_id: Uuid::new_v4().to_string(),
            request_type: CoordinationRequestType::WorkloadExecution,
            data: serde_json::json!({
                "workload_spec": workload_spec,
                "target_primal": primal_name,
                "coordination_mode": "universal",
                "resource_requirements": workload_spec.resource_requirements
            }),
            timestamp: Utc::now(),
        }
    }
    
    /// Create a new workload coordination session
    async fn create_workload_session(&self, primal_name: &str, request: &CoordinationRequest) -> WorkloadSession {
        let session = WorkloadSession {
            session_id: Uuid::new_v4().to_string(),
            workload_id: request.request_id.clone(),
            primal_name: primal_name.to_string(),
            started_at: Utc::now(),
            last_activity: Utc::now(),
            status: SessionStatus::Initializing,
            resource_allocation: ResourceAllocation {
                cpu_cores: 0.0,
                memory_bytes: 0,
                gpu_count: 0,
                storage_bytes: 0,
                network_bandwidth: None,
            },
        };
        
        let mut sessions = self.active_sessions.write().await;
        sessions.insert(session.session_id.clone(), session.clone());
        
        session
    }
    
    /// Update session status based on coordination result
    async fn update_session_status(&self, session_id: &str, result: &Result<CoordinationResult, AdapterError>) {
        let mut sessions = self.active_sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.last_activity = Utc::now();
            session.status = match result {
                Ok(coord_result) => match coord_result.status {
                    CoordinationStatus::Success => SessionStatus::Running,
                    CoordinationStatus::Failed => SessionStatus::Failed,
                    _ => SessionStatus::Paused,
                },
                Err(_) => SessionStatus::Failed,
            };
        }
    }
    
    /// Update compute capabilities from system detection
    pub fn update_compute_info(&mut self, compute_info: ComputeCapabilities) {
        self.compute_identity.compute_info = compute_info;
    }
    
    /// Add or update a Primal configuration dynamically
    pub async fn add_primal_config(&self, primal_name: String, config: PrimalCoordination) {
        let mut configs = self.primal_configs.write().await;
        configs.insert(primal_name.clone(), config);
        info!("Added/updated Primal configuration for: {}", primal_name);
    }
    
    /// Remove a Primal configuration
    pub async fn remove_primal_config(&self, primal_name: &str) {
        let mut configs = self.primal_configs.write().await;
        configs.remove(primal_name);
        info!("Removed Primal configuration for: {}", primal_name);
    }
    
    /// Get current Primal configurations
    pub async fn get_primal_configs(&self) -> HashMap<String, PrimalCoordination> {
        self.primal_configs.read().await.clone()
    }
    
    /// Get active workload sessions
    pub async fn get_active_sessions(&self) -> HashMap<String, WorkloadSession> {
        self.active_sessions.read().await.clone()
    }
    
    /// Get current resource usage
    pub async fn get_resource_usage(&self) -> ResourceTracker {
        self.resource_tracker.read().await.clone()
    }
}

/// Coordination request for universal API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationRequest {
    pub request_id: String,
    pub request_type: CoordinationRequestType,
    pub data: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoordinationRequestType {
    WorkloadExecution,
    ResourceRequest,
    HealthCheck,
    Custom(String),
}

/// Result of coordination with a Primal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationResult {
    pub primal_name: String,
    pub status: CoordinationStatus,
    pub message: Option<String>,
    pub response_data: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoordinationStatus {
    Success,
    Failed,
    Skipped,
    Unavailable,
}

impl CoordinationResult {
    pub fn success(primal_name: &str, response_data: Option<serde_json::Value>) -> Self {
        Self {
            primal_name: primal_name.to_string(),
            status: CoordinationStatus::Success,
            message: Some("Coordination successful".to_string()),
            response_data,
            timestamp: Utc::now(),
        }
    }
    
    pub fn failed(primal_name: &str, error: String) -> Self {
        Self {
            primal_name: primal_name.to_string(),
            status: CoordinationStatus::Failed,
            message: Some(error),
            response_data: None,
            timestamp: Utc::now(),
        }
    }
    
    pub fn skipped(primal_name: &str) -> Self {
        Self {
            primal_name: primal_name.to_string(),
            status: CoordinationStatus::Skipped,
            message: Some("Coordination disabled".to_string()),
            response_data: None,
            timestamp: Utc::now(),
        }
    }
    
    pub fn unavailable(primal_name: &str) -> Self {
        Self {
            primal_name: primal_name.to_string(),
            status: CoordinationStatus::Unavailable,
            message: Some("Endpoint not available".to_string()),
            response_data: None,
            timestamp: Utc::now(),
        }
    }
}

/// Errors that can occur during universal coordination
#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("Primal not found: {0}")]
    PrimalNotFound(String),
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    
    #[error("Coordination failed: {0}")]
    CoordinationFailed(String),
    
    #[error("Resource allocation failed: {0}")]
    ResourceAllocationFailed(String),
}

/// Workload specification for coordination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadSpec {
    pub name: String,
    pub runtime_type: RuntimeType,
    pub resource_requirements: ResourceRequirements,
    pub security_requirements: Option<SecurityRequirements>,
    pub storage_requirements: Option<StorageRequirements>,
    pub ai_requirements: Option<AIRequirements>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuntimeType {
    Container,
    Native,
    Wasm,
    Python,
    GPU,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub cpu_cores: f64,
    pub memory_bytes: u64,
    pub gpu_count: u32,
    pub storage_bytes: u64,
    pub network_bandwidth: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityRequirements {
    pub encryption_required: bool,
    pub isolated_execution: bool,
    pub access_control: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageRequirements {
    pub persistent_volumes: Vec<VolumeSpec>,
    pub temporary_storage: u64,
    pub backup_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeSpec {
    pub name: String,
    pub size_bytes: u64,
    pub mount_path: String,
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIRequirements {
    pub model_types: Vec<String>,
    pub frameworks: Vec<String>,
    pub gpu_memory_required: u64,
}

impl WorkloadSpec {
    pub fn requires_storage(&self) -> bool {
        self.storage_requirements.is_some()
    }
    
    pub fn requires_security(&self) -> bool {
        self.security_requirements.is_some()
    }
    
    pub fn is_ai_workload(&self) -> bool {
        self.ai_requirements.is_some()
    }
    
    pub fn requires_capability(&self, capability: &str) -> bool {
        match capability {
            "storage" | "volume_management" => self.requires_storage(),
            "security" | "authentication" => self.requires_security(),
            "ai" | "ml" => self.is_ai_workload(),
            "gpu" => self.resource_requirements.gpu_count > 0,
            "container" => matches!(self.runtime_type, RuntimeType::Container),
            "native" => matches!(self.runtime_type, RuntimeType::Native),
            "wasm" => matches!(self.runtime_type, RuntimeType::Wasm),
            "python" => matches!(self.runtime_type, RuntimeType::Python),
            _ => false,
        }
    }
}

/// Trait for implementing universal coordination in Toadstool components
#[async_trait]
pub trait UniversalComputeCoordination {
    /// Coordinate workload execution with other Primals
    async fn coordinate_workload_execution(&self, workload: WorkloadSpec) -> Result<Vec<CoordinationResult>, AdapterError>;
    
    /// Coordinate resource allocation with other Primals
    async fn coordinate_resource_allocation(&self, requirements: ResourceRequirements) -> Result<Vec<CoordinationResult>, AdapterError>;
    
    /// Coordinate storage mounting with NestGate
    async fn coordinate_storage_mounting(&self, volumes: Vec<VolumeSpec>) -> Result<Vec<CoordinationResult>, AdapterError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_universal_adapter_creation() {
        let adapter = ToadstoolUniversalAdapter::new();
        assert!(adapter.compute_identity.capabilities.contains(&"compute".to_string()));
    }

    #[tokio::test]
    async fn test_primal_configuration() {
        let adapter = ToadstoolUniversalAdapter::new();
        adapter.initialize_with_defaults().await;
        
        let configs = adapter.get_primal_configs().await;
        assert!(configs.contains_key("songbird"));
        assert!(configs.contains_key("nestgate"));
    }

    #[test]
    fn test_workload_capability_matching() {
        let workload = WorkloadSpec {
            name: "test-workload".to_string(),
            runtime_type: RuntimeType::Container,
            resource_requirements: ResourceRequirements {
                cpu_cores: 2.0,
                memory_bytes: 4 * 1024 * 1024 * 1024, // 4GB
                gpu_count: 1,
                storage_bytes: 10 * 1024 * 1024 * 1024, // 10GB
                network_bandwidth: None,
            },
            security_requirements: Some(SecurityRequirements {
                encryption_required: true,
                isolated_execution: true,
                access_control: Some("rbac".to_string()),
            }),
            storage_requirements: Some(StorageRequirements {
                persistent_volumes: vec![],
                temporary_storage: 1024 * 1024 * 1024, // 1GB
                backup_required: false,
            }),
            ai_requirements: None,
        };
        
        assert!(workload.requires_storage());
        assert!(workload.requires_security());
        assert!(!workload.is_ai_workload());
        assert!(workload.requires_capability("gpu"));
        assert!(workload.requires_capability("container"));
    }

    #[test]
    fn test_api_path_determination() {
        let adapter = ToadstoolUniversalAdapter::new();
        
        let workload_path = adapter.determine_api_path(
            "songbird", 
            &["orchestration".to_string()], 
            &CoordinationRequestType::WorkloadExecution
        );
        assert_eq!(workload_path, "/api/v1/coordinate-workload");
        
        let resource_path = adapter.determine_api_path(
            "any", 
            &[], 
            &CoordinationRequestType::ResourceRequest
        );
        assert_eq!(resource_path, "/api/v1/resources");
    }
} 