// SPDX-License-Identifier: AGPL-3.0-or-later
//! Universal GPU Compute Engine Implementation

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

use toadstool::{
    error::{ToadStoolError, ToadStoolResult},
    execution::{
        ExecutionOutput, ExecutionRequest, ExecutionResponse, ExecutionStatus, RuntimeCapabilities,
        RuntimeConfig, RuntimeEngine, RuntimeType,
    },
    resources::{ResourceMonitor, RuntimeMetrics},
    WorkloadSpec, WorkloadType,
};

use super::compiler::UniversalKernelCompiler;
use super::config::{CompilationConfig, ResourceConfig, UniversalGpuConfig};
use super::coordinator::ComputeResourceCoordinator;
use super::strategy::{BackendSelectionStrategy, EvolutionMetrics};
use super::traits::ParallelComputeFramework;
use super::types::{
    ComputeEngineStatistics, ComputeResult, ComputeSession, ComputeWorkload, DeviceId,
    DeviceRequirements, GpuFramework, KernelFormat, SessionStatus, UniversalComputeDevice,
};

/// Universal GPU Compute Engine - the heart of parallel compute orchestration
pub struct UniversalGpuEngine {
    /// Discovered compute frameworks and their capabilities
    frameworks: Arc<RwLock<HashMap<GpuFramework, Arc<dyn ParallelComputeFramework>>>>,
    /// Available compute devices across all frameworks
    devices: Arc<RwLock<HashMap<DeviceId, UniversalComputeDevice>>>,
    /// Active compute sessions (supports recursive execution)
    active_sessions: Arc<RwLock<HashMap<Uuid, ComputeSession>>>,
    /// Universal kernel compiler and optimizer
    _kernel_compiler: Arc<UniversalKernelCompiler>,
    /// Device resource coordinator
    resource_coordinator: Arc<ComputeResourceCoordinator>,
    /// Configuration
    config: UniversalGpuConfig,
    /// Resource monitor
    resource_monitor: Option<Arc<dyn ResourceMonitor>>,
    /// Backend selection strategy (sovereign vs pragmatic)
    selection_strategy: BackendSelectionStrategy,
    /// Evolution metrics (ecosystem maturity tracking)
    evolution_metrics: Arc<RwLock<EvolutionMetrics>>,
}

impl UniversalGpuEngine {
    /// Create new GPU engine with default configuration
    pub async fn new() -> ToadStoolResult<Self> {
        Self::with_config(UniversalGpuConfig::default()).await
    }

    /// Create new GPU engine with custom configuration
    pub async fn with_config(config: UniversalGpuConfig) -> ToadStoolResult<Self> {
        Self::with_config_and_strategy(config, BackendSelectionStrategy::default()).await
    }

    /// Create new GPU engine with custom configuration and selection strategy
    pub async fn with_config_and_strategy(
        config: UniversalGpuConfig,
        selection_strategy: BackendSelectionStrategy,
    ) -> ToadStoolResult<Self> {
        let frameworks = Arc::new(RwLock::new(HashMap::new()));
        let devices = Arc::new(RwLock::new(HashMap::new()));
        let active_sessions = Arc::new(RwLock::new(HashMap::new()));
        let kernel_compiler = Arc::new(UniversalKernelCompiler::new(config.compilation.clone()));
        let resource_coordinator =
            Arc::new(ComputeResourceCoordinator::new(config.resources.clone()));
        let evolution_metrics = Arc::new(RwLock::new(EvolutionMetrics::default()));

        let engine = Self {
            frameworks,
            devices,
            active_sessions,
            _kernel_compiler: kernel_compiler,
            resource_coordinator,
            config,
            resource_monitor: None,
            selection_strategy,
            evolution_metrics,
        };

        // Log evolution status on startup
        engine.log_evolution_status().await;

        // Initialize frameworks and discover devices
        engine.discover_frameworks().await?;
        engine.discover_devices().await?;

        Ok(engine)
    }

    /// Discover and initialize available compute frameworks
    async fn discover_frameworks(&self) -> ToadStoolResult<()> {
        let mut frameworks = self.frameworks.write().await;

        for framework_type in &self.config.discovery.enabled_frameworks {
            match self.create_framework_instance(framework_type.clone()).await {
                Ok(framework) => {
                    frameworks.insert(framework_type.clone(), framework);
                    info!("Initialized framework: {}", framework_type.name());
                }
                Err(e) => {
                    if self.config.discovery.auto_fallback {
                        warn!(
                            "Failed to initialize framework {}: {}",
                            framework_type.name(),
                            e
                        );
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        if frameworks.is_empty() {
            return Err(ToadStoolError::runtime(
                "No compute frameworks could be initialized",
            ));
        }

        Ok(())
    }

    /// Create instance of specific framework
    async fn create_framework_instance(
        &self,
        framework_type: GpuFramework,
    ) -> ToadStoolResult<Arc<dyn ParallelComputeFramework>> {
        match framework_type {
            GpuFramework::WebGpu => {
                let framework = super::frameworks::WebGpuFramework::new()?;
                Ok(Arc::new(framework))
            }
            GpuFramework::Vulkan => {
                // Vulkan support requires additional platform-specific dependencies
                // Users should use WebGPU for cross-platform GPU compute
                Err(ToadStoolError::configuration(
                    "Vulkan framework requires manual enablement via 'vulkan' feature flag. \
                     Consider using WebGPU for cross-platform compatibility.",
                ))
            }
            GpuFramework::OpenCl => {
                // OpenCL support requires additional platform-specific dependencies
                // Users should use WebGPU for cross-platform GPU compute
                Err(ToadStoolError::configuration(
                    "OpenCL framework requires manual enablement via 'opencl' feature flag. \
                     Consider using WebGPU for cross-platform compatibility.",
                ))
            }
            _ => {
                // For other frameworks, use fallback implementation
                let framework = super::frameworks::FallbackFramework::new(framework_type);
                Ok(Arc::new(framework))
            }
        }
    }

    /// Discover available compute devices
    async fn discover_devices(&self) -> ToadStoolResult<()> {
        let frameworks = self.frameworks.read().await;
        let mut devices = self.devices.write().await;

        for framework in frameworks.values() {
            match framework.discover_devices().await {
                Ok(framework_devices) => {
                    for device in framework_devices {
                        devices.insert(device.id.clone(), device);
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to discover devices for framework {}: {}",
                        framework.framework_type().name(),
                        e
                    );
                }
            }
        }

        info!("Discovered {} compute devices", devices.len());
        Ok(())
    }

    /// Get list of available devices
    pub async fn get_available_devices(&self) -> Vec<UniversalComputeDevice> {
        self.devices.read().await.values().cloned().collect()
    }

    /// Get specific device by ID
    pub async fn get_device(&self, device_id: &DeviceId) -> Option<UniversalComputeDevice> {
        self.devices.read().await.get(device_id).cloned()
    }

    /// Execute compute workload
    pub async fn execute_workload(
        &self,
        workload: ComputeWorkload,
    ) -> ToadStoolResult<ComputeResult> {
        // Select optimal device
        let device_id = self.select_optimal_device(&workload.requirements).await?;

        // Create compute session
        let session_id = self
            .create_compute_session(&device_id, workload.parent_session)
            .await?;

        // Execute workload
        let result = self
            .execute_workload_on_device(session_id, &device_id, workload)
            .await;

        // Cleanup session
        if let Err(e) = self.destroy_compute_session(session_id).await {
            warn!("Failed to cleanup session {}: {}", session_id, e);
        }

        result
    }

    /// Select optimal device for workload
    async fn select_optimal_device(
        &self,
        requirements: &DeviceRequirements,
    ) -> ToadStoolResult<DeviceId> {
        let devices = self.devices.read().await;
        let available_devices: Vec<DeviceId> = devices.keys().cloned().collect();

        if available_devices.is_empty() {
            return Err(ToadStoolError::runtime("No devices available"));
        }

        // Use load balancer to select device
        let coordinator = Arc::clone(&self.resource_coordinator);
        coordinator
            .select_device(&available_devices, requirements)
            .await
    }

    /// Create compute session
    async fn create_compute_session(
        &self,
        device_id: &DeviceId,
        parent_session: Option<Uuid>,
    ) -> ToadStoolResult<Uuid> {
        let frameworks = self.frameworks.read().await;
        let device = self
            .devices
            .read()
            .await
            .get(device_id)
            .cloned()
            .ok_or_else(|| ToadStoolError::runtime("Device not found"))?;

        let framework = frameworks
            .get(&device.id.framework)
            .ok_or_else(|| ToadStoolError::runtime("Framework not available"))?;

        let session_id = framework.create_session(device_id).await?;

        // Calculate recursion depth
        let recursion_depth = if let Some(parent_id) = parent_session {
            let sessions = self.active_sessions.read().await;
            sessions
                .get(&parent_id)
                .map_or(0, |s| s.recursion_depth + 1)
        } else {
            0
        };

        // Check recursion limits
        if recursion_depth > self.config.recursion.max_recursion_depth {
            return Err(ToadStoolError::runtime("Maximum recursion depth exceeded"));
        }

        // Allocate resources
        let resource_allocation = self
            .resource_coordinator
            .allocate_resources(device_id, &DeviceRequirements::minimal())
            .await?;

        let session = ComputeSession {
            id: session_id,
            device_id: device_id.clone(),
            parent_session,
            child_sessions: Vec::new(),
            recursion_depth,
            start_time: Instant::now(),
            resource_allocation,
            status: SessionStatus::Initializing,
        };

        // Update parent session if this is recursive
        if let Some(parent_id) = parent_session {
            let mut sessions = self.active_sessions.write().await;
            if let Some(parent_session) = sessions.get_mut(&parent_id) {
                parent_session.child_sessions.push(session_id);
            }
        }

        self.active_sessions
            .write()
            .await
            .insert(session_id, session);
        Ok(session_id)
    }

    /// Execute workload on specific device
    async fn execute_workload_on_device(
        &self,
        session_id: Uuid,
        device_id: &DeviceId,
        workload: ComputeWorkload,
    ) -> ToadStoolResult<ComputeResult> {
        let start_time = Instant::now();

        // Update session status
        {
            let mut sessions = self.active_sessions.write().await;
            if let Some(session) = sessions.get_mut(&session_id) {
                session.status = SessionStatus::Running;
            }
        }

        let frameworks = self.frameworks.read().await;
        let device = self
            .devices
            .read()
            .await
            .get(device_id)
            .cloned()
            .ok_or_else(|| ToadStoolError::runtime("Device not found"))?;

        let framework = frameworks
            .get(&device.id.framework)
            .ok_or_else(|| ToadStoolError::runtime("Framework not available"))?;

        // Compile kernel
        let compiled_kernel = framework
            .compile_kernel(session_id, &workload.kernel_source, workload.kernel_format)
            .await?;

        // Execute kernel
        let primary_output = framework
            .execute_kernel(session_id, &compiled_kernel, &workload.inputs)
            .await?;

        // Execute recursive workloads
        let mut recursive_results = Vec::new();
        for recursive_workload in workload.recursive_workloads {
            let recursive_result = Box::pin(self.execute_workload(recursive_workload)).await?;
            recursive_results.push(recursive_result);
        }

        // Update session status
        {
            let mut sessions = self.active_sessions.write().await;
            if let Some(session) = sessions.get_mut(&session_id) {
                session.status = SessionStatus::Completed;
            }
        }

        Ok(ComputeResult {
            session_id,
            device_id: device_id.clone(),
            primary_output,
            recursive_results,
            total_execution_time: start_time.elapsed(),
        })
    }

    /// Destroy compute session
    async fn destroy_compute_session(&self, session_id: Uuid) -> ToadStoolResult<()> {
        let session = {
            let mut sessions = self.active_sessions.write().await;
            sessions.remove(&session_id)
        };

        if let Some(session) = session {
            let frameworks = self.frameworks.read().await;
            let device = self
                .devices
                .read()
                .await
                .get(&session.device_id)
                .cloned()
                .ok_or_else(|| ToadStoolError::runtime("Device not found"))?;

            let framework = frameworks
                .get(&device.id.framework)
                .ok_or_else(|| ToadStoolError::runtime("Framework not available"))?;

            // Destroy all child sessions first
            for child_session_id in &session.child_sessions {
                if let Err(e) = Box::pin(self.destroy_compute_session(*child_session_id)).await {
                    warn!(
                        "Failed to destroy child session {}: {}",
                        child_session_id, e
                    );
                }
            }

            // Destroy the session in the framework
            framework.destroy_session(session_id).await?;

            // Release resources
            self.resource_coordinator
                .release_resources(&session.device_id, &session.resource_allocation)
                .await?;
        }

        Ok(())
    }

    /// Set resource monitor
    #[must_use]
    pub fn with_resource_monitor(mut self, monitor: Arc<dyn ResourceMonitor>) -> Self {
        self.resource_monitor = Some(monitor);
        self
    }

    /// Get engine statistics
    pub async fn get_statistics(&self) -> ComputeEngineStatistics {
        let devices = self.devices.read().await;
        let sessions = self.active_sessions.read().await;
        let frameworks = self.frameworks.read().await;

        let recursive_sessions = sessions.values().filter(|s| s.recursion_depth > 0).count();
        let max_recursion_depth = sessions
            .values()
            .map(|s| s.recursion_depth)
            .max()
            .unwrap_or(0);

        ComputeEngineStatistics {
            total_devices: devices.len(),
            active_sessions: sessions.len(),
            frameworks_available: frameworks.len(),
            recursive_sessions,
            max_recursion_depth,
        }
    }
}

impl RuntimeEngine for UniversalGpuEngine {
    fn initialize(
        &mut self,
        _config: RuntimeConfig,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async {
            // Already initialized in constructor
            Ok(())
        })
    }

    fn execute(
        &self,
        request: ExecutionRequest,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_>> {
        Box::pin(async move {
            let workload = Self::convert_request_to_workload(&request)?;
            let result = self.execute_workload(workload).await?;

            Ok(ExecutionResponse {
                execution_id: request.execution_id,
                status: ExecutionStatus::Success,
                output: ExecutionOutput {
                    data: result
                        .primary_output
                        .buffers
                        .values()
                        .flatten()
                        .copied()
                        .collect(),
                    stdout: Some(format!(
                        "GPU execution completed on device: {:?}",
                        result.device_id
                    )),
                    stderr: if result.primary_output.errors.is_empty() {
                        None
                    } else {
                        Some(result.primary_output.errors.join("\n"))
                    },
                    exit_code: Some(0),
                    format: Some("gpu-compute".to_string()),
                    result: HashMap::new(),
                    metadata: HashMap::new(),
                },
                metrics: self.create_runtime_metrics(&result),
                duration: result.total_execution_time,
                runtime_used: RuntimeType::Gpu,
                warnings: if result.primary_output.errors.is_empty() {
                    vec![]
                } else {
                    result.primary_output.errors
                },
            })
        })
    }

    fn get_capabilities(&self) -> RuntimeCapabilities {
        let mut platform_features = HashMap::new();
        platform_features.insert("parallel_compute".to_string(), true);
        platform_features.insert("recursive_execution".to_string(), true);
        platform_features.insert("multi_framework".to_string(), true);
        platform_features.insert("universal_kernels".to_string(), true);
        platform_features.insert("auto_optimization".to_string(), true);

        RuntimeCapabilities {
            supported_workloads: vec![WorkloadType::Gpu],
            max_concurrent_executions: Some(64),
            supported_architectures: vec![
                "x86_64".to_string(),
                "aarch64".to_string(),
                "wasm32".to_string(),
            ],
            platform_features,
            version: "1.0.0".to_string(),
        }
    }

    fn supports_workload(&self, workload_type: &WorkloadType) -> bool {
        matches!(workload_type, WorkloadType::Gpu)
    }

    fn get_metrics(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<RuntimeMetrics>> + Send + '_>> {
        Box::pin(async {
            Ok(RuntimeMetrics {
                cpu: toadstool::resources::CpuMetrics {
                    usage_percent: 0.0, // GPU doesn't use CPU metrics
                    cores_used: 0.0,
                    cpu_time_seconds: 0.0,
                },
                memory: toadstool::resources::MemoryMetrics {
                    usage_percent: 0.0,
                    used_bytes: 0,
                    peak_bytes: 0,
                },
                storage: toadstool::resources::StorageMetrics {
                    usage_percent: 0.0,
                    used_bytes: 0,
                    bytes_read: 0,
                    bytes_written: 0,
                },
                network: toadstool::resources::NetworkMetrics {
                    bytes_sent: 0,
                    bytes_received: 0,
                    packets_sent: 0,
                    packets_received: 0,
                },
                gpu: Some(toadstool::resources::GpuMetrics {
                    usage_percent: 0.0,
                    memory_usage_percent: 0.0,
                    memory_used_bytes: 0,
                    temperature_celsius: None,
                }),
                timing: toadstool::resources::TimingMetrics {
                    start_time: SystemTime::now(),
                    end_time: Some(SystemTime::now()),
                    duration: Duration::ZERO,
                },
            })
        })
    }

    fn shutdown(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async {
            // Destroy all active sessions
            let session_ids: Vec<Uuid> = {
                let sessions = self.active_sessions.read().await;
                sessions.keys().copied().collect()
            };

            for session_id in session_ids {
                if let Err(e) = self.destroy_compute_session(session_id).await {
                    warn!(
                        "Failed to destroy session {} during shutdown: {}",
                        session_id, e
                    );
                }
            }

            info!("Universal GPU Engine shutdown complete");
            Ok(())
        })
    }
}

impl UniversalGpuEngine {
    /// Convert execution request to compute workload
    fn convert_request_to_workload(request: &ExecutionRequest) -> ToadStoolResult<ComputeWorkload> {
        let kernel_source = match &request.workload {
            WorkloadSpec::Gpu { program, .. } => {
                match program {
                    toadstool::workload::GpuProgramSource::OpenCL { source }
                    | toadstool::workload::GpuProgramSource::Cuda { source } => source.clone(),
                    toadstool::workload::GpuProgramSource::Vulkan { spirv } => {
                        // Convert SPIR-V bytes to string representation
                        format!("SPIR-V binary: {} bytes", spirv.len())
                    }
                }
            }
            _ => {
                return Err(ToadStoolError::runtime(
                    "Only GPU workloads are supported by GPU runtime",
                ));
            }
        };

        Ok(ComputeWorkload {
            name: request.execution_id.to_string(),
            kernel_source,
            kernel_format: KernelFormat::OpenClC, // Default, could be inferred from program type
            inputs: Vec::new(),                   // Would need to extract from request
            requirements: DeviceRequirements::minimal(),
            parent_session: None,
            recursive_workloads: Vec::new(),
            priority: 1,
        })
    }

    /// Create runtime metrics from compute result
    fn create_runtime_metrics(&self, result: &ComputeResult) -> RuntimeMetrics {
        RuntimeMetrics {
            cpu: toadstool::resources::CpuMetrics {
                usage_percent: 0.0,
                cores_used: 0.0,
                cpu_time_seconds: 0.0,
            },
            memory: toadstool::resources::MemoryMetrics {
                usage_percent: 0.0,
                used_bytes: result.primary_output.metrics.memory_used,
                peak_bytes: result.primary_output.metrics.memory_used,
            },
            storage: toadstool::resources::StorageMetrics {
                usage_percent: 0.0,
                used_bytes: 0,
                bytes_read: 0,
                bytes_written: 0,
            },
            network: toadstool::resources::NetworkMetrics {
                bytes_sent: 0,
                bytes_received: 0,
                packets_sent: 0,
                packets_received: 0,
            },
            gpu: Some(toadstool::resources::GpuMetrics {
                usage_percent: 0.0,
                memory_usage_percent: 0.0,
                memory_used_bytes: result.primary_output.metrics.memory_used,
                temperature_celsius: None,
            }),
            timing: toadstool::resources::TimingMetrics {
                start_time: SystemTime::now(),
                end_time: Some(SystemTime::now()),
                duration: result.total_execution_time,
            },
        }
    }

    /// Log current evolution status
    async fn log_evolution_status(&self) {
        let metrics = self.evolution_metrics.read().await;
        metrics.log_status();
    }

    /// Get evolution metrics
    pub async fn get_evolution_metrics(&self) -> EvolutionMetrics {
        self.evolution_metrics.read().await.clone()
    }

    /// Update evolution metrics (for future dynamic tracking)
    pub async fn update_evolution_metrics(&self, metrics: EvolutionMetrics) {
        *self.evolution_metrics.write().await = metrics;
        self.log_evolution_status().await;
    }

    /// Get backend selection strategy
    pub fn get_selection_strategy(&self) -> BackendSelectionStrategy {
        self.selection_strategy.clone()
    }

    /// Select best framework for a workload
    pub async fn select_framework_for_workload(
        &self,
        workload: Option<&WorkloadType>,
    ) -> Option<GpuFramework> {
        let frameworks = self.frameworks.read().await;
        let available: Vec<GpuFramework> = frameworks.keys().cloned().collect();

        self.selection_strategy
            .select_framework(workload, &available)
    }
}

impl Default for UniversalGpuEngine {
    fn default() -> Self {
        // Note: Default construction creates an uninitialized engine
        // Use UniversalGpuEngine::new() for proper async initialization
        Self {
            frameworks: Arc::new(RwLock::new(HashMap::new())),
            devices: Arc::new(RwLock::new(HashMap::new())),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            _kernel_compiler: Arc::new(UniversalKernelCompiler::new(CompilationConfig::default())),
            resource_coordinator: Arc::new(ComputeResourceCoordinator::new(
                ResourceConfig::default(),
            )),
            config: UniversalGpuConfig::default(),
            resource_monitor: None,
            selection_strategy: BackendSelectionStrategy::default(),
            evolution_metrics: Arc::new(RwLock::new(EvolutionMetrics::default())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;
    use std::pin::Pin;
    use toadstool::execution::RuntimeConfig;

    #[tokio::test]
    async fn test_engine_default() {
        let engine = UniversalGpuEngine::default();
        let devices = engine.get_available_devices().await;
        assert!(devices.is_empty());
    }

    #[tokio::test]
    async fn test_engine_get_device_none() {
        let engine = UniversalGpuEngine::default();
        let device_id = DeviceId::new(GpuFramework::WebGpu, 0, "test-uuid".to_string());
        assert!(engine.get_device(&device_id).await.is_none());
    }

    #[tokio::test]
    async fn test_engine_get_statistics_empty() {
        let engine = UniversalGpuEngine::default();
        let stats = engine.get_statistics().await;
        assert_eq!(stats.total_devices, 0);
        assert_eq!(stats.active_sessions, 0);
        assert_eq!(stats.frameworks_available, 0);
        assert_eq!(stats.recursive_sessions, 0);
        assert_eq!(stats.max_recursion_depth, 0);
    }

    #[test]
    fn test_engine_get_capabilities() {
        let engine = UniversalGpuEngine::default();
        let caps = engine.get_capabilities();
        assert!(caps.supported_workloads.contains(&WorkloadType::Gpu));
        assert_eq!(caps.max_concurrent_executions, Some(64));
        assert!(caps
            .platform_features
            .get("parallel_compute")
            .copied()
            .unwrap_or(false));
    }

    #[test]
    fn test_engine_supports_workload() {
        let engine = UniversalGpuEngine::default();
        assert!(engine.supports_workload(&WorkloadType::Gpu));
        assert!(!engine.supports_workload(&WorkloadType::Wasm));
    }

    #[tokio::test]
    async fn test_engine_get_evolution_metrics() {
        let engine = UniversalGpuEngine::default();
        let metrics = engine.get_evolution_metrics().await;
        assert!(metrics.webgpu_ai_coverage >= 0.0);
    }

    #[tokio::test]
    async fn test_engine_update_evolution_metrics() {
        let engine = UniversalGpuEngine::default();
        let mut metrics = engine.get_evolution_metrics().await;
        metrics.webgpu_ai_coverage = 0.5;
        engine.update_evolution_metrics(metrics.clone()).await;
        let updated = engine.get_evolution_metrics().await;
        assert!((updated.webgpu_ai_coverage - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_engine_get_selection_strategy() {
        let engine = UniversalGpuEngine::default();
        let _strategy = engine.get_selection_strategy();
    }

    #[tokio::test]
    async fn test_engine_select_framework_empty() {
        let engine = UniversalGpuEngine::default();
        let framework = engine
            .select_framework_for_workload(Some(&WorkloadType::Gpu))
            .await;
        assert!(framework.is_none());
    }

    #[tokio::test]
    async fn test_engine_initialize() {
        let mut engine = UniversalGpuEngine::default();
        let result = engine.initialize(RuntimeConfig::default()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_engine_get_metrics() {
        let engine = UniversalGpuEngine::default();
        let result = engine.get_metrics().await;
        assert!(result.is_ok());
        let metrics = result.unwrap();
        assert!(metrics.gpu.is_some());
    }

    #[tokio::test]
    async fn test_engine_execute_workload_no_devices() {
        let engine = UniversalGpuEngine::default();
        let workload = ComputeWorkload {
            name: "test".to_string(),
            kernel_source: "void kernel main() {}".to_string(),
            kernel_format: KernelFormat::OpenClC,
            inputs: vec![],
            requirements: DeviceRequirements::minimal(),
            parent_session: None,
            recursive_workloads: vec![],
            priority: 1,
        };
        let result = engine.execute_workload(workload).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_engine_shutdown_empty() {
        let mut engine = UniversalGpuEngine::default();
        let result = engine.shutdown().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_convert_request_to_workload_opencl() {
        use toadstool::workload::GpuProgramSource;
        let request = ExecutionRequest {
            execution_id: Uuid::new_v4(),
            workload: WorkloadSpec::Gpu {
                program: GpuProgramSource::OpenCL {
                    source: "void kernel main() {}".to_string(),
                },
                kernel_name: "main".to_string(),
                global_work_size: (1, 1, 1),
                work_group_size: Some((1, 1, 1)),
                args: vec![],
            },
            runtime_hint: None,
            resources: Default::default(),
            security_context: Default::default(),
            timeout: None,
            environment: Default::default(),
            input_data: Default::default(),
            callback_config: None,
            encryption_config: None,
        };
        let result = UniversalGpuEngine::convert_request_to_workload(&request);
        assert!(result.is_ok());
        let workload = result.unwrap();
        assert_eq!(workload.kernel_source, "void kernel main() {}");
    }

    #[test]
    fn test_convert_request_to_workload_cuda() {
        use toadstool::workload::GpuProgramSource;
        let request = ExecutionRequest {
            execution_id: Uuid::new_v4(),
            workload: WorkloadSpec::Gpu {
                program: GpuProgramSource::Cuda {
                    source: "__global__ void kernel() {}".to_string(),
                },
                kernel_name: "kernel".to_string(),
                global_work_size: (1, 1, 1),
                work_group_size: Some((1, 1, 1)),
                args: vec![],
            },
            runtime_hint: None,
            resources: Default::default(),
            security_context: Default::default(),
            timeout: None,
            environment: Default::default(),
            input_data: Default::default(),
            callback_config: None,
            encryption_config: None,
        };
        let result = UniversalGpuEngine::convert_request_to_workload(&request);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().kernel_source, "__global__ void kernel() {}");
    }

    #[test]
    fn test_convert_request_to_workload_vulkan_spirv() {
        use toadstool::workload::GpuProgramSource;
        let spirv_bytes = vec![0x03, 0x02, 0x23, 0x07, 0x00, 0x00, 0x01, 0x00];
        let request = ExecutionRequest {
            execution_id: Uuid::new_v4(),
            workload: WorkloadSpec::Gpu {
                program: GpuProgramSource::Vulkan {
                    spirv: spirv_bytes.clone(),
                },
                kernel_name: "main".to_string(),
                global_work_size: (1, 1, 1),
                work_group_size: Some((1, 1, 1)),
                args: vec![],
            },
            runtime_hint: None,
            resources: Default::default(),
            security_context: Default::default(),
            timeout: None,
            environment: Default::default(),
            input_data: Default::default(),
            callback_config: None,
            encryption_config: None,
        };
        let result = UniversalGpuEngine::convert_request_to_workload(&request);
        assert!(result.is_ok());
        let workload = result.unwrap();
        assert!(workload.kernel_source.contains("SPIR-V binary"));
        assert!(workload.kernel_source.contains("8 bytes"));
    }

    #[test]
    fn test_convert_request_to_workload_non_gpu_fails() {
        use toadstool::workload::ExecutableSource;
        let request = ExecutionRequest {
            execution_id: Uuid::new_v4(),
            workload: WorkloadSpec::Native {
                executable: ExecutableSource::File {
                    path: std::path::PathBuf::from("/bin/echo"),
                },
                args: Some(vec!["hello".to_string()]),
                working_dir: None,
                env_vars: Default::default(),
                user: None,
            },
            runtime_hint: None,
            resources: Default::default(),
            security_context: Default::default(),
            timeout: None,
            environment: Default::default(),
            input_data: Default::default(),
            callback_config: None,
            encryption_config: None,
        };
        let result = UniversalGpuEngine::convert_request_to_workload(&request);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_engine_execute_non_gpu_workload_fails() {
        use toadstool::workload::ExecutableSource;
        let engine = UniversalGpuEngine::default();
        let request = ExecutionRequest {
            execution_id: Uuid::new_v4(),
            workload: WorkloadSpec::Native {
                executable: ExecutableSource::File {
                    path: std::path::PathBuf::from("/bin/echo"),
                },
                args: Some(vec!["test".to_string()]),
                working_dir: None,
                env_vars: Default::default(),
                user: None,
            },
            runtime_hint: None,
            resources: Default::default(),
            security_context: Default::default(),
            timeout: None,
            environment: Default::default(),
            input_data: Default::default(),
            callback_config: None,
            encryption_config: None,
        };
        let result = engine.execute(request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_engine_select_framework_none_workload() {
        let engine = UniversalGpuEngine::default();
        let framework = engine.select_framework_for_workload(None).await;
        assert!(framework.is_none());
    }

    #[tokio::test]
    async fn test_engine_with_config_and_strategy() {
        use crate::strategy::BackendSelectionStrategy;
        let mut config = UniversalGpuConfig::default();
        config.discovery.enabled_frameworks = vec![GpuFramework::Metal];
        config.discovery.auto_fallback = true;
        let strategy = BackendSelectionStrategy::default();
        let result = UniversalGpuEngine::with_config_and_strategy(config, strategy).await;
        assert!(result.is_ok());
        let engine = result.unwrap();
        let devices = engine.get_available_devices().await;
        assert!(devices.is_empty(), "FallbackFramework returns no devices");
    }

    #[tokio::test]
    async fn test_engine_with_resource_monitor() {
        use toadstool::resources::{ResourceMonitor, RuntimeMetrics, SystemResources};
        struct MockMonitor;
        impl ResourceMonitor for MockMonitor {
            fn start_monitoring(&self, _workload_id: &str) -> toadstool::ToadStoolResult<()> {
                Ok(())
            }
            fn stop_monitoring(&self, _workload_id: &str) -> toadstool::ToadStoolResult<()> {
                Ok(())
            }
            fn get_metrics(
                &self,
                _workload_id: &str,
            ) -> Pin<Box<dyn Future<Output = toadstool::ToadStoolResult<RuntimeMetrics>> + Send + '_>>
            {
                Box::pin(async { Ok(Default::default()) })
            }
            fn get_system_resources(
                &self,
            ) -> Pin<
                Box<dyn Future<Output = toadstool::ToadStoolResult<SystemResources>> + Send + '_>,
            > {
                Box::pin(async { Ok(Default::default()) })
            }
        }
        let engine = UniversalGpuEngine::default().with_resource_monitor(Arc::new(MockMonitor));
        let stats = engine.get_statistics().await;
        assert_eq!(stats.total_devices, 0);
    }

    #[test]
    fn test_convert_request_to_workload_with_errors_in_output() {
        use toadstool::workload::GpuProgramSource;
        let request = ExecutionRequest {
            execution_id: Uuid::new_v4(),
            workload: WorkloadSpec::Gpu {
                program: GpuProgramSource::OpenCL {
                    source: "kernel void main() {}".to_string(),
                },
                kernel_name: "main".to_string(),
                global_work_size: (1, 1, 1),
                work_group_size: Some((1, 1, 1)),
                args: vec![],
            },
            runtime_hint: None,
            resources: Default::default(),
            security_context: Default::default(),
            timeout: None,
            environment: Default::default(),
            input_data: Default::default(),
            callback_config: None,
            encryption_config: None,
        };
        let workload = UniversalGpuEngine::convert_request_to_workload(&request).unwrap();
        assert!(workload.recursive_workloads.is_empty());
        assert_eq!(workload.priority, 1);
    }

    #[tokio::test]
    async fn test_engine_auto_fallback_on_framework_failure() {
        let mut config = UniversalGpuConfig::default();
        config.discovery.auto_fallback = true;
        config.discovery.enabled_frameworks = vec![
            GpuFramework::Vulkan,
            GpuFramework::OpenCl,
            GpuFramework::Metal,
        ];
        let result = UniversalGpuEngine::with_config(config).await;
        assert!(result.is_ok(), "Should fallback when Vulkan/OpenCL fail");
        let engine = result.unwrap();
        let frameworks = engine.get_statistics().await;
        assert!(frameworks.frameworks_available >= 1);
    }

    #[tokio::test]
    async fn test_engine_no_fallback_fails_on_first_error() {
        let mut config = UniversalGpuConfig::default();
        config.discovery.auto_fallback = false;
        config.discovery.enabled_frameworks = vec![GpuFramework::Vulkan];
        let result = UniversalGpuEngine::with_config(config).await;
        assert!(result.is_err());
    }
}
