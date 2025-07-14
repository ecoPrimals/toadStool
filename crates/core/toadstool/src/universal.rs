//! # Universal Compute Platform
//!
//! The heart of ToadStool's universal compute capabilities. This module implements
//! the core principle: "If it computes, we can run it"

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use base64::Engine;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use tracing::{debug, info};

use crate::error::{ToadStoolError, ToadStoolResult};
use crate::execution::{ExecutionResponse, RuntimeEngine, RuntimeType};
use crate::resources::ResourceRequirements;
use toadstool_config::constants::network;

//
// ============================================================================
// CORE UNIVERSAL TYPES
// ============================================================================
//

/// Security level for primal operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SecurityLevel {
    /// Basic security
    Basic,
    /// Standard security
    Standard,
    /// High security
    High,
    /// Maximum security
    Maximum,
}

/// Network location information
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NetworkLocation {
    /// IP address
    pub ip_address: String,
    /// Subnet
    pub subnet: Option<String>,
    /// Network identifier
    pub network_id: Option<String>,
    /// Geographic location
    pub geo_location: Option<String>,
}

/// Context for user/device-specific primal routing
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrimalContext {
    /// User identifier
    pub user_id: String,
    /// Device identifier
    pub device_id: String,
    /// Session identifier
    pub session_id: String,
    /// Network location
    pub network_location: NetworkLocation,
    /// Security level required
    pub security_level: SecurityLevel,
    /// Additional context metadata
    pub metadata: HashMap<String, String>,
}

/// Primal type categories
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PrimalType {
    /// Compute primal (ToadStool)
    Compute,
    /// Security primal (BearDog)
    Security,
    /// Storage primal (NestGate)
    Storage,
    /// AI primal (Squirrel)
    AI,
    /// Network primal (Songbird)
    Network,
    /// OS primal (BiomeOS)
    OS,
    /// Custom primal type
    Custom(String),
}

/// Primal capabilities
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrimalCapability {
    // Compute capabilities
    /// Container runtime support
    ContainerRuntime { orchestrators: Vec<String> },
    /// Serverless execution
    ServerlessExecution { languages: Vec<String> },
    /// GPU acceleration
    GpuAcceleration { cuda_support: bool },
    /// Load balancing
    LoadBalancing { algorithms: Vec<String> },
    /// Auto-scaling
    AutoScaling { metrics: Vec<String> },
    /// Native execution
    NativeExecution { architectures: Vec<String> },
    /// WASM execution
    WasmExecution { wasi_support: bool },
    
    // Security capabilities
    /// Authentication
    Authentication { methods: Vec<String> },
    /// Encryption
    Encryption { algorithms: Vec<String> },
    /// Key management
    KeyManagement { hsm_support: bool },
    
    // Storage capabilities
    /// File system support
    FileSystem { supports_zfs: bool },
    /// Object storage
    ObjectStorage { backends: Vec<String> },
    /// Data replication
    DataReplication { consistency: String },
    
    // AI capabilities
    /// Model inference
    ModelInference { models: Vec<String> },
    /// Agent framework
    AgentFramework { mcp_support: bool },
    /// Machine learning
    MachineLearning { training_support: bool },
    
    // Network capabilities
    /// Service discovery
    ServiceDiscovery { protocols: Vec<String> },
    /// Network routing
    NetworkRouting { protocols: Vec<String> },
    /// Proxy services
    ProxyServices { types: Vec<String> },
    
    // OS capabilities
    /// Process management
    ProcessManagement { container_support: bool },
    /// Resource management
    ResourceManagement { quota_support: bool },
    /// Team isolation
    TeamIsolation { multi_tenant: bool },
    
    // Custom capability
    Custom { name: String, attributes: HashMap<String, String> },
}

/// Primal health status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrimalHealth {
    /// Healthy and operational
    Healthy,
    /// Degraded but operational
    Degraded { issues: Vec<String> },
    /// Unhealthy and not operational
    Unhealthy { reason: String },
}

/// Primal API endpoints
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrimalEndpoints {
    /// Primary API endpoint
    pub primary: String,
    /// Health check endpoint
    pub health: String,
    /// Metrics endpoint
    pub metrics: Option<String>,
    /// Admin endpoint
    pub admin: Option<String>,
    /// WebSocket endpoint
    pub websocket: Option<String>,
    /// Custom endpoints
    pub custom: HashMap<String, String>,
}

/// Inter-primal request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalRequest {
    /// Request ID
    pub id: Uuid,
    /// Source primal
    pub source: String,
    /// Target primal
    pub target: String,
    /// Request type
    pub request_type: String,
    /// Request payload
    pub payload: serde_json::Value,
    /// Request context
    pub context: PrimalContext,
    /// Request metadata
    pub metadata: HashMap<String, String>,
    /// Request timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Response status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResponseStatus {
    /// Success
    Success,
    /// Error with details
    Error { code: String, message: String },
    /// Timeout
    Timeout,
    /// Service unavailable
    ServiceUnavailable,
}

/// Inter-primal response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalResponse {
    /// Request ID this response is for
    pub request_id: Uuid,
    /// Response status
    pub status: ResponseStatus,
    /// Response payload
    pub payload: serde_json::Value,
    /// Response metadata
    pub metadata: HashMap<String, String>,
    /// Response timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

//
// ============================================================================
// UNIVERSAL PRIMAL PROVIDER TRAIT
// ============================================================================
//

/// Universal primal provider trait
#[async_trait]
pub trait UniversalPrimalProvider: Send + Sync {
    /// Unique primal identifier
    fn primal_id(&self) -> &str;
    
    /// Instance identifier
    fn instance_id(&self) -> &str;
    
    /// Context this primal serves
    fn context(&self) -> &PrimalContext;
    
    /// Primal type
    fn primal_type(&self) -> PrimalType;
    
    /// Capabilities provided
    fn capabilities(&self) -> Vec<PrimalCapability>;
    
    /// Health check
    async fn health_check(&self) -> PrimalHealth;
    
    /// API endpoints
    fn endpoints(&self) -> PrimalEndpoints;
    
    /// Handle inter-primal requests
    async fn handle_primal_request(&self, request: PrimalRequest) -> ToadStoolResult<PrimalResponse>;
    
    /// Initialize with configuration
    async fn initialize(&mut self, config: serde_json::Value) -> ToadStoolResult<()>;
    
    /// Shutdown gracefully
    async fn shutdown(&mut self) -> ToadStoolResult<()>;
    
    /// Check if can serve context
    fn can_serve_context(&self, context: &PrimalContext) -> bool;
}

//
// ============================================================================
// UNIVERSAL PRIMAL REGISTRY
// ============================================================================
//

/// Universal primal registry for capability-based discovery
pub struct UniversalPrimalRegistry {
    /// Registered primal providers
    providers: RwLock<HashMap<String, Arc<dyn UniversalPrimalProvider>>>,
    /// Capability index: capability -> provider instance IDs
    capability_index: RwLock<HashMap<String, Vec<String>>>,
    /// Context index: user_id -> provider instance IDs
    context_index: RwLock<HashMap<String, Vec<String>>>,
    /// Type index: primal_type -> provider instance IDs
    type_index: RwLock<HashMap<String, Vec<String>>>,
}

impl Default for UniversalPrimalRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl UniversalPrimalRegistry {
    /// Create new registry
    pub fn new() -> Self {
        Self {
            providers: RwLock::new(HashMap::new()),
            capability_index: RwLock::new(HashMap::new()),
            context_index: RwLock::new(HashMap::new()),
            type_index: RwLock::new(HashMap::new()),
        }
    }
    
    /// Register a primal provider
    pub async fn register_primal(&self, provider: Arc<dyn UniversalPrimalProvider>) -> ToadStoolResult<()> {
        let instance_id = provider.instance_id().to_string();
        let capabilities = provider.capabilities();
        let context = provider.context().clone();
        let primal_type = provider.primal_type();
        
        // Register provider
        self.providers.write().await.insert(instance_id.clone(), provider);
        
        // Index capabilities
        let mut capability_index = self.capability_index.write().await;
        for capability in capabilities {
            let cap_key = format!("{capability:?}");
            capability_index.entry(cap_key).or_insert_with(Vec::new).push(instance_id.clone());
        }
        
        // Index context
        let mut context_index = self.context_index.write().await;
        context_index.entry(context.user_id.clone()).or_insert_with(Vec::new).push(instance_id.clone());
        
        // Index type
        let mut type_index = self.type_index.write().await;
        let type_key = format!("{primal_type:?}");
        type_index.entry(type_key).or_insert_with(Vec::new).push(instance_id.clone());
        
        info!("Registered primal provider: {}", instance_id);
        Ok(())
    }
    
    /// Find providers by capability
    pub async fn find_by_capability(&self, capability: &PrimalCapability) -> Vec<Arc<dyn UniversalPrimalProvider>> {
        let cap_key = format!("{capability:?}");
        let capability_index = self.capability_index.read().await;
        let providers = self.providers.read().await;
        
        if let Some(instance_ids) = capability_index.get(&cap_key) {
            instance_ids.iter()
                .filter_map(|id| providers.get(id))
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Find providers by context
    pub async fn find_by_context(&self, context: &PrimalContext) -> Vec<Arc<dyn UniversalPrimalProvider>> {
        let context_index = self.context_index.read().await;
        let providers = self.providers.read().await;
        
        if let Some(instance_ids) = context_index.get(&context.user_id) {
            instance_ids.iter()
                .filter_map(|id| providers.get(id))
                .filter(|provider| provider.can_serve_context(context))
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Route a request to appropriate provider
    pub async fn route_request(&self, request: PrimalRequest) -> ToadStoolResult<PrimalResponse> {
        let providers = self.providers.read().await;
        
        if let Some(provider) = providers.get(&request.target) {
            provider.handle_primal_request(request).await
        } else {
            Err(ToadStoolError::execution(format!("Target primal not found: {}", request.target)))
        }
    }
    
    /// Get all registered providers
    pub async fn get_all_providers(&self) -> Vec<Arc<dyn UniversalPrimalProvider>> {
        self.providers.read().await.values().cloned().collect()
    }
}

//
// ============================================================================
// UNIVERSAL JOB TYPES
// ============================================================================
//

/// Job priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum JobPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
    Emergency = 4,
}

/// Universal job types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UniversalJobType {
    /// Native process execution
    Native {
        executable: String,
        args: Vec<String>,
        env: HashMap<String, String>,
    },
    /// WebAssembly execution
    Wasm {
        module: Vec<u8>,
        args: Vec<String>,
        env: HashMap<String, String>,
    },
    /// Primal delegation
    Primal {
        primal_type: String,
        endpoint: String,
        payload: serde_json::Value,
    },
    /// BiomeOS orchestration
    BiomeOS {
        biome_manifest: serde_json::Value,
        team_id: String,
    },
}

/// Universal job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalJob {
    /// Job ID
    pub id: Uuid,
    /// Job type
    pub job_type: UniversalJobType,
    /// Job priority
    pub priority: JobPriority,
    /// Resource requirements
    pub resources: ResourceRequirements,
    /// Execution timeout
    pub timeout: Option<Duration>,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Context
    pub context: PrimalContext,
}

//
// ============================================================================
// RESOURCE MANAGEMENT
// ============================================================================
//

/// System resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemResources {
    /// CPU cores
    pub cpu_cores: f64,
    /// Memory in bytes
    pub memory_bytes: u64,
    /// Storage in bytes
    pub storage_bytes: u64,
    /// Network bandwidth
    pub network_bandwidth: u64,
    /// GPU units
    pub gpu_units: u32,
    /// Special hardware
    pub special_hardware: HashMap<String, u32>,
}

/// Resource allocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocation {
    /// Job ID
    pub job_id: Uuid,
    /// Allocated resources
    pub allocated_resources: ResourceRequirements,
    /// Allocation timestamp
    pub allocated_at: chrono::DateTime<chrono::Utc>,
    /// Release timestamp
    pub released_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Resource coordinator
pub struct ResourceCoordinator {
    /// Available resources
    available_resources: Arc<RwLock<SystemResources>>,
    /// Allocation history
    allocation_history: Arc<RwLock<Vec<ResourceAllocation>>>,
}

impl ResourceCoordinator {
    /// Create new resource coordinator
    pub async fn new() -> ToadStoolResult<Self> {
        let available_resources = SystemResources {
            cpu_cores: 8.0, // Default to 8 cores
            memory_bytes: 8 * 1024 * 1024 * 1024, // 8GB default
            storage_bytes: 100 * 1024 * 1024 * 1024, // 100GB default
            network_bandwidth: 1000 * 1024 * 1024, // 1Gbps default
            gpu_units: 0,
            special_hardware: HashMap::new(),
        };
        
        Ok(Self {
            available_resources: Arc::new(RwLock::new(available_resources)),
            allocation_history: Arc::new(RwLock::new(Vec::new())),
        })
    }
    
    /// Allocate resources
    pub async fn allocate_resources(&self, requirements: &ResourceRequirements) -> ToadStoolResult<ResourceAllocation> {
        let allocation = ResourceAllocation {
            job_id: Uuid::new_v4(),
            allocated_resources: requirements.clone(),
            allocated_at: chrono::Utc::now(),
            released_at: None,
        };
        
        self.allocation_history.write().await.push(allocation.clone());
        debug!("Allocated resources for job: {}", allocation.job_id);
        Ok(allocation)
    }
    
    /// Release resources
    pub async fn release_resources(&self, mut allocation: ResourceAllocation) -> ToadStoolResult<()> {
        allocation.released_at = Some(chrono::Utc::now());
        
        // Add to history
        self.allocation_history.write().await.push(allocation);
        
        debug!("Released resources for job");
        Ok(())
    }
    
    /// Get available resources
    pub async fn get_available_resources(&self) -> SystemResources {
        self.available_resources.read().await.clone()
    }
}

//
// ============================================================================
// UNIVERSAL SCHEDULER
// ============================================================================
//

/// Universal scheduler for any substrate
pub struct UniversalScheduler {
    /// Primal registry
    primal_registry: Arc<UniversalPrimalRegistry>,
    /// Resource coordinator
    resource_coordinator: Arc<ResourceCoordinator>,
    /// Active jobs
    active_jobs: Arc<RwLock<HashMap<Uuid, UniversalJob>>>,
}

impl UniversalScheduler {
    /// Create new scheduler
    pub async fn new(primal_registry: Arc<UniversalPrimalRegistry>) -> ToadStoolResult<Self> {
        Ok(Self {
            primal_registry,
            resource_coordinator: Arc::new(ResourceCoordinator::new().await?),
            active_jobs: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    /// Schedule a job
    pub async fn schedule_job(&self, job: UniversalJob) -> ToadStoolResult<ExecutionResponse> {
        let job_id = job.id;
        info!("Scheduling job: {}", job_id);
        
        // Add to active jobs
        self.active_jobs.write().await.insert(job_id, job.clone());
        
        // Allocate resources
        let _allocation = self.resource_coordinator.allocate_resources(&job.resources).await?;
        
        // Execute based on job type
        let result = match &job.job_type {
            UniversalJobType::Native { executable, args, env } => {
                self.execute_native(executable, args, env).await
            }
            UniversalJobType::Wasm { module, args, env } => {
                self.execute_wasm(module, args, env).await
            }
            UniversalJobType::Primal { primal_type, endpoint, payload } => {
                self.execute_primal(primal_type, endpoint, payload).await
            }
            UniversalJobType::BiomeOS { biome_manifest, team_id } => {
                self.execute_biome_os(biome_manifest, team_id).await
            }
        };
        
        // Remove from active jobs
        self.active_jobs.write().await.remove(&job_id);
        
        result
    }

    /// Get active job count
    pub async fn get_active_job_count(&self) -> usize {
        self.active_jobs.read().await.len()
    }

    /// Find primals by capability using the registry
    pub async fn find_primals_by_capability(&self, capability: &PrimalCapability) -> Vec<Arc<dyn UniversalPrimalProvider>> {
        self.primal_registry.find_by_capability(capability).await
    }
    
    async fn execute_native(&self, executable: &str, args: &[String], env: &HashMap<String, String>) -> ToadStoolResult<ExecutionResponse> {
        debug!("Executing native job: {} with args: {:?}", executable, args);
        
        // Try to find a native runtime engine through the primal registry
        let native_capability = PrimalCapability::NativeExecution { 
            architectures: vec!["x86_64".to_string(), "aarch64".to_string()] 
        };
        
        let providers = self.primal_registry.find_by_capability(&native_capability).await;
        
        if let Some(provider) = providers.first() {
            // Create a primal request for native execution
            let request = PrimalRequest {
                id: Uuid::new_v4(),
                source: "toadstool".to_string(),
                target: provider.primal_id().to_string(),
                request_type: "execute_native".to_string(),
                payload: serde_json::json!({
                    "executable": executable,
                    "args": args,
                    "env": env
                }),
                context: PrimalContext {
                    user_id: "system".to_string(),
                    device_id: "local".to_string(),
                    session_id: Uuid::new_v4().to_string(),
                    network_location: NetworkLocation {
                        ip_address: "127.0.0.1".to_string(),
                        subnet: None,
                        network_id: None,
                        geo_location: None,
                    },
                    security_level: SecurityLevel::Standard,
                    metadata: HashMap::new(),
                },
                metadata: HashMap::new(),
                timestamp: chrono::Utc::now(),
            };
            
            let response = provider.handle_primal_request(request).await?;
            
            // Convert primal response to execution response
            Ok(ExecutionResponse {
                execution_id: response.request_id,
                status: match response.status {
                    ResponseStatus::Success => crate::execution::ExecutionStatus::Success,
                    ResponseStatus::Error { message, .. } => crate::execution::ExecutionStatus::Failed { error: message },
                    ResponseStatus::Timeout => crate::execution::ExecutionStatus::TimedOut,
                    ResponseStatus::ServiceUnavailable => crate::execution::ExecutionStatus::Failed { 
                        error: "Service unavailable".to_string() 
                    },
                },
                output: crate::execution::ExecutionOutput {
                    data: response.payload.to_string().into_bytes(),
                    stdout: response.payload.get("stdout").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    stderr: response.payload.get("stderr").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    exit_code: response.payload.get("exit_code").and_then(|v| v.as_i64()).map(|i| i as i32),
                    format: Some("application/json".to_string()),
                    result: HashMap::new(),
                    metadata: response.metadata,
                },
                metrics: crate::RuntimeMetrics::default(),
                duration: Duration::from_millis(100),
                runtime_used: crate::execution::RuntimeType::Native,
                warnings: Vec::new(),
            })
        } else {
            // Fallback to local execution if no primal provider available
            use std::process::Command;
            
            let start_time = std::time::Instant::now();
            let mut cmd = Command::new(executable);
            cmd.args(args);
            
            for (key, value) in env {
                cmd.env(key, value);
            }
            
            match cmd.output() {
                Ok(output) => {
                    let duration = start_time.elapsed();
                    Ok(ExecutionResponse {
                        execution_id: Uuid::new_v4(),
                        status: if output.status.success() {
                            crate::execution::ExecutionStatus::Success
                        } else {
                            crate::execution::ExecutionStatus::Failed {
                                error: format!("Process exited with code: {:?}", output.status.code())
                            }
                        },
                        output: crate::execution::ExecutionOutput {
                            data: output.stdout.clone(),
                            stdout: Some(String::from_utf8_lossy(&output.stdout).to_string()),
                            stderr: Some(String::from_utf8_lossy(&output.stderr).to_string()),
                            exit_code: output.status.code(),
                            format: Some("text/plain".to_string()),
                            result: HashMap::new(),
                            metadata: HashMap::new(),
                        },
                        metrics: crate::RuntimeMetrics::default(),
                        duration,
                        runtime_used: crate::execution::RuntimeType::Native,
                        warnings: Vec::new(),
                    })
                }
                Err(e) => Err(ToadStoolError::execution(format!("Failed to execute native command: {}", e))),
            }
        }
    }
    
    async fn execute_wasm(&self, module: &[u8], args: &[String], env: &HashMap<String, String>) -> ToadStoolResult<ExecutionResponse> {
        debug!("Executing WASM job with {} bytes, args: {:?}", module.len(), args);
        
        // Try to find a WASM runtime engine through the primal registry
        let wasm_capability = PrimalCapability::WasmExecution { wasi_support: true };
        let providers = self.primal_registry.find_by_capability(&wasm_capability).await;
        
        if let Some(provider) = providers.first() {
            // Create a primal request for WASM execution
            let request = PrimalRequest {
                id: Uuid::new_v4(),
                source: "toadstool".to_string(),
                target: provider.primal_id().to_string(),
                request_type: "execute_wasm".to_string(),
                payload: serde_json::json!({
                    "module": base64::engine::general_purpose::STANDARD.encode(module),
                    "args": args,
                    "env": env
                }),
                context: PrimalContext {
                    user_id: "system".to_string(),
                    device_id: "local".to_string(),
                    session_id: Uuid::new_v4().to_string(),
                    network_location: NetworkLocation {
                        ip_address: "127.0.0.1".to_string(),
                        subnet: None,
                        network_id: None,
                        geo_location: None,
                    },
                    security_level: SecurityLevel::Standard,
                    metadata: HashMap::new(),
                },
                metadata: HashMap::new(),
                timestamp: chrono::Utc::now(),
            };
            
            let response = provider.handle_primal_request(request).await?;
            
            // Convert primal response to execution response
            Ok(ExecutionResponse {
                execution_id: response.request_id,
                status: match response.status {
                    ResponseStatus::Success => crate::execution::ExecutionStatus::Success,
                    ResponseStatus::Error { message, .. } => crate::execution::ExecutionStatus::Failed { error: message },
                    ResponseStatus::Timeout => crate::execution::ExecutionStatus::TimedOut,
                    ResponseStatus::ServiceUnavailable => crate::execution::ExecutionStatus::Failed { 
                        error: "Service unavailable".to_string() 
                    },
                },
                output: crate::execution::ExecutionOutput {
                    data: response.payload.to_string().into_bytes(),
                    stdout: response.payload.get("stdout").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    stderr: response.payload.get("stderr").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    exit_code: response.payload.get("exit_code").and_then(|v| v.as_i64()).map(|i| i as i32),
                    format: Some("application/json".to_string()),
                    result: HashMap::new(),
                    metadata: response.metadata,
                },
                metrics: crate::RuntimeMetrics::default(),
                duration: Duration::from_millis(150),
                runtime_used: crate::execution::RuntimeType::Wasm,
                warnings: Vec::new(),
            })
        } else {
            // Return error if no WASM runtime available
            Err(ToadStoolError::execution("No WASM runtime engine available"))
        }
    }
    
    async fn execute_primal(&self, primal_type: &str, endpoint: &str, payload: &serde_json::Value) -> ToadStoolResult<ExecutionResponse> {
        debug!("Executing primal job: {} at {}", primal_type, endpoint);
        
        // Find the appropriate primal provider by type
        let providers = self.primal_registry.get_all_providers().await;
        let provider = providers.iter()
            .find(|p| p.primal_id() == primal_type || 
                     format!("{:?}", p.primal_type()).to_lowercase().contains(primal_type))
            .ok_or_else(|| ToadStoolError::execution(format!("Primal provider {} not found", primal_type)))?;
        
        // Create a primal request
        let request = PrimalRequest {
            id: Uuid::new_v4(),
            source: "toadstool".to_string(),
            target: provider.primal_id().to_string(),
            request_type: "execute".to_string(),
            payload: payload.clone(),
            context: PrimalContext {
                user_id: "system".to_string(),
                device_id: "local".to_string(),
                session_id: Uuid::new_v4().to_string(),
                network_location: NetworkLocation {
                    ip_address: "127.0.0.1".to_string(),
                    subnet: None,
                    network_id: None,
                    geo_location: None,
                },
                security_level: SecurityLevel::Standard,
                metadata: HashMap::new(),
            },
            metadata: HashMap::new(),
            timestamp: chrono::Utc::now(),
        };
        
        let response = provider.handle_primal_request(request).await?;
        
        // Convert primal response to execution response
        Ok(ExecutionResponse {
            execution_id: response.request_id,
            status: match response.status {
                ResponseStatus::Success => crate::execution::ExecutionStatus::Success,
                ResponseStatus::Error { message, .. } => crate::execution::ExecutionStatus::Failed { error: message },
                ResponseStatus::Timeout => crate::execution::ExecutionStatus::TimedOut,
                ResponseStatus::ServiceUnavailable => crate::execution::ExecutionStatus::Failed { 
                    error: "Service unavailable".to_string() 
                },
            },
            output: crate::execution::ExecutionOutput {
                data: response.payload.to_string().into_bytes(),
                stdout: Some(format!("Primal {primal_type} execution completed")),
                stderr: None,
                exit_code: Some(0),
                format: Some("application/json".to_string()),
                result: HashMap::new(),
                metadata: response.metadata,
            },
            metrics: crate::RuntimeMetrics::default(),
            duration: Duration::from_millis(200),
            runtime_used: crate::execution::RuntimeType::Native,
            warnings: Vec::new(),
        })
    }
    
    async fn execute_biome_os(&self, biome_manifest: &serde_json::Value, team_id: &str) -> ToadStoolResult<ExecutionResponse> {
        debug!("Executing BiomeOS job for team: {}", team_id);
        
        // Try to find a BiomeOS primal provider
        let providers = self.primal_registry.get_all_providers().await;
        let biomeos_provider = providers.iter()
            .find(|p| matches!(p.primal_type(), PrimalType::OS))
            .ok_or_else(|| ToadStoolError::execution("BiomeOS primal provider not found"))?;
        
        // Create a primal request for BiomeOS execution
        let request = PrimalRequest {
            id: Uuid::new_v4(),
            source: "toadstool".to_string(),
            target: biomeos_provider.primal_id().to_string(),
            request_type: "deploy_biome".to_string(),
            payload: serde_json::json!({
                "biome_manifest": biome_manifest,
                "team_id": team_id
            }),
            context: PrimalContext {
                user_id: team_id.to_string(),
                device_id: "local".to_string(),
                session_id: Uuid::new_v4().to_string(),
                network_location: NetworkLocation {
                    ip_address: "127.0.0.1".to_string(),
                    subnet: None,
                    network_id: None,
                    geo_location: None,
                },
                security_level: SecurityLevel::High,
                metadata: HashMap::new(),
            },
            metadata: HashMap::new(),
            timestamp: chrono::Utc::now(),
        };
        
        let response = biomeos_provider.handle_primal_request(request).await?;
        
        // Convert primal response to execution response
        Ok(ExecutionResponse {
            execution_id: response.request_id,
            status: match response.status {
                ResponseStatus::Success => crate::execution::ExecutionStatus::Success,
                ResponseStatus::Error { message, .. } => crate::execution::ExecutionStatus::Failed { error: message },
                ResponseStatus::Timeout => crate::execution::ExecutionStatus::TimedOut,
                ResponseStatus::ServiceUnavailable => crate::execution::ExecutionStatus::Failed { 
                    error: "BiomeOS service unavailable".to_string() 
                },
            },
            output: crate::execution::ExecutionOutput {
                data: response.payload.to_string().into_bytes(),
                stdout: Some(format!("BiomeOS deployment for team {team_id} completed")),
                stderr: None,
                exit_code: Some(0),
                format: Some("text/plain".to_string()),
                result: HashMap::new(),
                metadata: response.metadata,
            },
            metrics: crate::RuntimeMetrics::default(),
            duration: Duration::from_millis(300),
            runtime_used: crate::execution::RuntimeType::Native,
            warnings: Vec::new(),
        })
    }
}

//
// ============================================================================
// UNIVERSAL COMPUTE PLATFORM
// ============================================================================
//

/// Platform configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalPlatformConfig {
    /// Enable recursive hosting
    pub recursive_hosting: bool,
    /// Enable ecosystem integration
    pub ecosystem_integration: bool,
    /// Enable BiomeOS integration
    pub biomeos_integration: bool,
    /// Maximum concurrent jobs
    pub max_concurrent_jobs: u32,
    /// Pure ecosystem mode
    pub pure_ecosystem: bool,
}

impl Default for UniversalPlatformConfig {
    fn default() -> Self {
        Self {
            recursive_hosting: true,
            ecosystem_integration: true,
            biomeos_integration: true,
            max_concurrent_jobs: 100,
            pure_ecosystem: false,
        }
    }
}

/// Universal compute platform
pub struct UniversalComputePlatform {
    /// Platform configuration
    config: UniversalPlatformConfig,
    /// Runtime engines
    runtime_engines: Arc<RwLock<HashMap<RuntimeType, Box<dyn RuntimeEngine>>>>,
    /// Universal scheduler
    scheduler: Arc<UniversalScheduler>,
    /// Primal registry
    primal_registry: Arc<UniversalPrimalRegistry>,
    /// ToadStool primal provider
    toadstool_provider: Option<Arc<ToadStoolPrimalProvider>>,
}

impl UniversalComputePlatform {
    /// Create new platform
    pub async fn new() -> ToadStoolResult<Self> {
        Self::new_with_config(UniversalPlatformConfig::default()).await
    }
    
    /// Create new platform with config
    pub async fn new_with_config(config: UniversalPlatformConfig) -> ToadStoolResult<Self> {
        let primal_registry = Arc::new(UniversalPrimalRegistry::new());
        let scheduler = Arc::new(UniversalScheduler::new(primal_registry.clone()).await?);
        
        let mut platform = Self {
            config,
            runtime_engines: Arc::new(RwLock::new(HashMap::new())),
            scheduler,
            primal_registry,
            toadstool_provider: None,
        };
        
        // Register ToadStool as a primal provider
        platform.register_as_universal_primal().await?;
        
        info!("Universal compute platform initialized");
        Ok(platform)
    }
    
    /// Register ToadStool as a universal primal
    async fn register_as_universal_primal(&mut self) -> ToadStoolResult<()> {
        let context = PrimalContext {
            user_id: "system".to_string(),
            device_id: "localhost".to_string(),
            session_id: Uuid::new_v4().to_string(),
            network_location: NetworkLocation {
                ip_address: "127.0.0.1".to_string(),
                subnet: None,
                network_id: None,
                geo_location: None,
            },
            security_level: SecurityLevel::Standard,
            metadata: HashMap::new(),
        };
        
        let provider = Arc::new(ToadStoolPrimalProvider::new(context));
        self.primal_registry.register_primal(provider.clone()).await?;
        self.toadstool_provider = Some(provider);
        
        info!("ToadStool registered as universal primal");
        Ok(())
    }
    
    /// Execute a universal job
    pub async fn execute_universal_job(&self, job: UniversalJob) -> ToadStoolResult<ExecutionResponse> {
        self.scheduler.schedule_job(job).await
    }
    
    /// Register a runtime engine
    pub async fn register_runtime_engine(&self, runtime_type: RuntimeType, engine: Box<dyn RuntimeEngine>) -> ToadStoolResult<()> {
        self.runtime_engines.write().await.insert(runtime_type, engine);
        Ok(())
    }
    
    /// Get available runtime types
    pub async fn get_available_runtimes(&self) -> Vec<RuntimeType> {
        self.runtime_engines.read().await.keys().cloned().collect()
    }
    
    /// Find primals by capability
    pub async fn find_primals_by_capability(&self, capability: &PrimalCapability) -> Vec<Arc<dyn UniversalPrimalProvider>> {
        self.primal_registry.find_by_capability(capability).await
    }
    
    /// Route primal request
    pub async fn route_primal_request(&self, request: PrimalRequest) -> ToadStoolResult<PrimalResponse> {
        self.primal_registry.route_request(request).await
    }
    
    /// Discover ecosystem (legacy compatibility)
    pub async fn discover_ecosystem(&self) -> ToadStoolResult<()> {
        if !self.config.ecosystem_integration {
            debug!("Ecosystem integration disabled in configuration");
            return Ok(());
        }
        
        info!("Discovering ecosystem through universal primal discovery");
        let _providers = self.primal_registry.get_all_providers().await;
        Ok(())
    }

    /// Get platform configuration
    pub fn get_config(&self) -> &UniversalPlatformConfig {
        &self.config
    }

    /// Check if recursive hosting is enabled
    pub fn is_recursive_hosting_enabled(&self) -> bool {
        self.config.recursive_hosting
    }

    /// Check if BiomeOS integration is enabled
    pub fn is_biomeos_integration_enabled(&self) -> bool {
        self.config.biomeos_integration
    }
}

//
// ============================================================================
// TOADSTOOL PRIMAL PROVIDER
// ============================================================================
//

/// ToadStool primal provider implementation
pub struct ToadStoolPrimalProvider {
    /// Context
    context: PrimalContext,
    /// Health status
    health_status: Arc<RwLock<PrimalHealth>>,
}

impl ToadStoolPrimalProvider {
    /// Create new ToadStool primal provider
    pub fn new(context: PrimalContext) -> Self {
        Self {
            context,
            health_status: Arc::new(RwLock::new(PrimalHealth::Healthy)),
        }
    }
}

#[async_trait]
impl UniversalPrimalProvider for ToadStoolPrimalProvider {
    fn primal_id(&self) -> &str {
        "toadstool"
    }
    
    fn instance_id(&self) -> &str {
        "toadstool-main"
    }
    
    fn context(&self) -> &PrimalContext {
        &self.context
    }
    
    fn primal_type(&self) -> PrimalType {
        PrimalType::Compute
    }
    
    fn capabilities(&self) -> Vec<PrimalCapability> {
        vec![
            PrimalCapability::NativeExecution {
                architectures: vec!["x86_64".to_string(), "aarch64".to_string()],
            },
            PrimalCapability::ContainerRuntime {
                orchestrators: vec!["docker".to_string(), "podman".to_string()],
            },
            PrimalCapability::WasmExecution {
                wasi_support: true,
            },
            PrimalCapability::ServerlessExecution {
                languages: vec!["rust".to_string(), "python".to_string(), "javascript".to_string()],
            },
            PrimalCapability::LoadBalancing {
                algorithms: vec!["round_robin".to_string(), "least_connections".to_string()],
            },
            PrimalCapability::AutoScaling {
                metrics: vec!["cpu".to_string(), "memory".to_string(), "requests".to_string()],
            },
        ]
    }
    
    async fn health_check(&self) -> PrimalHealth {
        self.health_status.read().await.clone()
    }
    
    fn endpoints(&self) -> PrimalEndpoints {
        PrimalEndpoints {
            primary: format!("http://{}:{}", network::DEFAULT_LOCALHOST, network::DEFAULT_TOADSTOOL_PORT),
            health: format!("http://{}:{}/health", network::DEFAULT_LOCALHOST, network::DEFAULT_TOADSTOOL_PORT),
            metrics: Some(format!("http://{}:{}/metrics", network::DEFAULT_LOCALHOST, network::DEFAULT_TOADSTOOL_PORT)),
            admin: Some(format!("http://{}:{}/admin", network::DEFAULT_LOCALHOST, network::DEFAULT_TOADSTOOL_PORT)),
            websocket: Some(format!("ws://{}:{}/ws", network::DEFAULT_LOCALHOST, network::DEFAULT_TOADSTOOL_PORT)),
            custom: HashMap::new(),
        }
    }
    
    async fn handle_primal_request(&self, request: PrimalRequest) -> ToadStoolResult<PrimalResponse> {
        debug!("Handling primal request: {:?}", request.request_type);
        
        Ok(PrimalResponse {
            request_id: request.id,
            status: ResponseStatus::Success,
            payload: serde_json::json!({
                "message": "Request processed successfully",
                "request_type": request.request_type
            }),
            metadata: HashMap::new(),
            timestamp: chrono::Utc::now(),
        })
    }
    
    async fn initialize(&mut self, _config: serde_json::Value) -> ToadStoolResult<()> {
        info!("ToadStool primal provider initialized");
        Ok(())
    }
    
    async fn shutdown(&mut self) -> ToadStoolResult<()> {
        info!("ToadStool primal provider shutting down");
        Ok(())
    }
    
    fn can_serve_context(&self, context: &PrimalContext) -> bool {
        // ToadStool can serve any context with appropriate security level
        context.security_level <= self.context.security_level
    }
}

//
// ============================================================================
// PLATFORM STATUS AND INITIALIZATION
// ============================================================================
//

/// Platform status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlatformStatus {
    /// Initializing
    Initializing,
    /// Running
    Running,
    /// Degraded
    Degraded,
    /// Stopped
    Stopped,
}

/// Initialize platform with runtime engines
pub async fn init_with_runtime_engines(
    engines: Vec<(RuntimeType, Box<dyn RuntimeEngine>)>
) -> ToadStoolResult<UniversalComputePlatform> {
    let platform = UniversalComputePlatform::new().await?;
    
    for (runtime_type, engine) in engines {
        platform.register_runtime_engine(runtime_type, engine).await?;
    }
    
    Ok(platform)
}

/// Get platform status
pub async fn get_platform_status() -> PlatformStatus {
    // For now, always return running
    PlatformStatus::Running
}

