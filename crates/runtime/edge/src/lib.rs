//! # ToadStool Edge/IoT Runtime Engine
//!
//! Universal compute orchestration for edge devices, IoT platforms, and embedded systems.
//! Supports Arduino, ESP32, Raspberry Pi, and other edge computing platforms.

pub mod platforms;
pub mod discovery;
pub mod toolchain;
pub mod communication;
pub mod deployment;

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{info, warn, error};
use uuid::Uuid;

use toadstool::{
    error::{ToadStoolError, ToadStoolResult},
    execution::{
        ExecutionOutput, ExecutionRequest, ExecutionResponse, ExecutionStatus,
        RuntimeEngine,
    },
    resources::ResourceRequirements,
    security::SecurityContext,
};

pub use platforms::*;
pub use discovery::*;
pub use toolchain::*;
pub use communication::*;
pub use deployment::*;

/// Edge/IoT Runtime Engine Configuration
#[derive(Debug, Clone)]
pub struct EdgeRuntimeConfig {
    /// Enable device discovery
    pub discovery_enabled: bool,
    /// Device discovery timeout in seconds
    pub discovery_timeout_secs: u64,
    /// Maximum number of connected devices
    pub max_devices: usize,
    /// Default communication timeout in milliseconds
    pub communication_timeout_ms: u64,
    /// Cross-compilation cache path
    pub cross_compile_cache_path: String,
    /// Enable automatic device provisioning
    pub auto_provisioning: bool,
    /// Security isolation level for edge devices
    pub security_level: EdgeSecurityLevel,
    /// Resource allocation strategy
    pub resource_strategy: ResourceAllocationStrategy,
    /// Port registry for dynamic port management
    pub port_registry: toadstool_config::ports::PortRegistry,
}

#[derive(Debug, Clone)]
pub enum EdgeSecurityLevel {
    /// Minimal security for trusted environments
    Minimal,
    /// Standard security with basic isolation
    Standard,
    /// High security with comprehensive isolation
    High,
    /// Maximum security for critical systems
    Maximum,
}

#[derive(Debug, Clone)]
pub enum ResourceAllocationStrategy {
    /// Allocate resources based on device capabilities
    Adaptive,
    /// Conservative allocation with safety margins
    Conservative,
    /// Aggressive allocation for maximum performance
    Aggressive,
    /// Custom allocation based on predefined rules
    Custom(HashMap<String, f64>),
}

/// Edge Runtime Engine
pub struct EdgeRuntimeEngine {
    /// Configuration
    config: EdgeRuntimeConfig,
    /// Device discovery service
    discovery: Arc<DeviceDiscoveryService>,
    /// Connected devices
    devices: Arc<RwLock<HashMap<Uuid, Arc<dyn EdgeDevice>>>>,
    /// Cross-compilation toolchain
    toolchain: Arc<CrossCompilationToolchain>,
    /// Communication manager
    communication: Arc<CommunicationManager>,
    /// Deployment coordinator
    deployment: Arc<DeploymentCoordinator>,
    /// Active executions
    active_executions: Arc<RwLock<HashMap<Uuid, EdgeExecutionHandle>>>,
}

#[derive(Debug, Clone)]
pub struct EdgeExecutionHandle {
    pub id: Uuid,
    pub device_id: Uuid,
    pub platform: EdgePlatform,
    pub status: ExecutionStatus,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub resource_usage: ResourceUsage,
}

#[derive(Debug, Clone)]
pub struct ResourceUsage {
    pub cpu_percent: f64,
    pub memory_bytes: u64,
    pub storage_bytes: u64,
    pub network_bytes_sent: u64,
    pub network_bytes_received: u64,
}

impl Default for EdgeRuntimeConfig {
    fn default() -> Self {
        Self {
            discovery_enabled: true,
            discovery_timeout_secs: 30,
            max_devices: 100,
            communication_timeout_ms: 5000,
            cross_compile_cache_path: "/tmp/toadstool_edge_cache".to_string(),
            auto_provisioning: true,
            security_level: EdgeSecurityLevel::Standard,
            resource_strategy: ResourceAllocationStrategy::Adaptive,
            port_registry: toadstool_config::ports::PortRegistry::default(),
        }
    }
}

impl EdgeRuntimeEngine {
    /// Create a new edge runtime engine
    pub async fn new(config: EdgeRuntimeConfig) -> ToadStoolResult<Self> {
        info!("Initializing ToadStool Edge Runtime Engine");
        
        let discovery = Arc::new(DeviceDiscoveryService::new(&config).await?);
        let toolchain = Arc::new(CrossCompilationToolchain::new(&config).await?);
        let communication = Arc::new(CommunicationManager::new(&config).await?);
        let deployment = Arc::new(DeploymentCoordinator::new(&config).await?);
        
        let engine = Self {
            config,
            discovery,
            devices: Arc::new(RwLock::new(HashMap::new())),
            toolchain,
            communication,
            deployment,
            active_executions: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Start device discovery if enabled
        if engine.config.discovery_enabled {
            engine.start_device_discovery().await?;
        }
        
        info!("Edge Runtime Engine initialized successfully");
        Ok(engine)
    }
    
    /// Start device discovery process
    async fn start_device_discovery(&self) -> ToadStoolResult<()> {
        info!("Starting device discovery");
        
        let discovered_devices = self.discovery.discover_devices().await?;
        let mut devices = self.devices.write().await;
        
        for device in discovered_devices {
            let device_id = device.get_id();
            info!("Discovered device: {} ({})", device_id, device.get_platform());
            devices.insert(device_id, device);
        }
        
        info!("Device discovery completed. Found {} devices", devices.len());
        Ok(())
    }
    
    /// Get connected devices
    pub async fn get_connected_devices(&self) -> Vec<EdgeDeviceInfo> {
        let devices = self.devices.read().await;
        devices.values().map(|d| d.get_info()).collect()
    }
    
    /// Deploy code to specific device
    pub async fn deploy_to_device(
        &self,
        device_id: Uuid,
        code: &[u8],
        target_platform: EdgePlatform,
    ) -> ToadStoolResult<String> {
        let devices = self.devices.read().await;
        let device = devices.get(&device_id)
            .ok_or_else(|| ToadStoolError::not_found(format!("Device {} not found", device_id)))?;
        
        // Cross-compile if necessary
        let compiled_code = self.toolchain
            .cross_compile(code, &target_platform)
            .await?;
        
        // Deploy to device
        let deployment_id = self.deployment
            .deploy_to_device(device.as_ref(), &compiled_code)
            .await?;
        
        Ok(deployment_id)
    }
    
    /// Execute code on edge device
    pub async fn execute_on_device(
        &self,
        device_id: Uuid,
        request: ExecutionRequest,
    ) -> ToadStoolResult<ExecutionResponse> {
        let devices = self.devices.read().await;
        let device = devices.get(&device_id)
            .ok_or_else(|| ToadStoolError::not_found(format!("Device {} not found", device_id)))?;
        
        let execution_id = Uuid::new_v4();
        let started_at = chrono::Utc::now();
        
        // Create execution handle
        let handle = EdgeExecutionHandle {
            id: execution_id,
            device_id,
            platform: device.get_platform().clone(),
            status: ExecutionStatus::Running,
            started_at,
            resource_usage: ResourceUsage {
                cpu_percent: 0.0,
                memory_bytes: 0,
                storage_bytes: 0,
                network_bytes_sent: 0,
                network_bytes_received: 0,
            },
        };
        
        // Store execution handle
        {
            let mut executions = self.active_executions.write().await;
            executions.insert(execution_id, handle);
        }
        
        // Execute on device
        let result = device.execute(&request).await;
        
        // Update execution status
        {
            let mut executions = self.active_executions.write().await;
            if let Some(handle) = executions.get_mut(&execution_id) {
                handle.status = match &result {
                    Ok(_) => ExecutionStatus::Success,
                    Err(_) => ExecutionStatus::Failed,
                };
            }
        }
        
        result
    }
}

#[async_trait]
impl RuntimeEngine for EdgeRuntimeEngine {
    async fn execute(&self, request: ExecutionRequest) -> ToadStoolResult<ExecutionResponse> {
        info!("Executing request on edge runtime: {}", request.id);
        
        // Find suitable device for execution
        let suitable_device = self.find_suitable_device(&request).await?;
        
        // Execute on the selected device
        self.execute_on_device(suitable_device, request).await
    }
    
    async fn get_capabilities(&self) -> ToadStoolResult<Vec<String>> {
        let devices = self.devices.read().await;
        let mut capabilities = Vec::new();
        
        for device in devices.values() {
            capabilities.extend(device.get_capabilities());
        }
        
        // Add edge-specific capabilities
        capabilities.extend(vec![
            "edge_computing".to_string(),
            "iot_orchestration".to_string(),
            "embedded_systems".to_string(),
            "cross_compilation".to_string(),
            "device_discovery".to_string(),
            "real_time_execution".to_string(),
        ]);
        
        Ok(capabilities)
    }
    
    async fn get_resource_usage(&self) -> ToadStoolResult<HashMap<String, f64>> {
        let executions = self.active_executions.read().await;
        let mut usage = HashMap::new();
        
        let mut total_cpu = 0.0;
        let mut total_memory = 0;
        let mut total_storage = 0;
        let mut total_network_sent = 0;
        let mut total_network_received = 0;
        
        for handle in executions.values() {
            total_cpu += handle.resource_usage.cpu_percent;
            total_memory += handle.resource_usage.memory_bytes;
            total_storage += handle.resource_usage.storage_bytes;
            total_network_sent += handle.resource_usage.network_bytes_sent;
            total_network_received += handle.resource_usage.network_bytes_received;
        }
        
        usage.insert("cpu_percent".to_string(), total_cpu);
        usage.insert("memory_bytes".to_string(), total_memory as f64);
        usage.insert("storage_bytes".to_string(), total_storage as f64);
        usage.insert("network_bytes_sent".to_string(), total_network_sent as f64);
        usage.insert("network_bytes_received".to_string(), total_network_received as f64);
        usage.insert("active_executions".to_string(), executions.len() as f64);
        
        Ok(usage)
    }
    
    async fn cleanup(&self) -> ToadStoolResult<()> {
        info!("Cleaning up edge runtime engine");
        
        // Stop all active executions
        let mut executions = self.active_executions.write().await;
        for (id, handle) in executions.drain() {
            info!("Stopping execution: {} on device: {}", id, handle.device_id);
            // Send stop signal to device
            if let Ok(devices) = self.devices.try_read() {
                if let Some(device) = devices.get(&handle.device_id) {
                    if let Err(e) = device.stop_execution(id).await {
                        warn!("Failed to stop execution {} on device {}: {}", id, handle.device_id, e);
                    }
                }
            }
        }
        
        // Cleanup devices
        let mut devices = self.devices.write().await;
        for (id, device) in devices.drain() {
            info!("Disconnecting device: {}", id);
            if let Err(e) = device.disconnect().await {
                warn!("Failed to disconnect device {}: {}", id, e);
            }
        }
        
        info!("Edge runtime engine cleanup completed");
        Ok(())
    }
}

impl EdgeRuntimeEngine {
    /// Find suitable device for execution based on requirements
    async fn find_suitable_device(&self, request: &ExecutionRequest) -> ToadStoolResult<Uuid> {
        let devices = self.devices.read().await;
        
        if devices.is_empty() {
            return Err(ToadStoolError::not_found("No edge devices available".to_string()));
        }
        
        // Simple selection strategy - pick first available device
        // Future enhancement: Implement sophisticated device selection based on:
        // - Resource requirements
        // - Device capabilities
        // - Current load
        // - Network latency
        // - Power constraints
        // Current implementation uses simple first-available selection
        
        let device_id = devices.keys().next().copied()
            .ok_or_else(|| ToadStoolError::not_found("No suitable device found".to_string()))?;
        
        Ok(device_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_runtime_config_default() {
        let config = EdgeRuntimeConfig::default();
        assert_eq!(config.discovery_enabled, true);
        assert_eq!(config.discovery_timeout_secs, 30);
        assert_eq!(config.max_devices, 100);
        assert_eq!(config.auto_provisioning, true);
    }

    #[test]
    fn test_edge_runtime_config_custom() {
        let config = EdgeRuntimeConfig {
            discovery_enabled: false,
            discovery_timeout_secs: 60,
            max_devices: 50,
            communication_timeout_ms: 10000,
            cross_compile_cache_path: "/custom/cache".to_string(),
            auto_provisioning: false,
            security_level: EdgeSecurityLevel::High,
            resource_strategy: ResourceAllocationStrategy::Conservative,
        };

        assert_eq!(config.discovery_enabled, false);
        assert_eq!(config.max_devices, 50);
        assert_eq!(config.cross_compile_cache_path, "/custom/cache");
    }

    #[test]
    fn test_edge_security_level_variants() {
        let levels = vec![
            EdgeSecurityLevel::Minimal,
            EdgeSecurityLevel::Standard,
            EdgeSecurityLevel::High,
            EdgeSecurityLevel::Maximum,
        ];

        assert_eq!(levels.len(), 4);
    }

    #[test]
    fn test_resource_allocation_strategy_adaptive() {
        let strategy = ResourceAllocationStrategy::Adaptive;
        assert!(matches!(strategy, ResourceAllocationStrategy::Adaptive));
    }

    #[test]
    fn test_resource_allocation_strategy_custom() {
        let mut rules = HashMap::new();
        rules.insert("cpu_limit".to_string(), 0.8);
        rules.insert("memory_limit".to_string(), 0.7);

        let strategy = ResourceAllocationStrategy::Custom(rules);
        if let ResourceAllocationStrategy::Custom(rules) = strategy {
            assert_eq!(rules.len(), 2);
            assert_eq!(rules.get("cpu_limit"), Some(&0.8));
        } else {
            panic!("Expected Custom strategy");
        }
    }

    #[test]
    fn test_edge_execution_handle_creation() {
        let handle = EdgeExecutionHandle {
            id: Uuid::new_v4(),
            device_id: Uuid::new_v4(),
            platform: EdgePlatform::RaspberryPi,
            status: ExecutionStatus::Running,
            started_at: chrono::Utc::now(),
            resource_usage: ResourceUsage {
                cpu_percent: 25.5,
                memory_bytes: 1024000,
                storage_bytes: 512000,
                network_bytes_sent: 2048,
                network_bytes_received: 4096,
            },
        };

        assert_eq!(handle.status, ExecutionStatus::Running);
        assert_eq!(handle.resource_usage.cpu_percent, 25.5);
        assert_eq!(handle.resource_usage.memory_bytes, 1024000);
    }

    #[test]
    fn test_resource_usage_tracking() {
        let usage = ResourceUsage {
            cpu_percent: 50.0,
            memory_bytes: 2048000,
            storage_bytes: 1024000,
            network_bytes_sent: 5000,
            network_bytes_received: 10000,
        };

        assert_eq!(usage.cpu_percent, 50.0);
        assert_eq!(usage.memory_bytes, 2048000);
        assert_eq!(usage.network_bytes_sent, 5000);
        assert_eq!(usage.network_bytes_received, 10000);
    }

    #[test]
    fn test_edge_security_level_debug() {
        let level = EdgeSecurityLevel::High;
        let debug_str = format!("{:?}", level);
        assert!(debug_str.contains("High"));
    }

    #[test]
    fn test_resource_allocation_strategy_debug() {
        let strategy = ResourceAllocationStrategy::Aggressive;
        let debug_str = format!("{:?}", strategy);
        assert!(debug_str.contains("Aggressive"));
    }

    #[test]
    fn test_edge_runtime_config_clone() {
        let config1 = EdgeRuntimeConfig::default();
        let config2 = config1.clone();

        assert_eq!(config1.discovery_enabled, config2.discovery_enabled);
        assert_eq!(config1.max_devices, config2.max_devices);
    }

    #[test]
    fn test_edge_execution_handle_clone() {
        let handle1 = EdgeExecutionHandle {
            id: Uuid::new_v4(),
            device_id: Uuid::new_v4(),
            platform: EdgePlatform::ESP32,
            status: ExecutionStatus::Success,
            started_at: chrono::Utc::now(),
            resource_usage: ResourceUsage {
                cpu_percent: 10.0,
                memory_bytes: 512000,
                storage_bytes: 256000,
                network_bytes_sent: 1024,
                network_bytes_received: 2048,
            },
        };
        let handle2 = handle1.clone();

        assert_eq!(handle1.id, handle2.id);
        assert_eq!(handle1.device_id, handle2.device_id);
        assert_eq!(handle1.status, handle2.status);
    }

    #[test]
    fn test_resource_usage_clone() {
        let usage1 = ResourceUsage {
            cpu_percent: 75.0,
            memory_bytes: 4096000,
            storage_bytes: 2048000,
            network_bytes_sent: 8192,
            network_bytes_received: 16384,
        };
        let usage2 = usage1.clone();

        assert_eq!(usage1.cpu_percent, usage2.cpu_percent);
        assert_eq!(usage1.memory_bytes, usage2.memory_bytes);
        assert_eq!(usage1.storage_bytes, usage2.storage_bytes);
    }

    #[test]
    fn test_edge_runtime_config_cache_path() {
        let config = EdgeRuntimeConfig {
            discovery_enabled: true,
            discovery_timeout_secs: 30,
            max_devices: 100,
            communication_timeout_ms: 5000,
            cross_compile_cache_path: "/custom/edge/cache".to_string(),
            auto_provisioning: true,
            security_level: EdgeSecurityLevel::Standard,
            resource_strategy: ResourceAllocationStrategy::Adaptive,
        };

        assert_eq!(config.cross_compile_cache_path, "/custom/edge/cache");
    }

    #[test]
    fn test_edge_runtime_config_timeouts() {
        let config = EdgeRuntimeConfig {
            discovery_enabled: true,
            discovery_timeout_secs: 120,
            max_devices: 100,
            communication_timeout_ms: 15000,
            cross_compile_cache_path: "/tmp/cache".to_string(),
            auto_provisioning: true,
            security_level: EdgeSecurityLevel::Standard,
            resource_strategy: ResourceAllocationStrategy::Adaptive,
        };

        assert_eq!(config.discovery_timeout_secs, 120);
        assert_eq!(config.communication_timeout_ms, 15000);
    }

    #[test]
    fn test_resource_allocation_strategies() {
        let strategies = vec![
            ResourceAllocationStrategy::Adaptive,
            ResourceAllocationStrategy::Conservative,
            ResourceAllocationStrategy::Aggressive,
        ];

        assert_eq!(strategies.len(), 3);
    }

    #[test]
    fn test_edge_security_levels_ordering() {
        let minimal = EdgeSecurityLevel::Minimal;
        let standard = EdgeSecurityLevel::Standard;
        let high = EdgeSecurityLevel::High;
        let maximum = EdgeSecurityLevel::Maximum;

        assert!(matches!(minimal, EdgeSecurityLevel::Minimal));
        assert!(matches!(standard, EdgeSecurityLevel::Standard));
        assert!(matches!(high, EdgeSecurityLevel::High));
        assert!(matches!(maximum, EdgeSecurityLevel::Maximum));
    }
} 