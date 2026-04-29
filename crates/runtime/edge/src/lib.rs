// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]
#![allow(
    missing_docs,
    reason = "edge/IoT platform types: hardware enum variants are self-documenting by name"
)]

//! # ToadStool Edge/IoT Runtime Engine
//!
//! Universal compute orchestration for edge devices, IoT platforms, and embedded systems.
//! Supports Arduino, ESP32, Raspberry Pi, and other edge computing platforms.

pub mod communication;
pub mod deployment;
pub mod discovery;
pub mod platforms;
pub mod toolchain;

#[cfg(target_os = "linux")]
pub mod udev_pure;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

use toadstool::{
    WorkloadType,
    error::{ToadStoolError, ToadStoolResult},
    execution::{
        ExecutionRequest, ExecutionResponse, ExecutionStatus, RuntimeCapabilities, RuntimeConfig,
        RuntimeEngine,
    },
    resources::{
        CpuMetrics, MemoryMetrics, NetworkMetrics, RuntimeMetrics, StorageMetrics, TimingMetrics,
    },
};

// platforms: 30+ items (enums, structs, traits, device types); wildcard retained
pub use communication::{CommunicationManager, CommunicationProtocol};
pub use deployment::{DeploymentCoordinator, DeploymentInfo, DeploymentStatus, DeploymentStrategy};
pub use discovery::{
    BluetoothDiscovery, DeviceDiscoveryService, DiscoveryMethod, MDNSDiscovery, NetworkDiscovery,
    SerialPortDiscovery, USBDiscovery,
};
pub use platforms::*;
pub use toolchain::{
    CompilationCache, CompilationTarget, CrossCompilationToolchain, OutputFormat, ToolchainInfo,
};

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
    #[expect(
        dead_code,
        reason = "held for lifecycle; protocols accessed via discovery + deployment"
    )]
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
    pub started_at: std::time::SystemTime,
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
        // Use XDG-compliant path resolution for cache
        let cache_path = toadstool_common::platform_paths::PlatformPaths::new(
            &toadstool_common::platform_paths::PathEnv::from_env(),
        )
        .toadstool_cache_dir()
        .join("edge_cache")
        .to_string_lossy()
        .into_owned();

        Self {
            discovery_enabled: true,
            discovery_timeout_secs: 30,
            max_devices: 100,
            communication_timeout_ms: 5000,
            cross_compile_cache_path: cache_path,
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
            info!(
                "Discovered device: {} ({})",
                device_id,
                device.get_platform()
            );
            devices.insert(device_id, device);
        }

        info!(
            "Device discovery completed. Found {} devices",
            devices.len()
        );
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
        let device = devices
            .get(&device_id)
            .ok_or_else(|| ToadStoolError::not_found(format!("Device {} not found", device_id)))?;

        // Cross-compile if necessary
        let compiled_code = self.toolchain.cross_compile(code, &target_platform).await?;

        // Deploy to device
        let deployment_id = self
            .deployment
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
        let device = devices
            .get(&device_id)
            .ok_or_else(|| ToadStoolError::not_found(format!("Device {} not found", device_id)))?;

        let execution_id = Uuid::new_v4();
        let started_at = std::time::SystemTime::now();

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
                    Err(e) => ExecutionStatus::Failed {
                        error: e.to_string().into(),
                    },
                };
            }
        }

        result
    }
}

impl RuntimeEngine for EdgeRuntimeEngine {
    fn initialize(
        &mut self,
        _config: RuntimeConfig,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        Box::pin(async { Ok(()) })
    }

    fn execute(
        &self,
        request: ExecutionRequest,
    ) -> impl Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_ {
        Box::pin(async move {
            info!(
                "Executing request on edge runtime: {}",
                request.execution_id
            );

            let suitable_device = self.find_suitable_device(&request).await?;
            self.execute_on_device(suitable_device, request).await
        })
    }

    fn get_capabilities(&self) -> RuntimeCapabilities {
        let mut platform_features = HashMap::new();
        platform_features.insert("edge_computing".to_string(), true);
        platform_features.insert("iot_orchestration".to_string(), true);
        platform_features.insert("embedded_systems".to_string(), true);
        platform_features.insert("cross_compilation".to_string(), true);
        platform_features.insert("device_discovery".to_string(), true);
        platform_features.insert("real_time_execution".to_string(), true);

        RuntimeCapabilities {
            supported_workloads: vec![
                WorkloadType::Native,
                WorkloadType::Wasm,
                WorkloadType::Python,
                WorkloadType::AiMl,
            ],
            max_concurrent_executions: Some(self.config.max_devices as u32),
            supported_architectures: vec![
                "x86_64".to_string(),
                "aarch64".to_string(),
                "arm".to_string(),
            ],
            platform_features,
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    fn supports_workload(&self, workload_type: &WorkloadType) -> bool {
        matches!(
            workload_type,
            WorkloadType::Native | WorkloadType::Wasm | WorkloadType::Python | WorkloadType::AiMl
        )
    }

    fn get_metrics(&self) -> impl Future<Output = ToadStoolResult<RuntimeMetrics>> + Send + '_ {
        Box::pin(async {
            let executions = self.active_executions.read().await;
            let mut total_cpu = 0.0;
            let mut total_memory = 0u64;
            let mut total_storage = 0u64;
            let mut total_network_sent = 0u64;
            let mut total_network_received = 0u64;

            for handle in executions.values() {
                total_cpu += handle.resource_usage.cpu_percent;
                total_memory += handle.resource_usage.memory_bytes;
                total_storage += handle.resource_usage.storage_bytes;
                total_network_sent += handle.resource_usage.network_bytes_sent;
                total_network_received += handle.resource_usage.network_bytes_received;
            }

            let start_time = SystemTime::now();
            Ok(RuntimeMetrics {
                cpu: CpuMetrics {
                    usage_percent: total_cpu,
                    cores_used: 0.0,
                    cpu_time_seconds: 0.0,
                },
                memory: MemoryMetrics {
                    usage_percent: 0.0,
                    used_bytes: total_memory,
                    peak_bytes: total_memory,
                },
                storage: StorageMetrics {
                    usage_percent: 0.0,
                    used_bytes: total_storage,
                    bytes_read: 0,
                    bytes_written: 0,
                },
                network: NetworkMetrics {
                    bytes_sent: total_network_sent,
                    bytes_received: total_network_received,
                    packets_sent: 0,
                    packets_received: 0,
                },
                gpu: None,
                timing: TimingMetrics {
                    start_time,
                    end_time: Some(SystemTime::now()),
                    duration: start_time.elapsed().unwrap_or_default(),
                },
            })
        })
    }

    fn shutdown(&mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        Box::pin(async {
            info!("Cleaning up edge runtime engine");

            let mut executions = self.active_executions.write().await;
            for (id, handle) in executions.drain() {
                info!("Stopping execution: {} on device: {}", id, handle.device_id);
                if let Ok(devices) = self.devices.try_read()
                    && let Some(device) = devices.get(&handle.device_id)
                    && let Err(e) = device.stop_execution(id).await
                {
                    warn!(
                        "Failed to stop execution {} on device {}: {}",
                        id, handle.device_id, e
                    );
                }
            }

            let mut devices = self.devices.write().await;
            for (id, device) in devices.drain() {
                info!("Disconnecting device: {}", id);
                if let Err(e) = device.disconnect().await {
                    warn!("Failed to disconnect device {}: {}", id, e);
                }
            }

            info!("Edge runtime engine cleanup completed");
            Ok(())
        })
    }
}

impl EdgeRuntimeEngine {
    /// Find suitable device for execution based on requirements
    async fn find_suitable_device(&self, _request: &ExecutionRequest) -> ToadStoolResult<Uuid> {
        let devices = self.devices.read().await;

        if devices.is_empty() {
            return Err(ToadStoolError::not_found(
                "No edge devices available".to_string(),
            ));
        }

        // Simple selection strategy - pick first available device
        // Future enhancement: Implement sophisticated device selection based on:
        // - Resource requirements
        // - Device capabilities
        // - Current load
        // - Network latency
        // - Power constraints
        // Current implementation uses simple first-available selection

        let device_id = devices
            .keys()
            .next()
            .copied()
            .ok_or_else(|| ToadStoolError::not_found("No suitable device found".to_string()))?;

        Ok(device_id)
    }
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
